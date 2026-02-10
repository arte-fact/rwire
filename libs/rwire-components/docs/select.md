---
title: Select
description: Dropdown select input with configurable options
order: 203
component: select
---

## Import

```rust
use rwire_components::Select;
```

## Usage

```rust
Select::new()
    .option("us", "United States")
    .option("ca", "Canada")
    .option("uk", "United Kingdom")
    .value("us")
    .build()
```

## Options

```rust
Select::new()
    .option("free", "Free Plan")
    .option("pro", "Pro Plan")
    .option("enterprise", "Enterprise")
    .name("plan")
    .build()
```

## States

```rust
Select::new().disabled(true).build()
Select::new().required(true).build()
Select::new().invalid(true).build()
```

## Accessibility

- Renders a native `<select>` element with `<option>` children
- Includes a custom dropdown arrow indicator
- Pair with `FormField` for accessible labelling
