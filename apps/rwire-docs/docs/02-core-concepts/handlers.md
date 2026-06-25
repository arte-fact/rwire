---
title: Handlers
description: Functions that mutate state in response to events
order: 2
---

# Handlers

```rust
use rwire::{handler, State};

#[derive(State, Default)]
#[storage(memory)]
struct Counter { count: i32 }

#[handler]
fn increment(state: &mut Counter) {
    state.count += 1;
}
```

A handler is a plain Rust function annotated with `#[handler]`. It receives a mutable reference to your state and runs on the server when the bound event fires. After the handler completes, rwire automatically re-renders any affected synced regions.

## Event Binding

Handlers are bound to DOM elements using `.on()`:

```rust
el(El::Button)
    .text("Click me")
    .on(Ev::Click, increment())
```

The `increment()` call does not execute the handler -- it returns a `HandlerSpec` that tells rwire which function to call when the event occurs.

## EventContext

For events that carry data (text input, form submission, item references), add an `&EventContext` parameter:

```rust
use rwire::{handler, EventContext, State};

#[handler]
fn add_todo(state: &mut TodoState, ctx: &EventContext) {
    if let Some(text) = ctx.text() {
        state.items.push(Todo::new(text.to_string()));
    }
}
```

`EventContext` provides typed accessors for different payloads:

| Method | Returns | Source |
|--------|---------|--------|
| `ctx.text()` | `Option<&str>` | Input/textarea value |
| `ctx.data("key")` | `Option<&str>` | Element `data-*` attributes |
| `ctx.field("name")` | `Option<&str>` | A single form field value on submit |
| `ctx.item_ref::<T>()` | `Option<ItemRef<T>>` | Typed item reference from `.on_ref()` |

## ItemRef Handlers

For list items, use `.on_ref()` to bind a handler with a type-safe item reference:

```rust
use rwire::{handler, EventContext, ItemRef, IterWithRef};

#[handler]
fn toggle_item(state: &mut TodoState, ctx: &EventContext) {
    if let Some(item_ref) = ctx.item_ref::<TodoItem>() {
        if let Some(item) = item_ref.get_mut(&mut state.items) {
            item.done = !item.done;
        }
    }
}
```

The `ItemRef` encodes as a compact varint (1-3 bytes) rather than a string, and the type parameter ensures you access the correct collection at compile time.

## Debounced Handlers

Use `.debounce(ms)` to delay handler execution until events stop arriving. This is useful for search-as-you-type or resize handlers:

```rust
el(El::Input)
    .on(Ev::Input, search().debounce(300))
```

The handler fires 300ms after the last input event. If the user types more before the delay expires, the timer resets.
