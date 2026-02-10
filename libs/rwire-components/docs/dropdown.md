---
title: DropdownMenu
description: Action menu triggered by a button with server-controlled open state
order: 602
component: dropdown
---

## Import

```rust
use rwire_components::{DropdownMenu, DropdownItem};
```

## Usage

```rust
#[derive(State, Default)]
#[storage(memory)]
struct AppState {
    menu_open: bool,
}

#[renderer]
fn render_menu(state: &AppState) -> ElementBuilder {
    DropdownMenu::new()
        .open(state.menu_open)
        .on_toggle(toggle_menu())
        .trigger(Button::secondary("Actions").build())
        .item(DropdownItem::new("Edit").on_click(edit_handler()))
        .item(DropdownItem::new("Duplicate").on_click(duplicate_handler()))
        .item(DropdownItem::new("Delete").destructive().divider().on_click(delete_handler()))
        .build()
}
```

## Dropdown Items

```rust
DropdownItem::new("Edit")                          // regular item
DropdownItem::new("Delete").destructive()           // red text
DropdownItem::new("Remove").divider()               // divider line before
DropdownItem::new("Save").on_click(save_handler())  // with click handler
```

## Accessibility

- Trigger button toggles the menu open/closed
- Menu visibility is server-controlled via the `open` flag
- Destructive items are visually marked in red
- Dividers separate groups of related actions
