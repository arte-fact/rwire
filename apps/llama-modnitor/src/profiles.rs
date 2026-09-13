//! Save, load, and list reusable node configuration profiles on disk.
//!
//! A profile is a JSON snapshot of one node's configuration (material, devices,
//! engine, model, context, and flag rows) written under a configurable directory
//! (`NODE_PROFILES_DIR`, default `./profiles`). Profiles let a user reuse a
//! tuned configuration across restarts; in-session reuse is handled by node
//! duplication in the UI. See `docs/llm-launcher-plan.md` Phase 5.

// Rust guideline compliant 2026-02-21

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::snapshot::{EngineKind, FlagEntry, LlmNode, MaterialClass};

/// A persisted node configuration. Only the configurable fields are stored;
/// runtime fields (phase, port, stats) are not.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeProfile {
    /// Profile name (also the file stem).
    pub name: String,
    /// Material class tag (`"nvidia"` / `"amd"` / `"cpu"`).
    pub material: String,
    /// Device keys captured at save time (host-specific; re-checked on load).
    pub devices: Vec<String>,
    /// Engine tag (`"flambeau"` / `"llamacpp"` / `"vllm"`), if chosen.
    pub engine: Option<String>,
    /// Model path or HF repo id.
    pub model: String,
    /// Context length as typed.
    pub ctx: String,
    /// Configured port as typed (blank = auto).
    #[serde(default)]
    pub port: String,
    /// Flag rows as `(flag, value)` pairs.
    pub flags: Vec<(String, String)>,
}

impl NodeProfile {
    /// Build a profile from a node's current configuration.
    pub fn from_node(name: &str, node: &LlmNode) -> Self {
        Self {
            name: name.to_string(),
            material: material_tag(node.material).to_string(),
            devices: node.devices.clone(),
            engine: node.engine.map(|e| e.tag().to_string()),
            model: node.model.clone(),
            ctx: node.ctx.clone(),
            port: node.port_cfg.clone(),
            flags: node
                .flags
                .iter()
                .filter(|f| !f.flag.trim().is_empty())
                .map(|f| (f.flag.clone(), f.value.clone()))
                .collect(),
        }
    }

    /// Materialize this profile into a fresh `Configuring` node with the given id.
    pub fn to_node(&self, id: u64) -> LlmNode {
        let mut flags: Vec<FlagEntry> = self
            .flags
            .iter()
            .map(|(flag, value)| FlagEntry {
                flag: flag.clone(),
                value: value.clone(),
                // Restore UI mode from the data: a flag/value not in the catalog
                // renders as a free-text ("Custom…") cell. Computed lazily by the
                // UI from the catalog, so default both to false here.
                custom_flag: false,
                raw_value: false,
            })
            .collect();
        // Always keep a trailing empty "new row" so the UI's + Add is present.
        flags.push(FlagEntry::default());
        LlmNode {
            id,
            material: material_from_tag(&self.material),
            devices: self.devices.clone(),
            engine: self.engine.as_deref().and_then(EngineKind::from_tag),
            model: self.model.clone(),
            ctx: self.ctx.clone(),
            port_cfg: self.port.clone(),
            autofit: false,
            vram_target: 90,
            // Recomputed from live hardware by the load handler.
            device_vram_mb: 0,
            device_used_mb: 0,
            flags,
            profile_name: String::new(),
            phase: crate::snapshot::NodePhase::Configuring,
            port: 0,
            view: crate::snapshot::NodeView::default(),
            log_tail: String::new(),
            chat_input: String::new(),
            chat_log: Vec::new(),
            bench_results: Vec::new(),
            bench_running: false,
            pending: None,
        }
    }
}

/// Directory profiles are stored in (`NODE_PROFILES_DIR`, default `./profiles`).
fn profiles_dir() -> PathBuf {
    std::env::var("NODE_PROFILES_DIR")
        .unwrap_or_else(|_| "./profiles".to_string())
        .into()
}

/// Sanitize a profile name into a safe file stem (alphanumerics, `-`, `_`).
fn safe_stem(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = stem.trim_matches('-');
    if trimmed.is_empty() {
        "profile".to_string()
    } else {
        trimmed.to_string()
    }
}

/// List saved profile names (file stems), sorted. Missing dir → empty.
pub fn list() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(profiles_dir()).map_or_else(
        |_| Vec::new(),
        |rd| {
            rd.filter_map(Result::ok)
                .filter_map(|e| {
                    let path = e.path();
                    (path.extension()?.to_str()? == "json")
                        .then(|| path.file_stem()?.to_str().map(str::to_string))
                        .flatten()
                })
                .collect()
        },
    );
    names.sort();
    names
}

/// Save a profile as `<dir>/<safe_stem>.json`, creating the dir if needed.
pub fn save(profile: &NodeProfile) -> std::io::Result<()> {
    let dir = profiles_dir();
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", safe_stem(&profile.name)));
    let json = serde_json::to_string_pretty(profile)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(path, json)
}

/// Load a profile by name (its file stem). Returns `None` if missing/invalid.
pub fn load(name: &str) -> Option<NodeProfile> {
    let path = profiles_dir().join(format!("{}.json", safe_stem(name)));
    let json = fs::read_to_string(path).ok()?;
    serde_json::from_str(&json).ok()
}

/// Stable tag for a material class (matches the selection-key vendor tags).
const fn material_tag(class: MaterialClass) -> &'static str {
    match class {
        MaterialClass::Nvidia => "nvidia",
        MaterialClass::Amd => "amd",
        MaterialClass::Cpu => "cpu",
    }
}

/// Parse a [`material_tag`] back into a class (`"cpu"`/unknown → CPU).
fn material_from_tag(tag: &str) -> MaterialClass {
    match tag {
        "nvidia" => MaterialClass::Nvidia,
        "amd" => MaterialClass::Amd,
        _ => MaterialClass::Cpu,
    }
}
