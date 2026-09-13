//! Off-thread telemetry and process probing: metrics scraping, test chat,
//! benchmarks, HTTP helpers, and orphan-process detection/killing.

// Rust guideline compliant 2026-02-21

use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rwire::{ChangeSet, SharedServerState};

use crate::convert;
use crate::snapshot::{App, EngineKind, NodeStats, OrphanProc};

use super::ChildEntry;

/// Run a chat completion on a background thread, replacing the node's trailing
/// (pending) assistant turn with the reply.
pub fn spawn_chat(
    shared: Arc<SharedServerState>,
    id: u64,
    port: u16,
    model: String,
    messages: Vec<(String, String)>,
) {
    std::thread::spawn(move || {
        let reply = chat_once(port, &model, &messages).unwrap_or_else(|e| format!("[error] {e}"));
        shared.update_shared_changed::<App>(ChangeSet::from_fields(&[App::FIELD_NODES]), |app| {
            if let Some(node) = app.nodes.iter_mut().find(|n| n.id == id)
                && let Some(last) = node.chat_log.last_mut()
            {
                last.content = reply;
            }
        });
    });
}

/// POST a chat completion with the full message history; return the reply text.
pub fn chat_once(port: u16, model: &str, messages: &[(String, String)]) -> Result<String, String> {
    let msgs: Vec<serde_json::Value> = messages
        .iter()
        .map(|(role, content)| serde_json::json!({ "role": role, "content": content }))
        .collect();
    let body = serde_json::json!({
        "model": model,
        "messages": msgs,
        "max_tokens": 1024,
        "stream": false,
    })
    .to_string();
    let resp = http_post(port, "/v1/chat/completions", &body).ok_or("request failed")?;
    let json_part = resp.split("\r\n\r\n").nth(1).unwrap_or(&resp);
    let v: serde_json::Value =
        serde_json::from_str(json_part.trim()).map_err(|e| format!("bad response: {e}"))?;
    v.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(str::to_string)
        .ok_or_else(|| {
            format!(
                "no content in response: {}",
                json_part.chars().take(200).collect::<String>()
            )
        })
}

/// Standard pp (prompt-processing) + tg (token-generation) benchmark sizes.
pub const BENCH_PP: [u32; 3] = [512, 1024, 4096];

pub const BENCH_TG: [u32; 3] = [64, 128, 256];

/// Run the pp/tg benchmark suite on a background thread, writing each result
/// into the node as it completes and clearing `bench_running` at the end.
pub fn spawn_bench(shared: Arc<SharedServerState>, id: u64, port: u16) {
    std::thread::spawn(move || {
        let push = |label: String, value: String, done: bool| {
            let sh = &shared;
            sh.update_shared_changed::<App>(ChangeSet::from_fields(&[App::FIELD_NODES]), |app| {
                if let Some(node) = app.nodes.iter_mut().find(|n| n.id == id) {
                    node.bench_results.push((label.clone(), value.clone()));
                    if done {
                        node.bench_running = false;
                    }
                }
            });
        };
        for &n in &BENCH_PP {
            let r = bench_one(port, n, 1)
                .map_or_else(|| "failed".into(), |(pp, _)| format!("{pp:.0} t/s"));
            push(format!("pp{n}"), r, false);
        }
        for (i, &n) in BENCH_TG.iter().enumerate() {
            let last = i == BENCH_TG.len() - 1;
            let r = bench_one(port, 64, n)
                .map_or_else(|| "failed".into(), |(_, tg)| format!("{tg:.0} t/s"));
            push(format!("tg{n}"), r, last);
        }
    });
}

/// Kill a process by PID: SIGTERM now, then SIGKILL shortly after if it lingers.
pub fn kill_pid(pid: u32) {
    if pid == 0 {
        return;
    }
    // PIDs always fit `i32` (`pid_t`); saturate defensively rather than wrap.
    let signed_pid: libc::pid_t = i32::try_from(pid).unwrap_or(i32::MAX);
    unsafe {
        libc::kill(signed_pid, libc::SIGTERM);
    }
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(1500));
        unsafe {
            libc::kill(signed_pid, libc::SIGKILL);
        }
    });
}

/// Scan `/proc` for engine processes (llama.cpp / flambeau / vLLM) as `(pid, label)`.
pub fn scan_engine_procs() -> Vec<(u32, String)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return out;
    };
    for e in entries.flatten() {
        let Some(pid) = e.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let Ok(bytes) = std::fs::read(format!("/proc/{pid}/cmdline")) else {
            continue;
        };
        if bytes.is_empty() {
            continue; // kernel threads / zombies have empty cmdline
        }
        let cmd = String::from_utf8_lossy(&bytes).replace('\0', " ");
        let is_engine =
            cmd.contains("llama-server") || cmd.contains("/flambeau") || cmd.contains("vllm");
        if is_engine {
            out.push((pid, summarize_cmd(&cmd)));
        }
    }
    out
}

/// A short `exe :port model` label for an engine command line.
pub fn summarize_cmd(cmd: &str) -> String {
    let toks: Vec<&str> = cmd.split_whitespace().collect();
    let base = |p: &str| p.rsplit('/').next().unwrap_or(p).to_string();
    let exe = toks.first().map(|t| base(t)).unwrap_or_default();
    let after = |flags: &[&str]| {
        toks.iter()
            .position(|t| flags.contains(t))
            .and_then(|i| toks.get(i + 1))
            .map(|s| base(s))
    };
    let port = after(&["--port"])
        .map(|p| format!(":{p}"))
        .unwrap_or_default();
    let model = after(&["-m", "--model"]).unwrap_or_default();
    format!("{exe} {port} {model}")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Detect engine processes not owned by this supervisor and publish them as
/// orphans (only when the set changes, to avoid needless re-renders).
pub fn scan_orphans(
    children: &HashMap<u64, ChildEntry>,
    prev: &mut Vec<OrphanProc>,
    shared: &SharedServerState,
) {
    let owned: HashSet<u32> = children.values().map(|e| e.child.id()).collect();
    let self_pid = std::process::id();
    let mut orphans: Vec<OrphanProc> = scan_engine_procs()
        .into_iter()
        .filter(|(pid, _)| !owned.contains(pid) && *pid != self_pid)
        .map(|(pid, label)| OrphanProc { pid, label })
        .collect();
    orphans.sort_by_key(|o| o.pid);
    if orphans != *prev {
        prev.clone_from(&orphans);
        shared.update_shared_changed::<App>(ChangeSet::from_fields(&[App::FIELD_ORPHANS]), |app| {
            app.orphans.clone_from(&orphans);
        });
    }
}

/// Run one benchmark request via `/completion`: a `prompt_tokens`-token prompt
/// generating `n_predict` tokens. Returns `(prompt_per_second, predicted_per_second)`.
pub fn bench_one(port: u16, prompt_tokens: u32, n_predict: u32) -> Option<(f64, f64)> {
    // A repeated single-token word approximates the target prompt length.
    let prompt = "the ".repeat(prompt_tokens as usize);
    let body = serde_json::json!({
        "prompt": prompt,
        "n_predict": n_predict,
        "temperature": 0.0,
        "cache_prompt": false,
        "stream": false,
    })
    .to_string();
    let resp = http_post(port, "/completion", &body)?;
    let json_part = resp.split("\r\n\r\n").nth(1).unwrap_or(&resp);
    let v: serde_json::Value = serde_json::from_str(json_part.trim()).ok()?;
    let t = v.get("timings")?;
    let pp = t
        .get("prompt_per_second")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let tg = t
        .get("predicted_per_second")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    Some((pp, tg))
}

/// Minimal blocking HTTP POST of a JSON body; returns the full response text.
pub fn http_post(port: u16, path: &str, body: &str) -> Option<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_mins(2)));
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes()).ok()?;
    let mut out = String::new();
    stream.read_to_string(&mut out).ok()?;
    Some(out)
}

/// Scrape an engine's metrics endpoint into [`NodeStats`].
///
/// Returns `None` if the endpoint is momentarily unreachable (keep the last
/// stats rather than blanking them). flambeau has no `/metrics` yet → empty.
pub fn scrape_stats(
    engine: EngineKind,
    port: u16,
    prev: &mut Option<(f64, f64, Instant)>,
) -> Option<NodeStats> {
    match engine {
        EngineKind::LlamaCpp => {
            let body = http_get(port, "/metrics")?;
            Some(NodeStats {
                prefill_tps: parse_metric(&body, "llamacpp:prompt_tokens_seconds")
                    .map(convert::f64_f32),
                decode_tps: parse_metric(&body, "llamacpp:predicted_tokens_seconds")
                    .map(convert::f64_f32),
                // Newer builds expose this; older ones omit it (then the UI shows
                // the estimated KV allocation instead of a live ratio).
                kv_usage: parse_metric(&body, "llamacpp:kv_cache_usage_ratio")
                    .map(convert::f64_f32),
                running: parse_metric(&body, "llamacpp:requests_processing").map(convert::f64_u32),
                waiting: parse_metric(&body, "llamacpp:requests_deferred").map(convert::f64_u32),
            })
        }
        EngineKind::Vllm => {
            let body = http_get(port, "/metrics")?;
            // tok/s = rate of the cumulative token counters between samples.
            let now = Instant::now();
            let prompt = parse_metric(&body, "vllm:prompt_tokens_total");
            let gen_total = parse_metric(&body, "vllm:generation_tokens_total");
            let (prefill_tps, decode_tps) = match (*prev, prompt, gen_total) {
                (Some((pp, pg, pt)), Some(cp), Some(cg)) => {
                    let dt = now.duration_since(pt).as_secs_f64().max(0.001);
                    (
                        Some(convert::f64_f32((cp - pp).max(0.0) / dt)),
                        Some(convert::f64_f32((cg - pg).max(0.0) / dt)),
                    )
                }
                _ => (None, None),
            };
            *prev = match (prompt, gen_total) {
                (Some(p), Some(g)) => Some((p, g, now)),
                _ => None,
            };
            Some(NodeStats {
                prefill_tps,
                decode_tps,
                kv_usage: parse_metric(&body, "vllm:kv_cache_usage_perc").map(convert::f64_f32),
                running: parse_metric(&body, "vllm:num_requests_running").map(convert::f64_u32),
                waiting: parse_metric(&body, "vllm:num_requests_waiting").map(convert::f64_u32),
            })
        }
        // No /metrics endpoint yet (see docs/tree-shaking-redesign + launcher plan).
        EngineKind::Flambeau => Some(NodeStats::default()),
    }
}

/// Parse a Prometheus metric value by name, tolerating `{labels}` suffixes.
///
/// Matches lines like `name 1.5` or `name{k="v"} 1.5`; returns the last
/// whitespace-separated token as a float.
pub fn parse_metric(body: &str, name: &str) -> Option<f64> {
    for line in body.lines() {
        if let Some(rest) = line.strip_prefix(name)
            && (rest.starts_with(' ') || rest.starts_with('{'))
            && let Some(value) = line.split_whitespace().last()
        {
            return value.parse().ok();
        }
    }
    None
}

/// Minimal blocking HTTP GET returning the full response text (headers + body).
pub fn http_get(port: u16, path: &str) -> Option<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).ok()?;
    let mut out = String::new();
    stream.read_to_string(&mut out).ok()?;
    Some(out)
}
