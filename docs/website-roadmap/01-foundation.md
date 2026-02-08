# Phase 1: Design System Foundation

**Goal**: Extend the style token system and icon library with everything needed for a modern framework website.

**Estimated scope**: ~200 lines of Rust across `style_tokens.rs`, `icons.rs`, and a new `grid.rs` component.

---

## Step 1.1 — Grid Layout Style Tokens

Add CSS Grid tokens to the `St` enum in `rwire/src/style_tokens.rs`.

```rust
// Grid layout
St::DisplayGrid       = 0x2C0  // display:grid
St::GridCols1         = 0x2C1  // grid-template-columns:repeat(1,1fr)
St::GridCols2         = 0x2C2  // grid-template-columns:repeat(2,1fr)
St::GridCols3         = 0x2C3  // grid-template-columns:repeat(3,1fr)
St::GridCols4         = 0x2C4  // grid-template-columns:repeat(4,1fr)
St::GridColsAuto      = 0x2C5  // grid-template-columns:repeat(auto-fill,minmax(280px,1fr))
St::ColSpan2          = 0x2C6  // grid-column:span 2
St::ColSpan3          = 0x2C7  // grid-column:span 3
St::ColSpanFull       = 0x2C8  // grid-column:1/-1
```

**Files**: `rwire/src/style_tokens.rs` (St enum + css() + UTIL_MAPPINGS)

**Verification**: `cargo test --workspace` — token CSS output matches expected strings.

---

## Step 1.2 — Typography & Spacing Tokens

Larger heading sizes for hero sections and tighter leading for display text.

```rust
// Large headings
St::Text4xl       = 0x2D0  // font-size:2.25rem;line-height:2.5rem
St::Text5xl       = 0x2D1  // font-size:3rem;line-height:1
St::Text6xl       = 0x2D2  // font-size:3.75rem;line-height:1

// Leading control
St::LeadingTight  = 0x2D3  // line-height:1.25
St::LeadingSnug   = 0x2D4  // line-height:1.375

// Container widths
St::MaxW4xl       = 0x2D5  // max-width:56rem
St::MaxW5xl       = 0x2D6  // max-width:64rem
```

**Files**: `rwire/src/style_tokens.rs`

---

## Step 1.3 — Utility Tokens

Visual effects needed for logo clouds, overlays, and decorative elements.

```rust
St::Grayscale     = 0x2E0  // filter:grayscale(1)
St::OpacityHalf   = 0x2E1  // opacity:0.5
St::Opacity75     = 0x2E2  // opacity:0.75
St::ScaleUp       = 0x2E3  // transform:scale(1.05)
St::Rotate1       = 0x2E4  // transform:rotate(1deg)
```

**Files**: `rwire/src/style_tokens.rs`

---

## Step 1.4 — Social & Brand Icons

Add 7 new icons to `rwire/src/icons.rs` for social links and terminal display.

| Icon | Usage | SVG Path Source |
|------|-------|-----------------|
| `Icon::GitHub` | Header, footer social links | Simple Octicon-style mark |
| `Icon::Discord` | Community links | Simplified Discord logomark |
| `Icon::Twitter` | Social links | X/Twitter mark |
| `Icon::Crate` | crates.io link | Box/package icon |
| `Icon::Terminal` | Install command decoration | `>_` prompt icon |
| `Icon::Clipboard` | Copy button default state | Clipboard outline |
| `Icon::ClipboardCheck` | Copy button success state | Clipboard with checkmark |

Each icon follows the existing pattern: 24x24 viewBox, stroke-based, 2px stroke width.

**Files**: `rwire/src/icons.rs`

**Verification**: `cargo clippy --workspace` — no warnings. Visual spot-check with a test page.

---

## Step 1.5 — Grid Component

A responsive CSS Grid layout component, more flexible than Stack for multi-column layouts.

```rust
// API
Grid::new()
    .columns(3)              // Fixed column count
    .auto_columns("280px")   // Or responsive with auto-fill/minmax
    .gap(Gap::Lg)
    .children([card1, card2, card3])
    .build()
```

Implementation:

```rust
pub struct Grid {
    columns: GridColumns,
    gap: Gap,
    children: Vec<ElementBuilder>,
}

pub enum GridColumns {
    Fixed(u8),          // 1-4 fixed columns
    Auto(String),       // auto-fill with minmax
}

impl Grid {
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![St::DisplayGrid];
        match &self.columns {
            GridColumns::Fixed(1) => tokens.push(St::GridCols1),
            GridColumns::Fixed(2) => tokens.push(St::GridCols2),
            GridColumns::Fixed(3) => tokens.push(St::GridCols3),
            GridColumns::Fixed(4) => tokens.push(St::GridCols4),
            GridColumns::Auto(_) => tokens.push(St::GridColsAuto),
            _ => {}
        }
        tokens.push(self.gap.to_style_token());
        tokens
    }
}
```

**Files**: `rwire/src/components/grid.rs`, `rwire/src/components/mod.rs` (re-export)

**Tests**: Fixed column rendering, auto-responsive rendering, gap token selection.

---

## Verification Checklist

- [ ] `cargo clippy --workspace` — zero warnings
- [ ] `cargo test --workspace` — all pass
- [ ] New St tokens produce correct CSS strings
- [ ] New icons render in test page
- [ ] Grid component produces correct HTML structure
- [ ] All new tokens appear in tree-shaking output when used
