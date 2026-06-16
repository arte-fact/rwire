# NavMenu — Client Action Migration

**File**: `libs/rwire-components/src/nav_menu.rs`
**Primitive**: Selector
**Tier**: 4 (Low Impact)
**Complexity**: Low

## Current Behavior

NavMenu renders navigation links with active-path highlighting. The `active_path` is compared against each item's `href` to decide styling — active items get `compute_active_link_tokens()` (BgEmphasis, TextDefault), inactive get `compute_link_tokens()` (TextMuted) with hover effects.

```rust
NavMenu::new()
    .item(NavItem::new("Home", "/"))
    .item(NavItem::new("Docs", "/docs"))
    .active_path("/docs")
    .build()
```

The active path comes from server state (set by the route handler). Links use `el(El::A).at_str(At::Href, &item.href)` — raw anchor tags, not `Link::to()`.

## Why Low Priority

NavMenu's active state is fundamentally server-driven:
1. Navigation happens via the router (WebSocket route message)
2. The server determines which path is active
3. The server re-renders NavMenu with the new `active_path`

Client actions could provide instant visual feedback (highlight the clicked link before the server responds), but:
- The server will re-render anyway on route change
- The links should use `Link::to()` for proper SPA navigation, which already triggers server re-render
- Double-updating (client action + server re-render) could cause visual flicker

## Potential Use Case

If NavMenu items don't navigate (e.g., filter controls styled as nav), a Selector would make sense:

```rust
#[derive(Selector)]
enum Filter {
    #[default]
    All,
    Active,
    Completed,
}

NavMenu::new()
    .item(NavItem::label("All").select(Filter::All))
    .item(NavItem::label("Active").select(Filter::Active))
    .item(NavItem::label("Completed").select(Filter::Completed))
    .client_select::<Filter>()
    .build()
```

But this stretches NavMenu beyond its purpose. A dedicated `FilterBar` or `SegmentedControl` component would be more appropriate.

## Recommendation

**Skip migration.** NavMenu's active state is inherently server-driven via routing. If instant visual feedback is needed, the router system itself should handle it (e.g., optimistic active-link highlighting on click, reverted if navigation fails).

## If Implemented

### Selector binding on links

```rust
impl NavMenu {
    pub fn client_select<S: Selector>(mut self) -> Self {
        self.selector_type_id = Some(TypeId::of::<S>());
        self
    }
}
```

Each link gets:
- Default: `compute_link_tokens()` (inactive)
- `.when_eq_by_id(selector, variant, St::TextDefault)` + `.when_eq_by_id(selector, variant, St::BgEmphasis)` (active)
- `.select_by_id(selector, variant, Ev::Click)` on click

## Testing

- `test_nav_menu_server_mode_unchanged` — active_path still works
