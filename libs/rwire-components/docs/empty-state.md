---
title: EmptyState
description: Placeholder for empty lists, search results, or data views
order: 405
component: empty-state
---

## Import

```rust
use rwire_components::EmptyState;
```

## Usage

```rust
EmptyState::new()
    .title("No results found")
    .description("Try adjusting your search terms.")
    .action(Button::primary("Clear filters").on_click(clear()).build())
    .build()
```

## With Icon

```rust
EmptyState::new()
    .icon(icon_element)
    .title("No projects yet")
    .description("Create your first project to get started.")
    .action(Button::primary("New Project").on_click(create()).build())
    .build()
```

## Minimal

```rust
EmptyState::new()
    .title("Nothing here")
    .build()
```

## Accessibility

- Centered layout with clear messaging hierarchy
- Action buttons provide a clear path forward
- Icon is decorative; title conveys the primary message
