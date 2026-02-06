# Phase 3: Category B Component Migration — Hybrid Token + Pseudo Components

## Objective

Migrate 19 components that use pseudo-classes (`:hover`, `:focus`, `::after`, etc.) to the hybrid `.st()` + `.ps()` pattern. Each component's `*_CSS` constant is eliminated entirely — all styling comes from tokens.

## Prerequisites

- Phase 1 complete (class-based tokens + Ps system working)
- Phase 2 complete (Category A proves the token pattern)
- `cargo test --workspace` passes

## Category B Components

| Component | File | CSS Eliminated | St Tokens | Ps Tokens |
|-----------|------|---------------|-----------|-----------|
| Button | `button.rs` | 1,231B | 10-12 | 2-4 |
| Input | `input.rs` | 801B | 8-10 | 3-4 |
| Textarea | `textarea.rs` | 688B | 8-10 | 3-4 |
| Checkbox | `checkbox.rs` | 383B | 6-8 | 2-3 |
| Radio | `radio.rs` | 362B | 6-8 | 2-3 |
| Switch | `switch.rs` | 431B | 8-10 | 3-4 |
| Select | `select.rs` | 621B | 8-10 | 3-4 |
| Link | `link.rs` | 243B | 3-4 | 2-3 |
| Modal | `modal.rs` | 1,533B | 10-15 | 1-2 |
| Tabs | `tabs.rs` | 389B | 5-7 | 1-2 |
| Pagination | `pagination.rs` | 419B | 6-8 | 2-3 |
| Breadcrumb | `breadcrumb.rs` | 344B | 4-6 | 1-2 |
| ThemeToggle | `theme_toggle.rs` | 447B | 6-8 | 2-3 |
| Spinner | `spinner.rs` | 288B | 3-5 | 1 |
| Progress | `progress.rs` | 265B | 4-6 | 0-1 |
| Label | `label.rs` | 160B | 3-4 | 0-1 |
| List | `list.rs` | 218B | 3-4 | 1 |
| Table | `table.rs` | 318B | 4-6 | 1-2 |
| FormField | `form_field.rs` | 306B | 4-6 | 0-1 |
| **Total** | | **9,447B** | | |

## Hybrid Pattern

Each Category B component uses both St tokens (base visuals) and Ps tokens (interactive states):

```rust
pub fn build(self) -> ElementBuilder {
    let st = self.compute_tokens();    // Vec<St> — display, flex, colors, etc.
    let ps = self.compute_pseudo();     // Vec<Ps> — hover, focus, disabled, etc.
    let mut builder = el(El::Button).st(st).ps(ps);
    // ... text, attrs, children ...
    builder
}
```

No `.class()` needed (except for user escape hatch via `extra_class`).
No `*_CSS` constant needed.
No `mark_component_used()` needed.

---

## Component 1: Button (reference implementation)

**File**: `rwire/src/components/button.rs`

### Current CSS Analysis

```css
/* Base (12 properties) */
.rw-btn{display:inline-flex;align-items:center;justify-content:center;
  gap:var(--rw-space-2);font-weight:var(--rw-font-medium);
  border-radius:var(--rw-radius-md);border:1px solid transparent;
  cursor:pointer;transition:background .15s;height:2.25rem;
  padding:0 var(--rw-space-4);font-size:var(--rw-text-sm);
  background:var(--rw-accent-9);color:var(--rw-text-on-accent)}

/* Pseudo-classes */
.rw-btn:hover{background:var(--rw-accent-10)}
.rw-btn:focus-visible{outline:2px solid var(--rw-accent-8);outline-offset:2px}
.rw-btn-secondary:hover{background:var(--rw-bg-hover);border-color:var(--rw-border-emphasis)}
.rw-btn-ghost:hover{background:var(--rw-bg-hover)}
.rw-btn-destructive:hover{background:var(--rw-red-10)}

/* States */
.rw-btn-disabled{opacity:.5;cursor:not-allowed;pointer-events:none}
.rw-btn-loading{position:relative;color:transparent}
.rw-btn-loading::after{content:"";...;animation:rw-spin .6s linear infinite}
.rw-btn-full{width:100%}
@keyframes rw-spin{to{transform:rotate(360deg)}}
```

### Token Mapping

**Base St tokens** (all intents share these):
```rust
vec![
    St::DisplayInlineFlex,  // display:inline-flex
    St::ItemsCenter,        // align-items:center
    St::JustifyCenter,      // justify-content:center
    St::GapSm,              // gap:var(--rw-space-2)
    St::FontMedium,         // font-weight:500
    St::RoundedMd,          // border-radius:var(--rw-radius-md)
    St::BorderTransparent,  // border:1px solid transparent
    St::CursorPointer,      // cursor:pointer
    St::TransitionColors,   // transition:color,background-color 0.2s
    St::TextSm,             // font-size:var(--rw-text-sm)
]
```

**Height and padding**: `height:2.25rem` and `padding:0 var(--rw-space-4)` need special handling:
- Height: use `style_prop(StyleProp::Height, StyleValue::*)` or add custom height tokens
- Padding: `St::Py0` (new token) + `St::PxMd`

**Intent-specific tokens**:
```rust
match self.intent {
    ButtonIntent::Primary => {
        st.extend([St::BgAccent, St::TextOnAccent]);
        ps.push(Ps::HoverBgAccent10);
    }
    ButtonIntent::Secondary => {
        st.extend([St::BgMuted, St::TextHigh, St::BorderDefault]);
        ps.extend([Ps::HoverBgHover, Ps::HoverBorderEmphasis]);
    }
    ButtonIntent::Ghost => {
        st.extend([St::BgTransparent, St::TextHigh]);
        ps.push(Ps::HoverBgHover);
    }
    ButtonIntent::Destructive => {
        st.extend([St::BgRed, St::TextWhite]);
        ps.push(Ps::HoverBgRed10);
    }
}
```

**Size-specific tokens**:
```rust
match self.size {
    ButtonSize::Sm => {
        st.extend([St::TextXs, St::GapXs]);
        // height:1.75rem + padding:0 var(--rw-space-3)
    }
    ButtonSize::Md => {
        // defaults from base tokens
    }
    ButtonSize::Lg => {
        st.extend([St::TextBase, St::GapMd]);
        // height:2.75rem + padding:0 var(--rw-space-6)
    }
}
```

**State tokens**:
```rust
// Focus always present
ps.push(Ps::FocusVisibleOutlineAccent);

// Conditional states
if self.disabled {
    ps.push(Ps::DisabledOpacity);
}
if self.loading {
    st.push(St::PositionRelative);
    st.push(St::TextTransparent); // hide text, spinner shows
    ps.push(Ps::AfterSpinner);
}
if self.full_width {
    st.push(St::WFull);
}
```

### Full Implementation

```rust
fn compute_tokens(&self) -> Vec<St> {
    let mut tokens = vec![
        St::DisplayInlineFlex, St::ItemsCenter, St::JustifyCenter,
        St::GapSm, St::FontMedium, St::RoundedMd, St::BorderTransparent,
        St::CursorPointer, St::TransitionColors, St::TextSm,
        St::Py0, St::PxMd, // padding
    ];

    match self.intent {
        ButtonIntent::Primary => tokens.extend([St::BgAccent, St::TextOnAccent]),
        ButtonIntent::Secondary => tokens.extend([St::BgMuted, St::TextHigh, St::BorderDefault]),
        ButtonIntent::Ghost => tokens.extend([St::BgTransparent, St::TextHigh]),
        ButtonIntent::Destructive => tokens.extend([St::BgRed, St::TextWhite]),
    }

    match self.size {
        ButtonSize::Sm => {
            tokens.retain(|t| !matches!(t, St::GapSm | St::TextSm | St::PxMd));
            tokens.extend([St::TextXs, St::GapXs, St::PxSm]);
        }
        ButtonSize::Md => {} // defaults
        ButtonSize::Lg => {
            tokens.retain(|t| !matches!(t, St::GapSm | St::TextSm | St::PxMd));
            tokens.extend([St::TextBase, St::GapMd, St::PxLg]);
        }
    }

    if self.loading {
        tokens.push(St::PositionRelative);
    }
    if self.full_width {
        tokens.push(St::WFull);
    }

    tokens
}

fn compute_pseudo(&self) -> Vec<Ps> {
    let mut pseudo = vec![Ps::FocusVisibleOutlineAccent];

    match self.intent {
        ButtonIntent::Primary => pseudo.push(Ps::HoverBgAccent10),
        ButtonIntent::Secondary => pseudo.extend([Ps::HoverBgHover, Ps::HoverBorderEmphasis]),
        ButtonIntent::Ghost => pseudo.push(Ps::HoverBgHover),
        ButtonIntent::Destructive => pseudo.push(Ps::HoverBgRed10),
    }

    if self.disabled { pseudo.push(Ps::DisabledOpacity); }
    if self.loading { pseudo.push(Ps::AfterSpinner); }

    pseudo
}

pub fn build(self) -> ElementBuilder {
    let st = self.compute_tokens();
    let ps = self.compute_pseudo();
    let mut builder = el(El::Button).st(st).ps(ps);

    if let Some(text) = self.text {
        builder = builder.text(&text);
    }
    if self.disabled {
        builder = builder.attr("disabled", "");
    }
    if self.loading {
        builder = builder.attr("aria-busy", "true");
    }
    if let Some(ref extra) = self.extra_class {
        builder = builder.class(extra.as_ref());
    }
    builder
}
```

### Variant Trait Removed

The `Variant` trait and `impl Variant for ButtonIntent/ButtonSize` are no longer needed since we don't produce class strings. They can be removed or kept if used elsewhere.

---

## Component 2: Input

**File**: `rwire/src/components/input.rs`

### St Tokens
```
DisplayBlock, WFull, Py0, PxMd, TextSm, LeadingNormal,
RoundedMd, BorderDefault, BgApp, TextDefault, TransitionColors
```

Size variants modify font-size and padding.

### Ps Tokens
```
FocusBorderAccent     // :focus{border-color:var(--rw-accent-8)}
HoverBorderEmphasis   // :hover{border-color:var(--rw-border-emphasis)}
PlaceholderMuted      // ::placeholder{color:var(--rw-text-muted)}
DisabledOpacity       // (if disabled)
```

Invalid state: add `St::BorderRed` (or similar) conditionally.

---

## Component 3: Textarea

Same pattern as Input, with additional resize handling via `style()`.

---

## Component 4: Checkbox

### St Tokens (wrapper)
```
DisplayInlineFlex, ItemsCenter, GapSm, CursorPointer
```

### Ps Tokens
```
HoverBorderEmphasis   // checkbox hover
FocusRingAccent       // checkbox focus ring
CheckedBgAccent       // :checked background
DisabledOpacity       // if disabled
```

The custom checkbox visual (hiding native, showing styled box) may need additional inline styles or a minimal CSS approach for the `appearance:none` + custom styling.

---

## Component 5: Radio

Same pattern as Checkbox with `RoundedFull` instead of `RoundedMd`.

---

## Component 6: Switch

### St Tokens (track)
```
PositionRelative, DisplayInlineFlex, ItemsCenter,
BgMuted, RoundedFull, CursorPointer, TransitionColors
```

### Ps Tokens
```
CheckedBgAccent       // :checked track color
AfterSwitchKnob       // ::after for the knob
CheckedTranslateX     // :checked::after translateX
FocusRingAccent       // focus ring
DisabledOpacity
```

---

## Component 7: Select

Same pattern as Input, with additional custom arrow styling via `style()` for the SVG background-image.

---

## Component 8: Link

### St Tokens
```
TextAccent, CursorPointer
```

### Ps Tokens
```
HoverTextAccent       // :hover color change (or darker shade)
HoverUnderline        // :hover text-decoration
FocusVisibleOutlineAccent
```

---

## Component 9: Modal

### St Tokens (container)
```
PositionFixed, Inset0 (or AbsoluteFill/FixedFill composite),
DisplayFlex, ItemsCenter, JustifyCenter
```

### Sub-elements
- Backdrop: `St::FixedFill` + `St::BgEmphasis` + `St::Opacity75`
- Content: `St::BgApp` + `St::RoundedLg` + `St::ShadowXl` + `St::PMd`
- Header: `St::DisplayFlex` + `St::JustifyBetween` + `St::ItemsCenter`
- Close button: inherits from Button component (reuse)

### Ps Tokens
```
HoverBgHover          // close button hover (or delegate to Button)
```

Modal visibility toggling uses `St::DisplayNone` conditionally.

---

## Component 10: Tabs

### St Tokens (tab list)
```
DisplayFlex, GapXs, BorderB
```

### St Tokens (individual tab)
```
PxMd, PySm, TextSm, CursorPointer, TextMuted
```

Active tab: `St::TextAccent` + `St::FontMedium` + bottom border via token

### Ps Tokens
```
HoverBgSubtle         // tab hover
```

---

## Component 11: Pagination

### St Tokens (button)
```
DisplayInlineFlex, ItemsCenter, JustifyCenter, PxSm, PySm,
RoundedMd, TextSm, CursorPointer
```

Active: `St::BgAccent` + `St::TextOnAccent`

### Ps Tokens
```
HoverBgHover          // page button hover
DisabledOpacity       // prev/next when at boundary
```

---

## Component 12: Breadcrumb

### St Tokens (list)
```
DisplayFlex, ItemsCenter, GapXs, TextSm
```

### Ps Tokens (link items)
```
HoverTextAccent
AfterBreadcrumbSep    // ::after for "/" separator on non-last items
```

Current item: `St::TextMuted` (no link, no hover)

---

## Component 13: ThemeToggle

### St Tokens
```
DisplayInlineFlex, ItemsCenter, JustifyCenter,
PxSm, PySm, RoundedMd, BorderDefault, CursorPointer,
TransitionColors, BgApp
```

### Ps Tokens
```
HoverBgHover
FocusVisibleOutlineAccent
ActiveScale98
```

---

## Component 14: Spinner

### St Tokens
```
DisplayInlineBlock, RoundedFull, BorderDefault
```

Size via style_prop for width/height.

### Ps Tokens / Animation
The spin animation is the core feature. Options:
- Use `Ps::AnimateSpin` which generates both the class and @keyframes
- Or keep a minimal CSS constant for the animation only

Recommendation: Add `Ps::AnimateSpin` that generates:
```css
.p{code}{animation:rw-spin .6s linear infinite}
```
With `@keyframes rw-spin{to{transform:rotate(360deg)}}` injected as a side-effect.

The border-right-color:transparent (for the gap in the spinner) can be a St token or inline style.

---

## Component 15: Progress

### St Tokens (track)
```
WFull, BgMuted, RoundedFull, OverflowHidden
```

### St Tokens (bar)
```
HFull, BgAccent, RoundedFull, TransitionAll
```

Bar width is dynamic (set via `style("width:{pct}%")`).

---

## Component 16: Label

### St Tokens
```
DisplayBlock, TextSm, FontMedium, TextHigh, MbXs
```

### Ps Tokens (if required asterisk)
```
AfterRequiredAsterisk   // ::after{content:' *';color:var(--rw-red-9)}
```

---

## Component 17: List

### St Tokens
```
M0, PLg (for ordered list indent), TextDefault
```

### Ps Tokens
```
LastChildMb0          // :last-child{margin-bottom:0} on list items
```

---

## Component 18: Table

### St Tokens (table)
```
WFull, BorderSubtle
```

### St Tokens (row/cell)
- Header row: `St::BgSubtle` + `St::FontMedium`
- Cell: `St::PxMd` + `St::PySm` + `St::TextSm`

### Ps Tokens
```
NthChildEvenBgSubtle    // :nth-child(even) for striped rows
NotLastChildBorderB     // :not(:last-child) bottom border
```

---

## Component 19: FormField

### St Tokens
```
DisplayFlex, FlexCol, GapXs
```

Sub-elements:
- Label: reuse Label tokens
- Help text: `St::TextXs` + `St::TextMuted`
- Error message: `St::TextXs` + `St::TextError`

Error state: conditionally add `St::TextError` to relevant children.

---

## Migration Order Strategy

Migrate in dependency order — components that are used by other components first:

1. **Label** (used by FormField, Checkbox, Radio, Switch)
2. **Spinner** (used by Button loading state concept)
3. **Button** (reference implementation, most complex)
4. **Input, Textarea, Select** (form input family, similar pattern)
5. **Checkbox, Radio, Switch** (form control family)
6. **FormField** (wrapper, uses Label)
7. **Link** (simple)
8. **List, Table** (data display)
9. **Tabs, Pagination, Breadcrumb** (navigation)
10. **Progress** (simple)
11. **Modal** (complex, multiple sub-elements)
12. **ThemeToggle** (standalone)

## Test Updates

For each component:

1. Replace `compute_class()` tests with `compute_tokens()` + `compute_pseudo()` tests
2. Verify specific tokens present for each variant
3. Remove CSS size/structure tests
4. Add integration test: build element, verify both `.style_utils` and `.pseudo_tokens` populated

Example:
```rust
#[test]
fn test_button_primary_tokens() {
    let btn = Button::primary("Save");
    let tokens = btn.compute_tokens();
    assert!(tokens.contains(&(St::BgAccent as u16)));
    assert!(tokens.contains(&(St::TextOnAccent as u16)));

    let pseudo = btn.compute_pseudo();
    assert!(pseudo.contains(&(Ps::HoverBgAccent10 as u16)));
    assert!(pseudo.contains(&(Ps::FocusVisibleOutlineAccent as u16)));
}

#[test]
fn test_button_disabled_pseudo() {
    let btn = Button::primary("Save").disabled(true);
    let pseudo = btn.compute_pseudo();
    assert!(pseudo.contains(&(Ps::DisabledOpacity as u16)));
}
```

## Verification Checklist (per component)

- [ ] `compute_class()` removed, replaced with `compute_tokens()` + `compute_pseudo()`
- [ ] `*_CSS` constant set to `""`
- [ ] `mark_component_used()` removed
- [ ] No `.class()` calls except for `extra_class` escape hatch
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` — zero warnings
- [ ] Visual verification via Playwright (for components used in examples)
- [ ] Hover states work (verify via browser interaction)
- [ ] Focus states work (verify via Tab key navigation)
- [ ] Disabled states work (verify opacity and pointer-events)

## Estimated Savings After Phase 3

| Metric | Change |
|--------|--------|
| STYLE_INJECT CSS | -9,447 bytes (all remaining component CSS eliminated) |
| Symbol table | -~1,600 bytes (class strings for 19 components) |
| Token CSS rules | +~1,500 bytes (new utility + pseudo rules for components) |
| **Net per connection** | **-~9,547 bytes** |

Combined with Phase 2: **-~13,247 bytes total CSS reduction (87%)**
