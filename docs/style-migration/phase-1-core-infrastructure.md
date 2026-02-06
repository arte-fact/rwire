# Phase 1: Core Infrastructure — Class-Based Tokens + Pseudo-Class System

## Objective

Transform the style token runtime from inline styles to CSS classes and add a new pseudo-class token system. This phase modifies 6 core files and is the foundation for all subsequent component migrations.

## Prerequisites

- All existing tests pass (`cargo test --workspace`)
- Understanding of varint encoding (`rwire/src/protocol/varint.rs`)
- Understanding of tree-shaking in `BuildContext` (`rwire/src/builder.rs`)

## Deliverables

1. New St tokens for palette colors used by components
2. New `Ps` enum with ~25 pseudo-class/pseudo-element tokens
3. New opcodes: `STYLE_PS` (0x87), `STYLE_PS_MULTI` (0x88)
4. New encoder methods: `style_ps()`, `style_ps_multi()`
5. New builder API: `.ps()` method on `ElementBuilder`
6. Updated JS runtime: class-based STYLE_UTIL + new STYLE_PS handlers
7. Server-side CSS generation for utility + pseudo token rules

---

## Step 1: Add New St Tokens

**File**: `rwire/src/style_tokens.rs`

### New Tokens

Add to the `St` enum (all in 2-byte varint range, 0x200+):

```rust
// Padding (0x212)
Py0 = 0x212,           // padding-block:0

// Border (0x213)
BorderL4 = 0x213,      // border-left:4px solid

// Palette Backgrounds (0x220-0x226)
BgGreen4 = 0x220,      // background:var(--rw-green-4)
BgAmber4 = 0x221,      // background:var(--rw-amber-4)
BgRed4 = 0x222,        // background:var(--rw-red-4)
BgBlue2 = 0x223,       // background:var(--rw-blue-2)
BgYellow2 = 0x224,     // background:var(--rw-yellow-2)

// Palette Text Colors (0x225-0x227)
TextGreen11 = 0x225,   // color:var(--rw-green-11)
TextAmber11 = 0x226,   // color:var(--rw-amber-11)
TextRed11 = 0x227,     // color:var(--rw-red-11)

// Palette Border Colors (0x228-0x22B)
BorderGreen8 = 0x228,  // border-color:var(--rw-green-8)
BorderBlue8 = 0x229,   // border-color:var(--rw-blue-8)
BorderYellow8 = 0x22A, // border-color:var(--rw-yellow-8)
BorderRed8 = 0x22B,    // border-color:var(--rw-red-8)
```

### Update `St::css()` Match

```rust
Self::Py0 => "padding-block:0",
Self::BorderL4 => "border-left:4px solid",
Self::BgGreen4 => "background:var(--rw-green-4)",
Self::BgAmber4 => "background:var(--rw-amber-4)",
Self::BgRed4 => "background:var(--rw-red-4)",
Self::BgBlue2 => "background:var(--rw-blue-2)",
Self::BgYellow2 => "background:var(--rw-yellow-2)",
Self::TextGreen11 => "color:var(--rw-green-11)",
Self::TextAmber11 => "color:var(--rw-amber-11)",
Self::TextRed11 => "color:var(--rw-red-11)",
Self::BorderGreen8 => "border-color:var(--rw-green-8)",
Self::BorderBlue8 => "border-color:var(--rw-blue-8)",
Self::BorderYellow8 => "border-color:var(--rw-yellow-8)",
Self::BorderRed8 => "border-color:var(--rw-red-8)",
```

### Update `UTIL_MAPPINGS`

Add corresponding `(code, css_string)` entries for each new token.

### Add CSS Generation Function

```rust
/// Generate CSS rules for all used utility tokens.
///
/// Each used token becomes a CSS class rule: `.u{code}{declaration}`
/// These rules are injected via STYLE_INJECT and replace the JS lookup table.
pub fn generate_utility_css(used: &HashSet<u16>) -> String {
    let mut css = String::with_capacity(used.len() * 30);
    for &(code, declaration) in UTIL_MAPPINGS {
        if used.contains(&code) {
            // Use hex code for class name to match JS `'u'+code` where code is decimal
            // Actually, use decimal to match the varint value directly
            css.push_str(&format!(".u{}{{{}}}", code, declaration));
        }
    }
    css
}
```

---

## Step 2: Create the Ps Enum

**File**: `rwire/src/style_tokens.rs` (same file, new section)

### Pseudo Token Enum

```rust
/// Pseudo-class/pseudo-element style tokens.
///
/// Each token maps to a CSS rule with a pseudo selector.
/// Applied via `classList.add('p' + code)` with pre-injected CSS rules.
///
/// # Naming Convention
///
/// `{PseudoType}{Property}{Value}` e.g. `HoverBgAccent10`, `FocusBorderAccent`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum Ps {
    // Hover backgrounds (0x00-0x0F)
    HoverBgAccent10 = 0x00,
    HoverBgHover = 0x01,
    HoverBgRed10 = 0x02,
    HoverBgSubtle = 0x03,
    HoverBgEmphasis = 0x04,

    // Hover borders (0x05-0x07)
    HoverBorderEmphasis = 0x05,

    // Hover text (0x08-0x0B)
    HoverTextAccent = 0x08,
    HoverUnderline = 0x09,

    // Focus (0x10-0x1F)
    FocusVisibleOutlineAccent = 0x10,
    FocusBorderAccent = 0x11,
    FocusRingAccent = 0x12,

    // Active (0x20-0x2F)
    ActiveScale98 = 0x20,

    // Disabled (0x30-0x3F)
    DisabledOpacity = 0x30,

    // Placeholder (0x40-0x4F)
    PlaceholderMuted = 0x40,

    // Checked (0x48-0x4F)
    CheckedBgAccent = 0x48,
    CheckedTranslateX = 0x49,

    // Pseudo-elements (0x50-0x5F)
    AfterSpinner = 0x50,
    AfterRequiredAsterisk = 0x51,
    AfterBreadcrumbSep = 0x52,
    AfterSwitchKnob = 0x53,

    // Structural (0x60-0x6F)
    LastChildMb0 = 0x60,
    NthChildEvenBgSubtle = 0x61,
    NotLastChildBorderB = 0x62,
}
```

### Pseudo Token Mappings

```rust
/// Pseudo token mappings: (code, pseudo_selector, css_declaration).
///
/// The selector format:
/// - `:hover`, `:focus`, `:disabled` etc. for pseudo-classes
/// - `::placeholder`, `::after` etc. for pseudo-elements
/// - `:nth-child(even)`, `:last-child` etc. for structural
pub const PS_MAPPINGS: &[(u16, &str, &str)] = &[
    // Hover backgrounds
    (0x00, ":hover", "background:var(--rw-accent-10)"),
    (0x01, ":hover", "background:var(--rw-bg-hover)"),
    (0x02, ":hover", "background:var(--rw-red-10)"),
    (0x03, ":hover", "background:var(--rw-bg-subtle)"),
    (0x04, ":hover", "background:var(--rw-bg-emphasis)"),

    // Hover borders
    (0x05, ":hover", "border-color:var(--rw-border-emphasis)"),

    // Hover text
    (0x08, ":hover", "color:var(--rw-accent-11)"),
    (0x09, ":hover", "text-decoration:underline"),

    // Focus
    (0x10, ":focus-visible", "outline:2px solid var(--rw-accent-8);outline-offset:2px"),
    (0x11, ":focus", "border-color:var(--rw-accent-8);outline:none"),
    (0x12, ":focus-visible", "box-shadow:0 0 0 2px var(--rw-accent-4)"),

    // Active
    (0x20, ":active", "transform:scale(0.98)"),

    // Disabled
    (0x30, "", "opacity:.5;cursor:not-allowed;pointer-events:none"),
    // Note: empty selector means the class itself applies these properties
    // This handles both :disabled pseudo-class and explicit disabled state

    // Placeholder
    (0x40, "::placeholder", "color:var(--rw-text-muted)"),

    // Checked
    (0x48, ":checked", "background:var(--rw-accent-9)"),
    (0x49, ":checked::after", "transform:translateX(100%)"),

    // Pseudo-elements
    (0x50, "::after", "content:\"\";position:absolute;width:1em;height:1em;\
border:2px solid;border-right-color:transparent;border-radius:50%;\
animation:rw-spin .6s linear infinite"),
    (0x51, "::after", "content:' *';color:var(--rw-red-9)"),
    (0x52, "::after", "content:'/';margin:0 var(--rw-space-2);color:var(--rw-text-muted)"),
    (0x53, "::after", "content:\"\";position:absolute;width:1rem;height:1rem;\
background:white;border-radius:50%;transition:transform .2s"),

    // Structural
    (0x60, ":last-child", "margin-bottom:0"),
    (0x61, ":nth-child(even)", "background:var(--rw-bg-subtle)"),
    (0x62, ":not(:last-child)", "border-bottom:1px solid var(--rw-border-subtle)"),
];

/// Global CSS rules injected alongside pseudo tokens (e.g., @keyframes).
pub const PS_GLOBAL_CSS: &str = "@keyframes rw-spin{to{transform:rotate(360deg)}}";
```

### Pseudo CSS Generation Function

```rust
/// Generate CSS rules for all used pseudo tokens.
///
/// Each used token becomes: `.p{code}{selector}{declaration}`
pub fn generate_pseudo_css(used: &HashSet<u16>) -> String {
    let mut css = String::with_capacity(used.len() * 40);
    let mut needs_spin_keyframes = false;

    for &(code, selector, declaration) in PS_MAPPINGS {
        if used.contains(&code) {
            css.push_str(&format!(".p{}{}{{{}}}", code, selector, declaration));
            // Track if we need @keyframes rw-spin
            if declaration.contains("rw-spin") {
                needs_spin_keyframes = true;
            }
        }
    }

    if needs_spin_keyframes {
        css.push_str(PS_GLOBAL_CSS);
    }

    css
}
```

---

## Step 3: Add Protocol Opcodes

**File**: `rwire/src/protocol/opcodes.rs`

```rust
/// Apply pseudo-class style token. Format: [STYLE_PS, ref, ps_token_varint]
pub const STYLE_PS: u8 = 0x87;

/// Apply multiple pseudo-class style tokens. Format: [STYLE_PS_MULTI, ref, count, ps1_varint, ...]
pub const STYLE_PS_MULTI: u8 = 0x88;
```

---

## Step 4: Add Encoder Methods

**File**: `rwire/src/protocol/encoder.rs`

```rust
/// Apply a pseudo-class style token to an element.
///
/// Format: [STYLE_PS, ref, ps_token_varint]
pub fn style_ps(&mut self, ref_idx: u8, ps_token: u16) -> &mut Self {
    self.buf.put_u8(STYLE_PS);
    self.buf.put_u8(ref_idx);
    write_varint(&mut self.buf, ps_token as u32);
    self
}

/// Apply multiple pseudo-class style tokens to an element.
///
/// Format: [STYLE_PS_MULTI, ref, count, ps1_varint, ps2_varint, ...]
pub fn style_ps_multi(&mut self, ref_idx: u8, tokens: &[u16]) -> &mut Self {
    self.buf.put_u8(STYLE_PS_MULTI);
    self.buf.put_u8(ref_idx);
    self.buf.put_u8(tokens.len() as u8);
    for &token in tokens {
        write_varint(&mut self.buf, token as u32);
    }
    self
}
```

---

## Step 5: Update ElementBuilder

**File**: `rwire/src/builder.rs`

### Add field to ElementBuilder

```rust
pub struct ElementBuilder {
    // ... existing fields ...
    pub(crate) style_utils: Vec<u16>,
    pub(crate) style_props: Vec<(u8, u8)>,
    pub(crate) pseudo_tokens: Vec<u16>,   // NEW
    // ...
}
```

### Add `.ps()` method

```rust
/// Apply pseudo-class style tokens (hover, focus, disabled, etc.)
///
/// Pseudo tokens generate CSS class rules with pseudo-selectors.
/// Unlike `.st()` which sets base visual styles, `.ps()` handles
/// interactive state changes.
///
/// # Example
///
/// ```ignore
/// el(El::Button)
///     .st([St::BgAccent, St::TextOnAccent])
///     .ps([Ps::HoverBgAccent10, Ps::FocusVisibleOutlineAccent])
/// ```
pub fn ps(mut self, tokens: impl IntoIterator<Item = Ps>) -> Self {
    self.pseudo_tokens.extend(tokens.into_iter().map(|p| p as u16));
    self
}
```

### Add tracking to BuildContext

```rust
pub struct BuildContext {
    // ... existing fields ...
    pub used_style_utils: HashSet<u16>,
    pub used_style_props: HashSet<u8>,
    pub used_style_values: HashSet<u8>,
    pub used_pseudo_tokens: HashSet<u16>,   // NEW
    // ...
}
```

### Update `emit_element()` (~line 1003)

After the existing style_utils block, add:

```rust
// Emit pseudo-class tokens
if !el.pseudo_tokens.is_empty() {
    for &ps in &el.pseudo_tokens {
        self.used_pseudo_tokens.insert(ps);
    }

    if el.pseudo_tokens.len() >= 3 {
        self.buf.style_ps_multi(ref_idx, &el.pseudo_tokens);
    } else {
        for &ps in &el.pseudo_tokens {
            self.buf.style_ps(ref_idx, ps);
        }
    }
}
```

### Update `content_hash()`

Include `pseudo_tokens` in the hash for render dedup.

---

## Step 6: Update JS Runtime (capsule_gen.rs)

**File**: `rwire/src/capsule_gen.rs`

This is the most impactful change. The JS runtime switches from inline styles to class-based.

### Remove U Lookup Table from JS Capsule

**Before** (line 517):
```js
const U={{{utils_js}}};
```

**After**: Remove entirely. CSS declarations move to server-generated CSS rules.

### Update RUNTIME_JS Opcode Map

Add new opcodes:
```js
const O={...,PS:0x87,PM:0x88,...};
```

### Change STYLE_UTIL Handler (line 181)

**Before**:
```js
else if(o===O.SU){let f=d[i++],[u,l]=rv(d,i);i+=l;r[f].style.cssText+=';'+U[u]}
```

**After**:
```js
else if(o===O.SU){let f=d[i++],[u,l]=rv(d,i);i+=l;r[f].classList.add('u'+u)}
```

### Change STYLE_MULTI Handler (line 183)

**Before**:
```js
else if(o===O.SM){let f=d[i++],n=d[i++],css='';while(n--){let[u,l]=rv(d,i);i+=l;css+=';'+U[u]}r[f].style.cssText+=css}
```

**After**:
```js
else if(o===O.SM){let f=d[i++],n=d[i++];while(n--){let[u,l]=rv(d,i);i+=l;r[f].classList.add('u'+u)}}
```

### Add STYLE_PS Handler (new)

```js
else if(o===O.PS){let f=d[i++],[u,l]=rv(d,i);i+=l;r[f].classList.add('p'+u)}
```

### Add STYLE_PS_MULTI Handler (new)

```js
else if(o===O.PM){let f=d[i++],n=d[i++];while(n--){let[u,l]=rv(d,i);i+=l;r[f].classList.add('p'+u)}}
```

### Change COMPOSITE_TABLE Handler (line 184)

**Before** (builds CSS text in K):
```js
else if(o===O.CT){let[n,l]=rv(d,i);i+=l;while(n--){let[id,il]=rv(d,i);i+=il;let c=d[i++],css='';while(c--){let[u,ul]=rv(d,i);i+=ul;css+=';'+U[u]}K[id]=css}}
```

**After** (stores class name, CSS rule is pre-injected via STYLE_INJECT):
```js
else if(o===O.CT){let[n,l]=rv(d,i);i+=l;while(n--){let[id,il]=rv(d,i);i+=il;let c=d[i++];while(c--){let[u,ul]=rv(d,i);i+=ul}K[id]='c'+id}}
```

Or if composites are fully pre-generated in STYLE_INJECT CSS, simplify COMPOSITE_TABLE to just skip/ignore or remove entirely.

### Change STYLE_COMPOSITE Handler (line 185)

**Before**:
```js
else if(o===O.SC){let f=d[i++],[id,l]=rv(d,i);i+=l;r[f].style.cssText+=K[id]||''}
```

**After**:
```js
else if(o===O.SC){let f=d[i++],[id,l]=rv(d,i);i+=l;r[f].classList.add('c'+id)}
```

### Update `generate_styled_capsule()`

**Before** (line 499):
```rust
let utils_js = generate_style_util_map(&config.used_style_utils);
```

**After**: Remove `generate_style_util_map` call. Instead, generate CSS rules server-side.

The new `generate_css()` function in `capsule_gen.rs` or a helper:

```rust
fn generate_token_css(config: &CapsuleConfig) -> String {
    use crate::style_tokens::{generate_utility_css, generate_pseudo_css};
    let mut css = String::with_capacity(4096);

    // Utility token CSS rules (.u{code}{declaration})
    css.push_str(&generate_utility_css(&config.used_style_utils));

    // Pseudo token CSS rules (.p{code}{selector}{declaration})
    css.push_str(&generate_pseudo_css(&config.used_pseudo_tokens));

    // Component CSS is no longer needed here (will be removed in Phase 2-3)
    // But keep it for backward compatibility during migration:
    css.push_str(&config.components.generate_css());

    css
}
```

The `U` constant in the capsule HTML template is removed:
```rust
// Before:
// const U={{{utils_js}}};
// After: removed — CSS rules are in STYLE_INJECT instead
```

### Update Capsule HTML Template

```rust
format!(
    r#"<!DOCTYPE html><html {theme_attrs}><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
</head><body>
<div id="rw"></div>
<script>
const E={{{elements_js}}};
const V={{{events_js}}};
const P={{{props_js}}};
const Y={{{values_js}}};
{bind_and_local_js}
{RUNTIME_JS}
</script>
</body></html>"#
)
```

Note: `const U={...}` is removed from the template.

---

## Step 7: Update Style Groups (Composites)

**File**: `rwire/src/style_groups.rs`

### Composite CSS Generation

Add a method to generate CSS rules for composites:

```rust
impl CompositeTable {
    /// Generate CSS rules for all composites.
    ///
    /// Each composite becomes: `.c{id}{combined_declarations}`
    pub fn generate_css(&self) -> String {
        let mut css = String::new();
        for (id, tokens) in &self.composites {
            css.push_str(&format!(".c{}{{", id));
            for (i, &token_code) in tokens.iter().enumerate() {
                if i > 0 { css.push(';'); }
                // Look up the CSS declaration for this token
                if let Some(declaration) = lookup_util_css(token_code) {
                    css.push_str(declaration);
                }
            }
            css.push('}');
        }
        css
    }
}
```

### Wire Format Change

The COMPOSITE_TABLE opcode can be simplified or removed. If composite CSS is pre-injected via STYLE_INJECT, the client doesn't need the token breakdown — it just needs to know composite IDs exist as class names.

**Option A (simpler)**: Keep COMPOSITE_TABLE on wire for clients to skip token bytes, but CSS is pre-injected. The handler becomes a no-op that just advances the read pointer.

**Option B (cleaner)**: Remove COMPOSITE_TABLE from wire entirely. Server generates composite CSS in STYLE_INJECT. STYLE_COMPOSITE just adds class `'c'+id`. Client doesn't need composite table at all.

**Recommendation**: Option B. It eliminates ~100 bytes of JS handler code and composite table wire bytes.

---

## Testing Strategy

### Unit Tests

1. **style_tokens.rs**: Test `generate_utility_css()` and `generate_pseudo_css()` with known token sets
2. **encoder.rs**: Test `style_ps()` and `style_ps_multi()` encode correctly
3. **builder.rs**: Test `.ps()` API, verify `used_pseudo_tokens` tracking
4. **capsule_gen.rs**: Test capsule HTML no longer contains `const U=`

### Integration Tests

1. Build an element tree with both `.st()` and `.ps()` tokens
2. Verify STYLE_INJECT CSS contains utility + pseudo rules
3. Verify wire bytes contain STYLE_PS opcodes
4. Verify JS runtime applies correct class names

### E2E Verification

```bash
cargo test --workspace
cargo clippy --workspace
# Run an example that uses .st() tokens:
cargo run -p counter
# Navigate to http://127.0.0.1:9000, check browser console for errors
```

---

## Estimated Effort

| Task | Complexity | Lines Changed |
|------|-----------|---------------|
| New St tokens | Low | ~50 |
| Ps enum + mappings | Medium | ~150 |
| Protocol opcodes | Low | ~10 |
| Encoder methods | Low | ~25 |
| Builder .ps() + emit | Medium | ~60 |
| JS runtime rewrite | High | ~30 (compact, but critical) |
| CSS generation fns | Medium | ~40 |
| Capsule gen update | Medium | ~30 |
| Style groups update | Medium | ~30 |
| Tests | Medium | ~100 |
| **Total** | | **~525 lines** |
