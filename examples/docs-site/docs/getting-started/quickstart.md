---
title: Quick Start
description: Build your first rwire app in 5 minutes
order: 2
---
# Quick Start

This guide walks you through building a simple counter application.

## Create the Project

```bash
cargo new my-app
cd my-app
```

## Define State

State is a Rust struct that holds your application data. The `#[derive(State)]` macro handles serialization and change tracking.

```rust
#[derive(State, Default)]
#[storage(memory)]
struct Counter {
    count: i32,
}
```

## Build the UI

Use the fluent `el()` API to construct DOM trees:

```rust
fn app() -> ElementBuilder {
    el(El::Div).append([
        el(El::Button).text("-").on(Ev::Click, decrement()),
        render_count(),
        el(El::Button).text("+").on(Ev::Click, increment()),
    ])
}
```

## Add Handlers

Handlers are functions that mutate state in response to events:

```rust
#[handler]
fn increment(state: &mut Counter) {
    state.count += 1;
}
```

## Add Renderers

Renderers are synced regions that re-render when state changes:

```rust
#[renderer]
fn render_count(state: &Counter) -> ElementBuilder {
    el(El::Span).text(&state.count.to_string())
}
```

## Run It

```bash
cargo run
# Open http://127.0.0.1:9000
```

That's it! The server handles all state management, and the browser receives minimal binary updates over WebSocket.
