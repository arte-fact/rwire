# Handoff — rwire limitations plan (resume after compaction)

## Context
- **Goal:** rewrite `llama-monitor` using the `rwire` framework, then harden rwire.
- **App:** `/artefact/llama-modnitor-rwire` (edition 2024). Depends on rwire via path.
- **Framework:** `/artefact/rwire` (edition **2021** — no `let`-chains in lib code).
- **Serving:** app binds `0.0.0.0:7778`, reverse-proxied to **https://llama-monitor.raptors.pizza/**. The agent's shell can't reliably reach localhost (sandbox netns); verify with the MCP tool `mcp__claude_ai_Web_Search_Hacks__screenshot` on that URL.
- **Hardware here:** 2× AMD Instinct MI50/MI60 (gfx906, `rocm-smi` at `/opt/rocm/bin`, not on PATH) + 1× NVIDIA RTX 3090; 32-core AMD CPU (`k10temp`).
- Always invoke the **ms-rust** skill before writing Rust.

## Status: ALL 7 plan tasks DONE & verified
Everything is green: rwire **337 tests** (incl. doctests), app tests, **clippy clean** both, full workspace builds, live site renders 4 cards.

**#5 (tree-shaking) and #6 (nested renderers) closed post-compaction:**
- **#5**: implementation was already in place (macro scans `El/St/Ev/At/Av` across all branches → `TokenInventory` → `collect_from_inventory` merges all 7 fields). Added end-to-end test `libs/rwire/tests/tree_shaking_branches.rs` (untaken branch's `El::Table`/`St::BgApp` survive). Documented the residual gap (plain helper fns are opaque to the textual scan) on `TokenInventory` and in `docs/reactivity.md`. Monitor keeps priming (its `core_bars`/`gpu_card` are plain helpers) — correct + gives real first-paint data.
- **#6**: built `examples/nested` (a `#[renderer]` nested in another, different state), served on 7778, drove via MCP browser: inner×3 → outer×1 → inner×1 ⇒ **outer=1, inner=4** (old bug would leave inner stale at 3). Confirms the nested wrapper survives parent re-render and stays updatable. Monitor restored on 7778 afterward.

--- original handoff below (historical) ---

### DONE
- **#1 SO_REUSEADDR** — `server.rs::bind_reusable()` (uses `socket2 = "0.5"` dep). Immediate rebind after kill works (no retry loop needed anymore).
- **#2 Stale docs** — removed false nested-renderer comment in `examples/todo-combined/src/main.rs`; added `docs/reactivity.md`.
- **#3 First-class shared state** — `StorageType::Shared` + `#[storage(shared)]`. Type-discovered (NOT a runtime registry): `SyncedElement`/`HandlerFn` carry `storage_type()`/`table_name()`; `ElementBuilder::synced_with_storage::<S: State+Default>` (emitted by `#[renderer]` macro) reads `S::STORAGE_TYPE`/`TABLE_NAME`. One resolver `server.rs::shared_cache_key(storage, table, session)` → Memory=None, Persisted=`"{table}:{session}"`, Shared=`"__shared__:{table}"`. Helper `shared_persisted_keys(handlers, synced, session)` drives subscription + states-map override at all render sites. Off-connection mutation: `SharedServerState::update_shared::<T>(f)`. Also **fixed the never-called cross-tab broadcast** in the persisted handler path. Monitor migrated: `snapshot.rs::Metrics` is `#[storage(shared)]` (no more `OnceLock<Arc<RwLock>>`). Tests: `server.rs` `update_shared_mutates_and_broadcasts`, `shared_cache_key_per_storage_type`.
- **#4 Field-level fine-grained deps** — `State` derive emits `pub const FIELD_<NAME>: u8` (positional; first 64 effective vs ChangeSet u64 mask). `#[renderer]`/`#[handler]` use `infer_accessed_fields(block, param)` (syn visitor in `rwire-macros/src/lib.rs`): collects `param.field` accesses; ANY opaque use of `param` (passed to fn, `&param`, `param.method()`, closure capture) ⇒ returns None ⇒ fallback `RendererDeps::always()` / `ChangeSet::all()`. Over-approximation = correct in both directions. `Theme` got manual `FIELD_MODE/RADIUS/STYLE/PALETTE` consts (hand-written State impl in `theme.rs`). Test: `tests/field_deps.rs`.
- **#7 Consistency** — verified N/A on main: async-std only (tokio was canvas-branch, removed). Lock pattern `unwrap_or_else(|e| e.into_inner())` is consistent/acceptable.

Earlier same-session fixes (pre-plan): `wss://` scheme in `capsule_gen.rs` client runtime (was hardcoded `ws://`, broke on HTTPS).

## REMAINING

### #5 Robust tree-shaking (large)
- **Problem:** capsule's used element/CSS set is collected by rendering the root ONCE with **default state** at startup (`server.rs::run()`, ~the `extract_renderers` + `default_states` + `collect_symbols_multi`/`emit_multi` block). Element types (`El::*`) and component-internal tokens that only appear in a conditional branch NOT taken at default state are missed.
- **Already mitigated:** the `#[renderer]` macro statically scans the body for `St` tokens across all branches via `rwire-macros/src/token_scanner.rs` → `TokenInventory` (so St tokens in untaken branches ARE captured). Also: `Server::routes()` (multi-view), `CapsuleConfig::extra_elements()/extra_styles()`, and "prime state before run()" (the monitor primes via `update_shared` so startup analysis — which now reads `shared_cache` for shared types — walks real data).
- **Proposed approach:** extend `token_scanner` + the renderer-macro inventory to also capture `El::*` element types referenced in the renderer body, and merge them into the capsule's `used_elements` at generation (`capsule_gen.rs`). Closes the element-type gap without executing branches. (Component-internal tokens for untaken branches remain a residual gap — components are opaque to the macro scan; document it.)
- **Files:** `libs/rwire-macros/src/token_scanner.rs`, `libs/rwire-macros/src/lib.rs` (renderer macro / `generate_inventory`), `libs/rwire/src/builder.rs` (`TokenInventory`, how merged), `libs/rwire/src/capsule_gen.rs`.
- **Verify:** a renderer with `if cond { el(El::Table)... }` where default-state `cond==false` should still include `El::Table` (and its CSS) in the served capsule. Then the monitor could drop the "prime before run()" reliance.

### #6 Nested-renderer browser e2e (needs browser)
- Protocol support EXISTS and is unit-tested: `CREATE_SYNCED` opcode + `libs/rwire/tests/nested_renderer.rs` (passing). The old prose claim "framework does NOT support nested renderers" is stale (already fixed in docs).
- **Remaining:** confirm at runtime in a browser that a nested synced region patches correctly on update (no "element not found", no broken DOM) — the old failure mode was "parent re-render clears wrapper children → nested wrapper destroyed".
- **Approach:** make a tiny example with a renderer nested inside another (todo-combined uses the flat-sibling workaround), run it, drive with MCP browser tools (`...__screenshot`, `...__interact`), click something that mutates the parent, confirm the nested region survives/updates. Client runtime that handles CREATE_SYNCED/GET_SYNCED is in `capsule_gen.rs`.

## Operational notes
- Build app: `cd /artefact/llama-modnitor-rwire && cargo build`. Tests: `cargo test` (app), `cd /artefact/rwire && cargo test -p rwire` (or `--workspace`).
- **Relaunch dance** (sandbox quirks): `pgrep -f` matches the shell itself and prior server pids show as zombies (state `Z`, unkillable/no-op). Find the LIVE one and kill it:
  `live=$(ps -eo pid,stat,comm | awk '/llama-modnitor/ && $2 !~ /Z/ {print $1}'); [ -n "$live" ] && kill -9 $live`
  then `nohup ./target/debug/llama-modnitor-rwire > /tmp/monitor.log 2>&1 & disown`. SO_REUSEADDR makes immediate rebind work. Run binary from the APP dir (not /artefact/rwire).
- `/tmp` scripts can be cleared between Bash calls. To check push frames, recreate a raw-socket WS client in python (HTTP upgrade with `Sec-WebSocket-Key`, read frames, count binary opcodes) — the monitor pushes ~1/s via `notify_all`.
- Verify visually: MCP `...__screenshot` on the public URL. Catching it mid-restart shows "Bad Gateway" — retry.

## Invariants / gotchas
- rwire = edition 2021: NO `if let ... && ...` let-chains. App = edition 2024 (OK).
- Shared-state instance is lazily created at connection setup AND by `update_shared`; prime before `run()` for tree-shaking.
- `update_shared` broadcasts `ChangeSet::all()`; the monitor's single `render_dashboard` is opaque (passes `state` to `cpu_card`) ⇒ `always()` ⇒ re-renders fully. Correct.
- Memory notes: `~/.claude/projects/-artefact-llama-modnitor-rwire/memory/` — `MEMORY.md` (index), `rwire-known-limitations.md` (fixed vs open), `rwire-live-updates.md`, `project-rewrite.md`.
