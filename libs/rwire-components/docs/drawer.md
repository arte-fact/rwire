---
title: Drawer
description: Slide-in panel from edge of screen with backdrop overlay
order: 601
component: drawer
---

## Import

```rust
use rwire_components::{Drawer, DrawerPosition};
```

## Usage

```rust
#[derive(State, Default)]
#[storage(memory)]
struct AppState {
    drawer_open: bool,
}

#[renderer]
fn render_drawer(state: &AppState) -> ElementBuilder {
    Drawer::new()
        .title("Navigation")
        .position(DrawerPosition::Left)
        .open(state.drawer_open)
        .on_close(close_drawer())
        .content(sidebar_content())
        .build()
}
```

## Positions

```rust
Drawer::new().position(DrawerPosition::Left).build()  // slide from left (default)
Drawer::new().position(DrawerPosition::Right).build()  // slide from right
```

## Parts

```rust
Drawer::new()
    .title("Panel Title")         // header title
    .open(true)                   // visibility (server-controlled)
    .on_close(close_handler())    // close button and backdrop click
    .content(panel_content)       // main content
    .build()
```

## Accessibility

- Renders with backdrop overlay
- Close button in header triggers `on_close`
- Drawer visibility is server-controlled via the `open` flag
