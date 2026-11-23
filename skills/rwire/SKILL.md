---
name: rwire
description: Load when writing, reviewing or debugging UI code on the rwire framework — the SSR + binary-WebSocket model, choosing a component from the design-system catalogue, the no-free-CSS rule and its no_free_css guard, the standard recipes (filterable list, form, panel, refresh, network errors, mobile), known traps, verification commands and version pinning.
---

# rwire

rwire is a server-driven UI framework: the Rust server owns every piece of state and renders
the DOM over a binary WebSocket protocol, while the browser runs a ~13 KB runtime that executes
DOM opcodes and sends events back. There is no JS or CSS source to write, and no build step.

Work order, top first. Never skip a rung because the one below is faster:

**theme → component → token (`St` / `At` / `Av`) → inline `Style` (last resort, dynamic values only)**

Two rules are not negotiable:

- **No free CSS, no hard-coded colour.** Colours come from semantic tokens, spacing/size from
  `St` tokens. The `no_free_css` guard in the consuming app fails the build on any bypass. See
  `references/components-theme.md`.
- **Nothing that only lives in one connection may live in a process-global.** Handler-local
  state is `#[storage(memory)]`; anything hand-rolled is keyed by session and dropped when that
  session goes away. See `references/pitfalls.md`.

## The model in one screen

| Piece | What it is |
|---|---|
| State | `#[derive(State)]` + `#[storage(memory \| persisted \| shared)]` — per-connection, SQLite-backed, or process-global |
| Page | A `Router` route returning an `ElementBuilder`; one routing model per page tree |
| Handler | `#[handler] fn h(state: &mut S, ctx: &EventContext)` — mutates state, never renders |
| Renderer | `#[renderer] fn r(state: &S) -> ElementBuilder` — re-run after every handler; only diffs go on the wire |
| Async work | `ctx.defer()` → `Resolver`, resolved later with `ok()` / `err(msg)`; the control spins in place |
| Navigation | `ctx.navigate(url)` pushes history, `ctx.replace_route(url)` does not; both flush after the handler's DOM update. In markup: `Link::route(…)` |
| Wire | One-byte opcodes + a per-message symbol table + varint lengths; runtime shipped by the same binary, so no compatibility matrix |
| SSR | Optional static first paint (`ssr(true)`): default-state content, replaced once the socket connects |

Full detail, with the exact opcodes and the state/session lifecycle:
`references/model.md`.

## Choose a component, don't hand-roll markup

The catalogue (`libs/rwire-components/src/catalog.rs`) is the source of truth and it generates
the design-system gallery — run `apps/rwire-design-system` and look before writing a `div`.
Categories: Layout, Forms, Data Display, Feedback, Navigation, Overlay. Every entry carries its
variants and a live demo, so the gallery answers "which one, with which variant" better than any
prose.

Quick index by need, full table and the theming API in `references/components-theme.md`.

⚠️ Do not quote a component count from memory: the README says "90+", `catalog.rs` holds 69
records (five of them guide pages), `lib.rs` exports 71 names. Count in the code if the number
matters.

## Recipes

Filterable list, form, panel/overlay, refresh after an action, navigation after a handler,
network errors and reconnection, mobile and touch targets — each with the real API and the trap it
avoids: `references/recipes.md`.

## Traps already paid for

Start here before debugging anything that "should work": stacking of `St` tokens, chat scroll
pinning, async POSTs without a timeout, process-global state shared across connections, tables
that are never pruned, shared-state renderers nested in memory-state ones, and the 64-field
`State` ceiling. `references/pitfalls.md`.

## Verify, then pin

`cargo test` (the `no_free_css` guard), the parity and capture scripts, and how to pin rwire by
`rev` rather than by version: `references/verification.md`.

## Reference map

- Component source and catalogue — `libs/rwire-components/src/`, `catalog.rs`
- Tokens — `libs/rwire/src/style_tokens.rs`, `attr_tokens.rs`
- Theme system — `libs/rwire/src/theme.rs`, palettes in `libs/rwire-themes/`
- Protocol — `libs/rwire/src/protocol/` (`opcodes.rs`, `varint.rs`, `encoder.rs`)
- State, actions, sessions — `libs/rwire/src/state.rs`, `actions.rs`, `session.rs`, `server.rs`
- Client runtime — `runtime/src/` (`connect.ts`, `executor.ts`); build and sync rules in
  `references/runtime.md`
- Status and process — `ETAT.md` (rwire is feature-frozen; one open PR at a time)
