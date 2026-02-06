# Phase 2: Category A Component Migration — Pure Token Components

## Objective

Migrate 9 components that have NO pseudo-class/pseudo-element CSS to use 100% St tokens. These components eliminate their `*_CSS` constants entirely.

## Prerequisites

- Phase 1 complete (class-based tokens working, `.ps()` API available)
- `cargo test --workspace` passes

## Category A Components

These components' CSS contains only static properties (display, flex, gap, padding, colors, borders, etc.) — no `:hover`, `:focus`, `::before`, `::after`, or `@keyframes`.

| Component | File | CSS Eliminated | Token Count (typical) |
|-----------|------|---------------|----------------------|
| Stack | `components/stack.rs` | 429B | 3-7 |
| Spacer | `components/spacer.rs` | 329B | 1-2 |
| Divider | `components/divider.rs` | 333B | 3-5 |
| Card | `components/card.rs` | 234B | 4-5 |
| Badge | `components/badge.rs` | 276B | 7-8 |
| Text | `components/text.rs` | 613B | 2-4 |
| Alert | `components/alert.rs` | 441B | 6-8 |
| Container | `components/container.rs` | 282B | 3-4 |
| Avatar | `components/avatar.rs` | 311B | 5-7 |
| **Total** | | **3,248B** | |

## Migration Pattern

Every Category A component follows the same transformation:

### Before

```rust
fn compute_class(&self) -> String {
    let mut classes = String::with_capacity(64);
    classes.push_str("rw-component");
    if some_variant { classes.push_str(" rw-component-variant"); }
    classes
}

pub fn build(self) -> ElementBuilder {
    mark_component_used(ComponentType::Foo);
    let class = self.compute_class();
    el(El::Div).class(&class)
}

pub const FOO_CSS: &str = ".rw-component{display:flex;...}...";
```

### After

```rust
fn compute_tokens(&self) -> Vec<St> {
    let mut tokens = vec![St::DisplayFlex, ...];
    if some_variant { tokens.push(St::SomeToken); }
    tokens
}

pub fn build(self) -> ElementBuilder {
    // No mark_component_used needed — zero CSS
    let mut builder = el(El::Div).st(self.compute_tokens());
    if let Some(ref extra) = self.extra_class {
        builder = builder.class(extra.as_ref());
    }
    // ... append children ...
    builder
}

// CSS constant becomes empty
pub const FOO_CSS: &str = "";
```

### Key Points

- `compute_class()` becomes `compute_tokens()` returning `Vec<St>`
- `.class(&class)` becomes `.st(tokens)`
- `mark_component_used()` removed (no CSS to tree-shake)
- `*_CSS` constant set to `""` (removed fully in Phase 4)
- `.class()` escape hatch preserved for user custom classes (`extra_class`)

---

## Component 1: Stack

**File**: `rwire/src/components/stack.rs`

### Token Mapping

| CSS Property | Current Class | St Token |
|-------------|---------------|----------|
| `display:flex` | `.rw-stack` (base) | `St::DisplayFlex` |
| `flex-direction:column` | `.rw-stack` (base) | `St::FlexCol` |
| `flex-direction:row` | `.rw-stack-row` | `St::FlexRow` |
| `gap:0` | `.rw-gap-0` | `St::Gap0` |
| `gap:var(--rw-space-1)` | `.rw-gap-xs` | `St::GapXs` |
| `gap:var(--rw-space-2)` | `.rw-gap-sm` | `St::GapSm` |
| `gap:var(--rw-space-4)` | `.rw-stack` (default) | `St::GapMd` |
| `gap:var(--rw-space-6)` | `.rw-gap-lg` | `St::GapLg` |
| `gap:var(--rw-space-8)` | `.rw-gap-xl` | `St::GapXl` |
| `align-items:flex-start` | `.rw-items-start` | `St::ItemsStart` |
| `align-items:center` | `.rw-items-center` | `St::ItemsCenter` |
| `align-items:flex-end` | `.rw-items-end` | `St::ItemsEnd` |
| `justify-content:center` | `.rw-justify-center` | `St::JustifyCenter` |
| `justify-content:flex-end` | `.rw-justify-end` | `St::JustifyEnd` |
| `justify-content:space-between` | `.rw-justify-between` | `St::JustifyBetween` |
| `justify-content:space-around` | `.rw-justify-around` | `St::JustifyAround` |
| `flex-wrap:wrap` | `.rw-flex-wrap` | `St::FlexWrap` |

### Implementation

```rust
fn compute_tokens(&self) -> Vec<St> {
    let mut tokens = vec![St::DisplayFlex];
    tokens.push(match self.direction {
        StackDirection::Column => St::FlexCol,
        StackDirection::Row => St::FlexRow,
    });
    match self.gap {
        Gap::None => tokens.push(St::Gap0),
        Gap::Xs => tokens.push(St::GapXs),
        Gap::Sm => tokens.push(St::GapSm),
        Gap::Md => tokens.push(St::GapMd),
        Gap::Lg => tokens.push(St::GapLg),
        Gap::Xl => tokens.push(St::GapXl),
    }
    match self.align {
        StackAlign::Stretch => {} // CSS default, no token needed
        StackAlign::Start => tokens.push(St::ItemsStart),
        StackAlign::Center => tokens.push(St::ItemsCenter),
        StackAlign::End => tokens.push(St::ItemsEnd),
    }
    match self.justify {
        StackJustify::Start => {} // CSS default
        StackJustify::Center => tokens.push(St::JustifyCenter),
        StackJustify::End => tokens.push(St::JustifyEnd),
        StackJustify::Between => tokens.push(St::JustifyBetween),
        StackJustify::Around => tokens.push(St::JustifyAround),
    }
    if self.wrap { tokens.push(St::FlexWrap); }
    tokens
}

pub fn build(self) -> ElementBuilder {
    let mut builder = el(El::Div).st(self.compute_tokens());
    if let Some(ref extra) = self.extra_class {
        builder = builder.class(extra.as_ref());
    }
    for child in self.children {
        builder = builder.append([child]);
    }
    builder
}
```

### Wire Comparison (default Stack::column())

| | Current | After |
|---|---------|-------|
| Symbol table entry | "rw-stack" (8B) | none |
| SET_CLASS | 3B | none |
| STYLE_MULTI | none | 5B (opcode + ref + count + 3 tokens) |
| CSS contribution | 429B shared | 0B |

With composites: if 5+ stacks share the same pattern, composite reduces to 3B per stack.

---

## Component 2: Spacer

**File**: `rwire/src/components/spacer.rs`

### Token Mapping

Each spacer variant maps to a single height or width token:

| Variant | Current Class | St Token |
|---------|---------------|----------|
| Vertical None | `.rw-spacer-0` | height:0 via `style_prop` |
| Vertical Xs | `.rw-spacer-xs` | height via `style_prop` or `St::PyXs` |
| Vertical Sm | `.rw-spacer-sm` | height via `style_prop` |
| Vertical Md | `.rw-spacer-md` | height via `style_prop` |
| Vertical Lg | `.rw-spacer-lg` | height via `style_prop` |
| Vertical Xl | `.rw-spacer-xl` | height via `style_prop` |
| Horizontal * | `.rw-spacer-h-*` | width via `style_prop` |

**Note**: Spacer uses specific heights like `var(--rw-space-1)` which don't map to existing St tokens as height values. Use `style_prop(StyleProp::Height, StyleValue::*)` or add a `style()` escape hatch.

---

## Component 3: Divider

**File**: `rwire/src/components/divider.rs`

### Token Mapping

| CSS Property | St Token |
|-------------|----------|
| `border:none` | `St::BorderNone` |
| `border-top:1px solid var(--rw-border-default)` | `St::BorderT` |
| `border-left:1px solid var(--rw-border-default)` | `St::BorderL` |
| `height:1px` | via `style_prop` |
| `width:1px` (vertical) | via `style_prop` |
| margin variants | `St::My0` / `St::MyXs` / `St::MySm` / `St::MyMd` / `St::MyLg` / `St::MyXl` |

---

## Component 4: Card

**File**: `rwire/src/components/card.rs`

### Token Mapping

| Property | Default | St Token |
|----------|---------|----------|
| background | app | `St::BgApp` |
| border | subtle | `St::BorderSubtle` |
| border-radius | lg | `St::RoundedLg` |
| padding | md | `St::PMd` |
| box-shadow | sm | `St::ShadowSm` |

Variants: `CardPadding` maps to `St::P0 / St::PSm / St::PMd / St::PLg`, `CardShadow` maps to `St::ShadowNone / St::ShadowMd / St::ShadowLg`.

---

## Component 5: Badge

**File**: `rwire/src/components/badge.rs`

### Token Mapping

| Property | St Token |
|----------|----------|
| `display:inline-flex` | `St::DisplayInlineFlex` |
| `align-items:center` | `St::ItemsCenter` |
| `padding-inline` | `St::PxSm` |
| `font-size` | `St::TextXs` |
| `font-weight:500` | `St::FontMedium` |
| `border-radius:9999px` | `St::RoundedFull` |
| Default bg | `St::BgEmphasis` |
| Default color | `St::TextHigh` |

Intent variants use palette tokens:
- Primary: `St::BgAccent` + `St::TextOnAccent` (or `St::BgAccent4` + `St::TextAccent`)
- Success: `St::BgGreen4` + `St::TextGreen11` (new tokens from Phase 1)
- Warning: `St::BgAmber4` + `St::TextAmber11`
- Error: `St::BgRed4` + `St::TextRed11`

---

## Component 6: Text

**File**: `rwire/src/components/text.rs`

### Token Mapping

| Variant | St Tokens |
|---------|-----------|
| Default (body) | `St::TextDefault` + `St::LeadingNormal` |
| Sm | `St::TextSm` |
| H1 | `St::Text3xl` + `St::FontBold` + `St::LeadingTight` |
| H2 | `St::Text2xl` + `St::FontSemibold` + `St::LeadingTight` |
| H3 | `St::TextXl` + `St::FontSemibold` + `St::LeadingSnug` |
| Label | `St::TextSm` + `St::FontMedium` |
| Caption | `St::TextXs` + `St::TextMuted` |

Color variants: `St::TextHigh`, `St::TextMuted`, `St::TextAccent`, `St::TextSuccess`, `St::TextWarning`, `St::TextError`

---

## Component 7: Alert

**File**: `rwire/src/components/alert.rs`

### Token Mapping

Base: `St::DisplayFlex` + `St::FlexCol` + `St::GapSm` + `St::PMd` + `St::RoundedMd` + `St::BorderL4` + `St::TextSm`

Intent colors use palette tokens (same as Badge but different shades):
- Info: `St::BgBlue2` + `St::BorderBlue8` + `St::TextDefault`
- Success: `St::BgGreen4` + `St::BorderGreen8` + `St::TextDefault`
- Warning: `St::BgYellow2` + `St::BorderYellow8` + `St::TextDefault`
- Error: `St::BgRed4` + `St::BorderRed8` + `St::TextDefault`

Child elements (title, message) use tokens on their own ElementBuilders:
- Title: `St::FontMedium` + `St::TextHigh`
- Message: `St::M0` + text color inherited

---

## Component 8: Container

**File**: `rwire/src/components/container.rs`

### Token Mapping

Base: `St::WFull` + `St::MxAuto` + `St::PxMd`

Size variants need max-width values. Options:
- Use `style()` escape hatch: `.style("max-width:40rem")`
- Use `StyleProp::MaxWidth` + `StyleValue` for specific sizes
- Add tokens if reused enough (recommendation: use style() since each max-width is one-off)

| Size | Approach |
|------|----------|
| Sm (40rem) | `.style("max-width:40rem")` |
| Md (48rem) | `.style("max-width:48rem")` |
| Lg (64rem) | `.style("max-width:64rem")` |
| Xl (80rem) | `.style("max-width:80rem")` |
| Full | No max-width (default) |

---

## Component 9: Avatar

**File**: `rwire/src/components/avatar.rs`

### Token Mapping

Base: `St::DisplayInlineFlex` + `St::ItemsCenter` + `St::JustifyCenter` + `St::RoundedFull` + `St::OverflowHidden`

Fallback: `St::BgMuted` + `St::TextHigh` + `St::FontMedium`

Size variants need width/height. Use `style_prop` for specific sizes (2rem, 2.5rem, 3rem) since these are component-specific.

Image child: `St::WFull` + `St::HFull` + object-fit via `style()`.

---

## Test Updates

For each component:

1. **Remove CSS assertion tests** (e.g., `test_stack_css_size`, `test_button_css_structure`)
2. **Update class assertion tests** to verify tokens instead:

```rust
#[test]
fn test_stack_default_tokens() {
    let stack = Stack::new();
    let tokens = stack.compute_tokens();
    assert!(tokens.contains(&St::DisplayFlex));
    assert!(tokens.contains(&St::FlexCol));
    assert!(tokens.contains(&St::GapMd));
}

#[test]
fn test_stack_row_tokens() {
    let stack = Stack::row();
    let tokens = stack.compute_tokens();
    assert!(tokens.contains(&St::FlexRow));
    assert!(!tokens.contains(&St::FlexCol));
}
```

3. **Remove `compute_class()` tests** — function no longer exists
4. **Keep behavioral tests** (builder chain, children, etc.)

## Verification Checklist

After migrating each component:

- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` — zero warnings
- [ ] Component's `*_CSS` constant is `""`
- [ ] No `mark_component_used()` call
- [ ] `compute_class()` replaced with `compute_tokens()`
- [ ] `.class()` escape hatch still works for custom classes
- [ ] Run example app, verify component renders correctly

## Estimated Savings After Phase 2

| Metric | Change |
|--------|--------|
| STYLE_INJECT CSS | -3,248 bytes (component CSS eliminated) |
| Symbol table | -~800 bytes (class strings for 9 components) |
| JS token lookups | +~350 bytes (new token CSS rules for ~14 new tokens used) |
| **Net per connection** | **-~3,700 bytes** |
