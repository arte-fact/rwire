---
title: Binary Protocol
description: How rwire encodes DOM operations
order: 1
---
# Binary Protocol

rwire uses a compact binary protocol for server-to-browser communication. Each message is a sequence of single-byte opcodes followed by their arguments.

## Opcode Format

| Opcode | Hex | Description |
|--------|-----|-------------|
| CREATE | 0x02 | Create a DOM element |
| SET_TEXT | 0x11 | Set text content |
| SET_ATTR | 0x12 | Set an attribute |
| APPEND | 0x20 | Append child to parent |
| BIND_REMOTE | 0x31 | Bind server round-trip event |
| BATCH_END | 0xFF | End of message |

## Symbol Table

Strings are interned in a symbol table sent at the start of each message. Subsequent references use 1-byte indices instead of full strings.

```
[SYMBOLS, count, len1, bytes1..., len2, bytes2..., ...]
```

Reserved indices (0x00-0x7F) are used for common strings. Session-specific symbols use 0x80-0xFF.

## Style Tokens

Instead of sending CSS class strings, rwire encodes styles as numeric tokens:

```
[STYLE_UTIL, ref, token_hi, token_lo]
```

The browser maps tokens to CSS classes using a generated lookup table.

## Wire Efficiency

A typical button click handler exchange:

- **Event** (browser to server): ~8 bytes
- **DOM update** (server to browser): ~20-40 bytes
- **Equivalent JSON**: ~200-400 bytes

This 10x reduction in message size enables responsive UIs even on slow connections.
