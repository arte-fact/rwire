---
title: Card
description: Surface container with padding, border, and shadow
order: 103
component: card
---

## Import

```rust
use rwire_components::{Card, CardPadding, CardShadow};
```

## Usage

```rust
Card::new()
    .child(
        Stack::column().gap(Gap::Sm).children([
            Text::heading3("Card Title").build(),
            Text::body("Card content goes here.").build(),
        ]).build()
    )
    .build()
```

## Padding

```rust
Card::new().padding(CardPadding::None).build() // no padding
Card::new().padding(CardPadding::Sm).build()   // small
Card::new().padding(CardPadding::Md).build()   // medium (default)
Card::new().padding(CardPadding::Lg).build()   // large
```

## Shadow

```rust
Card::new().shadow(CardShadow::None).build() // no shadow
Card::new().shadow(CardShadow::Sm).build()   // small (default)
Card::new().shadow(CardShadow::Md).build()   // medium
Card::new().shadow(CardShadow::Lg).build()   // large
```

## Border

```rust
Card::new().bordered(false).build() // no border (default: true)
```

## Accessibility

- Card has no implicit ARIA role
- Use headings within cards to describe content sections
