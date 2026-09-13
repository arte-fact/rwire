//! Local model discovery for the LLM launcher.
//!
//! Scans a configurable models directory for files/dirs an engine can load:
//! `*.gguf` files (flambeau, llama.cpp) and Hugging Face model directories
//! (`config.json` + weights) for vLLM.

// Rust guideline compliant 2026-02-21

use std::path::Path;

use crate::modelmeta::{self, ModelMeta};

/// On-disk format of a discovered model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFormat {
    /// A single `.gguf` file (flambeau, llama.cpp).
    Gguf,
    /// A Hugging Face model directory (`config.json` + weights), for vLLM.
    HfDir,
}

/// A model found under the configured models directory.
#[derive(Debug, Clone)]
pub struct DiscoveredModel {
    /// Display name (file or directory name).
    pub name: String,
    /// Path passed to the engine.
    pub path: String,
    /// Detected format.
    pub format: ModelFormat,
    /// Parsed architecture metadata for VRAM estimation, if readable.
    pub meta: Option<ModelMeta>,
}

/// Scan `dir` for models: top-level `*.gguf` files and HF model directories
/// (a subdirectory containing `config.json`).
///
/// Returns an empty list if the directory is missing or unreadable, so a
/// misconfigured path degrades to "no models" rather than an error.
pub fn discover(dir: &Path) -> Vec<DiscoveredModel> {
    let mut models = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return models;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
        if is_dir {
            if path.join("config.json").is_file() {
                let p = path.to_string_lossy().into_owned();
                let meta = modelmeta::read(&p, false);
                models.push(DiscoveredModel {
                    name,
                    path: p,
                    format: ModelFormat::HfDir,
                    meta,
                });
            }
        } else if path.extension().is_some_and(|e| e == "gguf") && !is_nonfirst_shard(&name) {
            // For split GGUFs, only list the first shard; engines load the rest.
            let p = path.to_string_lossy().into_owned();
            let meta = modelmeta::read(&p, true);
            models.push(DiscoveredModel {
                name,
                path: p,
                format: ModelFormat::Gguf,
                meta,
            });
        }
    }
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

/// Whether `name` is a non-first shard of a split GGUF (`…-00002-of-00003.gguf`).
///
/// The first shard (`…-00001-of-…`) and single-file models return `false`.
fn is_nonfirst_shard(name: &str) -> bool {
    let Some(of_pos) = name.rfind("-of-") else {
        return false;
    };
    let before = &name[..of_pos];
    let Some(dash) = before.rfind('-') else {
        return false;
    };
    matches!(before[dash + 1..].parse::<u32>(), Ok(idx) if idx != 1)
}
