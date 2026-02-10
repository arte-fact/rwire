---
title: Slider
description: Range input control with visual track and fill
order: 207
component: slider
---

## Import

```rust
use rwire_components::Slider;
```

## Usage

```rust
Slider::new()
    .min(0)
    .max(100)
    .value(42)
    .on_change(update_volume())
    .build()
```

## Options

```rust
Slider::new()
    .min(0)
    .max(10)
    .value(5)
    .step(1)                   // step increment
    .label("Volume")           // accessible label
    .disabled(true)
    .build()
```

## Accessibility

- Renders a native `<input type="range">` element
- `label()` sets `aria-label` for screen readers
- Keyboard accessible: adjust with arrow keys
