---
title: Tree Shaking
description: How rwire generates minimal client bundles
order: 4
---
# Tree Shaking

rwire generates a custom JavaScript capsule for each application, containing only the code needed for that specific app. A counter app gets ~1.5KB of JS. A full documentation site gets more, but never includes unused element types or event handlers.

```
Full runtime:  ~2.5KB (all elements, events, styles)
Counter app:   ~1.5KB (div, button, span + click event)
Savings:       ~40% smaller capsule
```

## What Gets Tree-Shaken

The capsule contains lookup tables that map binary opcodes to DOM operations. Tree shaking filters these tables to include only what the app uses:

| Table | Maps From | Maps To |
|-------|-----------|---------|
| Elements (E) | `u8` opcode | `"div"`, `"button"`, etc. |
| Events (V) | `u8` opcode | `"click"`, `"input"`, etc. |
| Style Utilities (U) | `u16` token | CSS class name |
| Style Props (P) | `u8` code | CSS property |
| Style Values (Y) | `u8` code | CSS value |
| Pseudo Classes | `(u8, u16)` pair | Pseudo CSS rule |
| Attributes (AT/AV) | `u8` code | Attribute key/value |

## How It Works

At server startup, the framework calls your root function and all registered router views to build the complete element tree:

```rust
Server::bind("0.0.0.0:9000")?
    .root(root)
    .routes(
        Router::new()
            .page("/", |_| build_home())
            .page("/about", |_| build_about())
    )
    .run()
    .await
```

`BuildContext::collect_symbols()` walks each tree and records every element type, event type, style token, and attribute used. The capsule generator then emits JS lookup tables filtered to only those entries.

## Router Integration

Without a router, tree shaking only sees the root view. If other pages use additional element types (e.g., `El::Table` only on `/reports`), those elements would be missing from the capsule.

The `.routes()` method solves this. It calls every registered view function with default parameters at startup:

```rust
// Router::tree_shake_views() calls each view with empty RouteParams
let trees = router.tree_shake_views();
```

All element types, events, and tokens discovered across all pages are merged into the capsule.

## Manual Extras

For dynamic content that is not part of the static render tree (e.g., markdown rendering that creates elements at runtime), you can manually include additional entries:

```rust
use rwire::protocol::El;

let config = CapsuleConfig::new()
    .extra_elements([El::Pre as u8, El::Code as u8].into())
    .extra_style_utils([St::BgSubtle as u16, St::PxMd as u16].into());
```

This is rarely needed when the router covers all pages.

## Capsule Size Breakdown

For a typical counter application:

```
Symbol table decoder:    ~100 bytes
Element creation:        ~80 bytes  (3 element types)
Event binding:           ~120 bytes (click only)
DOM operations:          ~200 bytes (append, set_text, set_class)
WebSocket connection:    ~150 bytes
Style token decoder:     ~100 bytes (if using St tokens)
Route handler:           ~80 bytes  (if using router)
Local mutation interp:   ~150 bytes (if using local state)
─────────────────────────────────
Total:                   ~1.0-1.5KB
```

Every feature has a cost, but that cost is only paid when the feature is used. An app without routing, local state, or style tokens gets a smaller capsule.
