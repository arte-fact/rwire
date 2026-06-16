# Tree-Shaking Hardening

rwire's tree-shaking discovers styles at startup by rendering every builder once with default state. Five concerns were identified where styles can be silently missing at runtime.

## Primary Solution: Compile-Time Token Scanning

Two proc macros scan source code at compile time for all `St::Variant`, `.hover([...])`, `.focus([...])`, `.sm([...])`, `.md([...])` etc. patterns:

- **`#[renderer]`** — scans renderer function bodies. Generates a static `TokenInventory` attached to the synced element.
- **`#[component]`** — scans entire component impl blocks (all methods). Wraps `build()` to attach a `TokenInventory` to the returned `ElementBuilder`.

This discovers tokens across **all branches**, not just the default-state path. Same philosophy as Tailwind CSS content scanning: analyze source code to find all possible tokens, regardless of runtime state.

## Concerns Addressed

| # | Concern | Fix | Status |
|---|---------|-----|--------|
| 1 | [Default state only](concern-1-default-state-only.md) | `#[renderer]` + `#[component]` proc macro token scanning | Done |
| 2 | [Router renderers ignored](concern-2-router-renderers.md) | `collect_tokens_from` handles synced elements + reads inventory | Done |
| 3 | [Composite CSS vars](concern-3-composite-vars.md) | Scan `composite_css` in `extract_used_variables` | Done |
| 4 | [Conditional pseudos](concern-4-conditional-pseudos.md) | Subcase of Concern 1; solved by proc macro scanning | Done |
| 5 | [Silent failures](concern-5-silent-failures.md) | Proc macro scanning prevents most cases | Done |

## Key Files

- `libs/rwire/src/builder.rs` — `TokenInventory`, `with_token_inventory()`, `SyncedRenderer::token_inventory()`, `collect_from_inventory()`
- `libs/rwire-macros/src/token_scanner.rs` — Compile-time token scanner (shared by both macros)
- `libs/rwire-macros/src/lib.rs` — `#[renderer]` generates `const TOKENS`, `#[component]` generates `const __COMPONENT_TOKENS`
- `libs/rwire/src/capsule_gen.rs` — Composite var scan
- `libs/rwire-components/src/*.rs` — All 50 components annotated with `#[component]`

## How It Works

### Renderers (`#[renderer]`)

```rust
#[renderer]
fn render_sidebar(state: &AppState) -> ElementBuilder {
    if state.expanded {
        el(El::Div).st([St::W64]).md([St::W80])
    } else {
        el(El::Div).st([St::W0]).md([St::W16])
    }
}
// → Generates TokenInventory with St::W64, St::W80, St::W0, St::W16
//   and breakpoint pairs (Md, W80), (Md, W16) — ALL branches covered
```

### Components (`#[component]`)

```rust
#[component]
impl Button {
    fn compute_tokens(&self) -> Vec<St> {
        match self.intent {
            ButtonIntent::Primary => vec![St::BgPrimary],
            ButtonIntent::Destructive => vec![St::BgDestructive],
        }
    }

    fn apply_pseudo(&self, builder: ElementBuilder) -> ElementBuilder {
        match self.intent {
            ButtonIntent::Primary => builder.hover([St::BgPrimaryHover]),
            ButtonIntent::Destructive => builder.hover([St::BgDestructiveHover]),
        }
    }

    pub fn build(self) -> ElementBuilder {
        // build() return is automatically wrapped with .with_token_inventory()
        self.apply_pseudo(el(El::Button).st(self.compute_tokens()))
    }
}
// → Scans ALL methods, discovers all St tokens + pseudo pairs from every branch
```

## Escape Hatch

For tokens generated inside called functions not visible to proc macro scanning:

```rust
CapsuleConfig::new()
    .extra_styles(&[St::BgDestructive])
```
