---
title: Responsive Breakpoints
description: Mobile-first responsive styling with the Bp system
order: 6
---
# Responsive Breakpoints

rwire provides a first-class breakpoint system for responsive design. Apply styles at specific viewport widths using mobile-first `min-width` methods:

```rust
use rwire::{el, El, St};

el(El::Div)
    .st([St::FlexCol, St::GapMd])   // base: column layout
    .md([St::FlexRow])               // 768px+: row layout
    .lg([St::GridCols3])             // 1024px+: 3-column grid
```

Breakpoint methods work exactly like `.st()` -- they accept any `St` tokens -- but the styles only apply at or above the specified viewport width.

## Breakpoint Values

| Method | Min-Width | Typical Devices |
|--------|-----------|-----------------|
| `.sm()` | 640px | Large phones, small tablets |
| `.md()` | 768px | Tablets |
| `.lg()` | 1024px | Laptops, small desktops |
| `.xl()` | 1280px | Large desktops |

These follow a mobile-first design philosophy. Base styles (`.st()`) apply at all sizes. Each breakpoint overrides or extends those styles at wider viewports.

## Common Patterns

### Responsive Grid

```rust
el(El::Div)
    .st([St::DisplayGrid, St::GridCols1, St::GapMd])
    .md([St::GridCols2])
    .lg([St::GridCols3])
    .xl([St::GridCols4])
```

### Show/Hide Elements

```rust
// Sidebar: hidden on mobile, visible at md+
el(El::Aside)
    .st([St::DisplayNone])
    .md([St::DisplayBlock])

// Hamburger: visible on mobile, hidden at md+
el(El::Button)
    .st([St::DisplayFlex])
    .md([St::DisplayNone])
```

### Responsive Typography

```rust
el(El::H1)
    .st([St::Text3xl])
    .md([St::Text5xl])
```

### Stacking Layouts

```rust
// Stack on mobile, side-by-side on tablet+
el(El::Div)
    .st([St::DisplayFlex, St::FlexCol, St::GapLg])
    .md([St::FlexRow])
```

## Combining with Pseudo-Classes

Breakpoints and pseudo-class methods can be used together on the same element:

```rust
el(El::Button)
    .st([St::BgSurface, St::PxMd, St::PySm])
    .hover([St::BgHover])
    .md([St::PxLg, St::PyMd])
```

Each system generates independent CSS classes. They compose without conflict.

## How It Works

The breakpoint system mirrors the pseudo-class architecture:

1. **Enum**: `Bp(u8)` with variants `Sm`, `Md`, `Lg`, `Xl`
2. **Wire opcode**: `STYLE_BREAKPOINT (0x8A)` sends `(ref, bp_code, count, st1, st2, ...)`
3. **JS runtime**: Applies CSS classes like `b1u0` (breakpoint 1, style utility 0)
4. **CSS generation**: Produces `@media(min-width:768px){.b1u0{display:none}}` rules

All breakpoint CSS is tree-shaken. Only the `(Bp, St)` pairs your app actually uses appear in the generated stylesheet.

## The Bp Enum

For advanced use, you can access the underlying `Bp` enum directly via the `.breakpoint()` method:

```rust
use rwire::style_tokens::{Bp, St};

el(El::Div)
    .breakpoint(Bp::Md, [St::FlexRow])  // equivalent to .md([St::FlexRow])
```

The convenience methods `.sm()`, `.md()`, `.lg()`, `.xl()` are preferred for readability.
