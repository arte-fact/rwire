# LLM launcher UI — implementation plan

> Status: **planned, not implemented.** Extends the read-only monitor into a
> hardware-selection + flambeau-node launcher/monitor. Built for a cold start.
> Invoke **ms-rust** before writing Rust.

## UX recap (what the user asked for)

- Left column: the existing hardware cards in a row/stack.
- A **selection state machine** over 3 groups — **NVIDIA GPUs**, **AMD GPUs**, **CPU** — that only lets you select *compatible* material.
- Selection is shown by a **bright outer ring** on chosen cards + a **hollow connector path** leading right to a **hollow "+" card** in a second column.
- Clicking "+" creates an **LLM node card** in the right column.
- Each node card is dynamic: configure a **flambeau** server (all options), launch it, then show **live stats** (prompt-processing rate, decode rate, context usage, …).

---

## Key findings that reshaped the plan (the refine pass)

These come from analysing `/artefact/flambeau`, `/artefact/rwire`, and the current app. They change the design, so read them first.

1. **flambeau is GPU-only — there is no CPU inference backend.** Backends are HIP (AMD gfx906/908/90a/942/11xx) and CUDA (NVIDIA sm_80/86/89/90), selected at *build time* via cargo features `hip_serve` / `cuda_serve` (`crates/cli/src/main.rs:316`). A flambeau node cannot run on CPU. → **Decision #1**: the "CPU" group can't back an LLM node. Options: (a) keep the CPU card in the left column for monitoring but make it *non-selectable for node creation* (greyed in the FSM); (b) treat CPU as a future/other-engine slot; (c) drop it from the selector. **Recommended: (a)** — show it, mark it "no flambeau CPU backend", don't let it form a node.

2. **A flambeau binary is single-vendor, and you can't mix vendors in one node.** Because the backend is a build-time feature, you need **two prebuilt binaries** (e.g. `flambeau-hip`, `flambeau-cuda`), and one node = one vendor's devices. This is *exactly* the compatibility rule: a node is backed by **one group only** (all-NVIDIA or all-AMD), multi-GPU within that vendor is fine, cross-vendor is invalid. → The FSM enforces "single vendor per node"; selecting an NVIDIA card disables the AMD group (and vice-versa) until cleared. **Decision #2**: confirm both binaries will be built/available and their paths (config/env).

3. **The requested live stats do not exist as a flambeau endpoint yet.** There is **no `/metrics`**. What exists (`crates/server/src/`): per-request `Usage {prompt_tokens, completion_tokens, total_tokens}` in completion responses, and `GET /v1/agent/stats` → per-iteration `{latency_ms, prompt_tokens, completion_tokens, finish_reason}` (agent loops only). No tokens/s, no KV/context-usage, no active-slots surface. → **Decision #3 (the big one)**: where do prompt-rate / decode-rate / context-usage come from? Options:
   - **(A) Add a small `/metrics` endpoint to flambeau** exposing prefill tok/s, decode tok/s, KV/context usage, active slots, queue depth — flambeau already has these counters internally (scheduler, slots). Cleanest; one focused flambeau PR. **Recommended.**
   - **(B) Probe-based estimation**: the monitor periodically sends a tiny completion and derives tok/s from `Usage` + wall-clock. No flambeau change, but synthetic load + can't measure real KV usage.
   - **(C) Parse server stdout logs** for the rates flambeau already logs. Brittle.
   The plan is structured so node launch/manage works regardless, and the stats source plugs in behind a `NodeStats` struct.

4. **SVG connector geometry can't be pixel-exact from the server.** rwire renders server-side; it doesn't know client layout coordinates, so an SVG `path` can't anchor precisely to a card's on-screen position. → Use a **stylized connector** (fixed-geometry SVG overlay in a known two-column grid, or CSS pseudo-element/border connectors) rather than geometric anchoring. Acceptable and good-looking; just not measured. (rwire *does* support `El::Svg/Path/Line` + `At::D/ViewBox/Stroke`, dynamic `d` — `protocol/opcodes.rs`, `attr_tokens.rs`.)

5. **The app is currently 100% read-only** (no `#[handler]`, no inputs — confirmed). This feature introduces the first interactivity *and* the first time the monitor takes write actions (spawning processes). Scaffolding + safety matter (see Risks).

---

## Data model

Add explicit vendor + device identity to the existing card (today `GpuCard {name, metrics}` has neither — vendor is only implicit in which backend produced it).

```rust
// snapshot.rs / gpu/mod.rs
enum Vendor { Nvidia, Amd }          // CPU handled separately (host card)
struct GpuCard {
    name: String,
    vendor: Vendor,                  // NEW — tag at collection time
    device_ordinal: u32,             // NEW — CUDA index / HIP ordinal for --devices
    metrics: GpuMetrics,
}
```
- NVIDIA: ordinal = parsed `index` (`gpu/nvidia.rs`). AMD: ordinal = numeric suffix of the `cardN` JSON key (`gpu/rocm.rs`). Both feed flambeau `--devices`.

New **`#[storage(shared)]`** types (multiple shared types are supported — same pattern as `Metrics`):

```rust
#[derive(State, Default)] #[storage(shared)]
struct Selection {
    // device keys currently selected, e.g. "nvidia:0", "amd:0"; FSM enforces one vendor
    selected: Vec<String>,
}

#[derive(State, Default)] #[storage(shared)]
struct LlmNodes { nodes: Vec<LlmNode> }

#[derive(Clone, Default)]
struct LlmNode {
    id: String,                 // stable key (counter/uuid)
    material: MaterialClass,    // Nvidia | Amd | Cpu (one class per node)
    devices: Vec<u32>,          // ordinals captured at creation (empty for CPU)
    engine: Option<EngineKind>, // chosen in the node card (Flambeau|LlamaCpp|Vllm)
    common: CommonConfig,       // engine-agnostic fields (model, ctx, kv, conc, port…)
    native: NativeConfig,       // per-engine flag set ("all configurable")
    raw_args: String,           // escape hatch appended verbatim
    preset: Preset,             // Balanced | Throughput | LowVram | Custom
    phase: NodePhase,           // Configuring | Starting | Running | Stopped | Error(String)
    port: u16,                  // allocated (e.g. 8090 + idx)
    stats: NodeStats,           // unified stats (see Stats adapters)
}
```

The node is a generic **engine instance**; `engine` is chosen inside the node card (see
UX). `CommonConfig`→per-engine flags via the unified mapping table (Multi-engine section);
`NativeConfig` exposes each engine's full native flag set; `raw_args` covers the rest.
flambeau's native set mirrors `ServeArgs` (`crates/cli/src/main.rs:381`): `mesh_mode`,
`tp_size`, `pp_size`, `layer_split`, `inflight_slots`, `prefill_ubatch`, `paged_kv`,
`max_queue_depth`, `decode_batch_window_us`, `no_gpu_sampler`, `no_batched_decode`,
`prefix_cache(+max_gb)`, `default_system`, … (llama.cpp / vLLM native sets per their tables).

---

## Selection state machine (hardware-first)

Three **material classes**: `Nvidia`, `Amd`, `Cpu`. A node is backed by **one class** (each
engine backend is single-vendor, and GPU vs CPU are different backends). Engine is chosen
*after* in the node card (UX: hardware-first). CPU **is** node-backable now — via llama.cpp or
vLLM — so it's a first-class selectable class (no longer dead).

- **States:** `Empty` → `ClassLocked(Nvidia|Amd|Cpu)` once ≥1 item of a class is selected →
  enables the "+" card.
- **Rules:**
  - Selecting an item of class X when `Empty` → `ClassLocked(X)`; the **other two classes dim**
    (`St::Opacity50` + pointer-events-none).
  - Multi-select within the locked class (multi-GPU node). CPU is a single "host" item.
  - Deselect all → `Empty`, all classes re-enabled.
  - "+" creates a node carrying `material = X` + selected device ordinals; the node card's
    engine picker then offers only engines compatible with **X × host-capability × opted-in
    executables** (e.g. AMD-MI50 → flambeau + llama.cpp, not vLLM).
- **Where it lives:** server-side `Selection` state + `#[handler]`s (`toggle_select`), since it
  gates a server action (node creation). Optionally mirror to a client-side `Selector` for the
  instant ring highlight (rwire `action.rs`) — start server-side for correctness.

---

## UI structure (validated UX — 2026-06-16)

Validated decisions: **hardware-first** flow (engine chosen in the node), **inline-expanding
node cards** (no drawer/modal), **presets + common + advanced + raw** config density.

Two-column grid (`St::DisplayGrid`/`GridCols2` or flex row), `PositionRelative` parent so a
connector overlay can sit between:

- **Left column** — reuse `node_card`/`gpu_card`/`cpu_card`. Add: a click handler per card
  (`toggle_select`), a **bright ring** when selected (`St::RingAccent` / box-shadow), and
  dimmed/disabled styling when its class is locked out.
- **Connector overlay** — absolutely-positioned `El::Svg` (`Inset0`, `PointerEventsNone`)
  drawing stylized hollow paths from selected cards toward the "+" card (fixed geometry per
  finding #4). Hollow = `fill:none; stroke; stroke-dasharray`.
- **Right column** — the LLM node column:
  - A **hollow "+" card** (dashed border, centered "+") — active only in `ClassLocked`;
    `on_click → add_node` (snapshots the selection into a new `LlmNode`, allocates a port).
  - One **inline-expanding node card per `LlmNode`** (reactive `Vec` via `iter_with_ref` +
    `on_ref` — `item_ref.rs`); the card grows to show a state-dependent body:
    - `Configuring`: **engine picker** (compatible engines only, per the FSM) → then
      **Preset** dropdown (Balanced/Throughput/LowVram/Custom) + a few **Common** fields
      (model, ctx, kv, max-concurrency, devices shown read-only) + collapsible **Advanced**
      (the chosen engine's native flags) + collapsible **Raw args**. Built from rwire-components
      `Input`/`Slider`/`Checkbox`/`DropdownMenu`; values via `EventContext.field/text`. Plus
      **Launch** + **Delete**. Changing the preset rewrites Common/Advanced defaults; switching
      engine re-renders the native panel.
    - `Starting`: spinner + tail of startup log; waiting for `/health`.
    - `Running`: the **same card** shows live **stats meters** (prefill tok/s, decode tok/s,
      KV/context usage, running/waiting) + **Stop**.
    - `Stopped`/`Error`: message + **Restart**/**Delete**.

---

## flambeau supervisor (the heavy backend)

A process registry + supervisor thread, owned in `main.rs` alongside the existing poller, keyed by `node.id`.

- **Launch** (`launch_node` handler → supervisor): pick the binary by vendor (`flambeau-hip`/`flambeau-cuda`, paths from config/env — Decision #2), build argv from `FlambeauConfig` (`serve --model … --devices <ordinals> --port <p> …`), `std::process::Command` with captured stdout/stderr, store the `Child` in a registry (`HashMap<String, Child>` behind a Mutex; NOT in shared State — handles aren't Clone/serializable).
- **Readiness**: read child stdout for the `"serving"` line (`serve_common.rs:316`) and/or poll `GET /health`; flip `phase` to `Running` via `update_shared::<LlmNodes>`. Model load is 5–15 s — show `Starting` meanwhile.
- **Ports**: allocate per node (base 8090 + index, skip in-use). flambeau binds `0.0.0.0:<port>`.
- **Stop/Delete**: SIGTERM the child (flambeau handles it — `serve_common.rs:327`), reap, update phase. Delete removes from `Vec` + registry.
- **Stats poller**: per running node, a loop (interval ~1 s) that pulls the stats source (Decision #3 → `/metrics` ideally) and writes `node.stats` via `update_shared` → cards re-render live (same reactive path the hardware poller already uses).

---

## Phasing (each milestone independently demoable)

1. **Selection FSM + visuals** — ✅ **DONE & verified** (2026-06-16). Added `Vendor`
   to `gpu/mod.rs`; combined shared state `App { hw: HardwareSnapshot, selection:
   Selection }` in `snapshot.rs` (poller writes `hw` only, preserving selection);
   `GpuCard` gained `vendor`/`ordinal` + `key()`/`class()`; `MaterialClass` (incl.
   CPU) + single-class-lock `Selection::toggle`. `ui.rs`: two-column layout, the
   `selectable` wrapper (bright frost ring + dim/disable other classes via inline
   `Style`), the hollow dashed "+" card that activates with a class-specific hint,
   and the `toggle_select` handler (`data-key` → `ctx.data`). All three classes
   work (CPU node-backable). Verified live: click rings the card, dims the other
   two classes, "+" → "New {class} node". 21 app tests, clippy clean.
   *Note:* selection ring/dim use inline `Style` (data-driven per-card state),
   not `St` class tokens — same rationale as `core_bars`.
2. **Node column scaffolding + config-form foundation** — ✅ **DONE & verified**
   (2026-06-16). `App.nodes: Vec<LlmNode>` + `next_node_id`; `EngineKind` +
   host-aware `engines_for(class)`; "+" card (`#plus-card`, `add_node` snapshots +
   clears selection); reactive node cards (`iter_with_ref`): material/device
   summary, **engine picker** (compatible engines only — verified vLLM hidden for
   AMD/MI50, selected highlighted), **Delete**, and — once an engine is picked — a
   **config form** with **Model** + **Context length** text inputs and a disabled
   **Launch** placeholder. Handlers `add_node`/`delete_node`/`set_engine`/
   `update_model`/`update_ctx`.
   - **Option A landed (rwire change):** added `SharedServerState::update_shared_changed(ChangeSet, f)`;
     the poller now broadcasts only `ChangeSet::from_fields(&[App::FIELD_HW])`. The UI
     is split into two synced regions — `render_hardware` (deps `hw`+`selection`)
     and `render_nodes` (deps `selection`+`nodes`) — so the 1 s hw tick does **not**
     re-render the node column. Combined with rwire's focus/cursor restore-by-id,
     **typing in a node input is smooth and survives ticks** (verified: typed text
     persisted across 3+ s of ticks).
   - **Layout:** two columns are a **CSS grid** (`minmax(280px,360px) 1fr`) on the
     static shell, because each synced region is wrapped in an inline `span` —
     grid blockifies them into tracks so columns fill correctly.
   - **Model discovery (Decision #4) DONE:** new `models.rs` scans a configurable
     `MODELS_DIR` (default `./models`) at startup → `App.models`. Classifies
     `*.gguf` files (→ `ModelFormat::Gguf`) and HF dirs with `config.json`
     (→ `HfDir`). The Model field is a **`<select>` filtered by
     `EngineKind::accepts_format`** (flambeau/llama.cpp → GGUF; vLLM → HF dir) —
     verified: a llama.cpp node lists the GGUFs, a vLLM node lists the HF dir.
   - **Still pending (config form depth):** Preset dropdown, Advanced (native
     per-engine flags), Raw-args, read-only devices display; and the stylized SVG
     connector overlay.
3. **Engine launch/stop** — ✅ **DONE & verified live** (2026-06-16). New `launcher.rs`:
   - **Executable registry** `resolve(engine, backend)` — env-configurable
     (`LLAMACPP_HIP_BIN`/`_LIBS`, `FLAMBEAU_HIP_BIN`/`_LIBS`, `*_CUDA_BIN`, `VLLM_*`,
     …) with **verified host defaults** for the two local builds: llama.cpp gfx906
     (`…/llama-cpp-gfx906-turbo/build/bin/llama-server`, `LD_LIBRARY_PATH=<dir>:/opt/rocm/lib`)
     and flambeau (`/artefact/flambeau/target/release/flambeau`, `LD_LIBRARY_PATH=/opt/rocm/lib`).
     Everything else is opt-in (env only) → launch yields `Error("no executable…")`.
   - **Per-engine command builder**: device mask via env (`HIP_VISIBLE_DEVICES`+`ROCR_…`
     for AMD, `CUDA_VISIBLE_DEVICES` for NVIDIA); llama.cpp `-m/--port/--metrics/-ngl 99`
     (`-ngl 0` for CPU, `--split-mode layer` for multi-GPU); flambeau `serve --model
     --devices hip:N --ctx-cap`; vLLM `serve --tensor-parallel-size --max-model-len`.
     Child stdout/stderr → `/tmp/llm-node-<id>.log`.
   - **Supervisor**: a thread + global mpsc channel; handlers send `Launch`/`Stop`
     (not spawn directly). Owns the `HashMap<id, Child>`. Readiness = off-thread
     `/health` poll (200) → `update_shared_changed(FIELD_NODES, …)` sets `Running`
     (or `Error` on timeout). Stop = `child.kill()` + reap → `Stopped`.
   - **UI**: `LlmNode` gained `phase: NodePhase` + `port`; node card is now
     phase-driven (Configuring form → Starting → Running+Stop → Stopped/Error+Retry)
     with a colored phase badge. Handlers `launch_node`/`stop_node`.
   - **Verified live**: an AMD llama.cpp node launched `Qwen3.5-9B-Q3_K_S.gguf` on
     **MI50 card0** → `/health` 200 → node `Running`, `/v1/models` served the model,
     and the dashboard showed **card0 VRAM 12.8/16 GiB** (card1 idle → device
     targeting confirmed); SIGTERM freed the VRAM.
   - **Orphan cleanup — ✅ DONE & verified** (2026-06-16): each child is spawned with
     `pre_exec` → `prctl(PR_SET_PDEATHSIG, SIGTERM)` (Linux), so the kernel SIGTERMs
     the engine the instant the monitor dies — covers normal exit, SIGTERM, SIGKILL,
     crash, and stale children on restart (what a userspace handler can't). The
     supervisor also kills its children when the channel closes. Verified: killing the
     monitor freed the MI50 VRAM (12.8 GB → 0). (Reaped children may briefly show as
     zombies until init collects them; they hold no VRAM.) Needs `libc` dep.
   - **Liveness — ✅ DONE & verified** (2026-06-16): the supervisor loop uses
     `recv_timeout(2 s)`; each idle tick `reconcile`s — `try_wait()` on every child
     turns an unexpected exit into `Error("process exited (…)")`, and a not-yet-ready
     child becomes `Running` once `/health` is 200. This unified readiness+liveness
     loop also removed the per-launch readiness-thread race on relaunch. Verified:
     SIGKILLing the engine flipped the node to `error: process exited (signal: 9
     (SIGKILL))` with a Retry button, VRAM freed.
   - **Remaining gap (→ Phase 5):** Stop uses `child.kill()` (SIGKILL); could SIGTERM
     via `libc` for graceful drain (orphan path already uses SIGTERM via PDEATHSIG).
4. **Live stats — ✅ DONE & verified LIVE (2026-06-16):** unified `NodeStats {prefill_tps,
   decode_tps, kv_usage, running, waiting}` (all `Option`) on `LlmNode`, scraped per ready
   node inside the same 2 s reconcile loop (no extra thread). `scrape_stats(engine, port,
   prev)` → `http_get` `/metrics` → `parse_metric` (strips prefix, tolerates `{labels}`,
   takes the last whitespace token):
   - **llama.cpp**: `llamacpp:prompt_tokens_seconds`→prefill, `predicted_tokens_seconds`→
     decode, `requests_processing`→running, `requests_deferred`→waiting. `kv_usage` = `None`
     (llama.cpp `/metrics` exposes no KV gauge; `/slots` would, deferred to Phase 5).
   - **vLLM**: `vllm:kv_cache_usage_perc`→kv_usage, `num_requests_running`/`_waiting`, and
     tok/s via `rate()` over `prev:(prompt_total, gen_total, Instant)` between ticks.
   - **flambeau**: no `/metrics` endpoint yet → no stats (cross-repo gap, Decision #3).
   `set_stats` writes via `ChangeSet::from_fields(&[App::FIELD_NODES])`. `stats_view` renders
   chips "prompt {x} tok/s" / "decode {y} tok/s", a KV meter **only when `Some`**, and
   "requests: {r} running · {w} queued". **Verified live:** AMD llama.cpp Qwen3.5-9B node
   fired a completion → card showed "prompt ~75–96 tok/s", "decode 48 tok/s", "requests:
   0 running · 0 queued", refreshing each 2 s tick (matched curl'd `/metrics`). Temp
   `AUTOLAUNCH` hook used then removed; rebuilt clean (clippy 0, 21 tests), restarted fresh.
5. **Polish** (in progress):
   - **Param-form REDESIGN — ✅ DONE & verified LIVE (2026-06-17):** the preset feature
     was removed and replaced with a **dynamic flag–value form** plus node save/load/
     duplicate. Each node holds `flags: Vec<FlagEntry { flag, value, custom_flag,
     raw_value }>`. The config body shows: engine picker → Model → Context (first-class,
     engine-mapped) → **read-only "Managed (auto)" chips** (the flags the launcher sets:
     `-m`, `--port`, `--metrics`, `-ngl`, ctx, device mask) → **Flags** list with
     `+ Add flag` → **Save as profile** → Launch; the header gains **Duplicate**.
     - Each flag row (stacked layout) is a **Flag** cell over a **Value** cell. The Flag
       cell is a grouped catalog `<select>` (Sampling / Template / Memory / MoE-Speculative
       / Vision) ending in **"Custom…"** → free-text flag. The Value cell **always offers a
       dropdown of analyzed, pertinent options** for the chosen flag, plus a **"Custom…"**
       entry that swaps in a free-text input; bare toggles (`--jinja`, `--no-mmap`,
       `--mlock`) show "(no value)".
     - The catalog (`snapshot::flag_catalog(engine)` → `FlagSpec { flag, label, group,
       value: ValueSpec }`, `ValueSpec = None | Choice(&[&str])`) is unsloth-informed for
       llama.cpp (Gemma 4 / Qwen3.6 / MTP — see `docs`/memory `unsloth-run-params`):
       `--temp`(1.0/0.7/0.6…), `--top-p`(0.95/0.8), `--top-k`(64/20), `--min-p`,
       `--presence-penalty`(1.5), `--samplers`, `--seed`(3407), `--chat-template-kwargs`
       (`enable_thinking` on/off), `--flash-attn`, `--cache-type-k/v`, `-ot`
       (`.ffn_.*_exps.=CPU`), `--spec-type draft-mtp` + `--spec-draft-n-max`, `--mmproj`,
       etc. vLLM and flambeau catalogs follow each binary's `--help`.
     - **Save / Load / Duplicate** (replacing presets for reuse): `profiles.rs` serializes
       a node config to JSON under `NODE_PROFILES_DIR` (default `./profiles`); per-node
       Duplicate (in-memory clone) + Save-as; a top-of-column "Load profile" `<select>`
       spawns a node from a saved profile; profiles load at startup.
     - `LaunchSpec.flags` (replacing the old advanced/extra_args) is appended to the
       command verbatim after the managed args (bare flag when value is empty). Tests:
       `flag_catalog_has_unsloth_sampling_flags`, `build_command_appends_dynamic_flags`
       (23 total, clippy 0). Verified live (value dropdowns, managed chips, Duplicate).
   - **UX/UI overhaul — ✅ DONE & verified LIVE (2026-06-17):** addressed "inputs ugly /
     layout weak / mobile off". UI-only (ui.rs). **Responsive**: the shell switched from
     CSS grid to **flex-wrap** (rwire has no `@media`) — hardware rail (`flex 1 1 260px`,
     max 380px) and node column (`flex 100 1 460px`, `min-width:0`) sit side-by-side on wide
     screens and **stack on narrow ones**. **Controls** restyled via `field_style` /
     `input_box_w` / `text_field_w`: monospace, padded, rounded, **focus ring**
     (`.focus([RingFocus])`) + hover-border (`.hover([BorderColorAccent])`), and **capped
     widths** (ctx 12rem, value 22rem, selects 32–34rem) so short values no longer stretch
     full-bleed. **Flag rows** are refined-stacked bordered blocks (flag select over value +
     adjacent `×`). The managed-chips row was replaced by a **monospace "Effective command"
     preview** (`effective_command`) showing the exact assembled command (device-mask env
     prefix + engine exec + managed args + ctx + user flags; mirrors `build_command`). A
     bottom **action bar** groups profile-name+Save (left) and Launch (right). Kept the Nord
     palette. clippy 0, 23 tests. *(Mobile reflow logic is sound but unverified visually —
     the MCP screenshot tool has no viewport control.)*
   - **Config-form depth — ✅ DONE & verified LIVE (2026-06-16)** *(superseded by the
     redesign above; kept for history):* the `Configuring` body
     now has, below the engine picker, a **Preset** `<select>` (per-engine quick configs
     that reset the form then set context + advanced flags — `Balanced`/`Long context`/
     `Max throughput`/`Low VRAM` for llama.cpp, etc.), a **read-only devices** caption,
     the **Model**/**Context** common fields, and a collapsible **▸/▾ Advanced** section.
     Advanced is a *curated, schema-driven* set of native flags per engine
     (`snapshot::advanced_fields(engine)` — verified against each binary's `--help`:
     llama.cpp `--threads/--batch-size/--ubatch-size/--flash-attn(on|off|auto)/--parallel/
     --cache-type-k`; flambeau `--mesh-mode(pp|tp|hybrid)/--inflight-slots/
     --prefill-chunk-tokens/--max-queue-depth`; vLLM `--gpu-memory-utilization/
     --max-num-seqs/--dtype/--quantization`) rendered as text/int inputs or `<select>`s,
     plus a **Raw args** escape-hatch field appended verbatim. `LlmNode` gained
     `preset/show_advanced/advanced: Vec<(String,String)>/extra_args`; helpers
     `adv()/set_adv()/apply_preset()`. `build_command` appends curated advanced flags
     (matched against the per-engine schema, so stale/cross-engine flags are ignored)
     then raw args **last** (user overrides win). Switching engines clears
     model/preset/advanced (formats differ). Handlers: `apply_preset/toggle_advanced/
     update_advanced/update_extra_args`. **Verified live** (DEV_FORM prime, now removed):
     "Long context" preset set ctx=32768 + flash-attn=on, advanced inputs rendered with
     engine defaults as placeholders. Tests: `apply_preset_resets_then_sets_fields`,
     `set_adv_replaces_existing_value`, `build_command_appends_advanced_then_raw_args`
     (24 total, clippy 0).
   - **Remaining:** SVG connector overlay; Stop → SIGTERM (graceful drain) not SIGKILL;
     flambeau `/metrics` + llama.cpp `/slots` KV; auth/exposure for public write actions;
     error states, port conflicts, multi-node.

---

## Risks / constraints

- **Write actions from a monitor**: spawning processes is a privilege/safety jump from read-only. Bind stays `0.0.0.0:7778` (publicly reverse-proxied) — consider auth/guard before exposing launch controls, or restrict to a trusted network. **Surface to user.**
- **Resource contention**: launching flambeau on GPUs the monitor is also reading; VRAM OOM if devices already loaded. Show current VRAM in the card; let `--ctx-cap` mitigate.
- **Two binaries must exist** and match the host GPUs (gfx906 / sm_86 here). Build/availability is a prerequisite (Decision #2).
- **Stats gap** (Decision #3) is the main unknown; everything else is standard.
- **Connector fidelity** is stylized, not measured (finding #4).
- rwire edition 2021 (lib) / app edition 2024; reuse the shared-state + poller pattern already proven in this app.

---

## Multi-engine support (flambeau / llama.cpp / vLLM)

All three are OpenAI-compatible servers. **Critical shared fact: every engine picks its
compute backend at build/install time, not runtime** — flambeau (cargo feature
`hip_serve`/`cuda_serve`), llama.cpp (CMake `GGML_CUDA`/`GGML_HIP`/`GGML_VULKAN`/CPU),
vLLM (wheel per `VLLM_TARGET_DEVICE` cuda/rocm/cpu). So "an engine on a vendor" is a
specific prebuilt **executable**, which is why executables must be **opt-in** (registry below).

### Capability matrix (engine × material), with this host noted
| | NVIDIA (RTX 3090, sm_86) | AMD (2× MI50, gfx906) | CPU |
|---|---|---|---|
| **flambeau** | ✅ cuda_serve | ✅ hip_serve (gfx906 supported) | ❌ no CPU backend |
| **llama.cpp** | ✅ CUDA | ✅ HIP or Vulkan (gfx906 ok) | ✅ `-ngl 0` / CPU build |
| **vLLM** | ✅ CUDA | ⚠️ ROCm wheel is **MI200/MI300+ only — NOT gfx906/MI50** | ✅ CPU wheel (slow) |

This **resolves the earlier CPU question**: CPU *is* node-backable — via llama.cpp or vLLM
(not flambeau). And compatibility is now per **(engine, material, host build)**, e.g. the
MI50s can run flambeau + llama.cpp but **not** vLLM.

### Executable opt-in registry (config + env)
The launcher assumes nothing is installed. A config declares available executables; only
opted-in `(engine, backend)` pairs — intersected with the capability matrix and detected
hardware — are offered in the UI. Sketch:
```
[[engine]] kind=flambeau backend=cuda exec="/opt/flambeau/flambeau-cuda"
[[engine]] kind=flambeau backend=hip  exec="/opt/flambeau/flambeau-hip"
[[engine]] kind=llamacpp backend=cuda exec="/usr/local/bin/llama-server-cuda"
[[engine]] kind=llamacpp backend=hip  exec="/usr/local/bin/llama-server-hip"
[[engine]] kind=llamacpp backend=cpu  exec="/usr/local/bin/llama-server-cpu"
[[engine]] kind=vllm     backend=cuda exec="vllm"           base_env={…}
[[engine]] kind=vllm     backend=cpu  exec="vllm-cpu"       base_env={…}
```
(Optional PATH probing pre-populates it.)

### Env-var rationalization (unified scheme)
**Principle: device masking via env; everything else via CLI flags.** Flags win over env in
all three engines, and masking must be process-wide *before* device enumeration. The
launcher builds each child's env **explicitly** (never inherits ambient masks).
- **Per-node device mask** (set on the child process):
  - NVIDIA → `CUDA_VISIBLE_DEVICES=<ordinals>`
  - AMD → `HIP_VISIBLE_DEVICES=<ordinals>` (+ `ROCR_VISIBLE_DEVICES` low-level)
  - CPU → vLLM: `VLLM_CPU_KVCACHE_SPACE`, `VLLM_CPU_OMP_THREADS_BIND`; llama.cpp: `-ngl 0` flag
- **Per-engine base env** (static, from the registry): `LD_LIBRARY_PATH`/`CUDA_HOME`/ROCm paths,
  `HF_HOME`, `VLLM_CACHE_ROOT`, pinned `VLLM_ATTENTION_BACKEND`, API keys.
- **All config knobs → CLI flags** (per-launch, deterministic). Do *not* use the
  `LLAMA_ARG_*` / `FLAMBEAU_*` env forms for per-node config.
- **Footguns encoded in the builder**: vLLM `VLLM_PORT`/`VLLM_HOST_IP` are *internal* (use
  `--port`); llama.cpp api-key env is `LLAMA_API_KEY` (breaks the `LLAMA_ARG_` rule);
  llama.cpp `/metrics` is **off** unless `--metrics`; flambeau/llama.cpp default 8080, vLLM 8000
  → launcher always assigns an explicit `--port`.

### Unified config model (common concept → per-engine flag)
A **common form** (maps to all three) + an **engine-specific "native/advanced" panel**
exposing the full flag set ("all configurable") + a **raw-args** escape hatch.
| Concept | flambeau | llama.cpp | vLLM |
|---|---|---|---|
| model | `--model` | `-m/--model` | `model_tag` / `--model` |
| port | `--port` | `--port` | `--port` |
| which devices | `--devices <ord>` | mask + `--device`/`--tensor-split` | mask + `--tensor-parallel-size` |
| context length | `--ctx-cap` | `--ctx-size` | `--max-model-len` |
| kv-cache dtype | `--kv f16/q8` | `--cache-type-k/-v` | `--kv-cache-dtype` |
| max concurrency | `--inflight-slots` | `--parallel` | `--max-num-seqs` |
| prefill batch | `--prefill-ubatch` | `--batch`/`--ubatch` | `--max-num-batched-tokens` |
| prefix cache | `--prefix-cache` | (no stable flag) | `--enable-prefix-caching` (on) |
| tensor-parallel | `--mesh-mode tp --tp-size` | `--split-mode row` | `--tensor-parallel-size` |
| pipeline-parallel | `--mesh-mode pp --pp-size` | `--split-mode layer` | `--pipeline-parallel-size` |
| eager / no graph | (n/a) | (n/a) | `--enforce-eager` |

### Stats adapters → unified `NodeStats { prefill_tps, decode_tps, kv_usage_pct, running, waiting, ttft_ms }`
- **llama.cpp**: `/metrics` (needs `--metrics`) → `llamacpp:prompt_tokens_seconds`,
  `llamacpp:predicted_tokens_seconds`, `llamacpp:requests_processing/deferred`; **KV/context
  usage from `/slots`** (the metric ratio was removed in the refactor); per-request `timings`.
- **vLLM**: `/metrics` → `vllm:kv_cache_usage_perc`, `vllm:num_requests_running/waiting`,
  `rate()` of `vllm:prompt_tokens_total` / `vllm:generation_tokens_total`,
  `vllm:time_to_first_token_seconds`, `vllm:inter_token_latency_seconds`.
- **flambeau**: still the **gap** (no `/metrics`) → add one (recommended) or estimate; same struct.
- **Lifecycle (unified supervisor)**: all expose `/health` (llama.cpp returns 503 while loading;
  vLLM 200 when ready); all handle SIGTERM (llama.cpp: a *second* SIGINT force-kills); engine
  ready can take minutes (model load) — show `Starting`.

This makes the node a generic **"engine instance"**: `{engine_kind, backend, exec, devices,
common_cfg, native_cfg/raw_args, port, phase, stats}`. The supervisor, arg-builder, and
stats-scraper dispatch on `engine_kind`.

## Presets (concrete)

A preset pre-fills Common + Advanced fields; the user can then tweak (which flips it to
`Custom`). Multi-GPU parallelism is also preset-driven (see last row). Values below are the
*intent* mapped to each engine's real flags.

| Knob (intent) | **Balanced** | **Throughput** | **Low-VRAM** |
|---|---|---|---|
| context length | model default (cap ~8k) | model default | small (~4k) |
| KV cache dtype | f16 | q8/fp8 (fit more) | q8/fp8 |
| max concurrency | moderate | high | low (1–2) |
| prefix cache | on (where available) | on | on |
| GPU mem headroom | default | aggressive | conservative |

Per-engine realization:
- **llama.cpp** — Balanced: `--ctx-size 8192 --parallel 4 -fa auto --cache-type-k/v f16 --batch 2048 --ubatch 512 --metrics`. Throughput: `--parallel 8 --batch 4096 --ubatch 1024 -fa on --cache-type-k/v q8_0`. Low-VRAM: `--ctx-size 4096 --parallel 2 --cache-type-k/v q8_0 -fa on` + partial offload (`-ngl <fit>`; CPU class → `-ngl 0`).
- **vLLM** — Balanced: `--max-model-len 8192 --max-num-seqs 128 --gpu-memory-utilization 0.90` (chunked-prefill + prefix-caching are on by default in V1). Throughput: `--max-num-seqs 256 --max-num-batched-tokens <high> --gpu-memory-utilization 0.95`. Low-VRAM: `--gpu-memory-utilization 0.80 --max-model-len 4096 --max-num-seqs 32 --kv-cache-dtype fp8 --enforce-eager`.
- **flambeau** — Balanced: `--inflight-slots 4 --prefill-ubatch 512 --kv f16`. Throughput: `--inflight-slots 12 --prefill-ubatch 1024 --kv q8 --prefix-cache --max-queue-depth 32`. Low-VRAM: `--ctx-cap <small> --kv q8 --paged-kv <N> --inflight-slots 2`.

**Multi-GPU strategy (when >1 device of the class is selected):** Balanced → pipeline/layer
split (memory-friendly): llama.cpp `--split-mode layer`, flambeau `--mesh-mode pp --pp-size N`,
vLLM `--pipeline-parallel-size N`. Throughput → tensor split (latency-friendly): llama.cpp
`--split-mode row --tensor-split …`, flambeau `--mesh-mode tp --tp-size N`, vLLM
`--tensor-parallel-size N`. Low-VRAM → pipeline (spreads weights), single device if it fits.
Device ordinals always come from the selection (env mask + the engine's device flag).

## Connector visual

Stylized, not geometry-measured (finding #4). An absolutely-positioned SVG overlay spanning the
gutter between columns; for each selected card, a **hollow cubic-bezier** path
(`fill:none; stroke:var(--accent); stroke-width:2; stroke-dasharray:6 6`) curves from the card's
right edge to the "+" card's left edge, with a subtle animated dash-offset ("flow") and a small
hollow node (`El::Circle fill:none`) at each end. Anchors use the known two-column grid
geometry (card row index → y), so it reads as "selection feeds the +". Degrades to a simple
straight dashed line if multiple selections overlap.

```
[NVIDIA 3090] ✓ ╮
                 ╲ (hollow dashed bezier, animated)
[AMD MI50]        ╲___________  ◯┄┄┄►◯  [ + ]
[AMD MI50]                              (new node)
[CPU]
```

## Decisions

**Resolved:**
- **UX (2026-06-16, validated)**: hardware-first → engine chosen in node; inline-expanding node
  cards; presets + common + advanced + raw config density.
- **CPU**: node-backable via llama.cpp / vLLM (not flambeau) → kept as a first-class material class.
- **Model discovery (Decision #4) DONE**: `MODELS_DIR` (default `./models`) scanned at startup →
  `<select>` filtered by engine format (GGUF vs HF dir). HF-repo-id free-text could be added later
  as a fallback option.

- **Executable opt-in registry (Decision #1) DONE**: `launcher::resolve(engine, backend)` reads
  env vars with verified host defaults for llama.cpp-gfx906 + flambeau (HIP); other pairs are
  env-only opt-in. Live-launch verified.

**Still needed before build:**
2. **Stats source for flambeau**: add a `/metrics` endpoint (recommended; llama.cpp & vLLM
   already expose Prometheus) vs probe-estimate vs log-parse.
3. **Exposure/auth** for launch controls given the public `0.0.0.0:7778` bind (this UI takes
   write actions / spawns processes).
4. ~~Model discovery~~ — **DONE** (see Resolved above).
