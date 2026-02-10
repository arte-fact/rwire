---
title: Progress
description: Progress bar for showing completion state
order: 403
component: progress
---

## Import

```rust
use rwire_components::Progress;
```

## Usage

```rust
Progress::new()
    .value(65)
    .max(100)
    .build()

Progress::new()
    .value(3)
    .max(5)
    .label("Step 3 of 5")
    .build()
```

## Options

```rust
Progress::new()
    .value(42)   // current value (default: 0)
    .max(100)    // maximum value (default: 100)
    .label("Loading...") // accessible label
    .build()
```

## Accessibility

- Uses `role="progressbar"` with `aria-valuenow` and `aria-valuemax`
- `label()` sets `aria-label` for screen readers
- Fill width is computed as a percentage of value/max
