---
title: Core Concepts
description: Understanding the rwire architecture
order: 3
---
# Core Concepts

rwire follows a server-centric architecture where the server owns all state and the browser is a thin rendering layer.

## Server-Owned State

All application state lives on the server. The browser never stores application data. This means:

- **No client-side state management** — no Redux, no Context, no stores
- **Server-side validation** — all mutations are validated on the server
- **Natural fit for Rust** — leverage Rust's type system and ownership model

## Binary Protocol

Communication uses a compact binary protocol instead of JSON:

- Single-byte opcodes for DOM operations
- Symbol table interning reduces string repetition
- Style tokens encoded as 1-2 byte integers
- Typical message is 10-50 bytes vs 200-500 bytes for JSON

## Tree Shaking

The JavaScript runtime served to the browser is tree-shaken at startup:

- Only includes element types actually used by your app
- Only includes event types actually bound
- A counter app gets ~1.5KB of JS

## Capsule

The "capsule" is the HTML page served to browsers. It contains:

- The minimal JS runtime
- Generated CSS for used style tokens
- WebSocket connection setup
- No build step or bundler required
