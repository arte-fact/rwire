---
title: Layout Components
description: Stack, Grid, Container, AppShell, and spacing utilities
order: 2
---
# Layout Components

## Stack

Flexbox layout for arranging children in rows or columns with consistent spacing.

```rust
use rwire::components::*;

// Vertical stack (default)
Stack::column()
    .gap(Gap::Lg)
    .children([child1, child2, child3])
    .build()

// Horizontal stack with alignment
Stack::row()
    .gap(Gap::Sm)
    .align(StackAlign::Center)
    .justify(StackJustify::Between)
    .children([left, right])
    .build()

// Centered content (both axes)
Stack::centered()
    .child(Spinner::new().build())
    .build()
```

### Direction

- `Stack::row()` -- horizontal (flex-direction: row)
- `Stack::column()` -- vertical (flex-direction: column)
- `Stack::centered()` -- column with both axes centered

### Gap

| Gap | Size |
|-----|------|
| `Gap::None` | 0 |
| `Gap::Xs` | 4px |
| `Gap::Sm` | 8px |
| `Gap::Md` | 16px (default) |
| `Gap::Lg` | 24px |
| `Gap::Xl` | 32px |

### Alignment

`align()` controls the cross-axis: `Start`, `Center`, `End`, `Stretch` (default).
`justify()` controls the main-axis: `Start` (default), `Center`, `End`, `Between`, `Around`.
Enable wrapping with `.wrap(true)`.

---

## Grid

CSS Grid layout with fixed or responsive auto-fill columns.

```rust
// Fixed 3-column grid
Grid::new()
    .columns(GridColumns::Fixed3)
    .gap(Gap::Lg)
    .children([card1, card2, card3])
    .build()

// Responsive auto-fill (cards wrap as viewport shrinks)
Grid::auto()
    .gap(Gap::Md)
    .children(cards)
    .build()
```

### Column Options

| Variant | Description |
|---------|-------------|
| `GridColumns::Auto` | Responsive auto-fill (default) |
| `GridColumns::Fixed1` | Single column |
| `GridColumns::Fixed2` | Two equal columns |
| `GridColumns::Fixed3` | Three equal columns |
| `GridColumns::Fixed4` | Four equal columns |

---

## Container

Centered, max-width-constrained wrapper for page content.

```rust
// Default: 768px max-width, centered, with horizontal padding
Container::new()
    .child(page_content)
    .build()

// Large container without padding
Container::new()
    .size(ContainerSize::Lg)
    .padding(false)
    .child(content)
    .build()
```

### Sizes

| Size | Max Width |
|------|-----------|
| `ContainerSize::Sm` | 640px |
| `ContainerSize::Md` | 768px (default) |
| `ContainerSize::Lg` | 1024px |
| `ContainerSize::Xl` | 1280px |
| `ContainerSize::Full` | No constraint |

---

## AppShell

Full-page layout with sticky header, optional sidebar, and scrollable main area. Uses CSS Grid internally.

```rust
AppShell::new()
    .header(header_bar)
    .sidebar(nav_panel)
    .main(page_content)
    .build()

// Custom dimensions
AppShell::new()
    .sidebar_width(280)
    .header_height(64)
    .header(header)
    .sidebar(sidebar)
    .main(content)
    .build()
```

Default sidebar width is 260px, header height is 56px. The header spans all columns and sticks to the top. The sidebar sticks below the header and scrolls independently.

---

## Spacer and Divider

Explicit spacing and visual separation between elements.

```rust
// Vertical space
Spacer::lg().build()

// Horizontal space (use inside a row)
Spacer::md().horizontal().build()

// Horizontal rule
Divider::horizontal().build()

// Vertical rule with custom margin
Divider::vertical().margin(SpacingSize::Lg).build()
```

Spacer sizes match Gap: `Xs` (4px), `Sm` (8px), `Md` (16px), `Lg` (24px), `Xl` (32px). Divider renders an `<hr>` element with a subtle border.
