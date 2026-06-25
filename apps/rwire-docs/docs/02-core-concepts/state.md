---
title: State
description: Server-owned application state in Rust
order: 1
---

# State

```rust
use rwire::State;

#[derive(State, Default)]
#[storage(memory)]
struct Counter {
    count: i32,
    label: String,
}
```

Every rwire app starts with a state struct. Derive `State` and `Default`, pick a storage type, and you have reactive state that handlers can mutate and renderers can read.

## Storage Types

The `#[storage(...)]` attribute controls where state lives:

```rust
// Server memory -- lost on restart, good for session data
#[derive(State, Default)]
#[storage(memory)]
struct AppState { count: i32 }

// Persisted -- survives server restarts (JSON files on disk)
#[derive(State, Default)]
#[storage(persisted)]
struct UserData { name: String }
```

| Storage | Location | Lifetime | Use case |
|---------|----------|----------|----------|
| `memory` | Server RAM | Per connection | Session data, counters, form input |
| `persisted` | Server disk (JSON) | Across restarts | User profiles, saved documents |

For purely visual state (menu toggles, tab switching), use [Client Actions](/docs/advanced/client-actions) instead of state -- they run entirely in the browser with zero server round-trips.

## Accessing State

Handlers receive `&mut State` for writing. Renderers receive `&State` for reading. The framework manages all the plumbing -- no global singletons, no prop drilling, no context providers.

```rust
#[handler]
fn increment(state: &mut Counter) {
    state.count += 1;  // &mut -- can write
}

#[renderer]
fn render_count(state: &Counter) -> ElementBuilder {
    // & -- read-only
    el(El::Span).text(&state.count.to_string())
}
```

## Default Derive Requirement

`Default` is required because rwire creates initial state instances for new connections. Each connected client gets its own independent state -- there is no shared global state between connections.

## Multiple State Types

An app can use multiple state structs with different storage types. Each handler and renderer declares which state type it operates on through its function signature:

```rust
#[derive(State, Default)]
#[storage(memory)]
struct AppState { items: Vec<String> }

#[derive(State, Default, Clone, Serialize, Deserialize)]
#[storage(persisted, table = "prefs", key = "session_id")]
struct UserPrefs { session_id: String, theme: String }
```

The framework routes events to the correct handler based on the state type in its signature.
