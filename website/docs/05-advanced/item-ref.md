---
title: ItemRef
description: Type-safe dynamic list binding
order: 2
---
# ItemRef

`ItemRef<T>` provides type-safe binding between list items in the DOM and their backing data in server state. When a user clicks an item in a rendered list, the handler knows exactly which item was targeted -- without data attributes, string parsing, or index juggling.

```rust
use rwire::{el, El, Ev, handler, renderer, State, ItemRef, IterWithRef, EventContext};

#[derive(State, Default)]
#[storage(memory)]
struct TodoState {
    items: Vec<TodoItem>,
}

#[derive(Default, Clone)]
struct TodoItem {
    text: String,
    done: bool,
}
```

## Rendering Lists with iter_with_ref

The `iter_with_ref()` method yields `(ItemRef<T>, &T)` tuples. Bind the ref to event handlers with `.on_ref()`:

```rust
#[renderer]
fn render_items(state: &TodoState) -> ElementBuilder {
    el(El::Ul).append(
        state.items.iter_with_ref().map(|(item_ref, item)| {
            el(El::Li)
                .text(&item.text)
                .on_ref(Ev::Click, toggle_item(), item_ref)
        })
    )
}
```

## Accessing Items in Handlers

In the handler, extract the `ItemRef` from the event context and use it to look up the item:

```rust
#[handler]
fn toggle_item(state: &mut TodoState, ctx: &EventContext) {
    if let Some(item_ref) = ctx.item_ref::<TodoItem>() {
        if let Some(item) = item_ref.get_mut(&mut state.items) {
            item.done = !item.done;
        }
    }
}
```

The `item_ref.get_mut(&mut state.items)` call returns `Option<&mut TodoItem>`, safely handling the case where the item may have been removed between render and click.

## Multiple Handlers Per Item

`ItemRef` implements `Copy`, so you can bind it to multiple events on the same element or pass it to child elements:

```rust
state.items.iter_with_ref().map(|(item_ref, item)| {
    el(El::Li).append([
        el(El::Span).text(&item.text),
        el(El::Button)
            .text("Toggle")
            .on_ref(Ev::Click, toggle_item(), item_ref),
        el(El::Button)
            .text("Delete")
            .on_ref(Ev::Click, delete_item(), item_ref),
    ])
})
```

## Wire Efficiency

On the wire, `ItemRef` encodes as a varint:

| List Size | Bytes per Ref |
|-----------|---------------|
| 0-127 items | 1 byte |
| 128-16383 items | 2 bytes |
| 16384+ items | 3 bytes |

Compare this to the JSON-in-data-attribute approach (`data-id="some-uuid"`), which costs 40+ bytes per element. For a 100-item list, `ItemRef` saves roughly 4KB of wire traffic.

## Type Safety

`ItemRef<T>` is generic over the collection item type. Attempting to use an `ItemRef<TodoItem>` with a `Vec<User>` is a compile-time error. This prevents a class of bugs where the wrong collection is indexed at runtime.
