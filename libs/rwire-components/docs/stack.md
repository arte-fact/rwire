---
title: Stack
description: Flexbox-based layout with configurable direction and spacing
order: 100
component: stack
---

## Import

```rust
use rwire_components::{Stack, StackDirection, StackAlign, StackJustify, Gap};
```

## Usage

```rust
// Vertical stack (default)
Stack::column()
    .gap(Gap::Lg)
    .children([child1.build(), child2.build()])
    .build()

// Horizontal stack
Stack::row()
    .gap(Gap::Sm)
    .justify(StackJustify::Between)
    .children([left.build(), right.build()])
    .build()

// Centered content
Stack::centered()
    .children([content.build()])
    .build()
```

## Direction

```rust
Stack::new().direction(StackDirection::Row).build()   // horizontal
Stack::new().direction(StackDirection::Column).build() // vertical (default)
```

## Gap

```rust
Stack::column().gap(Gap::None).build() // no gap
Stack::column().gap(Gap::Xs).build()   // 4px
Stack::column().gap(Gap::Sm).build()   // 8px
Stack::column().gap(Gap::Md).build()   // 16px (default)
Stack::column().gap(Gap::Lg).build()   // 24px
Stack::column().gap(Gap::Xl).build()   // 32px
```

## Alignment and Justify

```rust
Stack::row()
    .align(StackAlign::Center)     // cross-axis: Stretch, Start, Center, End
    .justify(StackJustify::Between) // main-axis: Start, Center, End, Between, Around
    .wrap(true)                     // flex-wrap
    .build()
```

## Accessibility

- Stack is a layout primitive with no semantic role
- Use appropriate landmarks or headings within stack children
