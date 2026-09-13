//! GPU metric collection across vendors (NVIDIA, AMD `ROCm`).
//!
//! Backends shell out to the vendor CLI (`nvidia-smi`, `rocm-smi`), parse the
//! output, and report per-card metrics. [`detect_backend`] auto-selects every
//! vendor whose tool is installed so mixed-vendor machines report all cards.

pub mod dummy;
pub mod nvidia;
pub mod rocm;

use anyhow::Result;
use std::collections::BTreeMap;
use std::sync::Arc;

/// GPU vendor / compute-backend family.
///
/// Determines which inference engines and device-mask env var apply to a card
/// (NVIDIA → CUDA, AMD → ROCm/HIP). See `docs/llm-launcher-plan.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Vendor {
    /// NVIDIA (CUDA).
    #[default]
    Nvidia,
    /// AMD (`ROCm` / HIP).
    Amd,
}

/// Point-in-time metrics for a single GPU.
#[derive(Debug, Clone, Default)]
pub struct GpuMetrics {
    /// Core temperature in degrees Celsius.
    pub temp: f32,
    /// Core utilization as a percentage (0-100).
    pub load: u32,
    /// Current power draw in watts.
    pub power_consumption: f32,
    /// Configured power limit in watts.
    pub power_limit: u32,
    /// VRAM in use in mebibytes.
    pub vram_used: u64,
    /// Total VRAM in mebibytes.
    pub vram_total: u64,
    /// Core (shader) clock in megahertz.
    pub sclk_mhz: u32,
    /// Memory clock in megahertz.
    pub mclk_mhz: u32,
}

/// A source of GPU metrics for one vendor.
pub trait GpuBackend: Send + Sync + 'static {
    /// Read current metrics, keyed by card name.
    ///
    /// # Errors
    ///
    /// Returns an error if the vendor tool cannot be run or its output cannot
    /// be parsed.
    fn read_metrics(&self) -> Result<BTreeMap<String, GpuMetrics>>;

    /// Human-readable backend identifier (e.g. `"nvidia"`).
    fn name(&self) -> &'static str;
}

/// Polls several backends and merges their metrics into one map, so machines
/// with a mix of vendors (e.g. AMD + NVIDIA cards) report every GPU. A failure
/// in one backend is logged and skipped without hiding the others.
pub struct MultiBackend {
    backends: Vec<Arc<dyn GpuBackend>>,
}

impl GpuBackend for MultiBackend {
    fn read_metrics(&self) -> Result<BTreeMap<String, GpuMetrics>> {
        let mut all = BTreeMap::new();
        for backend in &self.backends {
            match backend.read_metrics() {
                Ok(metrics) => all.extend(metrics),
                Err(e) => eprintln!("[error] GPU metrics ({}): {e}", backend.name()),
            }
        }
        Ok(all)
    }

    fn name(&self) -> &'static str {
        "multi"
    }
}

/// Select GPU backend(s) to monitor.
///
/// `force` may be `"rocm"`, `"nvidia"`, or `"none"`; anything else (e.g.
/// `"auto"`) monitors every vendor whose CLI is present.
pub fn detect_backend(force: &str) -> Arc<dyn GpuBackend> {
    match force {
        "rocm" => Arc::new(rocm::RocmBackend::new(
            rocm::find_rocm_smi().unwrap_or_else(|| "rocm-smi".to_string()),
        )),
        "nvidia" => Arc::new(nvidia::NvidiaBackend),
        "none" => Arc::new(dummy::DummyBackend),
        // "auto" / "all" / anything else: monitor every vendor whose tool is present
        _ => {
            let mut backends: Vec<Arc<dyn GpuBackend>> = Vec::new();
            if let Some(smi) = rocm::find_rocm_smi() {
                backends.push(Arc::new(rocm::RocmBackend::new(smi)));
            }
            if command_exists("nvidia-smi") {
                backends.push(Arc::new(nvidia::NvidiaBackend));
            }
            match backends.len() {
                0 => {
                    eprintln!("[warn] No GPU monitoring tool found (rocm-smi / nvidia-smi)");
                    Arc::new(dummy::DummyBackend)
                }
                1 => backends.into_iter().next().unwrap(),
                _ => Arc::new(MultiBackend { backends }),
            }
        }
    }
}

fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubBackend {
        name: &'static str,
        cards: Vec<&'static str>,
        fail: bool,
    }

    impl GpuBackend for StubBackend {
        fn read_metrics(&self) -> Result<BTreeMap<String, GpuMetrics>> {
            if self.fail {
                anyhow::bail!("stub failure");
            }
            Ok(self
                .cards
                .iter()
                .map(|c| (c.to_string(), GpuMetrics::default()))
                .collect())
        }

        fn name(&self) -> &'static str {
            self.name
        }
    }

    #[test]
    fn multi_backend_merges_all_vendors() {
        let multi = MultiBackend {
            backends: vec![
                Arc::new(StubBackend {
                    name: "rocm",
                    cards: vec!["card0", "card1"],
                    fail: false,
                }),
                Arc::new(StubBackend {
                    name: "nvidia",
                    cards: vec!["GPU0 NVIDIA"],
                    fail: false,
                }),
            ],
        };
        let metrics = multi.read_metrics().unwrap();
        assert_eq!(metrics.len(), 3);
        assert!(metrics.contains_key("card0"));
        assert!(metrics.contains_key("card1"));
        assert!(metrics.contains_key("GPU0 NVIDIA"));
    }

    #[test]
    fn multi_backend_skips_failing_backend() {
        let multi = MultiBackend {
            backends: vec![
                Arc::new(StubBackend {
                    name: "rocm",
                    cards: vec!["card0"],
                    fail: true,
                }),
                Arc::new(StubBackend {
                    name: "nvidia",
                    cards: vec!["GPU0 NVIDIA"],
                    fail: false,
                }),
            ],
        };
        let metrics = multi.read_metrics().unwrap();
        assert_eq!(metrics.len(), 1);
        assert!(metrics.contains_key("GPU0 NVIDIA"));
    }
}
