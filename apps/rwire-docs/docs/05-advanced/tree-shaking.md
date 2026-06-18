---
title: Capsule Size
description: How rwire keeps the client bundle small
order: 4
---
# Capsule Size

rwire serves a small JavaScript "capsule" — the entire client runtime — inline in
the first HTML response. It stays small (single-digit KB) without any per-app
configuration. rwire does **not** statically tree-shake the runtime; instead it
keeps the bundle small two ways:

> **Note:** Earlier versions filtered per-app lookup tables at startup
> (`collect_symbols` / `Router::tree_shake_views` / `CapsuleConfig::extra_elements`).
> That machinery was removed — it's unnecessary with the strategy below, and those
> APIs no longer exist.

## 1. Token maps are shipped whole

The capsule contains compact `u8`/`u16` lookup tables mapping binary opcodes to
DOM operations:

| Table | Maps From | Maps To |
|-------|-----------|---------|
| Elements (E) | `u8` opcode | `"div"`, `"button"`, … |
| Events (V) | `u8` opcode | `"click"`, `"input"`, … |
| Style Props (P) | `u8` code | CSS property |
| Style Values (Y) | `u8` code | CSS value |
| Attributes (AT/AV) | `u8` code | attribute key/value |

The full set is only ~1–2 KB, and a missing entry would be a structural break, so
these are **shipped whole** — filtering them isn't worth the complexity or risk.

## 2. CSS is delivered lazily over the wire

Utility / pseudo / breakpoint rules (`.u` / `.h` / `.b` classes) are the part that
*would* grow with app size, so they are **not** embedded in the capsule. The
static capsule ships only globals (reset, CSS variables, theme, keyframes,
composites). Each class rule is sent over the WebSocket via the `STYLE_DEF` opcode
the first time a connection actually references it, deduped per connection
(`ConnectionState.sent_css`).

So the delivered CSS is **exact and automatic**: a rule used only in a
deeply-nested or conditionally-rendered branch arrives the moment that branch
first renders — across any route, with nothing to declare. A hard refresh starts a
fresh connection that re-receives every referenced rule in the initial batch, so
the set cannot drift.

Composite classes (`.c{id}`, detected by a one-time startup analysis) are part of
the static globals, so they are always present.

## Practical effect

- No `extra_elements` / `extra_style_utils` / router-based collection needed.
- Capsule size is dominated by the (whole) token maps + runtime, not by your app's
  styling — a large app pays for CSS only as the user actually navigates into it.
