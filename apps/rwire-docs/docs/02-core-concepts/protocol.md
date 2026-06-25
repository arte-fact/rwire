---
title: Binary Protocol
description: How rwire encodes DOM operations as binary opcodes
order: 6
---

# Binary Protocol

```
Server                                 Browser
  │                                       │
  │─── [MAP_DEF, 1, (0,0,3,"div")]   ───> │  E[0] = "div"
  │─── [SYMBOLS, 1, 5, "hello"]      ───> │
  │─── [CREATE, 0]                   ───> │  createElement(E[0])
  │─── [SET_TEXT, ref, sym:0x80]     ───> │  el.textContent = "hello"
  │─── [APPEND, body, ref]           ───> │  body.appendChild(el)
  │─── [BATCH_END]                   ───> │
  │                                       │
```

rwire encodes all DOM operations as single-byte opcodes followed by their arguments. The browser
runtime parses these bytes and executes the corresponding DOM calls. No JSON, no HTML, no virtual
DOM.

`ref`, symbol index, and handler index are **varint**-encoded (1–3 bytes), so a message can carry
thousands of elements. Element/event/attribute *type* codes are single bytes.

## Core Opcodes

| Opcode | Hex | Format | Description |
|--------|-----|--------|-------------|
| `CREATE` | `0x02` | `[type]` | Create element, assigns next ref (`type` → name via `MAP_DEF`) |
| `SET_TEXT` | `0x11` | `[ref, sym]` | Set textContent from a symbol |
| `SET_TEXT_INT` | `0x15` | `[ref, zigzag_varint]` | Set textContent from an integer (no symbol) |
| `SET_TEXT_WORDS` | `0x13` | `[ref, count, idx…]` | Set textContent from word-table indices |
| `SET_CLASS` | `0x10` | `[ref, sym]` | Set className |
| `SET_ATTR` | `0x12` | `[ref, attr, val]` | setAttribute |
| `APPEND` | `0x20` | `[parent, child]` | appendChild |
| `CLEAR_CHILDREN` | `0x25` | `[ref]` | Remove all children |
| `BIND_REMOTE` | `0x31` | `[ref, ev, handler]` | Server round-trip event |
| `BIND_DEBOUNCED` | `0x33` | `[ref, ev, handler, ms_hi, ms_lo]` | Debounced event |
| `BIND_REMOTE_PARAM` | `0x34` | `[ref, ev, handler, len, params…]` | Event with item params |
| `BATCH_END` | `0xFF` | | End of message |

## Symbol Table

Strings are interned. The `SYMBOLS` header (`0xF0`) sends a table of strings, and subsequent
opcodes reference them by index:

```
[0xF0, count_varint, len1_varint, bytes1…, len2_varint, bytes2…, …]
```

Both the count and each length are varint-encoded (so long strings — e.g. a fenced code block —
are fine). Indices `0x00–0x7F` are reserved for common strings (e.g. `0x04 = "id"`);
session-specific strings start at `0x80`. The table **persists across renders**: later updates add
to it with `SYMBOLS_EXTEND` (`0xF1`) rather than resending. A text update reusing an existing
symbol is just `[SET_TEXT, ref, sym]` — a handful of bytes (a session symbol ≥ `0x80` takes 2
varint bytes).

## Lazy delivery: CSS and name maps

The capsule ships only the runtime and global CSS. Two things stream over the wire on first use,
each deduped per connection:

- **`STYLE_DEF` (`0x87`)** — utility/pseudo/breakpoint CSS rules, sent the first time a class is
  referenced: `[STYLE_DEF, count, (rule_len, rule_utf8)…]`. The client appends each to a dedicated
  stylesheet.
- **`MAP_DEF` (`0x88`)** — element/event/attribute/style-token **names**, sent the first time a
  code is referenced: `[MAP_DEF, count, (kind, code, name_len, name_utf8)…]`. The client fills the
  matching lookup map (`E`/`V`/`AT`/`AV`/`P`/`Y`; kind 6 also marks SVG elements). Definitions
  arrive *before* the opcodes that use them, so nothing is ever missing.

## Style Tokens

CSS utilities are encoded as numeric tokens rather than class-name strings:

```
[STYLE_UTIL, ref, token_varint]
[STYLE_MULTI, ref, count, token1_varint, …]
[STYLE_PSEUDO, ref, pseudo_code, count, token1_varint, …]
[STYLE_BREAKPOINT, ref, bp_code, count, token1_varint, …]
```

The `St` enum maps each token to a CSS declaration. Tokens use varint encoding (1–2 bytes for
values up to 16383). The browser applies styles via `element.classList.add('u'+code)`, and the
matching rule arrives lazily via `STYLE_DEF`.

## Wire Efficiency

A button click that updates a counter:

| Protocol | Bytes |
|----------|-------|
| rwire binary (`SET_TEXT_INT` + ref) | ~4 bytes |
| JSON patch `{"op":"replace","path":"/count","value":42}` | ~50 bytes |
| React-style VDOM diff (JSON) | ~200+ bytes |

The event message from browser to server is equally compact: a handler index, optional payload
bytes, and no JSON wrapper.
