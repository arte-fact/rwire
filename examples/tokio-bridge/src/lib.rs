//! Hosting rwire inside a tokio application, without changing rwire's runtime.
//!
//! rwire runs on async-std. A tokio application (an axum service, an SSE client…)
//! hosts it by giving it **its own OS thread** and talking to it through
//! runtime-agnostic channels:
//!
//! ```text
//!  tokio runtime                          rwire thread (async-std)
//!  ─────────────                          ────────────────────────
//!  snapshot producer ── mpsc<Snapshot> ──▶ apply_snapshots
//!                                          └─ update_shared_if::<Hub> (broadcasts changes only)
//!  Handle::spawn(work) ◀── ask_tokio handler (ctx.defer(), returns at once)
//!   └─ update_shared_changed::<Hub> + resolver.ok() ──▶ every connection re-renders
//! ```
//!
//! - **tokio → rwire.** A tokio task sends [`Snapshot`]s into a `tokio::sync::mpsc`
//!   channel. `tokio::sync` channels need no tokio reactor, so the receiving end is
//!   awaited by an async-std task on the rwire thread ([`apply_snapshots`]), which
//!   writes the shared [`Hub`] state with `update_shared_if`: an unchanged snapshot
//!   returns `None` and nothing is broadcast.
//! - **rwire → tokio.** The [`ask_tokio`] handler runs on the rwire thread. It must not
//!   block it, and `tokio::spawn` would panic there (no tokio runtime on that thread), so
//!   it spawns onto the tokio [`Handle`] captured at startup. The tokio task writes its
//!   result back into `Hub` (a plain synchronous call, safe from any thread) and resolves
//!   the deferred action.
//! - **Shutdown.** [`RwireThread::shutdown`] stops the accept loop and joins the thread;
//!   dropping the snapshot sender ends [`apply_snapshots`]. Limitation: rwire's `run()`
//!   has no graceful shutdown, so WebSocket connections already open keep running on
//!   async-std's global executor until the process exits.

use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::Duration;

use rwire::{
    el, handler, renderer, ChangeSet, El, ElementBuilder, Ev, Server, SharedServerState, State,
};
use tokio::runtime::Handle;
use tokio::sync::{mpsc, oneshot};

/// What the tokio side knows and the page shows. One instance for every connection.
#[derive(State, Default)]
#[storage(shared)]
pub struct Hub {
    /// Sequence number of the last applied snapshot.
    pub seq: u64,
    /// Content of the last applied snapshot.
    pub summary: String,
    /// How many `ask_tokio` actions tokio has completed.
    pub answers: u32,
    /// The last answer computed on the tokio runtime.
    pub answer: String,
}

/// One snapshot of upstream state, as an SSE client would decode it.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub seq: u64,
    pub summary: String,
}

/// What a handler needs to reach the tokio side. rwire handlers are plain functions,
/// so it lives in a process-wide slot, set once by [`start`].
struct Bridge {
    tokio: Handle,
    shared: Arc<SharedServerState>,
}

static BRIDGE: OnceLock<Bridge> = OnceLock::new();

/// Text the tokio task writes into [`Hub::answer`].
pub const ANSWER_PREFIX: &str = "answered on tokio";

/// Ask the tokio runtime to do some work; the button spins until it answers.
#[handler]
pub fn ask_tokio(_hub: &mut Hub, ctx: &EventContext) {
    let Some(bridge) = BRIDGE.get() else { return };
    let resolver = ctx.defer();
    let shared = Arc::clone(&bridge.shared);
    // `Handle::spawn` returns at once, from any thread: the rwire loop is never blocked.
    bridge.tokio.spawn(async move {
        // tokio-only work: `tokio::time::sleep` panics outside a tokio runtime.
        tokio::time::sleep(Duration::from_millis(50)).await;
        let worker = thread::current().name().unwrap_or("unnamed").to_string();
        shared.update_shared_changed::<Hub>(
            ChangeSet::from_fields(&[Hub::FIELD_ANSWERS, Hub::FIELD_ANSWER]),
            |hub| {
                hub.answers += 1;
                hub.answer = format!("{ANSWER_PREFIX} ({worker})");
            },
        );
        resolver.ok();
    });
}

/// Apply snapshots coming from tokio, on the rwire thread. Ends when every sender
/// is dropped.
pub async fn apply_snapshots(shared: Arc<SharedServerState>, mut rx: mpsc::Receiver<Snapshot>) {
    while let Some(snapshot) = rx.recv().await {
        shared.update_shared_if::<Hub>(|hub| {
            if hub.summary == snapshot.summary {
                // Same content: no broadcast, no re-render.
                return None;
            }
            hub.seq = snapshot.seq;
            hub.summary = snapshot.summary;
            Some(ChangeSet::from_fields(&[
                Hub::FIELD_SEQ,
                Hub::FIELD_SUMMARY,
            ]))
        });
    }
}

/// Stand-in for the hub's SSE stream: a snapshot every `period`, whose content only
/// changes every other tick (so half of them are deduplicated by `apply_snapshots`).
/// Ends when the rwire side is gone.
pub async fn simulate_hub(tx: mpsc::Sender<Snapshot>, period: Duration) {
    let mut ticker = tokio::time::interval(period);
    for seq in 1.. {
        ticker.tick().await;
        let snapshot = Snapshot {
            seq,
            summary: format!("{} agents online", seq / 2),
        };
        if tx.send(snapshot).await.is_err() {
            break;
        }
    }
}

#[renderer]
fn hub_view(hub: &Hub) -> ElementBuilder {
    el(El::Section).append([
        el(El::P).text(&format!("snapshot #{}: {}", hub.seq, hub.summary)),
        el(El::P).text(&format!("tokio answers: {} {}", hub.answers, hub.answer)),
    ])
}

/// The page: the shared hub view and a button that runs work on tokio.
pub fn app() -> ElementBuilder {
    el(El::Main).append([
        el(El::H1).text("rwire inside a tokio application"),
        hub_view(),
        el(El::Button).text("Ask tokio").on(Ev::Click, ask_tokio()),
    ])
}

/// The rwire server thread, as seen from tokio.
pub struct RwireThread {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<thread::JoinHandle<Result<(), String>>>,
}

impl RwireThread {
    /// Stop accepting connections and wait for the rwire thread to end.
    ///
    /// Blocks while the thread finishes: from async code, call it inside
    /// `tokio::task::spawn_blocking`. Returns `run()`'s error if the server had
    /// already failed (e.g. the address was taken).
    pub fn shutdown(mut self) -> Result<(), String> {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        match self.thread.take().map(thread::JoinHandle::join) {
            Some(Ok(result)) => result,
            Some(Err(_)) => Err("the rwire thread panicked".to_string()),
            None => Ok(()),
        }
    }
}

/// Start rwire on its own thread, bound to `addr`.
///
/// `tokio` is the runtime that handlers hand their work to (`Handle::current()` from
/// inside it). Returns the thread and the sender tokio pushes snapshots into. One
/// bridge per process: a second call fails.
pub fn start(addr: &str, tokio: Handle) -> Result<(RwireThread, mpsc::Sender<Snapshot>), String> {
    let mut server = Server::bind(addr).map_err(|e| e.to_string())?.root(app);
    let shared = server.shared_state();
    BRIDGE
        .set(Bridge {
            tokio,
            shared: Arc::clone(&shared),
        })
        .map_err(|_| "the tokio bridge is already started in this process".to_string())?;

    let (snapshots_tx, snapshots_rx) = mpsc::channel(16);
    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    let thread = thread::Builder::new()
        .name("rwire".to_string())
        .spawn(move || {
            // Never `block_on` from a tokio worker: this is a plain OS thread.
            async_std::task::block_on(async move {
                async_std::task::spawn(apply_snapshots(shared, snapshots_rx));
                let run = async { server.run().await.map_err(|e| e.to_string()) };
                let stop = async {
                    // A dropped sender (RwireThread dropped without shutdown) stops too.
                    let _ = stop_rx.await;
                    Ok(())
                };
                futures::pin_mut!(run, stop);
                futures::future::select(run, stop).await.factor_first().0
            })
        })
        .map_err(|e| e.to_string())?;

    Ok((
        RwireThread {
            stop: Some(stop_tx),
            thread: Some(thread),
        },
        snapshots_tx,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `apply_snapshots` broadcasts a snapshot only when its content changed.
    #[test]
    fn identical_snapshots_are_not_broadcast() {
        let shared = SharedServerState::new(Duration::from_secs(60));
        // A fake connection subscribed to the shared Hub, as a real one would be.
        let (conn_tx, conn_rx) = async_channel::unbounded();
        shared.register_connection(1, conn_tx);
        shared.subscribe(1, &format!("__shared__:{}", Hub::TABLE_NAME));

        let (tx, rx) = mpsc::channel(8);
        for (seq, summary) in [(1, "1 agent"), (2, "1 agent"), (3, "2 agents")] {
            tx.try_send(Snapshot {
                seq,
                summary: summary.to_string(),
            })
            .unwrap();
        }
        drop(tx);
        async_std::task::block_on(apply_snapshots(Arc::clone(&shared), rx));

        assert_eq!(conn_rx.len(), 2, "the repeated snapshot must not broadcast");
        shared.update_shared_if::<Hub>(|hub| {
            assert_eq!((hub.seq, hub.summary.as_str()), (3, "2 agents"));
            None
        });
    }
}
