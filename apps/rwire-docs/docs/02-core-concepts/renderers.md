---
title: Renderers
description: Reactive UI regions that auto-update when state changes
order: 3
---

# Renderers

```rust
use rwire::{renderer, el, El, ElementBuilder, State};

#[derive(State, Default)]
#[storage(memory)]
struct Counter { count: i32 }

#[renderer]
fn render_count(state: &Counter) -> ElementBuilder {
    el(El::Span).text(&state.count.to_string())
}
```

A renderer is a function annotated with `#[renderer]` that reads state and returns an `ElementBuilder`. Whenever a handler mutates state, rwire automatically re-renders every renderer that depends on the changed fields.

## Synced Regions

Renderers create "synced regions" in the DOM -- subtrees that the server keeps in sync with the current state. Place a renderer call anywhere in your UI tree:

```rust
fn app() -> ElementBuilder {
    el(El::Div).append([
        el(El::H1).text("Counter"),
        render_count(),   // <-- synced region
        el(El::Button).text("+").on(Ev::Click, increment()),
    ])
}
```

When `increment()` modifies `Counter::count`, rwire re-calls `render_count()`, diffs the output against the previous version, and sends only the changed bytes over the WebSocket.

## Independent Update Units

Each renderer is an independent update boundary. If your app has three renderers, only the ones affected by a state change will re-render:

```rust
#[renderer]
fn render_count(state: &Counter) -> ElementBuilder {
    el(El::Span).text(&state.count.to_string())
}

#[renderer]
fn render_label(state: &Counter) -> ElementBuilder {
    el(El::Span).text(&state.label)
}
```

If a handler only changes `count`, only `render_count` re-runs. The `#[renderer]` macro performs compile-time dependency analysis to determine which fields each renderer reads, using bitmask tracking for zero-cost runtime checks.

## Composing Renderers

Renderers return `ElementBuilder`, so they compose naturally with the rest of the element tree. You can nest renderers, place them inside loops, or use them conditionally:

```rust
fn dashboard() -> ElementBuilder {
    el(El::Div).append([
        render_header(),
        el(El::Div).class("content").append([
            render_stats(),
            render_items(),
        ]),
    ])
}
```

## Key Differences from React

Unlike React components, rwire renderers do not receive props or manage local component state. The state struct is the single source of truth, and renderers are pure read-only projections of that state. There is no virtual DOM diffing on the client -- the server computes the diff and sends binary patch operations.
