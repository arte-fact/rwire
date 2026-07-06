//! VRAM/RAM usage estimation and context auto-fit.
//!
//! A deliberately simple, engine-agnostic model: total ≈ weights + KV cache +
//! a fixed compute buffer. The KV term is the standard
//! `2 · n_layers · n_kv_heads · head_dim · ctx · bytes_per_element`. This is an
//! estimate for planning, not an exact accounting.

// Rust guideline compliant 2026-02-21

use crate::convert;
use crate::modelmeta::ModelMeta;
use crate::snapshot::EngineKind;

/// One mebibyte in bytes.
const MIB: f64 = 1024.0 * 1024.0;

/// Fixed compute/runtime buffer added to weights + KV (CUDA/HIP context plus
/// activation scratch), in MiB. Matches the intercept of empirical llama.cpp
/// VRAM fits (~1.5 GiB).
pub const BUFFER_MB: f64 = 1500.0;

/// A usage breakdown in mebibytes.
#[derive(Debug, Clone, Copy, Default)]
pub struct Usage {
    /// Model weights, in MiB.
    pub weights: f64,
    /// KV cache at the given context length, in MiB.
    pub kv: f64,
    /// Fixed compute/runtime buffer, in MiB.
    pub buffer: f64,
}

impl Usage {
    /// Total estimated usage in MiB.
    pub fn total_mb(&self) -> f64 {
        self.weights + self.kv + self.buffer
    }
}

/// KV-cache bytes per token: `2 (K+V) · n_layers · n_kv_heads · head_dim · elt`.
fn kv_bytes_per_token(meta: &ModelMeta, cache_elt_bytes: f64) -> f64 {
    2.0 * f64::from(meta.n_layers)
        * f64::from(meta.n_kv_heads)
        * f64::from(meta.head_dim())
        * cache_elt_bytes
}

/// Per-engine inputs that shape the estimate beyond the model + context.
#[derive(Debug, Clone, Copy)]
pub struct EngineParams {
    /// Engine the node uses.
    pub engine: EngineKind,
    /// KV-cache bytes per element (from the node's KV-quant flag).
    pub cache_elt_bytes: f64,
    /// Total device capacity (MiB) — used by vLLM's pool sizing.
    pub capacity_mb: f64,
    /// vLLM `--gpu-memory-utilization` (fraction); ignored by other engines.
    pub gpu_util: f64,
    /// Extra fixed reservation (MiB), e.g. flambeau `--prefix-cache-max-gb`.
    pub extra_mb: f64,
}

/// Estimate usage for a context length, accounting for engine specifics.
///
/// - **llama.cpp**: weights + KV(ctx) + buffer.
/// - **flambeau**: same, plus any prefix-cache reservation.
/// - **vLLM**: weights + a pre-allocated KV *pool* of `gpu_util · (capacity −
///   weights)` (independent of `ctx`, which only bounds how much of the pool a
///   request uses) + buffer.
pub fn estimate(meta: &ModelMeta, ctx: u32, p: EngineParams) -> Usage {
    let weights = convert::u64_f64(meta.weights_mb);
    match p.engine {
        EngineKind::Vllm => {
            let pool = (p.gpu_util * (p.capacity_mb - weights)).max(0.0);
            Usage {
                weights,
                kv: pool,
                buffer: BUFFER_MB,
            }
        }
        EngineKind::Flambeau => Usage {
            weights,
            kv: kv_bytes_per_token(meta, p.cache_elt_bytes) * f64::from(ctx) / MIB,
            buffer: BUFFER_MB + p.extra_mb,
        },
        EngineKind::LlamaCpp => Usage {
            weights,
            kv: kv_bytes_per_token(meta, p.cache_elt_bytes) * f64::from(ctx) / MIB,
            buffer: BUFFER_MB,
        },
    }
}

/// Largest context that keeps total usage at or below `budget_mb`, clamped to
/// the model's max context and rounded down to 256. For vLLM the bound is the
/// pre-allocated pool rather than `budget_mb`. Returns 0 when fixed costs alone
/// already exceed what's available.
pub fn fit_ctx(meta: &ModelMeta, budget_mb: f64, p: EngineParams) -> u32 {
    let per_tok_mb = kv_bytes_per_token(meta, p.cache_elt_bytes) / MIB;
    if per_tok_mb <= 0.0 {
        return meta.max_ctx;
    }
    let avail_kv_mb = match p.engine {
        // vLLM: context is limited by the pool, not the target budget.
        EngineKind::Vllm => {
            (p.gpu_util * (p.capacity_mb - convert::u64_f64(meta.weights_mb))).max(0.0)
        }
        EngineKind::Flambeau => {
            budget_mb - convert::u64_f64(meta.weights_mb) - BUFFER_MB - p.extra_mb
        }
        EngineKind::LlamaCpp => budget_mb - convert::u64_f64(meta.weights_mb) - BUFFER_MB,
    };
    if avail_kv_mb <= 0.0 {
        return 0;
    }
    let raw = convert::f64_u32(avail_kv_mb / per_tok_mb);
    let capped = if meta.max_ctx > 0 {
        raw.min(meta.max_ctx)
    } else {
        raw
    };
    capped / 256 * 256
}

/// Map a llama.cpp `--cache-type-*` / vLLM KV dtype string to bytes per element
/// (approximate; quantized caches carry small block-scale overhead).
pub fn cache_elt_bytes(name: &str) -> f64 {
    match name.trim() {
        "q4_0" | "q4_1" => 0.5,
        "q5_0" | "q5_1" => 0.65,
        "q8_0" | "fp8" | "fp8_e5m2" | "fp8_e4m3" => 1.0,
        "f32" | "float32" => 4.0,
        _ => 2.0, // f16 / bf16 default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> ModelMeta {
        // Llama-3-8B-ish: 32 layers, 32 heads, 8 KV heads, 4096 hidden.
        ModelMeta {
            n_layers: 32,
            n_heads: 32,
            n_kv_heads: 8,
            embedding_dim: 4096,
            max_ctx: 131_072,
            weights_mb: 4800,
        }
    }

    fn llama(cache: f64) -> EngineParams {
        EngineParams {
            engine: EngineKind::LlamaCpp,
            cache_elt_bytes: cache,
            capacity_mb: 16384.0,
            gpu_util: 0.9,
            extra_mb: 0.0,
        }
    }

    #[test]
    fn kv_scales_with_context_and_quant() {
        let m = meta();
        // head_dim = 4096/32 = 128; per-token KV = 2*32*8*128*2 = 131072 B = .125 MiB.
        let at8k = estimate(&m, 8192, llama(2.0));
        assert!((at8k.kv - 1024.0).abs() < 1.0); // 8192 * 0.125 MiB = 1024 MiB
        // q8_0 halves the KV cache.
        let q8 = estimate(&m, 8192, llama(1.0));
        assert!((q8.kv - 512.0).abs() < 1.0);
    }

    #[test]
    fn fit_ctx_respects_budget_and_rounding() {
        let m = meta();
        // Budget 16 GiB: KV budget = 16384 - 4800 - 1500 = 10084 MiB;
        // /0.125 = ~80672 tokens, rounded down to a 256 multiple.
        let ctx = fit_ctx(&m, 16384.0, llama(2.0));
        assert_eq!(ctx % 256, 0);
        assert!(ctx > 70000 && ctx <= m.max_ctx);
        // A budget below weights+buffer yields zero.
        assert_eq!(fit_ctx(&m, 5000.0, llama(2.0)), 0);
    }

    #[test]
    fn vllm_pool_is_context_independent() {
        let m = meta();
        let p = EngineParams {
            engine: EngineKind::Vllm,
            cache_elt_bytes: 2.0,
            capacity_mb: 16384.0,
            gpu_util: 0.9,
            extra_mb: 0.0,
        };
        // Pool = 0.9 * (16384 - 4800) = 10425.6 MiB, regardless of ctx.
        let a = estimate(&m, 4096, p);
        let b = estimate(&m, 65536, p);
        assert!((a.kv - b.kv).abs() < 1.0);
        assert!((a.kv - 10425.6).abs() < 1.0);
    }
}
