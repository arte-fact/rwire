---
title: Divider
description: Horizontal or vertical separator line
order: 106
component: divider
---

## Import

```rust
use rwire_components::{Divider, SpacingSize};
```

## Usage

```rust
Divider::horizontal().build()
Divider::vertical().build()
Divider::horizontal().margin(SpacingSize::Lg).build()
```

## Orientation

```rust
Divider::horizontal().build()           // horizontal line (default)
Divider::vertical().build()             // vertical line
Divider::new().is_vertical(true).build() // dynamic control
```

## Accessibility

- Renders as a visual separator
- Use `role="separator"` semantics implicitly
