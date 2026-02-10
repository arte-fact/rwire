---
title: Container
description: Responsive width container with max-width constraints
order: 102
component: container
---

## Import

```rust
use rwire_components::{Container, ContainerSize};
```

## Usage

```rust
Container::new()
    .size(ContainerSize::Lg)
    .child(page_content.build())
    .build()
```

## Sizes

```rust
Container::new().size(ContainerSize::Sm).build()   // max-width 640px
Container::new().size(ContainerSize::Md).build()   // max-width 768px (default)
Container::new().size(ContainerSize::Lg).build()   // max-width 1024px
Container::new().size(ContainerSize::Xl).build()   // max-width 1280px
Container::new().size(ContainerSize::Full).build()  // no max-width
```

## Options

```rust
Container::new()
    .centered(true)   // auto margins (default: true)
    .padding(true)    // horizontal padding (default: true)
    .build()
```

## Accessibility

- Container is a layout primitive with no semantic role
- Use landmarks (`<main>`, `<nav>`) for page structure
