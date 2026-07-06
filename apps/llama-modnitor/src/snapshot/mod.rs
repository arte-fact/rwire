//! Shared application state and hardware collection.
//!
//! [`App`] is the single `#[storage(shared)]` rwire state: one process-global
//! instance shared by every connection. The poller mutates `App::hw` via
//! `SharedServerState::update_shared`; UI handlers mutate `App::selection`. rwire
//! re-renders every client on any change. Keeping both in one state lets a single
//! `#[renderer]` read live hardware *and* the current selection together.

// Rust guideline compliant 2026-02-21

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use rwire::{ChangeSet, SharedServerState, State};

use crate::cpu::Cpu;
use crate::gpu::{GpuBackend, GpuMetrics, Vendor};
use crate::mem::{self, Memory};
use crate::models::{DiscoveredModel, ModelFormat};

mod catalog;
pub use catalog::{ValueSpec, flag_catalog, flag_spec};

/// Stable selection key for the host CPU card.
pub const CPU_KEY: &str = "cpu";

/// Display data for a single GPU card.
#[derive(Debug, Clone, Default)]
pub struct GpuCard {
    /// Card name, e.g. `"GPU0 NVIDIA GeForce RTX 4090"` or `"card0"`.
    pub name: String,
    /// Vendor family (drives engine compatibility + device-mask env var).
    pub vendor: Vendor,
    /// Device ordinal within its vendor (CUDA index / HIP ordinal).
    pub ordinal: u32,
    /// Latest metrics for this card.
    pub metrics: GpuMetrics,
}

impl GpuCard {
    /// Stable selection key, e.g. `"nvidia:0"` / `"amd:1"`.
    pub fn key(&self) -> String {
        format!("{}:{}", vendor_tag(self.vendor), self.ordinal)
    }

    /// Material class this card belongs to.
    pub const fn class(&self) -> MaterialClass {
        match self.vendor {
            Vendor::Nvidia => MaterialClass::Nvidia,
            Vendor::Amd => MaterialClass::Amd,
        }
    }
}

/// A point-in-time hardware reading: CPU, memory, and every detected GPU.
#[derive(Debug, Clone, Default)]
pub struct HardwareSnapshot {
    /// CPU model name.
    pub cpu_model: String,
    /// CPU package temperature in degrees Celsius, if exposed by hwmon.
    pub cpu_temp: Option<f32>,
    /// Aggregate CPU utilization percentage.
    pub cpu_usage: f32,
    /// Per-core utilization percentages.
    pub cpu_cores: Vec<f32>,
    /// System memory usage.
    pub mem: Memory,
    /// One entry per detected GPU.
    pub gpus: Vec<GpuCard>,
}

/// A selectable device, captured once at startup so the node form can offer a
/// stable device list without depending on the live hardware poll.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Selection key, e.g. `"amd:0"` / `"cpu"` (its class is derived from this).
    pub key: String,
    /// Short display label, e.g. `"card0"`.
    pub label: String,
}

/// One selectable hardware class. A node is backed by exactly one class
/// (each engine backend is single-vendor; GPU vs CPU are different backends).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialClass {
    /// NVIDIA GPUs.
    Nvidia,
    /// AMD GPUs.
    Amd,
    /// Host CPU.
    Cpu,
}

impl MaterialClass {
    /// Human-readable class name.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Nvidia => "NVIDIA",
            Self::Amd => "AMD",
            Self::Cpu => "CPU",
        }
    }
}

/// An inference engine that can back an LLM node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    /// flambeau (HIP/CUDA only).
    Flambeau,
    /// llama.cpp `llama-server`.
    LlamaCpp,
    /// vLLM (`vllm serve`).
    Vllm,
}

impl EngineKind {
    /// Display name.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Flambeau => "flambeau",
            Self::LlamaCpp => "llama.cpp",
            Self::Vllm => "vLLM",
        }
    }

    /// Stable tag used in `data-engine` attributes.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Flambeau => "flambeau",
            Self::LlamaCpp => "llamacpp",
            Self::Vllm => "vllm",
        }
    }

    /// Parse a [`EngineKind::tag`].
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "flambeau" => Some(Self::Flambeau),
            "llamacpp" => Some(Self::LlamaCpp),
            "vllm" => Some(Self::Vllm),
            _ => None,
        }
    }

    /// Whether this engine can load a model of the given on-disk format.
    ///
    /// flambeau and llama.cpp consume GGUF; vLLM consumes an HF model directory.
    pub fn accepts_format(self, format: ModelFormat) -> bool {
        match self {
            Self::Flambeau | Self::LlamaCpp => format == ModelFormat::Gguf,
            Self::Vllm => format == ModelFormat::HfDir,
        }
    }
}

/// Engines that can back a node of the given material class **on this host**.
///
/// Per the capability matrix (see `docs/llm-launcher-plan.md`): flambeau has no
/// CPU backend; vLLM's `ROCm` wheels do not support this host's gfx906 (MI50), so
/// AMD here is flambeau + llama.cpp only. Phase 3 will make this data-driven from
/// the executable opt-in registry.
pub const fn engines_for(class: MaterialClass) -> &'static [EngineKind] {
    match class {
        MaterialClass::Nvidia => &[EngineKind::Flambeau, EngineKind::LlamaCpp, EngineKind::Vllm],
        MaterialClass::Amd => &[EngineKind::Flambeau, EngineKind::LlamaCpp],
        MaterialClass::Cpu => &[EngineKind::LlamaCpp, EngineKind::Vllm],
    }
}

/// Lifecycle phase of an LLM node's server process.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NodePhase {
    /// Being configured (not launched).
    #[default]
    Configuring,
    /// Launch requested; process spawned, awaiting readiness.
    Starting,
    /// Server is up (passed `/health`).
    Running,
    /// Stopped by the user.
    Stopped,
    /// Failed to launch or exited unexpectedly.
    Error(String),
}

/// Live serving stats for a running node (filled by the supervisor's scraper).
///
/// Each field is `None` when the engine doesn't expose it: llama.cpp gives
/// throughput + request counts but no KV ratio; vLLM gives KV usage too; flambeau
/// has no `/metrics` yet.
#[derive(Debug, Clone, Default)]
pub struct NodeStats {
    /// Prompt-processing (prefill) rate, tokens/s.
    pub prefill_tps: Option<f32>,
    /// Generation (decode) rate, tokens/s.
    pub decode_tps: Option<f32>,
    /// KV-cache / context usage as a fraction 0..1.
    pub kv_usage: Option<f32>,
    /// Requests currently being processed.
    pub running: Option<u32>,
    /// Requests queued/deferred.
    pub waiting: Option<u32>,
}

/// One dynamic flag row in a node's config: a `(flag, value)` couple plus the
/// UI state for whether each cell is in "Custom…" (free text) mode.
#[derive(Debug, Clone, Default)]
pub struct FlagEntry {
    /// The CLI flag, e.g. `"--temp"` (empty until the row's flag is chosen).
    pub flag: String,
    /// The flag's value (empty for valueless flags or until typed).
    pub value: String,
    /// The flag was typed freely rather than picked from the catalog.
    pub custom_flag: bool,
    /// The value was typed freely rather than picked from the value dropdown.
    pub raw_value: bool,
}

/// An engine process running on the host that the UI doesn't manage (e.g. left
/// over from a deleted node or a previous monitor instance).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrphanProc {
    /// OS process id.
    pub pid: u32,
    /// Short description: `exe :port model`.
    pub label: String,
}

/// One turn in a node's test-chat conversation.
#[derive(Debug, Clone)]
pub struct ChatTurn {
    /// `"user"` or `"assistant"`.
    pub role: String,
    /// Message text (`"…"` while an assistant reply is pending).
    pub content: String,
}

/// Transient UI-view toggles for a node's optional detail blocks.
#[derive(Debug, Clone, Default)]
pub struct NodeView {
    /// Whether the (optional) server-log block is shown.
    pub logs: bool,
    /// Whether the (optional) test-chat block is shown.
    pub chat: bool,
    /// Whether the (optional) benchmark block is shown.
    pub bench: bool,
}

/// One LLM node: selected material + engine + common config + lifecycle.
#[derive(Debug, Clone)]
pub struct LlmNode {
    /// Stable id (monotonic).
    pub id: u64,
    /// Material class backing this node.
    pub material: MaterialClass,
    /// Device keys captured from the selection at creation.
    pub devices: Vec<String>,
    /// Chosen engine, or `None` until picked.
    pub engine: Option<EngineKind>,
    /// Model path or HF repo id (common config).
    pub model: String,
    /// Context length as typed (blank = engine default).
    pub ctx: String,
    /// Configured HTTP port as typed (blank = auto: 8090 + id).
    pub port_cfg: String,
    /// Auto-fit context to the VRAM target instead of using `ctx`.
    pub autofit: bool,
    /// Target VRAM usage percent (of selected-device capacity) for the bar +
    /// autofit (default 90).
    pub vram_target: u8,
    /// Total VRAM/RAM (MiB) of the selected devices, captured at creation.
    pub device_vram_mb: u64,
    /// Currently-used VRAM/RAM (MiB) on the selected devices; a snapshot
    /// refreshed on each node action (launch/stop/add/delete).
    pub device_used_mb: u64,
    /// Dynamic `(flag, value)` rows appended to the launch command.
    pub flags: Vec<FlagEntry>,
    /// Profile name typed in the "Save as…" field (transient UI state).
    pub profile_name: String,
    /// Lifecycle phase.
    pub phase: NodePhase,
    /// Allocated HTTP port (0 until launched).
    pub port: u16,
    /// Transient UI-view toggles for the optional detail blocks.
    pub view: NodeView,
    /// Last-fetched server-log tail.
    pub log_tail: String,
    /// Test-chat prompt input.
    pub chat_input: String,
    /// Multi-turn test-chat conversation.
    pub chat_log: Vec<ChatTurn>,
    /// Benchmark results as `(label, "N t/s")` pairs.
    pub bench_results: Vec<(String, String)>,
    /// Whether a benchmark run is in progress.
    pub bench_running: bool,
    /// A pending action awaiting confirmation (`"stop"` / `"configure"`).
    pub pending: Option<String>,
}

impl LlmNode {
    /// Device ordinals parsed from the selection keys (`"amd:0"` → `0`).
    pub fn device_ordinals(&self) -> Vec<u32> {
        self.devices
            .iter()
            .filter_map(|k| k.split(':').nth(1)?.parse().ok())
            .collect()
    }
}

/// The user's current hardware selection; drives node creation.
///
/// Invariant: every key belongs to the same [`MaterialClass`] (enforced by
/// [`Selection::toggle`]). An empty selection has no locked class.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    /// Selected device keys (`"nvidia:0"`, `"amd:1"`, or [`CPU_KEY`]).
    pub keys: Vec<String>,
}

impl Selection {
    /// Whether `key` is currently selected.
    pub fn contains(&self, key: &str) -> bool {
        self.keys.iter().any(|k| k == key)
    }

    /// The class all current selections belong to, or `None` when empty.
    pub fn locked_class(&self) -> Option<MaterialClass> {
        self.keys.first().map(|k| class_of_key(k))
    }

    /// Toggle a key, enforcing the single-class invariant.
    ///
    /// Removing a key is always allowed. Adding a key is ignored when another
    /// class is already locked (the corresponding card is also disabled in the
    /// UI, so this is a defensive backstop).
    pub fn toggle(&mut self, key: &str) {
        if let Some(pos) = self.keys.iter().position(|k| k == key) {
            self.keys.remove(pos);
            return;
        }
        match self.locked_class() {
            Some(locked) if locked != class_of_key(key) => {}
            _ => self.keys.push(key.to_string()),
        }
    }
}

/// Process-global application state shared by every connection.
#[derive(Debug, Default, State)]
#[storage(shared)]
pub struct App {
    /// Latest hardware reading (owned by the poller).
    pub hw: HardwareSnapshot,
    /// Current hardware selection (owned by UI handlers).
    pub selection: Selection,
    /// Created LLM nodes (owned by UI handlers).
    pub nodes: Vec<LlmNode>,
    /// Next node id to assign.
    pub next_node_id: u64,
    /// Models discovered under the configured models directory (set at startup).
    pub models: Vec<DiscoveredModel>,
    /// Names of saved node profiles (refreshed on save; loaded at startup).
    pub profiles: Vec<String>,
    /// Selectable GPU devices, captured once at startup (stable list for the
    /// node form's device editor).
    pub devices: Vec<DeviceInfo>,
    /// Engine processes detected on the host that no UI node manages.
    pub orphans: Vec<OrphanProc>,
    /// Live serving stats per node id, written by the metrics scraper.
    ///
    /// Kept in its own field (not on [`LlmNode`]) so the ~2 s stats poll only
    /// dirties `node_stats`; renderers that read node *config* (the form) don't
    /// depend on it and so are never rebuilt by the poll.
    pub node_stats: HashMap<u64, NodeStats>,
}

/// Short vendor tag used in selection keys.
const fn vendor_tag(vendor: Vendor) -> &'static str {
    match vendor {
        Vendor::Nvidia => "nvidia",
        Vendor::Amd => "amd",
    }
}

/// Material class of a selection key (`"cpu"` → CPU, `"nvidia:.."` → Nvidia, …).
pub fn class_of_key(key: &str) -> MaterialClass {
    match key.split(':').next() {
        Some("nvidia") => MaterialClass::Nvidia,
        Some("amd") => MaterialClass::Amd,
        _ => MaterialClass::Cpu,
    }
}

/// Derive a GPU's vendor and device ordinal from the backend-assigned name.
///
/// Names come from this crate's own backends: NVIDIA cards are `"GPU{n} …"`
/// (`nvidia.rs`) and AMD cards contain `"card{n}"` (`rocm.rs`).
fn classify(name: &str) -> (Vendor, u32) {
    if let Some(rest) = name.strip_prefix("GPU") {
        return (Vendor::Nvidia, leading_u32(rest));
    }
    if let Some(idx) = name.find("card") {
        return (Vendor::Amd, leading_u32(&name[idx + "card".len()..]));
    }
    (Vendor::Amd, 0)
}

/// Parse the leading run of ASCII digits in `s` as a `u32` (0 when absent).
fn leading_u32(s: &str) -> u32 {
    s.chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

/// Read one fresh hardware snapshot from the CPU reader and GPU backend.
pub fn collect(cpu: &mut Cpu, backend: &dyn GpuBackend) -> HardwareSnapshot {
    let (cpu_usage, cpu_cores) = cpu.sample();
    let gpus = backend
        .read_metrics()
        .unwrap_or_default()
        .into_iter()
        .map(|(name, metrics)| {
            let (vendor, ordinal) = classify(&name);
            GpuCard {
                name,
                vendor,
                ordinal,
                metrics,
            }
        })
        .collect();
    HardwareSnapshot {
        cpu_model: cpu.model().to_string(),
        cpu_temp: crate::cpu::read_cpu_temp(),
        cpu_usage,
        cpu_cores,
        mem: mem::read(),
        gpus,
    }
}

/// Poll hardware on a blocking thread, publishing each snapshot into [`App::hw`].
///
/// `update_shared` re-renders every connection; mutating only `hw` preserves the
/// user's selection across ticks. Reuses `cpu` so utilization deltas span exactly
/// `interval`. Runs until the process exits.
pub fn run_poller(
    cpu: &mut Cpu,
    backend: &dyn GpuBackend,
    shared: &Arc<SharedServerState>,
    interval: Duration,
) {
    loop {
        std::thread::sleep(interval);
        let next = collect(cpu, backend);
        // Broadcast only the `hw` field so the node column (which depends on
        // `selection`/`nodes`, not `hw`) is not re-rendered each tick — keeping
        // its inputs stable. See `docs/llm-launcher-plan.md` Phase 2.
        shared.update_shared_changed::<App>(ChangeSet::from_fields(&[App::FIELD_HW]), |app| {
            app.hw = next.clone();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_reads_vendor_and_ordinal() {
        assert_eq!(
            classify("GPU0 NVIDIA GeForce RTX 3090"),
            (Vendor::Nvidia, 0)
        );
        assert_eq!(classify("GPU1 NVIDIA"), (Vendor::Nvidia, 1));
        assert_eq!(classify("AMD Instinct MI50/MI60 (card0)"), (Vendor::Amd, 0));
        assert_eq!(classify("card2"), (Vendor::Amd, 2));
    }

    #[test]
    fn flag_catalog_has_unsloth_sampling_flags() {
        // The llama.cpp catalog must surface the key unsloth-recommended flags
        // with their suggested values, and Context must NOT be in the catalog
        // (it has a dedicated field).
        let temp = flag_spec(EngineKind::LlamaCpp, "--temp").expect("--temp present");
        match temp.value {
            ValueSpec::Choice(opts) => assert!(opts.contains(&"1.0")),
            ValueSpec::None => panic!("--temp should offer suggested values"),
        }
        // Current-llama.cpp flags are present (reasoning toggle, MoE→CPU offload).
        assert!(flag_spec(EngineKind::LlamaCpp, "--reasoning").is_some());
        assert!(flag_spec(EngineKind::LlamaCpp, "--n-cpu-moe").is_some());
        assert!(matches!(
            flag_spec(EngineKind::LlamaCpp, "--jinja").map(|s| s.value),
            Some(ValueSpec::None)
        ));
        assert!(flag_spec(EngineKind::LlamaCpp, "--ctx-size").is_none());
        // vLLM does not get llama.cpp's sampling flags.
        assert!(flag_spec(EngineKind::Vllm, "--temp").is_none());
    }

    #[test]
    fn selection_enforces_single_class() {
        let mut sel = Selection::default();
        sel.toggle("nvidia:0");
        sel.toggle("nvidia:1");
        assert_eq!(sel.keys.len(), 2);
        assert_eq!(sel.locked_class(), Some(MaterialClass::Nvidia));

        // Cross-class add is ignored while a class is locked.
        sel.toggle("amd:0");
        sel.toggle(CPU_KEY);
        assert_eq!(sel.keys.len(), 2);

        // Deselecting frees the lock.
        sel.toggle("nvidia:0");
        sel.toggle("nvidia:1");
        assert_eq!(sel.locked_class(), None);
        sel.toggle(CPU_KEY);
        assert_eq!(sel.locked_class(), Some(MaterialClass::Cpu));
    }
}
