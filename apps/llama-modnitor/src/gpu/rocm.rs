//! AMD GPU backend driven by `rocm-smi --json`.

use anyhow::Result;
use std::collections::BTreeMap;
use std::process::Command;

use super::{GpuBackend, GpuMetrics};
use crate::convert;

/// Reads AMD metrics via `rocm-smi` JSON output.
pub struct RocmBackend {
    /// Resolved path to the `rocm-smi` executable.
    smi: String,
}

impl RocmBackend {
    /// Create a backend that invokes `rocm-smi` at the given path.
    pub const fn new(smi: String) -> Self {
        Self { smi }
    }
}

/// Locate the `rocm-smi` executable.
///
/// Prefers `PATH`, then `/opt/rocm/bin`, then any versioned `/opt/rocm-*/bin`.
/// `ROCm` installs commonly leave `rocm-smi` out of `PATH` (only `rocminfo`),
/// so the well-known locations are checked as a fallback. Returns `None` when
/// no candidate exists.
pub fn find_rocm_smi() -> Option<String> {
    // Explicit override wins.
    if let Ok(explicit) = std::env::var("ROCM_SMI")
        && !explicit.is_empty()
    {
        return Some(explicit);
    }
    if super::command_exists("rocm-smi") {
        return Some("rocm-smi".to_string());
    }

    let mut candidates = Vec::new();
    // The configured ROCm root (matches the engines' ROCM_OVERRIDE), then the
    // conventional location, then any /opt/rocm* install.
    if let Ok(root) = std::env::var("ROCM_OVERRIDE") {
        candidates.push(std::path::PathBuf::from(format!(
            "{}/bin/rocm-smi",
            root.trim_end_matches('/')
        )));
    }
    candidates.push(std::path::PathBuf::from("/opt/rocm/bin/rocm-smi"));
    if let Ok(entries) = std::fs::read_dir("/opt") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("rocm"))
            {
                candidates.push(path.join("bin/rocm-smi"));
            }
        }
    }

    candidates
        .into_iter()
        .find(|p| p.is_file())
        .map(|p| p.to_string_lossy().into_owned())
}

impl GpuBackend for RocmBackend {
    fn read_metrics(&self) -> Result<BTreeMap<String, GpuMetrics>> {
        let output = Command::new(&self.smi)
            .args([
                "--json",
                "--showproductname",
                "--showclocks",
                "--showtemp",
                "--showuse",
                "--showpower",
                "--showmaxpower",
                "--showmeminfo",
                "vram",
            ])
            .output()
            .map_err(|e| anyhow::anyhow!("failed to run rocm-smi: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("rocm-smi failed: {stderr}");
        }

        let json: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| anyhow::anyhow!("failed to parse rocm-smi JSON: {e}"))?;

        parse_rocm_json(&json)
    }

    fn name(&self) -> &'static str {
        "rocm"
    }
}

/// Parse the `rocm-smi --json` object into per-card metrics.
///
/// # Errors
///
/// Returns an error if `json` is not a top-level object.
pub fn parse_rocm_json(json: &serde_json::Value) -> Result<BTreeMap<String, GpuMetrics>> {
    let mut metrics = BTreeMap::new();
    let object = json
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("expected JSON object"))?;

    for (card_name, card) in object {
        let temp = card
            .get("Temperature (Sensor junction) (C)")
            .or_else(|| card.get("Temperature (Sensor edge) (C)"))
            .and_then(|v| v.as_str())
            .and_then(|t| t.parse::<f32>().ok())
            .unwrap_or(0.0);

        let load = card
            .get("GPU use (%)")
            .and_then(|v| v.as_str())
            .and_then(|u| u.parse::<u32>().ok())
            .unwrap_or(0);

        let power_consumption = card
            .get("Current Socket Graphics Package Power (W)")
            .or_else(|| card.get("Average Graphics Package Power (W)"))
            .or_else(|| card.get("Average Package Power (W)"))
            .and_then(|v| v.as_str())
            .and_then(|p| p.parse::<f32>().ok())
            .unwrap_or(0.0);

        let power_limit = card
            .get("Max Graphics Package Power (W)")
            .or_else(|| card.get("Power Limit (W)"))
            .or_else(|| card.get("Power Limit (mW)"))
            .and_then(|v| v.as_str())
            .and_then(|p| p.parse::<f64>().ok())
            .map_or(0, |p| {
                if p > 1000.0 {
                    convert::f64_u32(p / 1000.0)
                } else {
                    convert::f64_u32(p)
                }
            });

        let vram_used = card
            .get("VRAM Total Used Memory (B)")
            .and_then(|v| v.as_str())
            .and_then(|v| v.parse::<u64>().ok())
            .map_or(0, |v| v / 1024 / 1024);

        let vram_total = card
            .get("VRAM Total Memory (B)")
            .and_then(|v| v.as_str())
            .and_then(|v| v.parse::<u64>().ok())
            .map_or(0, |v| v / 1024 / 1024);

        let parse_clock = |key: &str| -> u32 {
            card.get(key)
                .and_then(|v| v.as_str())
                .and_then(|s| {
                    let cleaned = s.replace(['(', ')'], "").replace("Mhz", "");
                    cleaned.trim().parse::<u32>().ok()
                })
                .unwrap_or(0)
        };

        let sclk_mhz = parse_clock("sclk clock speed:");
        let mclk_mhz = parse_clock("mclk clock speed:");

        // Resolve the raw "card0" key to the marketing name when available
        // (`--showproductname`), keeping the device id so two identical cards
        // don't collide on the same series string.
        let display_name = card
            .get("Card Series")
            .or_else(|| card.get("Card Model"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map_or_else(
                || card_name.clone(),
                |series| format!("{series} ({card_name})"),
            );

        metrics.insert(
            display_name,
            GpuMetrics {
                temp,
                load,
                power_consumption,
                power_limit,
                vram_used,
                vram_total,
                sclk_mhz,
                mclk_mhz,
            },
        );
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rocm_json() {
        let json: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/rocm_smi_output.json"))
                .unwrap();

        let metrics = parse_rocm_json(&json).unwrap();
        assert_eq!(metrics.len(), 1);

        let card = metrics.get("card0").unwrap();
        assert!((card.temp - 45.0).abs() < 0.1);
        assert_eq!(card.load, 87);
        assert!((card.power_consumption - 180.5).abs() < 0.1);
        assert_eq!(card.power_limit, 300);
        assert_eq!(card.vram_used, 15360); // 16106127360 / 1024 / 1024
        assert_eq!(card.vram_total, 16384); // 17179869184 / 1024 / 1024
        assert_eq!(card.sclk_mhz, 1725);
        assert_eq!(card.mclk_mhz, 1200);
    }

    #[test]
    fn test_parse_rocm_json_empty() {
        let json: serde_json::Value = serde_json::from_str("{}").unwrap();
        let metrics = parse_rocm_json(&json).unwrap();
        assert!(metrics.is_empty());
    }

    #[test]
    fn test_parse_rocm_json_resolves_product_name() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"card0": {"Card Series": "AMD Instinct MI50/MI60", "GPU use (%)": "10"}}"#,
        )
        .unwrap();
        let metrics = parse_rocm_json(&json).unwrap();
        assert!(metrics.contains_key("AMD Instinct MI50/MI60 (card0)"));
        assert_eq!(
            metrics.get("AMD Instinct MI50/MI60 (card0)").unwrap().load,
            10
        );
    }

    #[test]
    fn test_parse_rocm_json_missing_fields() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"card0": {"GPU use (%)": "50"}}"#).unwrap();
        let metrics = parse_rocm_json(&json).unwrap();
        let card = metrics.get("card0").unwrap();
        assert_eq!(card.load, 50);
        assert!(card.temp.abs() < f32::EPSILON);
        assert!(card.power_consumption.abs() < f32::EPSILON);
    }
}
