---
title: Style Tokens
description: The St enum and how CSS is encoded as integers
order: 1
---
# Style Tokens

Style tokens are the primary way to apply CSS in rwire. Instead of writing class names or inline styles, you pass `St` enum variants to the `.st()` method:

```rust
use rwire::{el, El, St};

el(El::Div).st([
    St::DisplayFlex,
    St::FlexCol,
    St::GapMd,
    St::PxLg,
    St::PyMd,
    St::BgApp,
    St::TextDefault,
    St::RoundedMd,
])
```

Each `St` variant maps to a single CSS declaration. The framework encodes tokens as `u16` integers on the wire, and the browser applies them as CSS classes via a generated lookup table.

## Token Categories

### Layout

```rust
St::DisplayFlex      // display: flex
St::DisplayGrid      // display: grid
St::FlexCol          // flex-direction: column
St::FlexRow          // flex-direction: row
St::FlexWrap         // flex-wrap: wrap
St::ItemsCenter      // align-items: center
St::JustifyCenter    // justify-content: center
St::JustifyBetween   // justify-content: space-between
```

### Spacing

```rust
St::PxSm     // padding-left/right: 0.5rem
St::PxMd     // padding-left/right: 1rem
St::PxLg     // padding-left/right: 1.5rem
St::PySm     // padding-top/bottom: 0.5rem
St::PyMd     // padding-top/bottom: 1rem
St::PyLg     // padding-top/bottom: 1.5rem
St::GapSm    // gap: 0.5rem
St::GapMd    // gap: 1rem
St::MbMd     // margin-bottom: 1rem
St::MtLg     // margin-top: 1.5rem
```

### Typography

```rust
St::TextXs      // font-size: 0.75rem
St::TextSm      // font-size: 0.875rem
St::TextBase    // font-size: 1rem
St::TextLg      // font-size: 1.125rem
St::TextXl      // font-size: 1.25rem
St::FontBold    // font-weight: 700
St::FontMedium  // font-weight: 500
St::TextCenter  // text-align: center
St::LeadingSnug // line-height: 1.375
```

### Colors (Semantic)

```rust
St::TextDefault   // color: var(--rw-text-default)
St::TextMuted     // color: var(--rw-text-muted)
St::TextHigh      // color: var(--rw-text-high)
St::BgApp         // background: var(--rw-bg-app)
St::BgSubtle      // background: var(--rw-bg-subtle)
St::BgSurface     // background: var(--rw-surface)
St::TextAccent    // color: var(--rw-accent-11)
St::BgAccentSubtle // background: var(--rw-accent-3)
```

### Borders and Effects

```rust
St::RoundedSm         // border-radius: sm
St::RoundedMd         // border-radius: md
St::RoundedLg         // border-radius: lg
St::BorderDefault     // border: 1px solid var(--rw-border-default)
St::BorderSubtle      // border: 1px solid var(--rw-border-subtle)
St::TransitionColors  // transition: colors 150ms
St::CursorPointer     // cursor: pointer
```

## Pseudo-Class Styles

Apply styles on hover, focus, or active states using chained methods:

```rust
el(El::Button)
    .st([St::BgSurface, St::TextDefault, St::PxMd, St::PySm, St::RoundedMd])
    .hover([St::BgHover])
    .focus([St::RingAccent])
    .active([St::BgActive])
```

The pseudo system generates CSS rules like `:hover`, `:focus-visible`, and `:active` scoped to each element.

## Wire Encoding

The `St` enum uses `#[repr(u16)]` internally. On the wire, tokens are varint-encoded: values under 128 take 1 byte, larger values take 2 bytes. A typical styled element sends 5-10 tokens at 1-2 bytes each, far smaller than equivalent CSS class strings.

The framework currently defines 580+ tokens covering CSS3 layout, spacing, typography, color, border, shadow, and animation properties.
