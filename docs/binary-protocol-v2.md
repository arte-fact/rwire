# Plan: Composable Pseudo-Classes + Binary Attributes + SVG Fixes

## Context

The style token migration (Phases 1-4) is complete — all 28 components use St/Ps tokens, ComponentRegistry removed, zero inline styles except truly dynamic values. Three architectural issues remain:

1. **Ps system is not composable** — each Ps variant is a fixed (selector, CSS) pair. You can't reuse existing St tokens under pseudo-selectors, causing combinatorial explosion.
2. **SVG as CSS background-image wastes bandwidth** — `St::SelectArrow` encodes a full SVG data URI in CSS instead of using El::Svg/El::Path.
3. **Attributes are all string-based** — common attr keys (type, role, disabled) and values ("button", "true") go through the symbol table as strings instead of binary enums.

Also: server.rs has a bug — `used_pseudo_tokens()` is never passed to CapsuleConfig, so pseudo CSS is never tree-shaken.

---

## Phase 1: Binary Attribute System (`At`/`Av` enums)

**Why first**: No dependencies, SVG improvements benefit from it, highest wire savings.

### New Enums

**`At` (Attribute key, `#[repr(u8)]`)**:
- 0x00-0x09: HTML core (Type, Name, Value, Id, For, Href, Target, Rel, Placeholder, Rows)
- 0x10-0x14: Booleans (Disabled, Required, Readonly, Checked, Selected)
- 0x20-0x32: ARIA (Role, AriaLabel, AriaSelected, AriaInvalid, AriaModal, AriaBusy, AriaValuenow/min/max, AriaLive, Tabindex, etc.)
- 0x40-0x47: SVG (Xmlns, ViewBox, Fill, Stroke, StrokeWidth, StrokeLinecap, StrokeLinejoin, D)

**`Av` (Attribute value, `#[repr(u8)]`)**:
- 0x00-0x03: Common (Empty, True, False, None)
- 0x10-0x1B: Input types (Text, Password, Email, Number, Checkbox, Radio, Button, Submit, etc.)
- 0x20-0x2A: ARIA roles (RoleButton, RoleCheckbox, RoleDialog, RoleSwitch, RoleAlert, RoleStatus, RoleTab, etc.)
- 0x30-0x31: Tabindex (Zero, MinusOne)
- 0x40-0x45: SVG common (SvgNs, ViewBox24, ViewBox12, CurrentColor, Round, Stroke2)
- 0x50-0x51: Link targets (Blank, Noopener)

### New Opcodes

| Opcode | Hex | Format | Description |
|--------|-----|--------|-------------|
| SET_ATTR_ENUM | 0x26 | `[ref, at, av]` | Enum key + enum value (4 bytes) |
| SET_ATTR_BOOL | 0x27 | `[ref, at]` | Boolean attr, no value (3 bytes) |
| SET_ATTR_KEY_SYM | 0x28 | `[ref, at, val_sym_varint]` | Enum key + symbol value (4-5 bytes) |

### Builder API

```rust
el(El::Button)
    .at(At::Type, Av::Button)           // SET_ATTR_ENUM: 4 bytes
    .bool_attr(At::Disabled)             // SET_ATTR_BOOL: 3 bytes
    .at_str(At::AriaLabel, "Close")      // SET_ATTR_KEY_SYM: 4-5 bytes
    .attr("custom-thing", "value")       // SET_ATTR: 5+ bytes (escape hatch)
```

### Icon savings (icons.rs)

```rust
// Before: 7 attr() calls → ~100 bytes symbol table + 35 bytes opcodes
// After: 7 at() calls → 0 bytes symbol table + 28 bytes opcodes
el(El::Svg)
    .at(At::Xmlns, Av::SvgNs)
    .at(At::ViewBox, Av::ViewBox24)
    .at(At::Fill, Av::None)
    .at(At::Stroke, Av::CurrentColor)
    .at(At::StrokeWidth, Av::Stroke2)
    .at(At::StrokeLinecap, Av::Round)
    .at(At::StrokeLinejoin, Av::Round)
    .append([el(El::Path).at_str(At::D, icon.svg_path())])
```

### Tree-shaking

Track `used_attr_keys: HashSet<u8>` and `used_attr_values: HashSet<u8>` in BuildContext. Generate tree-shaken `AT` and `AV` lookup tables in JS capsule.

### Files

| File | Changes |
|------|---------|
| `rwire/src/attr_tokens.rs` (new) | `At`, `Av` enums + AT_MAPPINGS, AV_MAPPINGS |
| `rwire/src/protocol/opcodes.rs` | Add SET_ATTR_ENUM (0x26), SET_ATTR_BOOL (0x27), SET_ATTR_KEY_SYM (0x28) |
| `rwire/src/protocol/encoder.rs` | Add `set_attr_enum()`, `set_attr_bool()`, `set_attr_key_sym()` |
| `rwire/src/builder.rs` | Add `TypedAttr` enum, `typed_attrs` field, `.at()/.bool_attr()/.at_str()` methods, emit logic, tree-shaking |
| `rwire/src/capsule_gen.rs` | Add AT/AV tree-shaken lookup tables, 3 new opcode handlers in JS |
| `rwire/src/lib.rs` | Export `At`, `Av` |
| All 28 component files | Migrate `.attr()` → `.at()/.bool_attr()/.at_str()` where applicable |
| `rwire/src/icons.rs` | Migrate all SVG attr() calls to binary |

---

## Phase 2: Composable Pseudo-Class System

**Why second**: Breaking change across all components, labor-intensive but well-scoped.

### Design: Remove Ps, add `Pc` selector enum

Replace fixed (selector, CSS) Ps tokens with composable (Pc selector, St tokens) pairs. Any St token works with any pseudo-selector.

**`Pc` (Pseudo-Class selector, `#[repr(u8)]`)**:
- 0x00: Hover, 0x01: Focus, 0x02: FocusVisible, 0x03: Active
- 0x04: Disabled, 0x05: Checked, 0x06: Placeholder
- 0x07: Before, 0x08: After
- 0x09: FirstChild, 0x0A: LastChild, 0x0B: NthEven, 0x0C: NthOdd
- 0x0D: NotLastChild, 0x0E: FocusWithin, 0x0F: CheckedAfter

### New Opcode

| Opcode | Hex | Format | Description |
|--------|-----|--------|-------------|
| STYLE_PSEUDO | 0x89 | `[ref, pc, count, st1_varint, st2_varint, ...]` | Pseudo selector + St tokens |

Replaces STYLE_PS (0x87) and STYLE_PS_MULTI (0x88).

### Builder API

```rust
el(El::Button)
    .st([St::BgAccent9, St::TextWhite, St::RoundedMd])
    .hover([St::BgAccentHover])              // reuses existing St token!
    .focus([St::OutlineAccent, St::OutlineOffset2])
    .active([St::Scale98])
    .disabled_style([St::Opacity50, St::CursorNotAllowed, St::PointerEventsNone])
```

### CSS Generation

Class naming: `h{pc_code}u{st_code}` → CSS selector from Pc + declaration from St.

```css
.h0u577:hover { background: var(--rw-accent-10) }
.h2u241:focus-visible { outline: 2px solid var(--rw-accent-8) }
```

Tree-shaking: `HashSet<(u8, u16)>` tracks used (Pc, St) pairs.

### New St tokens for decomposed Ps tokens

| Token | CSS | Why |
|-------|-----|-----|
| ContentEmpty (0x2B0) | `content:""` | ::after/::before pseudo-elements |
| ContentAsterisk (0x2B1) | `content:" *"` | Required field asterisk |
| ContentSlash (0x2B2) | `content:"/"` | Breadcrumb separator |
| TranslateXFull (0x2B3) | `transform:translateX(100%)` | Switch knob checked state |
| TransitionTransformFast (0x2B4) | `transition:transform .2s` | Switch animation |

### Component migration pattern

```rust
// Before:
.ps([Ps::HoverBgAccent10, Ps::FocusVisibleOutlineAccent, Ps::ActiveScale98])

// After:
.hover([St::BgAccentHover])
.focus([St::OutlineAccent, St::OutlineOffset2])
.active([St::Scale98])
```

### Files

| File | Changes |
|------|---------|
| `rwire/src/style_tokens.rs` | Add `Pc` enum, remove `Ps` enum + PS_MAPPINGS, add new St tokens, rewrite `generate_pseudo_css()` |
| `rwire/src/protocol/opcodes.rs` | Replace STYLE_PS/STYLE_PS_MULTI with STYLE_PSEUDO (0x89) |
| `rwire/src/protocol/encoder.rs` | Replace `style_ps()`/`style_ps_multi()` with `style_pseudo(ref, pc, tokens)` |
| `rwire/src/builder.rs` | Replace `pseudo_tokens: Vec<u16>` with `pseudo_pairs: Vec<(Pc, St)>`, add `.hover()/.focus()/.active()/.checked()/.disabled_style()/.pseudo()`, update emit + tree-shaking |
| `rwire/src/capsule_gen.rs` | New JS STYLE_PSEUDO handler, new pseudo CSS generation |
| `rwire/src/server.rs` | **Fix bug**: add `with_pseudo_pairs(ctx.used_pseudo_pairs())` to CapsuleConfig |
| `rwire/src/lib.rs` | Export `Pc` instead of `Ps` |
| All 17 component files with `.ps()` | Migrate to `.hover()/.focus()/.active()/.checked()` etc. |

---

## Phase 3: SVG Cleanup

**Why last**: Depends on Phase 1 for binary attrs.

1. **select.rs**: Replace `St::SelectArrow` CSS data URI with proper SVG element (El::Svg + El::Path) positioned inside a wrapper div
2. **style_tokens.rs**: Remove `SelectArrow = 0x2A8` from St enum, css(), UTIL_MAPPINGS
3. **icons.rs**: Already migrated to binary attrs in Phase 1

---

## Verification

1. `cargo test --workspace` — all tests pass
2. `cargo clippy --workspace` — zero warnings
3. Run design-system example, inspect in browser:
   - Verify pseudo-class styles work (hover buttons, focus inputs, checked checkboxes)
   - Verify select dropdown arrow renders as SVG
   - Check WebSocket message size is smaller than before
4. Compare capsule size before/after
