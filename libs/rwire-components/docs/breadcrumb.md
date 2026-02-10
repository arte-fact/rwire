---
title: Breadcrumb
description: Navigation breadcrumb trail
order: 502
component: breadcrumb
---

## Import

```rust
use rwire_components::Breadcrumb;
```

## Usage

```rust
Breadcrumb::new()
    .item("Home", Some("/"))
    .item("Products", Some("/products"))
    .item("Laptop", None::<&str>) // current page, no link
    .build()
```

## Building Trails

```rust
// Last item has no link (current page)
Breadcrumb::new()
    .item("Docs", Some("/docs"))
    .item("Components", Some("/docs/components"))
    .item("Breadcrumb", None::<&str>)
    .build()
```

## Accessibility

- Wraps items in a `<nav>` element with `aria-label="Breadcrumb"`
- Current page (last item without link) is not wrapped in an anchor
- Separators are rendered between items
