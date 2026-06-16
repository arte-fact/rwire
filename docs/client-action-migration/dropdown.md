# Dropdown — Client Action Migration

**File**: `libs/rwire-components/src/dropdown.rs`
**Primitive**: Target
**Tier**: 1 (High Impact)
**Complexity**: Medium

## Current Behavior

DropdownMenu uses a server-owned `open: bool`. When closed, only the trigger button is rendered. When open, the menu panel and invisible backdrop are appended.

```rust
// Current — server round-trip for every open/close
DropdownMenu::new()
    .open(state.menu_open)
    .on_toggle(toggle_menu())
    .trigger(Button::secondary("Actions").build())
    .item(DropdownItem::new("Edit").on_click(edit_handler()))
    .build()
```

**Latency**: Menu appears after server round-trip. Feels sluggish for a dropdown.

## Target State

```rust
#[derive(Target)]
struct MenuOpen;

fn action_menu() -> ElementBuilder {
    DropdownMenu::new()
        .client_toggle::<MenuOpen>()
        .trigger(Button::secondary("Actions").build())
        .item(DropdownItem::new("Edit").on_click(edit_handler()))
        .build()
}
```

The trigger automatically gets `.toggle::<MenuOpen>(Ev::Click)`. The menu panel is always in the DOM, hidden until toggled.

## Implementation

### 1. Always emit menu panel

Currently the menu is only rendered inside `if self.open { ... }`:

```rust
// Before
if self.open {
    let mut menu = el(El::Div).st(Self::compute_menu_tokens())...;
    // items...
    container = container.append([backdrop, menu]);
}

// After
let mut menu = el(El::Div).st(Self::compute_menu_tokens())...;
// items always rendered...

if let Some(target_type_id) = self.target_type_id {
    menu = menu.st([St::DisplayNone]).when_by_id(target_type_id, St::DisplayBlock);
    backdrop = backdrop.st([St::DisplayNone]).when_by_id(target_type_id, St::DisplayBlock);
} else if !self.open {
    menu = menu.st([St::DisplayNone]);
    // backdrop not appended
}
container = container.append([backdrop, menu]);
```

### 2. Add client_toggle method

```rust
impl DropdownMenu {
    pub fn client_toggle<T: Target>(mut self) -> Self {
        self.target_type_id = Some(TypeId::of::<T>());
        self
    }
}
```

### 3. Trigger auto-wired

When `client_toggle` is set, the trigger element gets `.toggle::<T>(Ev::Click)` automatically in `build()`. No need for `on_toggle` handler.

### 4. Backdrop closes menu

The invisible backdrop (catches outside clicks) gets `.toggle::<T>(Ev::Click)` to close the menu.

### 5. Item clicks close menu

Each `DropdownItem` click should also close the menu. When in client-action mode, each item button gets `.toggle::<T>(Ev::Click)` in addition to its `on_click` handler.

## Accessibility Considerations

- `role="menu"` preserved on panel
- `role="menuitem"` preserved on items
- Future: arrow key navigation (separate concern)

## Tokens Used

Menu panel (currently conditional):
- `St::PositionAbsolute`, `St::Top0`, `St::Left0`, `St::BgApp`, `St::BorderSubtle`, `St::RoundedMd`, `St::ShadowLg`, `St::PySm`, `St::Z50`

Backdrop (currently conditional):
- `St::PositionFixed`, `St::Inset0`, `St::Z40`

After migration: always emitted, visibility bound to Target.

## Testing

- `test_dropdown_client_toggle` — verify target set
- `test_dropdown_items_always_rendered` — menu items in DOM even when closed
- `test_dropdown_server_mode_unchanged` — `.open(false)` still hides menu
