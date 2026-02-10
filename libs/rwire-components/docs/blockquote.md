---
title: Blockquote
description: Styled blockquote for quotes and callouts
order: 311
component: blockquote
---

## Import

```rust
use rwire_components::Blockquote;
```

## Usage

```rust
Blockquote::new("rwire is a server-rendered framework.").build()

Blockquote::new("All application logic lives on the server.")
    .cite("https://rwire.dev/docs")
    .build()
```

## With Custom Children

```rust
Blockquote::empty()
    .child(Text::body("Complex quoted content").build())
    .build()
```

## Accessibility

- Renders a semantic `<blockquote>` element
- Citation URL is set via the `cite` attribute
