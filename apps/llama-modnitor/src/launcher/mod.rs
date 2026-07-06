//! Engine supervisor: launches and stops inference-server child processes.
//!
//! Handlers don't spawn processes directly. A `#[handler]` sets the node's phase
//! and sends a [`SupervisorMsg`] over a global channel; the supervisor thread
//! owns the child-process registry, builds the command for the chosen engine
//! ([`mod@engine`]), spawns it, polls `/health`, scrapes metrics ([`telemetry`]),
//! and writes the resulting [`NodePhase`] back via `update_shared_changed`.

// Rust guideline compliant 2026-02-21

use std::collections::HashMap;
use std::process::Child;
use std::sync::{Arc, OnceLock, mpsc};
use std::time::{Duration, Instant};

use rwire::{ChangeSet, SharedServerState};

use crate::snapshot::{App, EngineKind, NodePhase, NodeStats, OrphanProc};

mod engine;
mod telemetry;
pub use engine::LaunchSpec;
use engine::{health_ok, spawn};
use telemetry::{kill_pid, scan_orphans, scrape_stats, spawn_bench, spawn_chat};

/// Messages to the supervisor thread.
pub enum SupervisorMsg {
    Launch(LaunchSpec),
    /// Stop a node; the bool sets phase to `Stopped` (false = leave the phase
    /// alone, used when returning a node to configuration).
    Stop(u64, bool),
    /// Run a multi-turn test chat against a node, writing the reply back.
    Chat {
        id: u64,
        port: u16,
        model: String,
        messages: Vec<(String, String)>,
    },
    /// Run a pp/tg benchmark suite against a node.
    Bench {
        id: u64,
        port: u16,
    },
    /// Kill an arbitrary engine process by PID (orphan cleanup).
    KillPid(u32),
}

static TX: OnceLock<mpsc::Sender<SupervisorMsg>> = OnceLock::new();

/// Start the supervisor thread and register the global sender. Call once at boot.
pub fn init(shared: Arc<SharedServerState>) {
    let (tx, rx) = mpsc::channel();
    let _ = TX.set(tx);
    std::thread::spawn(move || run(&rx, &shared));
}

/// Request a node launch (used by the launch handler).
pub fn request_launch(spec: LaunchSpec) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(SupervisorMsg::Launch(spec));
    }
}

/// Request a node stop (used by the stop handler); sets phase to `Stopped`.
pub fn request_stop(id: u64) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(SupervisorMsg::Stop(id, true));
    }
}

/// Stop a node's process without changing its phase (used when returning a node
/// to configuration, so the handler-set `Configuring` phase isn't overwritten).
pub fn request_stop_silent(id: u64) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(SupervisorMsg::Stop(id, false));
    }
}

/// A supervised child process and its readiness/stats state.
pub struct ChildEntry {
    child: Child,
    port: u16,
    engine: EngineKind,
    ready: bool,
    /// Last (`prompt_total`, `generation_total`, time) sample for vLLM rate calc.
    prev: Option<(f64, f64, Instant)>,
}

/// How often the supervisor reconciles readiness + liveness when idle.
const RECONCILE_INTERVAL: Duration = Duration::from_secs(2);

/// Supervisor loop: owns the child registry, handles launch/stop, and on each
/// idle tick reconciles readiness (Starting → Running once `/health` is 200) and
/// liveness (a child that exited unexpectedly → Error).
fn run(rx: &mpsc::Receiver<SupervisorMsg>, shared: &Arc<SharedServerState>) {
    let mut children: HashMap<u64, ChildEntry> = HashMap::new();
    let mut prev_orphans: Vec<OrphanProc> = Vec::new();
    loop {
        match rx.recv_timeout(RECONCILE_INTERVAL) {
            Ok(SupervisorMsg::Launch(spec)) => {
                let id = spec.id;
                let port = spec.port;
                // Replace any existing child for this id (restart).
                kill_entry(children.remove(&id));
                let engine = spec.engine;
                match spawn(&spec) {
                    Ok(child) => {
                        children.insert(
                            id,
                            ChildEntry {
                                child,
                                port,
                                engine,
                                ready: false,
                                prev: None,
                            },
                        );
                    }
                    Err(e) => set_phase(shared, id, NodePhase::Error(e)),
                }
            }
            Ok(SupervisorMsg::Stop(id, set_stopped)) => {
                kill_entry(children.remove(&id));
                if set_stopped {
                    set_phase(shared, id, NodePhase::Stopped);
                }
            }
            Ok(SupervisorMsg::Chat {
                id,
                port,
                model,
                messages,
            }) => {
                spawn_chat(Arc::clone(shared), id, port, model, messages);
            }
            Ok(SupervisorMsg::Bench { id, port }) => {
                spawn_bench(Arc::clone(shared), id, port);
            }
            Ok(SupervisorMsg::KillPid(pid)) => {
                kill_pid(pid);
                // Drop any owned child with this pid so it's not double-tracked.
                children.retain(|_, e| e.child.id() != pid);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                reconcile(&mut children, shared);
                scan_orphans(&children, &mut prev_orphans, shared);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    // Channel closed (process shutting down): kill everything we own. (The OS
    // PR_SET_PDEATHSIG backstop also covers SIGKILL/crash of the monitor.)
    for (_, entry) in children.drain() {
        kill_entry(Some(entry));
    }
}

/// Path to a node's child log file (`NODE_LOG_DIR`, default the system temp dir).
pub fn node_log_path(id: u64) -> String {
    let dir = std::env::var("NODE_LOG_DIR")
        .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned());
    format!("{}/llm-node-{id}.log", dir.trim_end_matches('/'))
}

/// Read the tail of a node's child log (its stdout+stderr). `max` bounds the
/// number of trailing bytes. Returns an empty string if the log is unreadable.
pub fn log_tail(id: u64, max: usize) -> String {
    let path = node_log_path(id);
    let Ok(bytes) = std::fs::read(&path) else {
        return String::new();
    };
    let start = bytes.len().saturating_sub(max);
    String::from_utf8_lossy(&bytes[start..]).trim().to_string()
}

/// Short log tail (~2 KB) used in error messages.
fn read_log_tail(id: u64) -> String {
    log_tail(id, 2048)
}

/// Request a multi-turn test chat (handled off-thread by the supervisor). The
/// full conversation is sent; the reply replaces the node's pending turn.
pub fn request_chat(id: u64, port: u16, model: String, messages: Vec<(String, String)>) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(SupervisorMsg::Chat {
            id,
            port,
            model,
            messages,
        });
    }
}

/// Request a pp/tg benchmark run against a node (handled off-thread).
pub fn request_bench(id: u64, port: u16) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(SupervisorMsg::Bench { id, port });
    }
}

/// Request killing an orphan engine process by PID.
pub fn request_kill(pid: u32) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(SupervisorMsg::KillPid(pid));
    }
}

/// Reconcile readiness and liveness for all supervised children.
fn reconcile(children: &mut HashMap<u64, ChildEntry>, shared: &SharedServerState) {
    let mut dead = Vec::new();
    for (&id, entry) in children.iter_mut() {
        match entry.child.try_wait() {
            // Exited on its own — a user Stop removes the entry first, so this is
            // always an unexpected crash/exit. Surface the tail of the child's
            // log so the cause (bad flag, OOM, missing model…) is visible.
            Ok(Some(status)) => {
                let mut msg = format!("process exited ({status})");
                let tail = read_log_tail(id);
                if !tail.is_empty() {
                    msg.push_str("\n\n");
                    msg.push_str(&tail);
                }
                set_phase(shared, id, NodePhase::Error(msg));
                dead.push(id);
            }
            // Still alive: flip to Running once /health is 200, then scrape stats.
            Ok(None) => {
                if !entry.ready {
                    if health_ok(entry.port) {
                        entry.ready = true;
                        set_phase(shared, id, NodePhase::Running);
                    }
                } else if let Some(stats) = scrape_stats(entry.engine, entry.port, &mut entry.prev)
                {
                    set_stats(shared, id, stats);
                }
            }
            Err(_) => {}
        }
    }
    for id in dead {
        children.remove(&id);
    }
}

/// Kill and reap a child entry, if present.
fn kill_entry(entry: Option<ChildEntry>) {
    if let Some(mut entry) = entry {
        let _ = entry.child.kill();
        let _ = entry.child.wait();
    }
}

/// Write a node's phase back into shared state (only the `nodes` field changes).
pub fn set_phase(shared: &SharedServerState, id: u64, phase: NodePhase) {
    shared.update_shared_changed::<App>(ChangeSet::from_fields(&[App::FIELD_NODES]), |app| {
        if let Some(node) = app.nodes.iter_mut().find(|n| n.id == id) {
            node.phase = phase;
        }
    });
}

/// Write a node's live stats into shared state (only the `node_stats` field).
///
/// Scoped to `node_stats` so the ~2 s metrics poll never dirties `nodes`; the
/// config-form renderer depends on `nodes` (not `node_stats`) and is therefore
/// not rebuilt by the poll — keeping form inputs stable while typing.
pub fn set_stats(shared: &SharedServerState, id: u64, stats: NodeStats) {
    shared.update_shared_changed::<App>(ChangeSet::from_fields(&[App::FIELD_NODE_STATS]), |app| {
        app.node_stats.insert(id, stats);
    });
}
