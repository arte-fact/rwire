---
title: Capsule Size
description: How rwire keeps the client bundle small
order: 4
---
# Capsule Size

rwire serves a small JavaScript "capsule" — the entire client runtime — inline in the first HTML
response. A minimal app's capsule is around **17 KB** (~13 KB runtime + ~4 KB global CSS), and it
stays small as your app grows, with no per-app configuration.

The key idea: **the capsule is just the runtime.** Everything that *would* grow with app size —
the CSS rules and the element/event/attribute name tables — is **delivered lazily over the
WebSocket**, the first time a connection actually uses it, deduped per connection.

> **Note:** Earlier versions filtered per-app lookup tables at startup
> (`collect_symbols` / `Router::tree_shake_views` / `CapsuleConfig::extra_elements`). That
> machinery was removed — it's unnecessary with the lazy strategy below, and those APIs no longer
> exist. There is no static tree-shaking pass.

## Name maps stream over the wire (`MAP_DEF`)

The runtime keeps compact lookup tables mapping binary codes to DOM names:

| Table | Maps From | Maps To |
|-------|-----------|---------|
| Elements (E) | `u8` code | `"div"`, `"button"`, … |
| Events (V) | `u8` code | `"click"`, `"input"`, … |
| Style Props (P) | `u8` code | CSS property |
| Style Values (Y) | `u8` code | CSS value |
| Attributes (AT/AV) | `u8` code | attribute key/value |

The capsule ships these **empty**. The server sends each `(kind, code) → name` entry via the
`MAP_DEF` opcode the first time a connection references that code, deduped via
`ConnectionState.sent_maps`. The definition always arrives *before* the opcode that uses it, so an
entry can never be missing — even one reached only through a plain helper function. (SVG elements
are sent with a kind that also marks them in the SVG set.)

So instead of every capsule carrying the full ~3.5 KB of name tables, a connection receives a
one-time ~0.5 KB stream of only the names its pages actually use.

## CSS is delivered lazily over the wire (`STYLE_DEF`)

Utility / pseudo / breakpoint rules (`.u` / `.h` / `.b` classes) work the same way. The static
capsule ships only globals (reset, CSS variables, theme, keyframes, composites). Each class rule is
sent via the `STYLE_DEF` opcode the first time a connection references it, deduped via
`ConnectionState.sent_css`.

A rule used only in a deeply-nested or conditionally-rendered branch arrives the moment that branch
first renders — across any route, with nothing to declare. A hard refresh starts a fresh
connection that re-receives every referenced rule and name in the initial batch, so the set cannot
drift. Composite classes (`.c{id}`, detected by a one-time startup analysis) are part of the static
globals, so they are always present.

## Practical effect

- No `extra_elements` / `extra_style_utils` / router-based collection — and no risk of a
  filtered-out token breaking the page, since definitions are sent exactly when used.
- Capsule size is the runtime, period. A large app pays for CSS and names only as the user
  actually navigates into it.
