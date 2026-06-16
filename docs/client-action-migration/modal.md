# Modal — Client Action Migration

**File**: `libs/rwire-components/src/modal.rs`
**Primitive**: Target
**Tier**: 1 (High Impact)
**Complexity**: Medium

## Current Behavior

Modal uses a server-owned `open: bool` field. When `open` is false, `build()` returns `el(El::Div).st([St::DisplayNone])` — an empty hidden div. When open, it renders the full modal tree (backdrop + container + panel + header + content + footer).

```rust
// Current usage — requires server round-trip
#[derive(State, Default)]
#[storage(memory)]
struct AppState { modal_open: bool }

#[renderer]
fn render_modal(state: &AppState) -> ElementBuilder {
    Modal::new()
        .open(state.modal_open)
        .on_close(close_modal())
        .content(el(El::P).text("Are you sure?"))
        .build()
}

#[handler]
fn open_modal(state: &mut AppState) { state.modal_open = true; }
#[handler]
fn close_modal(state: &mut AppState) { state.modal_open = false; }
```

**Latency**: Open/close requires WebSocket round-trip (~10-50ms LAN, 100-300ms WAN).

## Target State

```rust
// After migration — zero-latency
#[derive(Target)]
struct ModalOpen;

fn my_page() -> ElementBuilder {
    el(El::Div).append([
        el(El::Button).text("Open").toggle::<ModalOpen>(Ev::Click),
        Modal::new()
            .client_toggle::<ModalOpen>()
            .content(el(El::P).text("Are you sure?"))
            .build(),
    ])
}
```

## Implementation

### 1. Always emit both states

Currently `build()` returns early when closed:
```rust
if !self.open {
    return el(El::Div).st([St::DisplayNone]);
}
```

Change to always build the full tree, with visibility controlled by Target:

```rust
pub fn build(self) -> ElementBuilder {
    let mut tokens = self.compute_tokens();
    tokens.extend(self.size_tokens());
    tokens.push(St::MaxH90vh);

    // ... build modal_inner, backdrop, header, content, footer ...

    let mut wrapper = el(El::Div)
        .st([St::PositionFixed, St::Inset0, St::PointerEventsAuto, St::Z1400]);

    if let Some(target_type_id) = self.target_type_id {
        // Client-action mode: hidden by default, shown when Target is true
        wrapper = wrapper
            .st([St::DisplayNone])
            .when_by_id(target_type_id, St::DisplayBlock);
    } else if !self.open {
        return el(El::Div).st([St::DisplayNone]);
    }

    wrapper.append([backdrop_el, center_wrapper.append([modal_inner])])
}
```

### 2. Add client_toggle method

```rust
impl Modal {
    pub fn client_toggle<T: Target>(mut self) -> Self {
        self.target_type_id = Some(TypeId::of::<T>());
        self
    }
}
```

### 3. Close button uses toggle

When in client-action mode, the close button and backdrop click use `.toggle::<T>()` instead of `on(Ev::Click, handler)`.

### 4. Preserve server-controlled API

The existing `.open(bool)` + `.on_close(handler)` path remains unchanged. `client_toggle()` is opt-in.

## Accessibility Considerations

- `aria-modal="true"` must remain on the dialog element
- `role="dialog"` must remain
- Focus trapping is currently not implemented (future work regardless of this migration)
- `aria-hidden` on backdrop should be added

## Tokens Used

Currently conditional:
- `St::DisplayNone` (closed state)
- `St::PositionFixed`, `St::Inset0`, `St::Z1400`, `St::PointerEventsAuto` (open wrapper)
- `St::BgOverlay50`, `St::Z1300` (backdrop)

After migration, all tokens are always emitted, visibility toggled by Target.

## Testing

- `test_modal_client_toggle` — verify `.client_toggle::<T>()` sets target_type_id
- `test_modal_server_controlled_unchanged` — verify `.open(false)` still returns DisplayNone
- `test_modal_always_emits_dom_with_target` — verify full tree built when client_toggle is set
