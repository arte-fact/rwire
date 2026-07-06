---
title: How rwire Compares
description: rwire vs LiveView, Blazor Server, htmx, and the Rust WASM frameworks
order: 6
---
# How rwire Compares

rwire is a **server-driven** UI framework: like Phoenix LiveView, Blazor
Server, or Laravel Livewire, all state and logic live on the server and the
browser renders what it's told. Within that family, rwire occupies one specific
point in the design space, chosen deliberately:

| | rwire | Phoenix LiveView | Blazor Server | htmx |
|---|---|---|---|---|
| Server language | Rust | Elixir | C# | any |
| Wire format | binary opcodes | JSON diffs | SignalR (JSON/MessagePack) | HTML fragments |
| Client runtime | ~13KB (4.7KB gz) | ~100KB+ | ~300KB+ | ~14KB |
| Templates | none — typed Rust builders | HEEx strings | Razor | your server's |
| Client build step | none | none | none | none |
| UI type safety | full (state → element tree) | partial | full | none |

## What's genuinely different

**Binary protocol, not JSON or HTML.** DOM operations are single-byte opcodes
with varint arguments and per-message string interning. A counter increment is
a handful of bytes each way. Numbers travel as zigzag varints, repeated words
via a word table, styles as u16 tokens.

**Runtime tree-shaking, per connection, with no build step.** The capsule
ships empty name maps and only global CSS; element/event/attribute names
(`MAP_DEF`) and utility CSS rules (`STYLE_DEF`) stream over the wire the first
time each connection references them, deduped per connection. A minimal app's
first paint is a ~17KB page, and it stays that size as the app grows.

**Compile-time reactivity.** `#[handler]` and `#[renderer]` statically analyze
field access into u64 bitmasks; deciding whether a region re-renders is one
bitwise AND. No signals runtime, no dirty-tracking proxies, nothing shipped to
the client for it.

**Typed client actions.** Modals, tabs, and toggles run entirely in the
browser through declarative `Target`/`Selector` bindings — zero-latency, but
still expressed in typed Rust, not an escape hatch into JavaScript.

**No protocol compatibility matrix.** The runtime is served by the same binary
that speaks the protocol. The wire format can change in any release;
recompiling is the whole migration.

## What the trade-offs are

Be honest with yourself about these before choosing rwire — or any
server-driven framework:

- **A round-trip per interaction.** Anything that isn't a client action
  crosses the network. On localhost or a LAN it's imperceptible; at 150ms RTT
  it isn't. Typing into bound inputs is debounced, not free.
- **Server memory scales with open connections**, and it's a single process.
  There is no horizontal-scaling story yet beyond sticky sessions.
- **No SSR / static first paint yet.** Crawlers and no-JS clients see a blank
  page. Weak for content sites, fine for apps behind a login.
- **Young ecosystem.** LiveView has a decade of production hardening and a
  large community; rwire has a security audit, a strong test harness, and a
  handful of real apps. The API is 0.x and moves.

Versus the Rust WASM frameworks (Leptos, Dioxus): those run your Rust *in the
browser* — megabyte-class WASM payloads, offline capability, no per-connection
server state. rwire is the opposite bet: nothing meaningful in the browser,
everything on the server, the smallest possible wire. Different problems.

## Where rwire fits best

Self-hosted software, dashboards, internal tools, personal apps, and
low-bandwidth or high-connection-density deployments — anywhere the server is
close, the audience is bounded, and you'd rather write Rust than maintain a
frontend build pipeline.
