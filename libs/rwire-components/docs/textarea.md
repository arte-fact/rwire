---
title: Textarea
description: Multi-line text input with size variants
order: 202
component: textarea
---

## Import

```rust
use rwire_components::{Textarea, TextareaSize};
```

## Usage

```rust
Textarea::new()
    .placeholder("Enter description")
    .rows(6)
    .build()
```

## Sizes

```rust
Textarea::new().size(TextareaSize::Sm).build() // compact spacing
Textarea::new().size(TextareaSize::Md).build() // default
Textarea::new().size(TextareaSize::Lg).build() // spacious
```

## States

```rust
Textarea::new().disabled(true).build()
Textarea::new().readonly(true).build()
Textarea::new().required(true).build()
Textarea::new().invalid(true).build()
```

## Options

```rust
Textarea::new()
    .rows(8)                     // visible rows (default: 4)
    .name("description")
    .value("Initial content")
    .placeholder("Write here...")
    .build()
```

## Accessibility

- Renders a native `<textarea>` element
- Pair with `FormField` for label and error messaging
- Invalid state adds `aria-invalid="true"`
