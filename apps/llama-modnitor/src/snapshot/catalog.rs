//! Per-engine flag catalog driving the dynamic flag form.
//!
//! Each engine exposes a curated set of [`FlagSpec`]s (label, group, value
//! kind) — unsloth-informed for llama.cpp, `--help`-derived for flambeau/vLLM.

// Rust guideline compliant 2026-02-21

use super::EngineKind;

/// What kind of value a catalog flag accepts (drives the value control).
///
/// Every value-bearing flag is a [`ValueSpec::Choice`] so the UI can always
/// offer a dropdown of pertinent, analyzed options; the dropdown also carries a
/// "Custom…" entry that swaps in a free-text input for any raw value. Only bare
/// toggles use [`ValueSpec::None`].
#[derive(Debug, Clone, Copy)]
pub enum ValueSpec {
    /// Bare toggle: the flag takes no value (e.g. `--jinja`).
    None,
    /// Suggested values offered in a dropdown; the user may still type a custom
    /// one via the "Custom…" entry. The first value is the most common default.
    Choice(&'static [&'static str]),
}

/// One curated flag in an engine's catalog. Drives **both** the flag dropdown
/// (label/group) and the adapted value control (`value`), so the UI and the
/// command builder never drift. Flags not in the catalog are still usable via
/// the dynamic form's "Custom…" entry.
#[derive(Debug, Clone, Copy)]
pub struct FlagSpec {
    /// Exact CLI flag, e.g. `"--temp"`.
    pub flag: &'static str,
    /// Label shown in the dropdown, e.g. `"--temp (temperature)"`.
    pub label: &'static str,
    /// Group heading the flag is listed under (an `<optgroup>`).
    pub group: &'static str,
    /// The value control this flag uses.
    pub value: ValueSpec,
}

/// Curated, unsloth-informed flag catalog for an engine.
///
/// llama.cpp values follow the Gemma 4 / Qwen3.6 run guides (see the
/// `unsloth-run-params` note); flambeau/vLLM follow each binary's `--help`.
/// Context length is intentionally excluded — it has a dedicated, engine-mapped
/// field — to avoid two ways to set the same thing.
pub const fn flag_catalog(engine: EngineKind) -> &'static [FlagSpec] {
    use ValueSpec::{Choice, None};
    match engine {
        EngineKind::LlamaCpp => &[
            // Sampling — unsloth-recommended values first (Gemma 4 / Qwen3.6).
            FlagSpec {
                flag: "--temp",
                label: "--temp (temperature)",
                group: "Sampling",
                value: Choice(&["1.0", "0.7", "0.6", "0.8", "0.0"]),
            },
            FlagSpec {
                flag: "--top-p",
                label: "--top-p (nucleus)",
                group: "Sampling",
                value: Choice(&["0.95", "0.8", "0.9", "1.0"]),
            },
            FlagSpec {
                flag: "--top-k",
                label: "--top-k",
                group: "Sampling",
                value: Choice(&["64", "20", "40", "0"]),
            },
            FlagSpec {
                flag: "--min-p",
                label: "--min-p",
                group: "Sampling",
                value: Choice(&["0.0", "0.01", "0.05"]),
            },
            FlagSpec {
                flag: "--presence-penalty",
                label: "--presence-penalty",
                group: "Sampling",
                value: Choice(&["0.0", "1.0", "1.5", "2.0"]),
            },
            FlagSpec {
                flag: "--repeat-penalty",
                label: "--repeat-penalty",
                group: "Sampling",
                value: Choice(&["1.0", "1.1", "1.2"]),
            },
            FlagSpec {
                flag: "--samplers",
                label: "--samplers (chain order)",
                group: "Sampling",
                value: Choice(&[
                    "temperature;top_p;top_k",
                    "top_k;top_p;temperature",
                    "min_p;temperature",
                ]),
            },
            FlagSpec {
                flag: "--seed",
                label: "--seed",
                group: "Sampling",
                value: Choice(&["3407", "-1", "0", "42"]),
            },
            // Reasoning / chat template (current llama.cpp uses --reasoning).
            FlagSpec {
                flag: "--reasoning",
                label: "--reasoning (thinking on/off)",
                group: "Reasoning",
                value: Choice(&["on", "off", "auto"]),
            },
            FlagSpec {
                flag: "--reasoning-budget",
                label: "--reasoning-budget (tokens)",
                group: "Reasoning",
                value: Choice(&["-1", "0", "2048", "4096"]),
            },
            FlagSpec {
                flag: "--jinja",
                label: "--jinja (use chat template)",
                group: "Reasoning",
                value: None,
            },
            FlagSpec {
                flag: "--chat-template-kwargs",
                label: "--chat-template-kwargs",
                group: "Reasoning",
                value: Choice(&["{\"enable_thinking\":false}", "{\"enable_thinking\":true}"]),
            },
            // Memory / performance.
            FlagSpec {
                flag: "--flash-attn",
                label: "--flash-attn",
                group: "Memory",
                value: Choice(&["on", "off", "auto"]),
            },
            FlagSpec {
                flag: "--cache-type-k",
                label: "--cache-type-k (KV K quant)",
                group: "Memory",
                value: Choice(&["f16", "q8_0", "q4_0", "q5_1"]),
            },
            FlagSpec {
                flag: "--cache-type-v",
                label: "--cache-type-v (KV V quant)",
                group: "Memory",
                value: Choice(&["f16", "q8_0", "q4_0", "q5_1"]),
            },
            FlagSpec {
                flag: "--n-gpu-layers",
                label: "--n-gpu-layers",
                group: "Memory",
                value: Choice(&["99", "0", "40", "20"]),
            },
            FlagSpec {
                flag: "--n-cpu-moe",
                label: "--n-cpu-moe (MoE layers → CPU)",
                group: "Memory",
                value: Choice(&["0", "8", "16", "24", "99"]),
            },
            FlagSpec {
                flag: "--swa-full",
                label: "--swa-full (full SWA cache)",
                group: "Memory",
                value: None,
            },
            FlagSpec {
                flag: "--no-kv-offload",
                label: "--no-kv-offload",
                group: "Memory",
                value: None,
            },
            FlagSpec {
                flag: "--cache-reuse",
                label: "--cache-reuse (KV shift reuse)",
                group: "Memory",
                value: Choice(&["256", "0", "128"]),
            },
            FlagSpec {
                flag: "--no-mmap",
                label: "--no-mmap",
                group: "Memory",
                value: None,
            },
            FlagSpec {
                flag: "--mlock",
                label: "--mlock",
                group: "Memory",
                value: None,
            },
            // Throughput / multi-GPU.
            FlagSpec {
                flag: "--threads",
                label: "--threads",
                group: "Performance",
                value: Choice(&["-1", "8", "16", "32"]),
            },
            FlagSpec {
                flag: "--batch-size",
                label: "--batch-size",
                group: "Performance",
                value: Choice(&["2048", "1024", "512", "4096"]),
            },
            FlagSpec {
                flag: "--ubatch-size",
                label: "--ubatch-size",
                group: "Performance",
                value: Choice(&["512", "256", "1024", "128"]),
            },
            FlagSpec {
                flag: "--parallel",
                label: "--parallel (slots)",
                group: "Performance",
                value: Choice(&["1", "2", "4", "8"]),
            },
            FlagSpec {
                flag: "--cont-batching",
                label: "--cont-batching",
                group: "Performance",
                value: None,
            },
            FlagSpec {
                flag: "--split-mode",
                label: "--split-mode (multi-GPU)",
                group: "Performance",
                value: Choice(&["layer", "row", "none", "tensor"]),
            },
            // MoE expert offload (regex form) + vision.
            FlagSpec {
                flag: "-ot",
                label: "-ot (override-tensor, MoE→CPU)",
                group: "Advanced",
                value: Choice(&[".ffn_.*_exps.=CPU", ".ffn_(up|down)_exps.=CPU"]),
            },
            FlagSpec {
                flag: "--mmproj",
                label: "--mmproj (vision projector)",
                group: "Vision",
                value: Choice(&["mmproj-BF16.gguf", "mmproj-F16.gguf"]),
            },
        ],
        EngineKind::Vllm => &[
            FlagSpec {
                flag: "--gpu-memory-utilization",
                label: "--gpu-memory-utilization",
                group: "Memory",
                value: Choice(&["0.9", "0.8", "0.95", "0.7"]),
            },
            FlagSpec {
                flag: "--max-num-seqs",
                label: "--max-num-seqs",
                group: "Memory",
                value: Choice(&["256", "128", "512", "64"]),
            },
            FlagSpec {
                flag: "--kv-cache-dtype",
                label: "--kv-cache-dtype",
                group: "Memory",
                value: Choice(&["auto", "fp8", "fp8_e5m2"]),
            },
            FlagSpec {
                flag: "--swap-space",
                label: "--swap-space (GiB)",
                group: "Memory",
                value: Choice(&["4", "8", "16"]),
            },
            FlagSpec {
                flag: "--enable-prefix-caching",
                label: "--enable-prefix-caching",
                group: "Memory",
                value: None,
            },
            FlagSpec {
                flag: "--dtype",
                label: "--dtype",
                group: "Compute",
                value: Choice(&["auto", "bfloat16", "float16", "half"]),
            },
            FlagSpec {
                flag: "--quantization",
                label: "--quantization",
                group: "Compute",
                value: Choice(&["awq", "gptq", "fp8", "bitsandbytes"]),
            },
            FlagSpec {
                flag: "--enforce-eager",
                label: "--enforce-eager",
                group: "Compute",
                value: None,
            },
            FlagSpec {
                flag: "--trust-remote-code",
                label: "--trust-remote-code",
                group: "Compute",
                value: None,
            },
        ],
        EngineKind::Flambeau => &[
            FlagSpec {
                flag: "--mesh-mode",
                label: "--mesh-mode (multi-GPU)",
                group: "Parallel",
                value: Choice(&["pp", "tp", "hybrid"]),
            },
            FlagSpec {
                flag: "--tp-size",
                label: "--tp-size",
                group: "Parallel",
                value: Choice(&["1", "2"]),
            },
            FlagSpec {
                flag: "--pp-size",
                label: "--pp-size",
                group: "Parallel",
                value: Choice(&["1", "2"]),
            },
            FlagSpec {
                flag: "--inflight-slots",
                label: "--inflight-slots",
                group: "Performance",
                value: Choice(&["1", "2", "4", "8"]),
            },
            FlagSpec {
                flag: "--prefill-chunk-tokens",
                label: "--prefill-chunk-tokens",
                group: "Performance",
                value: Choice(&["512", "1024", "2048"]),
            },
            FlagSpec {
                flag: "--prefill-ubatch",
                label: "--prefill-ubatch",
                group: "Performance",
                value: Choice(&["256", "512", "1024"]),
            },
            FlagSpec {
                flag: "--max-queue-depth",
                label: "--max-queue-depth",
                group: "Performance",
                value: Choice(&["8", "16", "32"]),
            },
            FlagSpec {
                flag: "--no-gpu-sampler",
                label: "--no-gpu-sampler",
                group: "Performance",
                value: None,
            },
            FlagSpec {
                flag: "--kv",
                label: "--kv (cache dtype)",
                group: "Memory",
                value: Choice(&["f16", "q8_0", "q4_0"]),
            },
            FlagSpec {
                flag: "--paged-kv",
                label: "--paged-kv",
                group: "Memory",
                value: None,
            },
            FlagSpec {
                flag: "--prefix-cache",
                label: "--prefix-cache",
                group: "Memory",
                value: None,
            },
            FlagSpec {
                flag: "--prefix-cache-max-gb",
                label: "--prefix-cache-max-gb",
                group: "Memory",
                value: Choice(&["2", "4", "8"]),
            },
        ],
    }
}

/// Look up a catalog flag by its exact name for the given engine.
pub fn flag_spec(engine: EngineKind, flag: &str) -> Option<&'static FlagSpec> {
    flag_catalog(engine).iter().find(|s| s.flag == flag)
}
