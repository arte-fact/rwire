---
title: List
description: Styled ordered and unordered lists
order: 310
component: list
---

## Import

```rust
use rwire_components::{List, ListItem};
```

## Usage

```rust
// Unordered (bulleted)
List::unordered()
    .children([
        ListItem::new("First item").build(),
        ListItem::new("Second item").build(),
    ])
    .build()

// Ordered (numbered)
List::ordered()
    .children([
        ListItem::new("Step 1: Install").build(),
        ListItem::new("Step 2: Configure").build(),
    ])
    .build()
```

## Accessibility

- Renders native `<ul>` or `<ol>` elements with `<li>` children
- Screen readers announce list length and item positions
