---
title: Radio
description: Radio button input with label and grouping support
order: 205
component: radio
---

## Import

```rust
use rwire_components::Radio;
```

## Usage

```rust
// Radio group (share the same name)
Radio::new()
    .name("plan")
    .value("free")
    .label("Free Plan")
    .on_change(select_plan())
    .build()

Radio::new()
    .name("plan")
    .value("pro")
    .label("Pro Plan")
    .checked(true)
    .on_change(select_plan())
    .build()
```

## States

```rust
Radio::new().name("opt").value("a").label("Option A").checked(true).build()
Radio::new().name("opt").value("b").label("Option B").disabled(true).build()
```

## Accessibility

- Renders a native `<input type="radio">` element
- `name` attribute is required for grouping radio buttons
- Label is automatically associated via `for`/`id` attributes
