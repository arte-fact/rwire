# rwire State System

Technical reference for rwire's three-tier state management: Local, Memory, and Persisted.

## Overview

rwire provides three storage strategies for state, each optimized for different use cases:

| Storage | Location | Survives Refresh | Survives Restart | Use Case |
|---------|----------|------------------|------------------|----------|
| **Local** | Browser (JS) | No | No | UI state, animations, form drafts |
| **Memory** | Server RAM | No | No | Session data, computed views |
| **Persisted** | SQLite + RAM | Yes | Yes | User data, preferences, documents |

All three use the same `#[derive(State)]` macro with a `#[storage(...)]` attribute:

```rust
use rwire::State;

#[derive(State, Default)]
#[storage(local)]
struct UiState { sidebar_open: bool }

#[derive(State, Default)]
#[storage(memory)]
struct SessionState { user_id: Option<u64> }

#[derive(State, Default)]
#[storage(persisted, table = "todos")]
struct TodoState { items: Vec<TodoItem> }
```

## Local State

Local state runs entirely in the browser with zero server round-trips.

### How It Works

```
User Action → JS Handler → State Mutation → DOM Update
     (all client-side, ~1ms)
```

1. **Compilation**: The `#[handler]` macro analyzes mutations and generates a JSON spec
2. **Initialization**: Server sends default state JSON with initial DOM
3. **Execution**: Browser executes handler spec, mutates local state, re-renders

### Supported Mutations

Local handlers support simple field mutations only:

```rust
#[handler]
fn toggle_sidebar(state: &mut UiState) {
    state.sidebar_open = !state.sidebar_open;  // ✓ Supported
}

#[handler]
fn complex_logic(state: &mut UiState) {
    if some_condition() {  // ✗ Not supported - use memory state
        state.sidebar_open = true;
    }
}
```

### Best For

- Toggle states (modals, dropdowns, accordions)
- Form input handling before submission
- Animations and transitions
- Any UI that needs instant feedback

## Memory State

Memory state lives in server RAM, one copy per WebSocket connection.

### How It Works

```
User Action → WebSocket → Server Handler → State Mutation → WebSocket → DOM Update
     (~10-50ms round-trip)
```

1. **Connection**: Each WebSocket gets its own `ConnectionState` with state instances
2. **Events**: Browser sends event to server via binary protocol
3. **Handler**: Server executes Rust handler, mutates state
4. **Update**: Server sends DOM diff back to browser

### Best For

- Complex business logic
- Validation requiring server data
- State derived from server-side computations
- Session-scoped data (login state, permissions)

## Persisted State

Persisted state combines in-memory performance with SQLite durability.

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         SERVER PROCESS                          │
├─────────────────────────────────────────────────────────────────┤
│  SharedServerState (Arc)                                        │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  shared_cache: HashMap<"table:session_id", State>         │  │
│  │  dirty_keys: HashSet<String>  ← pending writes            │  │
│  └───────────────────────────────────────────────────────────┘  │
│           │                              ▲                       │
│           │ background task              │ handler mutation      │
│           ▼                              │                       │
│  ┌─────────────────┐          ┌──────────┴─────────┐            │
│  │  persist_task   │          │  WebSocket Handler │            │
│  │  (100ms loop)   │          │  1. mutate cache   │            │
│  └────────┬────────┘          │  2. mark dirty     │            │
│           │                   │  3. return (~1ms)  │            │
│           ▼                   └────────────────────┘            │
│  ┌─────────────────┐                                            │
│  │     SQLite      │                                            │
│  └─────────────────┘                                            │
└─────────────────────────────────────────────────────────────────┘
```

### Key Properties

1. **Memory-first**: Handlers mutate RAM, return immediately (~1ms)
2. **Background persistence**: Dirty keys written to SQLite every 100ms
3. **Session-scoped**: State keyed by `table:session_id`
4. **Hydration**: State loaded from SQLite at server startup

### Session Persistence

Sessions are maintained via HTTP cookies:

```
First Visit:
  Browser → GET / → Server generates session ID
  Server → Set-Cookie: rwire_sid=abc123 (HttpOnly, 1 year)
  Browser → WebSocket with Cookie header
  Server → Uses session_id for state keying

Page Refresh:
  Browser → GET / with Cookie → Server finds existing session
  Browser → WebSocket with Cookie → Same session_id
  Server → Loads state from shared_cache (keyed by session)
```

### Data Flow

```
STARTUP
  SQLite ──hydrate──► shared_cache

HANDLER (instant)
  1. shared_cache.write(key, state)   ← RAM mutation
  2. dirty_keys.insert(key)           ← mark for persist
  3. return to client                 ← done (~1ms)

BACKGROUND TASK (async, every 100ms)
  for key in dirty_keys.drain():
      db.save(key, shared_cache.get(key))

SHUTDOWN
  flush all dirty_keys synchronously
```

### Best For

- User preferences and settings
- Document/content storage
- Shopping carts
- Any data that should survive restarts

## Fine-Grained Reactivity

rwire uses compile-time bitmasks for efficient change tracking.

### How It Works

Each state struct gets field IDs (0-63):

```rust
#[derive(State, Default)]
#[storage(memory)]
struct Counter {
    count: i32,      // FIELD_COUNT = 0
    name: String,    // FIELD_NAME = 1
}
```

Handlers declare which fields they change:

```rust
#[handler]
fn increment(state: &mut Counter) {
    state.count += 1;
    // Macro generates: ChangeSet::from_fields(&[Counter::FIELD_COUNT])
}
```

Renderers declare which fields they depend on:

```rust
#[renderer]
fn render_count(state: &Counter) -> ElementBuilder {
    el(El::Span).text(&state.count.to_string())
    // Macro generates: RendererDeps::from_fields(&[Counter::FIELD_COUNT])
}

#[renderer]
fn render_name(state: &Counter) -> ElementBuilder {
    el(El::Span).text(&state.name)
    // Macro generates: RendererDeps::from_fields(&[Counter::FIELD_NAME])
}
```

### Update Logic

After a handler runs, only matching renderers update:

```rust
// ChangeSet { mask: 0b01 } from increment()
// RendererDeps { mask: 0b01 } for render_count → UPDATE
// RendererDeps { mask: 0b10 } for render_name  → SKIP
```

Check is a single bitwise AND: `(deps.mask & changes.mask) != 0`

## API Reference

### Derive Macro

```rust
#[derive(State)]
#[storage(local | memory | persisted, table = "name")]
struct MyState { ... }
```

Options:
- `local` - Client-side only
- `memory` - Server RAM (default if omitted)
- `persisted` - SQLite-backed, requires `table = "name"`

### Handler Macro

```rust
#[handler]
fn my_handler(state: &mut MyState) {
    // Mutations here
}

#[handler]
fn with_context(state: &mut MyState, ctx: &EventContext) {
    // Access event data via ctx
}
```

### Renderer Macro

```rust
#[renderer]
fn my_renderer(state: &MyState) -> ElementBuilder {
    el(El::Div).text("...")
}
```

### EventContext

Available in handlers for event metadata:

```rust
ctx.value()              // Input value (String)
ctx.checked()            // Checkbox state (bool)
ctx.item_ref::<T>()      // ItemRef for list items
```

## Example: Multi-Storage Todo

```rust
use rwire::{State, el, El, Ev, handler, renderer};

// UI state - instant toggles
#[derive(State, Default)]
#[storage(local)]
struct UiState {
    show_completed: bool,
}

// Session state - server validation
#[derive(State, Default)]
#[storage(memory)]
struct DraftState {
    input_text: String,
}

// Persistent data - survives restarts
#[derive(State, Default)]
#[storage(persisted, table = "todos")]
struct TodoState {
    items: Vec<TodoItem>,
}

#[handler]
fn toggle_show_completed(state: &mut UiState) {
    state.show_completed = !state.show_completed;
}

#[handler]
fn update_draft(state: &mut DraftState, ctx: &EventContext) {
    state.input_text = ctx.value();
}

#[handler]
fn add_todo(state: &mut TodoState, ctx: &EventContext) {
    let text = ctx.value();
    if !text.is_empty() {
        state.items.push(TodoItem { text, done: false });
    }
}
```

## Configuration

```rust
use rwire::{Server, SharedServerState};
use rwire::persist::SqliteStore;
use std::time::Duration;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize SQLite store
    let store = SqliteStore::file("./app.db")?;

    // Create shared state and hydrate from DB
    let shared = SharedServerState::new(Duration::from_millis(100));
    shared.hydrate(&store)?;

    // Start background persistence task
    let store_clone = store.clone();
    let shared_clone = shared.clone();
    async_std::task::spawn(async move {
        rwire::server::persist_task(shared_clone, store_clone).await;
    });

    // Run server
    Server::bind("127.0.0.1:9000")?
        .with_shared_state(shared)
        .root(build_app)
        .run()
        .await
}
```
