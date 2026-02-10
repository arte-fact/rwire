---
title: Pagination
description: Page navigation with prev/next and page numbers
order: 504
component: pagination
---

## Import

```rust
use rwire_components::Pagination;
```

## Usage

```rust
Pagination::new()
    .current_page(3)
    .total_pages(10)
    .build()
```

## Options

```rust
Pagination::new()
    .current_page(5)     // 1-indexed current page
    .total_pages(20)     // total number of pages
    .max_visible(7)      // max page buttons shown (default: 5)
    .build()
```

## Accessibility

- Prev/Next buttons are disabled at boundaries
- Current page is visually highlighted with accent styling
- Page buttons render within a list for screen reader context
