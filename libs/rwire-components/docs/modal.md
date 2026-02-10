---
title: Modal
description: Modal dialog with backdrop, focus management, and keyboard handling
order: 600
component: modal
---

## Import

```rust
use rwire_components::{Modal, ModalSize};
```

## Usage

```rust
#[derive(State, Default)]
#[storage(memory)]
struct AppState {
    modal_open: bool,
}

#[renderer]
fn render_modal(state: &AppState) -> ElementBuilder {
    Modal::new()
        .title("Confirm Action")
        .open(state.modal_open)
        .on_close(close_modal())
        .content(el(El::P).text("Are you sure?"))
        .footer(
            Stack::row().gap(Gap::Sm).children([
                Button::secondary("Cancel").on_click(close_modal()).build(),
                Button::primary("Confirm").on_click(confirm_action()).build(),
            ]).build()
        )
        .build()
}
```

## Sizes

```rust
Modal::new().size(ModalSize::Sm).build()   // small
Modal::new().size(ModalSize::Md).build()   // medium (default)
Modal::new().size(ModalSize::Lg).build()   // large
Modal::new().size(ModalSize::Xl).build()   // extra large
Modal::new().size(ModalSize::Full).build()  // full screen
```

## Parts

```rust
Modal::new()
    .title("Dialog Title")          // header title
    .open(true)                     // visibility (server-controlled)
    .on_close(close_handler())      // close button and backdrop click
    .content(body_element)          // main content
    .footer(footer_element)         // footer with actions
    .build()
```

## Accessibility

- Renders with backdrop overlay and centered dialog
- Close button in header and backdrop click trigger `on_close`
- Modal visibility is server-controlled via the `open` flag
- Use `on_close` handler to update server state
