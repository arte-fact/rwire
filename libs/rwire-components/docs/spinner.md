---
title: Spinner
description: Loading spinner with CSS animation
order: 402
component: spinner
---

## Import

```rust
use rwire_components::{Spinner, SpinnerSize};
```

## Usage

```rust
Spinner::new().build()

Spinner::new()
    .size(SpinnerSize::Lg)
    .label("Loading data...")
    .build()
```

## Sizes

```rust
Spinner::new().size(SpinnerSize::Sm).build() // 1rem
Spinner::new().size(SpinnerSize::Md).build() // 1.5rem (default)
Spinner::new().size(SpinnerSize::Lg).build() // 2rem
```

## Accessibility

- `label()` sets `aria-label` for screen readers
- Defaults to "Loading" if no label is provided
- Uses CSS animation (no JavaScript)
