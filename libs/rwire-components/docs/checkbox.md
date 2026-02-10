---
title: Checkbox
description: Boolean checkbox input with optional label association
order: 204
component: checkbox
---

## Import

```rust
use rwire_components::Checkbox;
```

## Usage

```rust
// With label (auto-generates ID for association)
Checkbox::new()
    .label("Subscribe to newsletter")
    .checked(true)
    .on_change(toggle_subscribe())
    .build()

// Without label
Checkbox::new()
    .name("agree")
    .build()
```

## States

```rust
Checkbox::new().label("Agree").checked(true).build()
Checkbox::new().label("Agree").disabled(true).build()
Checkbox::new().label("Agree").required(true).build()
Checkbox::new().label("Agree").invalid(true).build()
```

## Accessibility

- Renders a native `<input type="checkbox">` element
- Label is automatically associated via `for`/`id` attributes
- Disabled and invalid states set appropriate ARIA attributes
