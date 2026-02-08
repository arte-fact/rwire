---
title: Events
description: Handling user interactions from the browser
order: 5
---

# Events

```rust
use rwire::{el, El, Ev, handler, State};

el(El::Button)
    .text("Save")
    .on(Ev::Click, save_item())
```

Events connect user interactions in the browser to handler functions on the server. When the user clicks, types, or submits a form, the browser sends a compact binary message over the WebSocket, and the server executes the bound handler.

## Event Types

The `Ev` enum covers common DOM events:

| Event | Trigger |
|-------|---------|
| `Click` | Mouse click or tap |
| `DblClick` | Double click |
| `Input` | Text typed into input/textarea |
| `Change` | Value changed (select, checkbox) |
| `Submit` | Form submission |
| `KeyDown` | Key pressed |
| `KeyUp` | Key released |
| `Focus` | Element gains focus |
| `Blur` | Element loses focus |
| `MouseDown` | Mouse button pressed |
| `MouseUp` | Mouse button released |
| `MouseMove` | Mouse pointer moves |
| `Scroll` | Element scrolled |

Like element types, event types are single bytes on the wire and tree-shaken from the runtime.

## Server Round-Trip Events

The standard `.on()` binding sends the event to the server:

```rust
el(El::Input)
    .attr("type", "text")
    .on(Ev::Input, update_search())
```

The browser serializes the event payload (e.g., the input's current value) and sends it as a binary WebSocket message. The server executes the handler and pushes DOM updates back.

## Debounced Events

For high-frequency events like typing, rwire supports debounced bindings at the protocol level. The `BIND_DEBOUNCED` opcode includes a delay in milliseconds, so the browser waits for a pause in activity before sending:

```
[BIND_DEBOUNCED, ref, event_type, handler_idx, ms_hi, ms_lo]
```

This prevents flooding the server with keystrokes while keeping the debounce logic in the thin client runtime.

## Item References with on_ref

For list items that need individual handlers, use `.on_ref()` with an `ItemRef<T>`:

```rust
use rwire::{ItemRef, IterWithRef};

state.items.iter_with_ref().map(|(item_ref, item)| {
    el(El::Li)
        .text(&item.text)
        .on_ref(Ev::Click, toggle_item(), item_ref)
})
```

The `ItemRef` is encoded as a varint (1-3 bytes) and sent back with the event. In the handler, extract it with `ctx.item_ref::<T>()` for type-safe access to the specific item in the collection. No data attributes, no string parsing.

## Event Payload

Different event types carry different payloads automatically:

- **Input/Change** on text fields: the current `.value` string
- **Submit** on forms: a map of all field name-value pairs
- **Click** with `data-*` attributes: a map of data attributes

Access these through `EventContext` in the handler:

```rust
#[handler]
fn on_submit(state: &mut AppState, ctx: &EventContext) {
    if let Some(form) = ctx.form() {
        state.name = form.get("name").cloned().unwrap_or_default();
    }
}
```
