---
title: Skeleton
description: Loading placeholder that shows content shape before data arrives
order: 404
component: skeleton
---

## Import

```rust
use rwire_components::{Skeleton, SkeletonShape};
```

## Usage

```rust
// Single text line
Skeleton::text().build()

// Multi-line text placeholder
Skeleton::text().lines(3).build()

// Circle (avatar placeholder)
Skeleton::circle().build()

// Rectangle (card/image placeholder)
Skeleton::rect().build()
```

## Shapes

```rust
Skeleton::text().build()   // text line (default)
Skeleton::circle().build() // circular shape
Skeleton::rect().build()   // rectangular block
```

## Multi-line Text

```rust
// Shows multiple animated lines with varying widths
Skeleton::text().lines(4).build()
```

## Accessibility

- Skeletons are purely visual loading indicators
- Use `aria-busy="true"` on the parent container while loading
