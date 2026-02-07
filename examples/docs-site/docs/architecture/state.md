---
title: State Management
description: How rwire handles application state
order: 2
---
# State Management

All state in rwire lives on the server. The `#[derive(State)]` macro provides the necessary serialization and change-tracking infrastructure.

## Storage Types

### Memory Storage

Server-side state that's lost on restart. Good for session data and transient UI state.

```rust
#[derive(State, Default)]
#[storage(memory)]
struct AppState {
    count: i32,
}
```

### Persisted Storage

State that survives server restarts via SQLite. Good for user data and application configuration.

```rust
#[derive(State, Default)]
#[storage(persisted)]
struct UserData {
    name: String,
    preferences: Preferences,
}
```

### Local Storage

Client-side state that never leaves the browser. Used for UI-only state like menu open/close.

```rust
#[derive(State, Default)]
#[storage(local)]
struct UIState {
    sidebar_open: bool,
}
```

## Handlers and Renderers

**Handlers** mutate state in response to events:

```rust
#[handler]
fn increment(state: &mut Counter) {
    state.count += 1;
}
```

**Renderers** produce UI from state and automatically re-render when relevant state changes:

```rust
#[renderer]
fn render_count(state: &Counter) -> ElementBuilder {
    el(El::Span).text(&state.count.to_string())
}
```

## Change Detection

When a handler modifies state, rwire compares the new state with the previous version and only re-renders synced regions that depend on changed fields.
