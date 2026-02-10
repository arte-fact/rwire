---
title: Switch
description: Toggle switch styled as a checkbox with switch appearance
order: 206
component: switch
---

## Import

```rust
use rwire_components::Switch;
```

## Usage

```rust
Switch::new()
    .label("Enable notifications")
    .checked(true)
    .on_change(toggle_notifications())
    .build()

// Without label
Switch::new()
    .name("dark_mode")
    .build()
```

## States

```rust
Switch::new().label("Wi-Fi").checked(true).build()
Switch::new().label("Airplane").disabled(true).build()
```

## Accessibility

- Built on a native checkbox with visual switch styling
- Label is automatically associated via `for`/`id` attributes
- Keyboard accessible: toggle with Space key
