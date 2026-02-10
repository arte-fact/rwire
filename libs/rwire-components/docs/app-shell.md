---
title: AppShell
description: Full-page layout with header, optional sidebar, and main content area
order: 104
component: app-shell
---

## Import

```rust
use rwire_components::AppShell;
```

## Usage

```rust
AppShell::new()
    .header(header_content.build())
    .sidebar(nav_menu.build())
    .main(page_content.build())
    .build()
```

## Customizing Dimensions

```rust
AppShell::new()
    .sidebar_width(300)   // sidebar width in pixels (default: 260)
    .header_height(64)    // header height in pixels (default: 56)
    .header(header.build())
    .main(content.build())
    .build()
```

## Without Sidebar

```rust
// Header + main content only
AppShell::new()
    .header(header.build())
    .main(content.build())
    .build()
```

## Accessibility

- Use semantic landmarks within each section (e.g., `<nav>` in sidebar, `<main>` for content)
- Header should contain the primary navigation or site title
