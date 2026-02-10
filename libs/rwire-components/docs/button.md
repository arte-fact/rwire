---
title: Button
description: Button with intent variants, sizes, and loading state
order: 200
component: button
---

## Import

```rust
use rwire_components::{Button, ButtonIntent, ButtonSize};
```

## Usage

```rust
// Convenience constructors
Button::primary("Save").build()
Button::secondary("Cancel").build()
Button::ghost("More options").build()
Button::destructive("Delete").build()

// Full configuration
Button::new()
    .intent(ButtonIntent::Primary)
    .size(ButtonSize::Lg)
    .text("Submit")
    .on_click(submit_handler())
    .build()
```

## Variants

```rust
Button::new().intent(ButtonIntent::Primary).build()     // solid accent
Button::new().intent(ButtonIntent::Secondary).build()   // subtle border
Button::new().intent(ButtonIntent::Ghost).build()       // transparent
Button::new().intent(ButtonIntent::Destructive).build()  // red
```

## Sizes

```rust
Button::new().size(ButtonSize::Sm).build() // 28px height
Button::new().size(ButtonSize::Md).build() // 36px height (default)
Button::new().size(ButtonSize::Lg).build() // 44px height
```

## States

```rust
Button::primary("Save").disabled(true).build()
Button::primary("Saving...").loading(true).build()
Button::primary("Submit").full_width(true).build()
```

## Accessibility

- Renders a native `<button>` element
- Disabled state sets the `disabled` attribute
- Use descriptive text for screen reader clarity
