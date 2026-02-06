# Phase 4: Cleanup — Remove Legacy CSS System

## Objective

Remove all legacy component CSS infrastructure now that all components use St + Ps tokens exclusively. Clean up the codebase, update documentation, and verify final payload sizes.

## Prerequisites

- Phase 1, 2, 3 complete
- All components migrated to `.st()` + `.ps()` pattern
- `cargo test --workspace` passes
- All `*_CSS` constants are `""`

---

## Step 1: Remove CSS Constants

**Files**: All `rwire/src/components/*.rs`

Delete all `*_CSS` constant definitions:

```rust
// DELETE from each component file:
pub const BUTTON_CSS: &str = "";
pub const INPUT_CSS: &str = "";
pub const STACK_CSS: &str = "";
// ... etc.
```

Also remove the `pub use` re-exports in `mod.rs`.

---

## Step 2: Simplify mod.rs

**File**: `rwire/src/components/mod.rs`

### Remove CSS Re-exports

```rust
// DELETE all *_CSS re-exports:
// pub use button::{Button, ButtonIntent, ButtonSize, BUTTON_CSS};
// becomes:
pub use button::{Button, ButtonIntent, ButtonSize};
```

### Remove `generate_components_css()`

The function that concatenates all component CSS is no longer needed:

```rust
// DELETE this entire function:
pub fn generate_components_css() -> String { ... }
```

### Remove CSS-related tests

```rust
// DELETE:
fn test_components_css_not_empty() { ... }
fn test_total_components_css_size() { ... }
```

---

## Step 3: Simplify ComponentRegistry

**File**: `rwire/src/components/registry.rs`

The registry was used for CSS tree-shaking. With zero component CSS, its primary purpose is gone.

### Option A: Remove entirely

If no other code depends on it, delete `registry.rs` and all references:
- Remove `pub mod registry;` from `mod.rs`
- Remove `pub use registry::*` from `mod.rs`
- Remove `mark_component_used()` calls (already done in Phase 2-3)
- Remove `begin_tracking()` / `end_tracking()` from server.rs

### Option B: Keep for metrics/debugging (lighter)

If component usage tracking is useful for debugging or analytics, keep a minimal version:

```rust
/// Lightweight component usage tracker (no CSS generation).
#[derive(Clone, Debug, Default)]
pub struct ComponentRegistry {
    used: HashSet<ComponentType>,
}

impl ComponentRegistry {
    pub fn new() -> Self { Self::default() }
    pub fn mark_used(&mut self, component: ComponentType) { self.used.insert(component); }
    pub fn is_used(&self, component: ComponentType) -> bool { self.used.contains(&component) }
    pub fn used_components(&self) -> impl Iterator<Item = ComponentType> + '_ { self.used.iter().copied() }
    pub fn len(&self) -> usize { self.used.len() }
    pub fn is_empty(&self) -> bool { self.used.is_empty() }
    // DELETE: generate_css(), css_size(), print_budget_report()
}
```

**Recommendation**: Option A (remove entirely). We're in experimental phase, and the registry adds code with no functional benefit after migration.

---

## Step 4: Remove Variant Trait (if unused)

**File**: `rwire/src/variants.rs` (if it exists) or wherever `Variant` trait is defined

The `Variant` trait was used to produce CSS class strings:
```rust
pub trait Variant {
    fn class(&self) -> Option<&'static str>;
}
```

Check if any code still uses it. If not, delete the trait and all implementations.

---

## Step 5: Update capsule_gen.rs

**File**: `rwire/src/capsule_gen.rs`

### Remove component CSS from STYLE_INJECT

The CSS generation path currently includes component CSS:

```rust
// REMOVE component_css from the CSS generation:
// let component_css = config.components.generate_css();  // DELETE
```

The STYLE_INJECT CSS now only contains:
1. Theme/design token CSS (base variables, semantic colors)
2. Utility token CSS rules (`.u{code}{declaration}`)
3. Pseudo token CSS rules (`.p{code}{selector}{declaration}`)
4. Composite CSS rules (`.c{id}{combined}`)

### Remove ComponentRegistry from CapsuleConfig

```rust
pub struct CapsuleConfig {
    pub theme: Theme,
    pub has_local_handlers: bool,
    pub used_style_utils: HashSet<u16>,
    pub used_style_props: HashSet<u8>,
    pub used_style_values: HashSet<u8>,
    pub used_pseudo_tokens: HashSet<u16>,  // Added in Phase 1
    // REMOVE: pub components: ComponentRegistry,
}
```

### Update CSS Generation

```rust
fn generate_all_css(config: &CapsuleConfig) -> String {
    let mut css = String::with_capacity(8192);

    // 1. Base CSS (reset, design tokens)
    css.push_str(&generate_base_css(&config.theme));

    // 2. Utility token rules (tree-shaken)
    css.push_str(&generate_utility_css(&config.used_style_utils));

    // 3. Pseudo token rules (tree-shaken)
    css.push_str(&generate_pseudo_css(&config.used_pseudo_tokens));

    // 4. Composite rules (if using composite optimization)
    // Generated from style_groups CompositeTable

    css
}
```

---

## Step 6: Update server.rs

**File**: `rwire/src/server.rs`

### Remove component tracking

```rust
// REMOVE begin_tracking/end_tracking:
// begin_tracking();
// let root_el = (self.root)(&mut ctx);
// let registry = end_tracking();
// config = config.components(registry);

// SIMPLIFY to:
let root_el = (self.root)(&mut ctx);
```

### Update CSS size logging

```rust
// Update the log line to reflect new CSS sources:
println!("CSS: {} bytes (utility:{} pseudo:{} base:{})",
    css_size, util_css_size, pseudo_css_size, base_css_size);
```

---

## Step 7: Update Documentation

### CLAUDE.md Updates

1. Remove the "Adding a New Element Type" section referencing CSS class patterns
2. Update the "Creating Components" section to show `.st()` + `.ps()` pattern
3. Remove references to `*_CSS` constants
4. Add section on St and Ps token usage for components
5. Update the Component CSS Budget section (no longer applicable)

### Memory Updates

Update `MEMORY.md` with:
- Component system now uses St + Ps tokens (no CSS constants)
- Ps enum for pseudo-class/pseudo-element tokens
- All styling flows through STYLE_INJECT with generated CSS rules
- ComponentRegistry removed (no CSS tree-shaking needed, token tree-shaking handles everything)

---

## Step 8: Final Verification

### Payload Size Measurement

Create a test or script to measure actual payload sizes:

```rust
#[test]
fn test_final_payload_sizes() {
    // Build a realistic app tree with multiple components
    let ctx = build_test_app();

    // Measure STYLE_INJECT CSS size
    let css = generate_all_css(&ctx.config);
    println!("Total CSS: {} bytes", css.len());
    assert!(css.len() < 5000, "CSS should be under 5KB after migration");

    // Measure JS capsule size
    let capsule = generate_styled_capsule(&ctx);
    println!("Capsule HTML: {} bytes", capsule.len());
    // Should be smaller without U lookup table

    // Measure wire bytes for a typical page
    let wire = ctx.finish();
    println!("Wire bytes: {} bytes", wire.len());
}
```

### Expected Final Numbers

| Metric | Before Migration | After Migration | Reduction |
|--------|-----------------|----------------|-----------|
| STYLE_INJECT CSS | ~15,300B | ~3,730B | 76% |
| JS capsule (U table) | ~1,500B | 0B | 100% |
| Symbol table (classes) | ~2,400B | ~200B | 92% |
| Per-element wire cost | ~3B (SET_CLASS) | ~3-6B (STYLE_MULTI) | +0-3B |
| Per-element with composite | ~3B | ~3B (STYLE_COMPOSITE) | 0% |
| **Total per connection** | **~19,200B** | **~3,930B** | **80%** |

### E2E Verification

```bash
# Run all tests
cargo test --workspace

# Lint check
cargo clippy --workspace

# Format
cargo fmt --all

# Run counter example
cargo run -p counter
# Navigate to http://127.0.0.1:9000
# Verify: counter renders, buttons click, state updates

# Run todo-combined example
cargo run -p todo-combined
# Navigate to http://127.0.0.1:9000
# Verify: todo items render, can add/toggle/delete
# Verify: hover states work on buttons
# Verify: focus states work on input
# Verify: loading states work

# Use Playwright MCP for automated verification:
# 1. browser_navigate to http://127.0.0.1:9000
# 2. browser_snapshot to verify DOM structure
# 3. browser_click on buttons to verify hover/click
# 4. browser_console_messages to check for JS errors
```

---

## Files to Delete

| File | Reason |
|------|--------|
| All `*_CSS` constants in component files | No component CSS needed |
| `generate_components_css()` in `mod.rs` | No CSS to aggregate |
| `registry.rs` (or simplify) | No CSS tree-shaking needed |
| CSS-related tests | Test token generation instead |

## Files to Modify

| File | Changes |
|------|---------|
| `components/mod.rs` | Remove CSS re-exports, remove `generate_components_css()` |
| `capsule_gen.rs` | Remove component CSS inclusion, update CSS generation path |
| `server.rs` | Remove component tracking, update CSS logging |
| `CLAUDE.md` | Update component documentation |

## Estimated Effort

| Task | Lines |
|------|-------|
| Remove CSS constants | ~200 (deletion) |
| Simplify mod.rs | ~50 |
| Remove/simplify registry | ~400 (deletion) |
| Update capsule_gen.rs | ~30 |
| Update server.rs | ~20 |
| Update documentation | ~100 |
| Final tests | ~50 |
| **Total** | **~850 lines changed (mostly deletions)** |
