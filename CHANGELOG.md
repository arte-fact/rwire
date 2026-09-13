# Changelog

Pre-1.0 policy: **0.x with semver discipline** — breaking changes bump the
minor version and are called out here; patch releases never break. The wire
protocol is explicitly unstable and MAY change in any minor release: the JS
runtime ships from the same binary that speaks the protocol, so applications
never see a compatibility matrix — recompiling is the whole migration.

## 0.1.0 — unreleased

First public release. Highlights of what the framework ships with:

- **Core**: server-owned state over a binary WebSocket protocol; ~13KB
  TypeScript-built browser runtime (`runtime/`, embedded as a vendored
  artifact); lazy per-connection delivery of element/event/attr names
  (`MAP_DEF`) and utility CSS (`STYLE_DEF`) — runtime tree-shaking with no
  build step.
- **Reactivity**: `#[renderer]`/`#[handler]` with compile-time field-dependency
  bitmasks; DOM morphing with keyed list diffing (`.key()`) that preserves
  input values, scroll, and focus across reorders.
- **State tiers**: per-connection memory, SQLite-persisted (schema generated
  from the derive), and shared-with-broadcast across connections.
- **Streaming**: one-shot visibility sentinels (`on_visible`) and the
  `StreamedContent` component — progressive content with structural
  one-request-in-flight backpressure.
- **Components**: 60+ including the Chat family (`ChatItem` trait,
  `ChatTranscript` with seamless history, `Composer`, `TypingIndicator`).
- **Theming**: theme-as-state with reactive CSS variables; 9 palettes; 700+
  typed style tokens with hover/breakpoint variants.
- **Security**: CSPRNG sessions, admission control, WS frame limits, event
  rate limiting, Origin validation, sanitized attribute sink.
- **Ops**: /health, /ready, Prometheus /metrics, PWA support, reconnect
  overlay, reverse-proxy base-path mounting.
