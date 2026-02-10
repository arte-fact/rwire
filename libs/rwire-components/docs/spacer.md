---
title: Spacer
description: Creates space between elements
order: 105
component: spacer
---

## Import

```rust
use rwire_components::{Spacer, SpacingSize};
```

## Usage

```rust
Spacer::md().build()                              // medium vertical space
Spacer::lg().build()                              // large vertical space
Spacer::new(SpacingSize::Xl).horizontal().build()  // horizontal space
```

## Size Shortcuts

```rust
Spacer::xs().build() // extra small
Spacer::sm().build() // small
Spacer::md().build() // medium
Spacer::lg().build() // large
Spacer::xl().build() // extra large
```

## Accessibility

- Spacer is a presentational element with no semantic meaning
