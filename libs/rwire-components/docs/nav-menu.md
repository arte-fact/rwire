---
title: NavMenu
description: Top-level navigation bar with links and active state highlighting
order: 501
component: nav-menu
---

## Import

```rust
use rwire_components::{NavMenu, NavItem};
```

## Usage

```rust
NavMenu::new()
    .item(NavItem::new("Home", "/"))
    .item(NavItem::new("Docs", "/docs"))
    .item(NavItem::new("API", "/api"))
    .active_path("/docs")
    .build()
```

## Active Path

```rust
// The item whose href matches active_path gets highlighted
NavMenu::new()
    .item(NavItem::new("Dashboard", "/dashboard"))
    .item(NavItem::new("Settings", "/settings"))
    .active_path("/settings")
    .build()
```

## Accessibility

- Items render as anchor elements within a flex container
- Active item is visually distinguished with accent styling
- Use within a `<nav>` landmark for proper navigation semantics
