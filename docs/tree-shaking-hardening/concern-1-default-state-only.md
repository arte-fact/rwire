# Concern 1: Renderers Only Scanned with Default State

## Problem

`collect_symbols()` and `collect_symbols_multi()` render synced elements with `Default::default()` state. Any style tokens that only appear in non-default code paths are silently omitted from the capsule CSS.

## Solution: Compile-Time Token Scanning

Two proc macros scan source code at compile time:

- **`#[renderer]`** scans renderer function bodies for all `St::Variant` references, `.hover([...])` pseudo-class calls, and `.sm([...])` / `.md([...])` breakpoint calls. Generates a static `TokenInventory` embedded in the synced renderer.

- **`#[component]`** scans entire component impl blocks (all methods including `compute_tokens()`, `apply_pseudo()`, `build()`). Generates a static `TokenInventory` attached to the `ElementBuilder` returned by `build()`.

During tree-shaking, `collect_from_inventory()` merges all compile-time tokens into the used token sets, covering every branch regardless of runtime state.

### Escape Hatch

For tokens generated inside called functions (invisible to the proc macros):

```rust
CapsuleConfig::new()
    .extra_styles(&[St::BgDestructive])
```

### Files Modified

- `libs/rwire-macros/src/token_scanner.rs` — Token scanner implementation (shared by both macros)
- `libs/rwire-macros/src/lib.rs` — `#[renderer]` generates `const TOKENS`, `#[component]` generates `const __COMPONENT_TOKENS`
- `libs/rwire/src/builder.rs` — `TokenInventory`, `with_token_inventory()`, `SyncedRenderer::token_inventory()`, `collect_from_inventory()`
- `libs/rwire-components/src/*.rs` — All 50 components annotated with `#[component]`
