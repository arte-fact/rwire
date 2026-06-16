# Drawer — Client Action Migration

**File**: `libs/rwire-components/src/drawer.rs`
**Primitive**: Target
**Tier**: 1 (High Impact)
**Complexity**: Medium

## Current Behavior

Drawer uses a server-owned `open: bool`. When closed, `build()` returns `el(El::Div).st([St::DisplayNone])`. When open, it renders backdrop + slide-in panel with header, close button, and content.

```rust
// Current — server round-trip
Drawer::new()
    .title("Navigation")
    .open(state.drawer_open)
    .on_close(close_drawer())
    .content(sidebar_content())
    .build()
```

**Latency**: Open/close requires WebSocket round-trip. Slide-in panel appears after server responds.

## Target State

```rust
#[derive(Target)]
struct DrawerOpen;

fn my_page() -> ElementBuilder {
    el(El::Div).append([
        el(El::Button).text("Menu").toggle::<DrawerOpen>(Ev::Click),
        Drawer::new()
            .title("Navigation")
            .client_toggle::<DrawerOpen>()
            .content(sidebar_content())
            .build(),
    ])
}
```

## Implementation

### 1. Always emit both states

Change the early return:
```rust
// Before
if !self.open { return el(El::Div).st([St::DisplayNone]); }

// After
let mut wrapper = el(El::Div);
if let Some(target_type_id) = self.target_type_id {
    wrapper = wrapper
        .st([St::DisplayNone])
        .when_by_id(target_type_id, St::DisplayBlock);
} else if !self.open {
    return el(El::Div).st([St::DisplayNone]);
}
```

### 2. Add client_toggle method

```rust
impl Drawer {
    pub fn client_toggle<T: Target>(mut self) -> Self {
        self.target_type_id = Some(TypeId::of::<T>());
        self
    }
}
```

### 3. Close button + backdrop use toggle

When in client-action mode:
- Close button: `.toggle::<T>(Ev::Click)` instead of `.on(Ev::Click, handler)`
- Backdrop: `.toggle::<T>(Ev::Click)` instead of `.on(Ev::Click, handler)`

### 4. Position handling

The drawer currently applies `DrawerPosition::Left` or `Right` via inline style. This doesn't change — position is a build-time config, not runtime state.

## Accessibility Considerations

- `role="dialog"` and `aria-modal="true"` preserved on panel
- `aria-label="Close drawer"` preserved on close button

## Tokens Used

Currently conditional:
- `St::DisplayNone` (closed)
- Panel tokens: `St::PositionFixed`, `St::Top0`, `St::Bottom0`, `St::W320px`, `St::BgApp`, `St::ShadowXl`, `St::Z1400`
- Backdrop: `St::PositionFixed`, `St::Inset0`, `St::Z1300`, `St::BgOverlay50`

After migration: all always emitted, visibility toggled by Target on wrapper.

## Testing

- `test_drawer_client_toggle` — verify target_type_id is set
- `test_drawer_server_controlled_unchanged` — `.open(false)` still returns DisplayNone
- `test_drawer_always_emits_dom_with_target` — full tree built when client_toggle is set
