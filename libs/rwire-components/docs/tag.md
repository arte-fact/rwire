---
title: Tag
description: Removable label or filter indicator with color variants
order: 302
component: tag
---

## Import

```rust
use rwire_components::{Tag, TagIntent};
```

## Usage

```rust
Tag::new("Rust").build()

Tag::new("rwire")
    .intent(TagIntent::Primary)
    .removable(true)
    .on_remove(remove_tag())
    .build()
```

## Variants

```rust
Tag::new("Default").intent(TagIntent::Default).build()
Tag::new("Primary").intent(TagIntent::Primary).build()
Tag::new("Success").intent(TagIntent::Success).build()
Tag::new("Warning").intent(TagIntent::Warning).build()
Tag::new("Error").intent(TagIntent::Error).build()
```

## Removable

```rust
// on_remove() implicitly sets removable(true)
Tag::new("Filter")
    .on_remove(handle_remove())
    .build()
```

## Accessibility

- Removable tags include a dismiss button with appropriate labelling
- Use `aria-label` on the remove button for screen reader clarity
