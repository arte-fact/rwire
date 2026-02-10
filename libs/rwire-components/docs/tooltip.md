---
title: Tooltip
description: CSS-only tooltip that appears on hover and focus
order: 406
component: tooltip
---

## Import

```rust
use rwire_components::{Tooltip, TooltipPosition};
```

## Usage

```rust
Tooltip::new("Delete this item")
    .position(TooltipPosition::Top)
    .child(Button::primary("Delete").build())
    .build()
```

## Positions

```rust
Tooltip::new("Top tooltip").position(TooltipPosition::Top).child(trigger).build()
Tooltip::new("Bottom").position(TooltipPosition::Bottom).child(trigger).build()
Tooltip::new("Left").position(TooltipPosition::Left).child(trigger).build()
Tooltip::new("Right").position(TooltipPosition::Right).child(trigger).build()
```

## Accessibility

- CSS-only implementation (no JavaScript needed)
- Appears on both hover and focus for keyboard users
- Uses `data-tip` attribute for content
- Default position is Top
