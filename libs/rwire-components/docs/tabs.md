---
title: Tabs
description: Tab navigation with content panels
order: 503
component: tabs
---

## Import

```rust
use rwire_components::{Tabs, Tab};
```

## Usage

```rust
Tabs::new()
    .tab(Tab::new("Overview", overview_content.build()))
    .tab(Tab::new("Settings", settings_content.build()))
    .tab(Tab::new("History", history_content.build()))
    .active(0)
    .build()
```

## Active Tab

```rust
// Set the active tab by 0-based index
Tabs::new()
    .tab(Tab::new("Tab 1", content1.build()))
    .tab(Tab::new("Tab 2", content2.build()))
    .tab(Tab::new("Tab 3", content3.build()))
    .active(1) // "Tab 2" is active
    .build()
```

## Dynamic Tabs

```rust
// Build tabs from a list
let mut tabs = Tabs::new();
for (i, section) in sections.iter().enumerate() {
    tabs = tabs.tab(Tab::new(&section.title, render_section(section).build()));
}
tabs.active(current_tab).build()
```

## Accessibility

- Tab headers are rendered as clickable elements
- Active tab content is visible; inactive panels are hidden
- Use server state to track the active tab index
