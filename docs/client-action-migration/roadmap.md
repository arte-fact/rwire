# Client Action Migration Roadmap

## Overview

Migrate interactive components from server round-trip state to client-side actions (Targets and Selectors). This eliminates latency for purely visual interactions like show/hide, tab switching, and expand/collapse.

## What Changes

Currently, interactive components use one of two patterns:
1. **Server state** — `#[storage(memory)]` struct with `open: bool` + `#[handler]` that toggles it → full WebSocket round-trip
2. **Conditional rendering** — `if self.open { ... } else { DisplayNone }` in `build()` → server re-renders the entire subtree

After migration, components will:
- Emit both open and closed DOM at build time (always present in the tree)
- Use `when::<T>()` / `unless::<T>()` to bind CSS visibility to a Target
- Use `when_eq()` to bind CSS visibility to a Selector variant
- Toggle/select entirely in the browser via CSS class add/remove

## Migration Priority

### Tier 1 — High Impact (Pure show/hide toggles)

These components have a single `open: bool` that gates visibility. Migration is straightforward — emit both states, bind with Target.

| # | Component | File | Primitive | Current Pattern | Complexity |
|---|-----------|------|-----------|----------------|------------|
| 1 | [Modal](modal.md) | `modal.rs` | Target | `if !self.open { DisplayNone }` | Medium |
| 2 | [Drawer](drawer.md) | `drawer.rs` | Target | `if !self.open { DisplayNone }` | Medium |
| 3 | [Dropdown](dropdown.md) | `dropdown.rs` | Target | `if self.open { render menu }` | Medium |
| 4 | [Accordion](accordion.md) | `accordion.rs` | Target (per item) | `if item.open { open_tokens } else { closed_tokens }` | High |

### Tier 2 — High Impact (Exclusive selection)

These components pick one-of-N with different styling per selection. Migration uses Selector.

| # | Component | File | Primitive | Current Pattern | Complexity |
|---|-----------|------|-----------|----------------|------------|
| 5 | [Tabs](tabs.md) | `tabs.rs` | Selector | `idx == active_index` branches styling + content | High |
| 6 | [Stepper](stepper.md) | `stepper.rs` | Selector | `i < current`, `i == current`, `i > current` | Medium |

### Tier 3 — Medium Impact (CSS-class based toggles)

These components already do partial client-side work or have simpler toggle needs.

| # | Component | File | Primitive | Current Pattern | Complexity |
|---|-----------|------|-----------|----------------|------------|
| 7 | [Toast](toast.md) | `toast.rs` | Target | `on_dismiss` handler removes from server state | Low |
| 8 | [CopyButton](copy-button.md) | `copy_button.rs` | Target | Custom `.copied` CSS class via inline JS | Low |
| 9 | [AppShell](app-shell.md) | `app_shell.rs` | Target | No toggle yet — sidebar is static | Low |

### Tier 4 — Low Impact (Server integration needed)

These components have interactive visuals but their state is inherently tied to server-side concerns (routing, form validation). Client actions can provide instant visual feedback, but the server still drives truth.

| # | Component | File | Primitive | Notes |
|---|-----------|------|-----------|-------|
| 10 | [NavMenu](nav-menu.md) | `nav_menu.rs` | Selector | Active path is server-driven (router) |
| 11 | [Tooltip](tooltip.md) | `tooltip.rs` | — | Already CSS-only (`HoverShowChild`), no migration needed |

## Approach

Each component migration follows the same pattern:

1. **Define action type** — `#[derive(Target)]` or `#[derive(Selector)]` inside the component module
2. **Always emit both states** — build both open and closed DOM, not conditionally
3. **Bind visibility** — `.when::<T>(St::DisplayFlex)` on the visible state, `.st([St::DisplayNone])` as default
4. **Wire trigger** — `.toggle::<T>(Ev::Click)` on the trigger element
5. **Preserve API** — keep `.open()` / `.on_close()` methods for server-controlled use; add `.client_toggle()` as opt-in
6. **Update doc example** — show both server-controlled and client-action patterns
7. **Update tests** — verify both code paths

## Key Design Decisions

### Dual API: Server-controlled vs Client-action

Components should support both patterns. The existing `.open(bool)` + `.on_toggle(handler)` API remains for apps that need server awareness. A new `.client_toggle()` method opts into client actions instead.

```rust
// Server-controlled (existing)
Modal::new().open(state.modal_open).on_close(close_modal())

// Client-action (new)
Modal::new().client_toggle()
```

### Accordion: Multiple Targets

Each accordion item needs its own Target. Since Target types are user-defined, the component can't predefine them. Two options:
- **Index-based**: Component generates internal targets keyed by item index
- **User-supplied**: User passes a Target type per item

Recommendation: index-based, since accordion items are typically declared inline.

### Tabs: Dynamic Variant Count

Selector enums have compile-time-fixed variants, but `Tabs` accepts a dynamic number of tabs. Two options:
- **User-supplied enum**: User defines `#[derive(Selector)] enum MyTabs { A, B, C }` and passes it
- **Index-based**: Component uses an internal numeric selector

Recommendation: user-supplied enum for type safety in the common case, with fallback to index-based for dynamic tab count.

## Verification Checklist

For each component migration:
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` — zero warnings
- [ ] Server-controlled API still works (backward compatible)
- [ ] Client-action API works without server round-trip
- [ ] Tree-shaking: no extra JS when client actions aren't used
- [ ] Accessibility attributes preserved (aria-expanded, role, tabindex)
