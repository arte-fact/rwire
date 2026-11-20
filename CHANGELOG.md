# Changelog

Pre-1.0 policy: **0.x with semver discipline** — breaking changes bump the
minor version and are called out here; patch releases never break. The wire
protocol is explicitly unstable and MAY change in any minor release: the JS
runtime ships from the same binary that speaks the protocol, so applications
never see a compatibility matrix — recompiling is the whole migration.

## 0.1.0 — unreleased (crates not yet published to crates.io)

### Breaking: rwire runs on tokio

async-std is discontinued; rwire now runs on **tokio** (and tokio-tungstenite
for the WebSocket). Applications start with `#[tokio::main]` instead of
`#[async_std::main]` and depend on `tokio = { version = "1", features =
["macros", "rt-multi-thread"] }` instead of async-std — that is the whole
migration for an app that only awaited `run()`. `Server::run()` must be awaited
inside a tokio runtime; inside an existing tokio application the server is a
plain task, and handlers can `tokio::spawn` directly. The `serve_*` helpers of
`health` and `capsule` take a tokio `TcpStream`; `BroadcastMsg` gains a
`Shutdown` variant.

- **Graceful stop**: `ServerWithRoot::run_until(shutdown)` stops accepting,
  closes open WebSockets with 1001 "going away", waits up to `SHUTDOWN_GRACE`
  (5 s) and returns.
- **Latency**: accepted sockets set `TCP_NODELAY`. An event's answer is two
  frames; with Nagle the second waited for the client's delayed ACK, so every
  click round trip cost ~40 ms on any runtime.

### Fixed: first render of routed views and the static first paint

- A region inside the router's `outlet()` that reads `#[storage(shared)]`
  state painted the state's empty default when its route was opened or
  navigated to, until the next broadcast. The route swap now finds every
  shared/persisted state the new view reads (nested regions included) and
  renders the cached copy.
- `ssr(true)` rendered the root once, at startup, at the default state and
  the default route: `/posts/x` served the `/` view with empty content. The
  static paint is now rendered per request, for the requested route, with
  the current shared state; memory and persisted state render their default
  (they belong to a connection or a session). It is rendered past the auth
  gate, like the live page.

First public release. Highlights of what the framework ships with:

- **Core**: server-owned state over a binary WebSocket protocol; ~16KB
  TypeScript-built browser runtime (`runtime/`, embedded as a vendored
  artifact); lazy per-connection delivery of element/event/attr names
  (`MAP_DEF`), utility CSS (`STYLE_DEF`), and lazy runtime extensions
  (`MOD_DEF` — e.g. the vim mode interpreter loads on first use) — runtime
  tree-shaking with no build step.
- **Reactivity**: `#[renderer]`/`#[handler]` with compile-time field-dependency
  bitmasks; DOM morphing with keyed list diffing (`.key()`) that preserves
  input values, scroll, and focus across reorders.
- **Async actions**: `ctx.defer()` hands a handler's work to a background
  thread and returns a `Send` `Resolver` (`ok()` / `err(msg)`); the action's
  `loading → done | errored` lifecycle is broadcast over shared state, so the
  triggering control reflects it with **no key and no extra code**. `Button`,
  `Composer`, and form-submit are action-aware by default (`.reflect(false)`
  opts out); `ActionErrors` / `ActionToasts` are opt-in page-level error
  surfaces. Identity is keyless `(session, handler_id, param)`. See
  `docs/async-action-design.md` and the `async-action` example.
- **State tiers**: per-connection memory, SQLite-persisted (schema generated
  from the derive), and shared-with-broadcast across connections.
- **Streaming**: one-shot visibility sentinels (`on_visible`) and the
  `StreamedContent` component — progressive content with structural
  one-request-in-flight backpressure.
- **Components**: 90+ including the Chat family (`ChatItem` trait,
  `ChatTranscript` with seamless history, `Composer`, `TypingIndicator`),
  a `FileEditor` kit (sandboxed file explorer + code editor with line
  numbers, dirty marks, autosave, guarded saves), and `StreamedContent`.
- **Theming**: theme-as-state with reactive CSS variables; 9 palettes; 700+
  typed style tokens with hover/breakpoint variants.
- **Security**: CSPRNG sessions, admission control, WS frame limits, event
  rate limiting, Origin validation, sanitized attribute sink.
- **Delegated authentication**: `ServerWithRoot::auth_with(DelegatedAuth)`
  hands the page and WebSocket gate to an external identity provider — an
  async validator over the request head returns an `Identity` (id + opaque
  data), read in handlers with `EventContext::identity()`. Anonymous pages are
  redirected to a configurable login URL with `next`, handshakes get `401`;
  successes only are cached (bounded, 30 s default), and open sockets are
  re-validated and closed once revoked. Exclusive with `auth()`: configuring
  both fails at startup (`AUTH_CONFLICT`).
- **Editor**: `rwire-editor` — `FileEditor` kit with optional vim mode
  (motions, operators, text objects, char-seek, registers, dot-repeat,
  status-bar search; lazy-loaded as a runtime extension, off by default).
- **Ops**: /health, /ready, Prometheus /metrics, PWA support, reconnect
  overlay, reverse-proxy base-path mounting.

### Guardrails

Two mistakes that used to fail silently now fail loudly. Both are breaking
for an app that hit them, which was already broken:

- `#[derive(State)]` refuses a struct with more than 64 named fields at
  compile time. Renderer dependencies are a 64-bit mask, and fields past
  the 64th never triggered a re-render. The error names the struct and its
  field count; split the state, or nest related fields.
- `run()` refuses to start when `.routes(..)` is set but the page tree has
  no `outlet()`, even inside a renderer's output (`ROUTER_WITHOUT_OUTLET`).
  Such a page served normally, then froze on the first navigation. A
  renderer that only reads `CurrentRoute` does not count as an outlet
  (`SyncedRenderer::is_outlet`).
