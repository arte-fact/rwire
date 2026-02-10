---
title: Grid
description: CSS Grid-based layout with configurable columns and spacing
order: 101
component: grid
---

## Import

```rust
use rwire_components::{Grid, GridColumns, Gap};
```

## Usage

```rust
// Responsive auto-fill grid
Grid::auto()
    .gap(Gap::Md)
    .children([card1.build(), card2.build(), card3.build()])
    .build()

// Fixed 3-column grid
Grid::new()
    .columns(GridColumns::Fixed3)
    .gap(Gap::Lg)
    .children([col1, col2, col3])
    .build()
```

## Columns

```rust
Grid::new().columns(GridColumns::Auto).build()   // responsive auto-fill (default)
Grid::new().columns(GridColumns::Fixed1).build()  // 1 column
Grid::new().columns(GridColumns::Fixed2).build()  // 2 columns
Grid::new().columns(GridColumns::Fixed3).build()  // 3 columns
Grid::new().columns(GridColumns::Fixed4).build()  // 4 columns
```

## Accessibility

- Grid is a layout primitive with no semantic role
- Content ordering should match visual order for screen readers
