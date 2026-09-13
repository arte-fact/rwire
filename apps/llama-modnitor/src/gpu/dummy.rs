//! Fallback backend used when no GPU tool is available.

use anyhow::Result;
use std::collections::BTreeMap;

use super::{GpuBackend, GpuMetrics};

/// Reports no GPUs; selected when neither `rocm-smi` nor `nvidia-smi` exist.
pub struct DummyBackend;

impl GpuBackend for DummyBackend {
    fn read_metrics(&self) -> Result<BTreeMap<String, GpuMetrics>> {
        Ok(BTreeMap::new())
    }

    fn name(&self) -> &'static str {
        "none"
    }
}
