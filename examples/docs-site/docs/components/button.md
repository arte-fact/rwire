---
title: Button
description: Interactive button component with intent variants
order: 1
---
# Button

Buttons trigger actions when clicked. They come in several intent variants and sizes.

## Usage

```rust
use rwire::components::{Button, ButtonIntent, ButtonSize};

// Primary button
Button::primary("Save Changes")
    .on_click(save_handler())
    .build()

// Secondary button
Button::secondary("Cancel")
    .on_click(cancel_handler())
    .build()

// Small destructive button
Button::new()
    .text("Delete")
    .intent(ButtonIntent::Destructive)
    .size(ButtonSize::Sm)
    .on_click(delete_handler())
    .build()
```

## Intent Variants

| Intent | Use Case |
|--------|----------|
| Primary | Main action on the page |
| Secondary | Alternative or less important actions |
| Ghost | Minimal visual weight, for toolbars |
| Destructive | Dangerous actions like delete |

## Sizes

- `Sm` — Compact, for dense UIs
- `Md` — Default size
- `Lg` — Prominent call-to-action

## Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `text` | `&str` | — | Button label |
| `intent` | `ButtonIntent` | `Primary` | Visual variant |
| `size` | `ButtonSize` | `Md` | Size variant |
| `disabled` | `bool` | `false` | Disable interaction |
| `loading` | `bool` | `false` | Show loading spinner |
| `on_click` | `HandlerSpec` | — | Click handler |
