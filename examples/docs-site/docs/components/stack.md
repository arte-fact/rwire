---
title: Stack
description: Flexbox layout component for arranging children
order: 3
---
# Stack

Stack is a flexbox layout component for arranging children in rows or columns with consistent spacing.

## Usage

```rust
use rwire::components::{Stack, StackDirection, Gap, StackAlign, StackJustify};

// Vertical stack with medium gap
Stack::column()
    .gap(Gap::Md)
    .children([child1, child2, child3])
    .build()

// Horizontal stack, centered
Stack::row()
    .gap(Gap::Sm)
    .align(StackAlign::Center)
    .justify(StackJustify::Between)
    .children([left, right])
    .build()
```

## Direction

- `Stack::row()` — Horizontal layout (flex-direction: row)
- `Stack::column()` — Vertical layout (flex-direction: column)

## Gap Sizes

| Gap | Value |
|-----|-------|
| `Xs` | 4px |
| `Sm` | 8px |
| `Md` | 16px |
| `Lg` | 24px |
| `Xl` | 32px |

## Alignment

Use `align` for cross-axis and `justify` for main-axis positioning:

| Align | Description |
|-------|-------------|
| `Start` | Align to start |
| `Center` | Center children |
| `End` | Align to end |
| `Stretch` | Stretch to fill |

## Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `direction` | `StackDirection` | `Column` | Layout direction |
| `gap` | `Gap` | `Md` | Spacing between children |
| `align` | `StackAlign` | `Stretch` | Cross-axis alignment |
| `justify` | `StackJustify` | `Start` | Main-axis alignment |
| `wrap` | `bool` | `false` | Allow wrapping |
