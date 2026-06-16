# Toast — Client Action Migration

**File**: `libs/rwire-components/src/toast.rs`
**Primitive**: Target
**Tier**: 3 (Medium Impact)
**Complexity**: Low

## Current Behavior

Toast notifications are server-pushed. The `ToastContainer` renders a list of `Toast` items from server state. Dismissing a toast calls `on_dismiss`, a server handler that removes the toast from the state vector, triggering a re-render.

```rust
// Current — dismiss round-trips to server, server removes from vec, re-renders
ToastContainer::new()
    .toast(Toast::success("Saved").on_dismiss(dismiss_toast_0()))
    .toast(Toast::error("Failed").on_dismiss(dismiss_toast_1()))
    .build()
```

**Latency**: Dismiss animation blocked on server response.

## Target State

```rust
#[derive(Target)]
struct Toast0Visible;
#[derive(Target)]
struct Toast1Visible;

// Each toast gets a Target for instant client-side dismiss
Toast::success("Saved")
    .client_dismiss::<Toast0Visible>()
    .build()
```

## Implementation

### 1. Add client_dismiss method

```rust
impl Toast {
    pub fn client_dismiss<T: Target>(mut self) -> Self {
        self.target_type_id = Some(TypeId::of::<T>());
        self
    }
}
```

### 2. Toast visibility bound to Target

```rust
// In build():
let mut toast = el(El::Div).st(self.compute_tokens());

if let Some(target_type_id) = self.target_type_id {
    // Start visible (Target default is false)
    // When Target becomes true (dismissed), hide
    toast = toast.unless_by_id(target_type_id, St::DisplayNone);
}
```

Wait — this is inverted. Target defaults to `false`. We want the toast visible by default and hidden when dismissed. So:
- Default (Target=false): toast visible
- After toggle (Target=true): toast hidden

Use `.when::<T>(St::DisplayNone)` — add DisplayNone when target is true (dismissed).

### 3. Dismiss button uses toggle

```rust
if let Some(target_type_id) = self.target_type_id {
    dismiss_btn = dismiss_btn.toggle_by_id(target_type_id, Ev::Click);
} else if let Some(handler) = self.on_dismiss {
    dismiss_btn = dismiss_btn.on(Ev::Click, handler);
}
```

## Limitations

- **No auto-dismiss timer**: Client actions don't support timed toggles. Auto-dismiss after N seconds would need a `setTimeout` in the JS runtime — out of scope for now.
- **No server awareness**: Server still has the toast in its state after client-side dismiss. Next server re-render would show it again. For true removal, still need `on_dismiss` handler.
- **Best for**: Toasts that are fire-and-forget (server pushes, user dismisses, toast stays dismissed until page change).

## Testing

- `test_toast_client_dismiss` — target_type_id set
- `test_toast_visible_by_default` — no DisplayNone when target is false
- `test_toast_dismiss_button_has_toggle` — dismiss button gets toggle binding
- `test_toast_server_mode_unchanged` — `on_dismiss(handler)` still works
