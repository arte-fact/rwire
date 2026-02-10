---
title: Kbd
description: Keyboard shortcut display with key-cap styling
order: 304
component: kbd
---

## Import

```rust
use rwire_components::Kbd;
```

## Usage

```rust
// Single key
Kbd::new("K").build()

// Key combination
Kbd::combo(&["Ctrl", "Shift", "P"]).build()
```

## Accessibility

- Renders with monospace font and key-cap visual styling
- Key combinations show separator between keys
