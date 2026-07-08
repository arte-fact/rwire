# Async actions — the lifecycle primitive (IMPLEMENTED 2026-07-08)

Shipped in `libs/rwire` (`actions.rs` + dispatch seam) and `libs/rwire-components`.
Live-verified by `examples/async-action` (`cargo run -p async-action`): a deferred
button spins in place then clears on success or turns destructive on failure; an
immediate handler never spins; per-instance rows spin only the row you click; a
form's submit reflects the same way; failures surface as toasts.

A generic **`loading → done | errored`** action lifecycle. A handler that hands work
to a background thread calls `ctx.defer()` — the action is marked `Loading` and that
fact is broadcast to the client, so the control that triggered it spins with no
further client event. When the work finishes, the worker calls `resolver.ok()`
(entry cleared → control returns to idle) or `resolver.err(msg)` (→ `Errored`, the
message surfaces). It **absorbs error reporting**: the `Errored` phase carries the
message, replacing the old unrendered failure-toast path.

The win: the **existing components are action-aware by default**. You keep writing
`Button::primary("Deliver").on_click(deliver())` — because its handler defers, the
button reflects the whole lifecycle. No `ActionButton` to remember, no key to author,
no reflecting renderer to wire.

## The contract

One learnable rule: **any control that triggers deferred work shows loading and
surfaces its error automatically; a handler that returns without deferring shows
nothing.** Because deferral is a *runtime fact* (not a signature guess), coverage is
complete — this is the fix for [pending-buttons'](./comparative-study.md) partial,
signature-classified coverage.

- **Immediate handler** (no `defer()`): the dispatch loop clears any stale entry in
  the same round-trip, so `Loading` never becomes visible.
- **Deferred handler**: `Loading` until the `Resolver` resolves.
- `Ready` is **not stored** — success removes the entry. The registry holds only
  in-flight `Loading` + unacknowledged `Errored`, so it stays naturally bounded.

## Identity — keyless

An action is `(session, handler_id, param)`, all three already on the event binding:

| Component | Source |
|---|---|
| `session` | `EventContext.session` |
| `handler_id` | the macro's `stable_handler_id` (== `event.handler_idx` on the wire) |
| `param` | the `BIND_REMOTE_PARAM` bytes the control already carries (`.with_param_bytes` / `on_ref`) |

The developer writes no correlation key. Two instances of the same handler with
different `param` (a list of rows, each with its own Deliver) are distinct actions —
only the clicked row reflects.

## Where it lives

- **`libs/rwire/src/actions.rs`** — `ActionKey` / `Phase` (`Loading | Errored(String)`)
  / `Actions` (a `#[storage(shared)]` registry, `TABLE_NAME = "__actions__"`) /
  `Resolver`. Plus `reflect_action(handler_id, param, idle, loading, errored)` — a
  **closure-backed `SyncedRenderer`** (the macro's fn-pointer renderer can't capture
  per-control identity) wrapped `display:contents` so it's layout-transparent — and
  `reflect_action_errors(render)` for page-level surfaces.
- **`state.rs`** — `EventContext::defer()` sets a `Cell<bool>` deferral flag (mirroring
  the existing `navigation` side-channel) and returns a `Send` `Resolver` capturing
  `Arc<SharedServerState>` + the `ActionKey`.
- **`server.rs`** — the dispatch seam builds the key, marks `Loading` before the
  handler, and after it: if the handler did **not** defer, resolves immediately (clear).
  `Resolver::ok/err` route through `resolve_action` →
  `update_shared_changed::<Actions>` → broadcast. A 30s `spawn_action_timeout` flips a
  stuck `Loading` → `Errored` (nothing hangs); `ConnectionGuard`'s drop GCs the
  session's entries on disconnect.
- **`libs/rwire-components`** — `Button` / `Composer` / form-submit reflect by default;
  `.reflect(false)` opts out. `ActionErrors` (inline alert slot) and `ActionToasts`
  (fixed, bottom-right toast notifications) are opt-in page-level error surfaces reading
  the same live list.

## Decisions

| Decision | Call |
|---|---|
| State-driven vs client-instant | **State-driven** — one round-trip to `Loading` is seamless at rwire's latency; no client-side optimistic hack |
| Identity | **Keyless** `(session, handler_id, param)` — nothing new for the developer to author |
| Component API | **Action-aware by default**, not a new opt-in `ActionButton` — automatic *and* complete because deferral is observed, not guessed |
| `Ready` phase | **Not stored** — success clears the entry; registry stays bounded |
| Loading visual | **In-place spinner**, constant footprint (label hidden by an `opacity:0` span so the box is kept; ring centred with `inset:0 + margin:auto`, tinted from `currentColor`) |
| Error visual | **Destructive recolour + tooltip** on the control (still clickable → retry) **and** an opt-in page surface (`ActionErrors` inline / `ActionToasts` notification) |
| Error clear-policy | Cleared on the **next attempt** (retry), on resolve, or on session end — the missing policy is why the old toast rotted |
| Reflection mechanism | **Synced-to-`Actions`** region filtered to `(handler_id, param)`: O(N) empty re-renders per change, O(1) patch — pure reuse of the shared-state→broadcast path, zero new wire opcodes |

## Known limitation

Reflection matches `(handler_id, param)` **session-agnostically**: renderers receive
only `&State`, with no session in the render context, so a control reflects any
session's action for the same `(handler_id, param)`. Correct for single-session apps
(the operator's own tools, e.g. claw). True multi-session isolation needs `session`
threaded into the render path — a later refinement, not a v1 blocker.

## Not in the design-system catalog

Reflection is a runtime behaviour a static `build_demo` can't show (there's no live
`Loading`/`Errored` state at catalog-render time). `Button`'s static `.loading` and
destructive looks already appear there; the lifecycle is documented here and in the
core-concepts page instead of faked as a static card.

## Adopting in claw (later pass)

Retire the read/write-gap handlers (`save_brief`, remote save, workflow edits, steer)
to `ctx.defer()`, and wire the bridge's completion sites to `resolver.ok/err`. A
separate pass once the primitive has soaked.
