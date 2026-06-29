# rwire Audit Remediation Roadmap

Tracking document for resolving findings from the whole-codebase audit (correctness,
security, code quality, performance). Each item is self-contained: location, problem,
fix direction, and acceptance criteria.

**Audit date:** 2026-06-29
**Method:** 4 parallel passes over the workspace, navigated via the codebase knowledge
graph and verified against source.

## Baseline (already healthy — do not regress)

- `cargo clippy --workspace --all-targets` → **0 warnings** across all 14 crates.
- Binary protocol decoder (`protocol/decoder.rs`) is bounds-checked, length-validated,
  payload-capped at 64KB, and has no reachable panic on malformed input.
- varint encode/decode and the JS runtime opcode decoder are mutually consistent.
- Auth-token path uses `/dev/urandom` + constant-time compare; auth cookies are
  `HttpOnly; SameSite=Strict`.

## Status legend

`[ ]` todo `[~]` in progress `[x]` done `[-]` won't fix / deferred (note why)

## Progress

| Priority | Total | Done |
|----------|-------|------|
| Critical | 1 | 1 |
| High     | 3 | 3 |
| Medium   | 6 | 6 |
| Low      | 6 | 5 |
| **All**  | **16** | **15** |

**Done so far:** C1 (CSPRNG session IDs), H1 (admission control + health endpoints +
session-cache cap), H2 (session-ID validation), H3 (per-event symbol-clone elimination),
M1 (client-action bindings re-emitted on synced update), M2 (WebSocket frame-size limits),
M3 (server-side event rate limiting), L1 (deleted dead `build_synced_update`),
L2 (removed `#[allow(dead_code)]`), L4 (bounded `sent_symbols`), L5 (Secure cookie,
auto-detected from `X-Forwarded-Proto`). See per-item notes below.

---

## 🔴 Critical

### C1 — Use a CSPRNG for session IDs
- **Status:** `[x]` Done. `SessionId::generate()` now reads 16 bytes from
  `/dev/urandom` (128-bit hex id), with a time-based fallback only if the OS RNG is
  unavailable. Tests added: `test_session_id_is_128bit_hex`,
  `test_session_id_high_entropy_no_collisions` (10k ids, all unique).
  `libs/rwire/src/session.rs:40`.
- **Location:** `libs/rwire/src/session.rs:40-51`
- **Problem:** The 128-bit-looking session ID is a deterministic function of one
  nanosecond timestamp (the "random" half is `timestamp.wrapping_mul(constant)`).
  Effective entropy ~30 bits; knowing the connect time reconstructs the whole ID.
  Persisted state is keyed solely by session ID (`shared_cache_key`, `server.rs:91-97`),
  so predictable IDs enable session hijacking and cross-user read/write of persisted state.
- **Fix:** Generate from the OS CSPRNG. Reuse the existing `/dev/urandom` logic already
  present in `generate_token` (`server.rs:1071`), or add `getrandom`.
- **Acceptance:** Session IDs are cryptographically random (≥128 bits real entropy),
  unguessable from connect time; existing session/cookie tests still pass.
- **Risk:** Low. Self-contained change to one generator function.

---

## 🟠 High

### H1 — Wire in admission control & connection limits
- **Status:** `[x]` Done (except `/metrics`, deferred). `run()` builds a shared
  `ConnectionRegistry`; the WS branch in `handle_client` calls `check_admission` (total
  + per-IP) before the upgrade and rejects with `serve_unavailable` (503), and holds a
  `ConnectionGuard` for the connection's lifetime. `/health` and `/ready` are routed
  before auth/session (so probes work unauthenticated and at capacity) and now return
  JSON instead of the capsule. `cache_session` is bounded to `MAX_CACHED_SESSIONS`
  (10k), evicting the oldest on overflow. New `.config(ServerConfig)` builder method.
  Verified live: handshake → `active_connections` 0→1→0; `/health` and `/ready` return
  JSON; `/` still serves the capsule. Test added: `session_cache_is_bounded`.
  **Deferred:** `/metrics` — `metrics.rs` has no HTTP serve helper and no server-wired
  metrics registry; wiring it is its own task (see new item M6 below).
- **Location:** `registry.rs`, `config.rs`, `health.rs` (all dead at runtime); accept
  loop at `libs/rwire/src/server.rs:879-901`
- **Problem:** `ConnectionRegistry`, `ServerConfig` (`max_connections`,
  `max_connections_per_ip`, `idle_timeout`, `state_memory_limit`), and the
  `/health`·`/ready`·`/metrics` endpoints are fully implemented but **never referenced by
  `Server::run()`**. Every connection is unconditionally spawned. `GET /health` returns
  capsule HTML (contradicts `apps/rwire-docs/docs/05-advanced/config.md:124`). Compounded
  by `cache_session` storing full per-connection state for a 5-min TTL on every disconnect
  (`server.rs:357-375`, `:2077-2078`) with no cap → memory exhaustion via
  connect→receive→disconnect looping with fresh cookies.
- **Fix:** Instantiate `ConnectionRegistry`; call `check_admission` before accept/spawn
  using `ServerConfig`; reject over-limit with `serve_unavailable`. Route `/health`,
  `/ready`, `/metrics` to the health module. Cap the session cache size (LRU/bounded).
- **Acceptance:** Connection caps enforced (verified by test exceeding limit);
  `/health` returns health JSON, not capsule HTML; session cache is bounded.
- **Risk:** Medium. Touches the accept loop; needs a connection-limit integration test.

### H2 — Validate client-supplied session ID (session fixation / key injection)
- **Status:** `[x]` Done. New `SessionId::is_valid_format()` requires exactly 32 hex
  chars; `handle_client` only trusts a cookie's session id if it passes, otherwise it
  mints a fresh server-generated id. This rules out `:`-bearing / oversized values that
  could collide with the `__shared__:` cache-key namespace, and prevents adopting an
  arbitrary client-chosen id (session fixation). Test: `test_session_id_is_valid_format`.
  Note: binding persisted-state keys to an *authenticated identity* (the deeper part of
  this item) is still open where auth is enabled — tracked under future auth work, not
  this fix. `session.rs`, `server.rs` cookie block.
- **Location:** `libs/rwire/src/server.rs:1204-1219`; key building at `:91-97`
- **Problem:** The cookie session ID is trusted verbatim (`SessionId::from_cookie` trims
  only, no validation) and interpolated into the persisted-state key
  `format!("{t}:{session_id}")`. An attacker who guesses/knows a victim's ID operates on
  their state; a value containing `:` or mimicking the `__shared__:` prefix (`:95`) can
  collide/confuse cache-key namespaces.
- **Fix:** Validate session-ID format (fixed-length hex) before trusting it; regenerate on
  first contact rather than adopting an arbitrary client value. When auth is enabled, bind
  persisted-state keys to the authenticated identity, not the raw cookie.
- **Acceptance:** Malformed/oversized/`:`-containing session cookies are rejected or
  replaced; no client value can reach the `__shared__:` namespace.
- **Risk:** Low–Medium. Pairs naturally with C1.

### H3 — Eliminate per-event O(N) symbol-table clones
- **Status:** `[x]` Done (full in-place fix). `build_synced_update_with_known_symbols`
  no longer clones the known-symbol table in or writes it back out. It interns new
  strings directly into the connection's `known` map (or a local map on full render),
  so per-event symbol overhead is O(new) instead of O(total-ever-sent). Wire output is
  byte-identical (existing 195+ synced-update tests pass unchanged).
  `libs/rwire/src/builder.rs:2295`.
- **Location:** `libs/rwire/src/builder.rs:2302-2311` (seed) and `:2391-2395` (write-back)
- **Problem:** `build_synced_update_with_known_symbols` runs on **every** client event,
  broadcast, and route change. It clones every known symbol string into a fresh map
  (`:2304`), then after emitting re-inserts the **entire merged** map back into `known`
  (`:2392`) — even though only `new_symbols` are actually new. `sent_symbols` grows
  unbounded for the connection lifetime, so every click/keystroke re-clones the whole
  symbol table twice and gets more expensive the longer the session lives.
- **Fix:** Minimum — write-back iterates `new_symbols` only (5-line change). Better —
  operate on the passed-in `known` map in place (it already holds all symbols + correct
  indices), push only newly-interned strings, track the delta for `SYMBOLS_EXTEND`.
  Drops per-event symbol overhead from O(N) to O(new).
- **Acceptance:** Per-event allocation no longer scales with total symbols sent; synced
  update + symbol-extend tests still pass; wire output byte-identical.
- **Risk:** Low for the write-back fix; Medium for the full in-place rework (touches the
  hot encode path — keep wire output identical).

---

## 🟡 Medium

### M1 — `emit_update_element` drops client-action bindings on re-render
- **Status:** `[x]` Done. The initial render's target/selector slot indices are now
  captured (`BuildContext::client_action_indices()` → new `ClientActionIndices`) and
  stored on `ConnectionState`. They're threaded through
  `build_synced_update_with_known_symbols` into `emit_update_element`, which re-emits
  `BIND_TARGET`/`BIND_TOGGLE`/`BIND_SELECTOR`/`BIND_SELECT`/`BIND_TIMED_TOGGLE`/
  `AUTO_TOGGLE` against the existing client slots (INIT_* not repeated — the client keeps
  them from initial render). Regression test: `synced_update_reemits_client_action_bindings`
  (asserts BIND_TOGGLE present with the index map, absent without it).
  **Known limitation:** a target/selector *type that first appears only on a re-render*
  (never registered at initial render) has no client slot and is skipped rather than
  assigned+INIT-ed mid-update. Rare; tracked as a follow-up if it surfaces.
  `builder.rs`, `server.rs`.
- **Location:** `libs/rwire/src/builder.rs:2504-2703`
- **Problem:** Initial render emits `INIT_TARGET`/`BIND_TARGET`/`BIND_TOGGLE`/
  `BIND_SELECTOR`/`BIND_SELECT`/`BIND_TIMED_TOGGLE`/`AUTO_TOGGLE` (via
  `emit_client_action_bindings`, `builder.rs:1898`/`:2035`). The update emitter
  `emit_update_element` emits none — it's a free fn with no access to the
  `target_indices`/`selector_indices` maps on `BuildContext`. Any element with a
  `.target()/.toggle()/.selector()/.timed_toggle()` binding inside a `#[renderer]` region
  loses client-side interactivity the moment that region re-renders (CLEAR_CHILDREN +
  rebuild). Same class as the historical missing-style-opcodes bug.
- **Fix:** Give the update path access to the index maps (make it a method on
  `BuildContext` or thread the maps through) and emit the client-action bindings, mirroring
  the initial-render emitter.
- **Acceptance:** A `.toggle()`/`.target()` element inside a synced region still works
  after the region re-renders (E2E or opcode-stream test).
- **Risk:** Medium. Structural change to a free fn; needs an interaction regression test.

### M2 — Bound WebSocket frame/message size
- **Status:** `[x]` Done. The upgrade now uses `accept_async_with_config` with a
  `WebSocketConfig` capping `max_message_size`/`max_frame_size` at
  `MAX_WS_MESSAGE_SIZE`/`MAX_WS_FRAME_SIZE` (256KB — generous vs the protocol's 64KB
  payload cap, far below tungstenite's 64 MiB/16 MiB defaults). These limit incoming
  reads only, so large server→client DOM messages are unaffected. Verified live: a
  200KB frame is accepted (connection stays open), a 300KB frame is rejected (connection
  reset by the WS layer) and the server stays healthy. `server.rs:1338`.
- **Location:** `libs/rwire/src/server.rs:1224` (`accept_async(stream)`)
- **Problem:** No `WebSocketConfig`, so tungstenite defaults apply (64 MiB message /
  16 MiB frame). The whole frame is allocated before the 64KB decode cap runs; with H1
  absent, repeated multi-MB frames amplify memory/CPU.
- **Fix:** `accept_async_with_config` with `max_message_size`/`max_frame_size` sized to the
  protocol (a few hundred KB).
- **Acceptance:** Frames above the configured limit are rejected by the WS layer before
  allocation/decode.
- **Risk:** Low.

### M3 — Server-side event rate limiting
- **Status:** `[x]` Done. Per-connection token bucket on `ConnectionState`
  (`EVENT_BUCKET_CAPACITY` 100 burst, `EVENT_REFILL_PER_SEC` 100/s). Both inbound binary
  events and text/route messages call `allow_event()` and are dropped before any
  handler/render/broadcast work if over budget. 100/s is far above human interaction
  (clicks, typing, 60 fps drag) but caps a flood. Unit test
  `event_rate_limit_caps_a_flood`; verified live: a 1000-event flood → only 101 processed
  (capacity + refill), server stayed healthy. `server.rs`.
- **Location:** `libs/rwire/src/server.rs:1667-1804`
- **Problem:** Debounce (`BIND_DEBOUNCED`) is enforced only in the browser — advisory. The
  `consecutive_decode_errors` cap (`:1570`) only catches malformed messages. A flood of
  well-formed events runs the handler, re-renders synced regions, and may broadcast to all
  subscribers (`shared.broadcast`) with no throttle — per-connection CPU/amplification DoS,
  worst for `Shared`-state handlers.
- **Fix:** Per-connection token bucket / minimum inter-event interval enforced server-side.
- **Acceptance:** Sustained event flood from one connection is throttled without affecting
  other connections.
- **Risk:** Low–Medium. Needs care not to drop legitimate bursts (typing).

### M4 — O(synced²) child-scan in the emit loop
- **Status:** `[x]` Done. Replaced the per-region rescan of all synced elements with a
  single O(synced) pass that indexes children into a `HashMap<parent_id,
  HashMap<TypeId, Vec<u32>>>` (and computes `emit_synced_counter` in the same pass).
  Each region now indexes its children by reference — no per-region scan, no clone.
  Children stay in `synced` order within each bucket so nested-region id reuse is
  unchanged; output is byte-identical (nested_renderer/synced_update/fine_grained tests
  pass). `builder.rs`.
- **Location:** `libs/rwire/src/builder.rs:2416-2430` (and `emit_synced_counter` `:2404-2410`)
- **Problem:** For each rendered region the loop rescans **all** synced elements to build
  `ids_by_type` (`synced.iter().filter(|s| s.parent == Some(se.id))`). Linear for narrow
  field updates, but quadratic for `ChangeSet::all()` updates (default for Vec push/retain
  and non-macro handlers) and router navigations. `emit_synced_counter` also does an
  O(synced) `max()` each event.
- **Fix:** Build a single `HashMap<(parent_id, TypeId), Vec<u32>>` once before the loop
  (O(total_synced)); index per region. Fold the counter `max()` into the same pass.
- **Acceptance:** Emit cost is linear in total synced regions; output byte-identical.
- **Risk:** Low–Medium. Hot path; keep output identical.

### M5 — Decompose `handle_websocket` (701 lines, cyclomatic 73)
- **Status:** `[x]` Done (substantially). `handle_websocket` is **737 → 377 lines** with
  four cohesive `ConnectionState` methods extracted, eliminating the 4× duplicated
  "build synced update" block and the 2× "dispatch handler → cache/memory + broadcast +
  subscribe" block:
  - `build_type_update` (39 lines) — the shared re-render path (broadcast + post-handler)
  - `dispatch_handler` (40) — run a handler against shared-cache or memory state
  - `render_route_view_swap` (113) — the router outlet swap (prune + render + register)
  - `render_initial_dom` (44) — the full initial render
  Bundled the 8 params into a `ConnContext<F>` struct and **removed
  `handle_websocket`'s `#[allow(clippy::too_many_arguments)]`**. Behavior validated live
  in a browser: counter events (0→3), router view swap Home→Counter→About→Counter with
  state preserved (count stays 2) and re-bound `+` working after swap.
  **Remaining (optional follow-up):** the main fn (377) and `render_route_view_swap`
  (113) are still >80 lines; fully extracting the loop's match arms into async helpers
  is higher-risk for diminishing returns and is left as future work. The `handle_client`
  `too_many_arguments` allow (11 params, connection setup) is separate and untouched.
- **Location:** `libs/rwire/src/server.rs:1383`
- **Problem:** By far the largest single function in the codebase — dwarfs everything else.
  Violates the >50-line rule and carries `#[allow(clippy::too_many_arguments)]` (`:1382`).
- **Fix:** Extract message-type handlers (binary event / route text / lifecycle) into
  focused functions; introduce a parameter struct to remove the `too_many_arguments`
  suppression. Coordinate with M1/H3 since they touch adjacent code.
- **Acceptance:** No sub-function >~80 lines; `too_many_arguments` allow removed; all
  tests pass; behavior unchanged.
- **Risk:** Medium. Large mechanical refactor; lean on tests + compiler.

### M6 — Wire a `/metrics` endpoint (split out of H1)
- **Status:** `[x]` Done. `run()` builds an `Arc<Metrics>`, threaded to `handle_client`
  (and into `ConnContext` for `handle_websocket`). `GET /metrics` is routed alongside
  `/health`/`/ready` (before auth) via a new `health::serve_metrics`, returning
  `metrics.to_prometheus()` with `Content-Type: text/plain; version=0.0.4`. Live counters:
  `connections_total`/`connections_rejected` incremented at admission, `active_connections`
  set from the registry at serve time, `messages_received` incremented per inbound WS
  message. Verified live: baseline all-zero → during a connection sending 5 events,
  `connections_total=1`, `active_connections=1`, `messages_received=5`; after close,
  `active_connections=0`. `server.rs`, `health.rs`.
- **Location:** `libs/rwire/src/metrics.rs`, `server.rs` accept/HTTP routing
- **Problem:** `metrics.rs` provides a Prometheus-format exporter but is never wired to
  the server, and the docs imply `/metrics` exists. Unlike `/health`·`/ready` (which
  only need the `ConnectionRegistry`), a metrics endpoint needs a server-owned metrics
  registry instance plumbed through `run()` → `handle_client`, plus counters/gauges
  updated on the hot path. Deferred from H1 to keep that change focused.
- **Fix:** Instantiate a metrics registry in `run()`, thread it through, increment on
  connection/event/error, and route `GET /metrics` to a Prometheus text serializer
  (mirroring the `/health` routing added in H1).
- **Acceptance:** `GET /metrics` returns Prometheus text with live connection/event counters.
- **Risk:** Medium. Touches the hot path for counter updates.

---

## 🟢 Low / cleanup

### L1 — Delete dead `build_synced_update`
- **Status:** `[x]` Done. Deleted; confirmed zero call sites across libs/apps/examples/tests
  first. Workspace builds, all tests pass.
- **Location:** `libs/rwire/src/builder.rs:2226`
- **Problem:** `pub fn` doc-commented "backwards compatible", **zero call sites**
  (libs/apps/examples/tests). Codebase uses `build_synced_update_multi`. Directly violates
  the "no backwards-compat shims / delete deprecated APIs" policy.
- **Fix:** Delete it.
- **Acceptance:** Removed; workspace builds.
- **Risk:** Very low.

### L2 — Remove `#[allow(dead_code)]` in tests
- **Status:** `[x]` Done. `MultiFieldState` in `tests/fine_grained_reactivity.rs` had three
  unused fields under `#[allow(dead_code)]`; the tests only use the `FIELD_*` id constants
  (RendererDeps masks), not field storage, so it's now a field-less marker struct and the
  allow is gone. Updated the `::default()` call sites to the unit value to keep clippy at
  zero warnings.
- **Location:** `libs/rwire/tests/fine_grained_reactivity.rs:200` (`struct MultiFieldState`)
- **Problem:** Violates the explicit "NO `#[allow(dead_code)]`" rule; fields `field_b`,
  `field_c` unused.
- **Fix:** Trim the unused fields or the struct; remove the allow.
- **Acceptance:** Allow removed; tests pass with no dead-code warning.
- **Risk:** Very low.

### L3 — Guard rail for DOM XSS via attribute sink
- **Status:** `[x]` Done. Added a `sa(e,n,v)` helper to the JS runtime that all
  app/user-controllable attribute writes go through (`O.A` dynamic name+value, `O.AK`
  enum-name + dynamic value). It (1) refuses attribute names starting with `on` (blocks
  inline event-handler injection — rwire events go through the binary BIND system, never
  inline handlers) and (2) refuses `javascript:` values on URL attributes
  (`href`/`src`/`xlink:href`/`formaction`/`action`), stripping control chars + whitespace
  first to defeat ` java\tscript:` tricks. `data:` is intentionally allowed (legitimate
  for inline images). Enum-only opcodes (`O.AE`/`O.AB`) and the morph attribute-copy read
  from already-guarded sources, so the two entry points are sufficient. Verified: 14 node
  behavioral cases (blocks/allows) + full runtime JS syntax check; runtime stays ~12.8KB.
  `capsule_gen.rs`.
- **Location:** `libs/rwire/src/capsule_gen.rs:94` (`O.A`) and `:97` (`O.AK`)
- **Problem:** The attribute path calls `setAttribute(name, value)` with both drawn from
  the symbol table and **no validation**. App code doing `.attr("href", user_input)` or a
  dynamic attribute *name* can yield `javascript:` URI XSS or inject an `on*` handler. Text
  path is safe (always `textContent` + server-side escaping at `builder.rs:95-111`). This
  is app responsibility, but the framework offers no guard rail.
- **Fix:** Document the hazard; consider blocking `javascript:`/`data:` schemes for
  `href`/`src` and rejecting `on*` attribute names in the runtime.
- **Acceptance:** Documented; optional runtime guard rejects the dangerous schemes/names.
- **Risk:** Low (runtime guard could affect legitimate `data:` images — make it opt-out).

### L4 — Bound `sent_symbols` growth (free-text interning)
- **Status:** `[x]` Done, via a per-connection ceiling (`MAX_SENT_SYMBOLS` = 50k): a
  connection whose interned symbol table exceeds it is disconnected (reconnect resets the
  table). Chosen over LRU eviction because the wire symbol indices are *positional*
  (`next_idx = 0x80 + map.len()`), so evicting an entry would desync the client's table —
  the JS runtime places `SYMBOLS_EXTEND` symbols at the explicit start index
  (`capsule_gen.rs` `O.SE`: `sc=si`), so a smooth LRU would first require decoupling the
  index into a persistent monotonic counter on `ConnectionState`. The ceiling is the
  low-risk bound; M3's rate limit also caps how fast the table can grow. **Possible future
  refinement:** monotonic-index decoupling + LRU eviction to avoid the disconnect entirely.
  `server.rs`.
- **Location:** `libs/rwire/src/server.rs:1280` (`sent_symbols: HashMap<String,u32>`)
- **Problem:** Interns text content with no eviction. Apps echoing distinct user text
  (chat, comments, search) add a permanent entry per unique string — slow unbounded growth
  on long-lived connections. (`sent_maps`/`sent_css` are bounded by enum/token counts and
  are fine.)
- **Fix:** Cap/evict the symbol map (LRU), or don't intern free-text content unlikely to
  repeat. Coordinates with H3.
- **Acceptance:** Per-connection symbol memory is bounded under a stream of unique strings.
- **Risk:** Low–Medium. Must keep symbol indices consistent with what the client holds.

### L5 — Add `Secure` flag to the session cookie
- **Status:** `[x]` Done, with **auto-detection** (better than a bare flag). `to_cookie`
  gained a `secure` param; `handle_client` sets it when the client-facing connection is
  HTTPS — auto-detected from the proxy's `X-Forwarded-Proto: https` header (rwire serves
  plain HTTP itself) — or when `ServerConfig::secure_cookies` forces it on. Off for
  plain-HTTP dev so the browser doesn't drop the cookie. Verified live: plain request →
  no `Secure`; `-H 'X-Forwarded-Proto: https'` → `Secure` present. Tests:
  `test_session_id_to_cookie_secure`, `test_forwarded_https`.

### L6 — Reduce duplication (rule-of-three)
- **Status:** `[ ]`
- **Locations:**
  - `libs/rwire-components/src/catalog.rs` — many `*_demo`/`*_rich` fns are jaccard 1.0
    copies (main reason the file is 2875 lines). Collapse to a data-driven table.
  - App `build_footer` (jaccard 1.0) across `apps/rwire-design-system/src/main.rs:920`,
    `rwire-examples:462`, `rwire-docs:589`, `rwire-website`; `build_header` ~0.95 across 3.
    All already call shared `Footer::new()` — hoist the wrapper config to a shared helper.
  - `Switch::build` / `Radio::build` / `Checkbox::build` — structurally identical (jaccard 1.0).
- **Fix:** Data table for catalog demos; shared header/footer helper; unify the form-control trio.
- **Acceptance:** Duplicated bodies collapsed; visual output unchanged; LOC reduced.
- **Risk:** Low–Medium. Mostly mechanical; verify rendered output unchanged.

---

## Suggested execution order

1. **C1** CSPRNG session IDs — smallest change, closes the worst hole.
2. **H1 + M2** Wire in admission control + frame-size limits — defenses already built.
3. **H3 (write-back fix)** Per-event symbol clone — quick hot-path relief.
4. **M1** `emit_update_element` client-action bindings — real interactivity bug.
5. **H2 + L5** Session-ID validation + `Secure` cookie — pairs with C1.
6. **L1 + L2** Delete dead code — unambiguous quick wins.
7. **M4 + H3 (full)** Hot-path algorithmic fixes.
8. **M5** Decompose `handle_websocket` (coordinate with M1/H3).
9. **M3, L3, L4, L6** Remaining hardening + cleanup.

## Verification per change

```bash
cargo clippy --workspace --all-targets   # must stay at 0 warnings
cargo test --workspace
cargo fmt --all
```

For hot-path changes (H3, M1, M4), assert the emitted opcode stream / wire bytes are
unchanged where behavior should be identical.
