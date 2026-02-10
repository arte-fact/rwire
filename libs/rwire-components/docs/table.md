---
title: Table
description: Semantic HTML table for structured data display
order: 305
component: table
---

## Import

```rust
use rwire_components::{Table, TableRow};
```

## Usage

```rust
Table::new()
    .headers(["Name", "Email", "Role"])
    .row(TableRow::new().cells(["Alice", "alice@example.com", "Admin"]))
    .row(TableRow::new().cells(["Bob", "bob@example.com", "User"]))
    .striped(true)
    .build()
```

## Building Rows

```rust
// From an array
TableRow::new().cells(["Col 1", "Col 2", "Col 3"])

// One cell at a time
TableRow::new()
    .cell("Name")
    .cell("Value")
    .cell("Action")
```

## Striped Rows

```rust
Table::new()
    .headers(["Item", "Price"])
    .row(TableRow::new().cells(["Widget", "$10"]))
    .row(TableRow::new().cells(["Gadget", "$25"]))
    .striped(true) // alternating row backgrounds
    .build()
```

## Accessibility

- Renders a semantic `<table>` with `<thead>` and `<tbody>`
- Headers are rendered as `<th>` elements
- Striped rows are purely visual
