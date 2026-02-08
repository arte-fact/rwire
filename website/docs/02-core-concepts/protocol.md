---
title: Binary Protocol
description: How rwire encodes DOM operations as binary opcodes
order: 6
---

# Binary Protocol

```
Server                              Browser
  │                                    │
  │─── [SYMBOLS, 2, 5, "hello",  ───> │
  │     3, "app"]                      │
  │─── [CREATE, Div]              ───> │  createElement("div")
  │─── [SET_TEXT, ref, sym:0x80]  ───> │  el.textContent = "hello"
  │─── [APPEND, body, ref]        ───> │  body.appendChild(el)
  │─── [BATCH_END]                ───> │
  │                                    │
```

rwire encodes all DOM operations as single-byte opcodes followed by their arguments. The browser runtime parses these bytes and executes the corresponding DOM calls. No JSON, no HTML, no virtual DOM.

## Core Opcodes

| Opcode | Hex | Format | Description |
|--------|-----|--------|-------------|
| `CREATE` | `0x02` | `[type]` | Create element, assigns next ref |
| `SET_TEXT` | `0x11` | `[ref, sym]` | Set textContent |
| `SET_CLASS` | `0x10` | `[ref, sym]` | Set className |
| `SET_ATTR` | `0x12` | `[ref, attr, val]` | setAttribute |
| `APPEND` | `0x20` | `[parent, child]` | appendChild |
| `CLEAR_CHILDREN` | `0x25` | `[ref]` | Remove all children |
| `BIND_REMOTE` | `0x31` | `[ref, ev, handler]` | Server round-trip event |
| `BIND_DEBOUNCED` | `0x33` | `[ref, ev, handler, ms_hi, ms_lo]` | Debounced event |
| `BIND_REMOTE_PARAM` | `0x34` | `[ref, ev, handler, len, params...]` | Event with item params |
| `BATCH_END` | `0xFF` | | End of message |

## Symbol Table

Strings are interned per message. The `SYMBOLS` header (0xF0) sends a table of strings, and all subsequent opcodes reference strings by index:

```
[0xF0, count, len1, bytes1..., len2, bytes2..., ...]
```

Indices 0x00-0x7F are reserved for common strings (e.g., 0x04 = "id"). Session-specific strings use 0x80-0xFF. This means a text update after the first render costs just 3 bytes: `[SET_TEXT, ref, sym]`.

## Style Tokens

CSS styles are encoded as numeric tokens rather than class name strings:

```
[STYLE_UTIL, ref, token_varint]
[STYLE_MULTI, ref, count, token1_varint, token2_varint, ...]
[STYLE_PSEUDO, ref, pseudo_code, count, token1_varint, ...]
```

The `St` enum maps each token to a CSS declaration. Tokens use varint encoding (1-2 bytes for values up to 16383). The browser applies styles via `element.classList.add('u'+code)` with generated CSS rules.

## Wire Efficiency

A button click that updates a counter:

| Protocol | Bytes |
|----------|-------|
| rwire binary (SET_TEXT + symbol) | 4-6 bytes |
| JSON patch `{"op":"replace","path":"/count","value":42}` | ~50 bytes |
| React-style VDOM diff (JSON) | ~200+ bytes |

The event message from browser to server is equally compact: a handler index byte, optional payload bytes, and no JSON wrapper. This 10-50x reduction enables responsive UIs even on constrained connections.
