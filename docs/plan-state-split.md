# State Split: Local, Memory, and Persisted State

## Problem Statement

Currently, rwire has a single `ClientState` type that lives in server memory. Every interaction requires a WebSocket round-trip:

```
User clicks → Event sent to server → Handler runs → State mutates → Update sent back
```

This is simple but has limitations:
1. **Latency**: Even trivial UI state (hover, accordion open) needs server
2. **Scalability**: Server holds all state in memory
3. **Persistence**: State lost when connection closes

## Proposed State Types

| Type | Location | Survives Disconnect | Use Case |
|------|----------|---------------------|----------|
| **Local** | Client (JS) | No | UI state, animations, form drafts |
| **Memory** | Server RAM | No | Session data, computed views |
| **Persisted** | Database | Yes | User data, preferences, documents |

## Design Goals

1. **API Simplicity**: Developers should use the same patterns for all state types
2. **Compile-Time Work**: Schema generation, validation, type checking at build
3. **Auto-Wiring**: No manual database setup, connection management, or serialization

---

## Solution A: Marker Traits with Derive Macros

### API

```rust
use rwire::{LocalState, MemoryState, PersistedState};

#[derive(LocalState, Default)]
struct UiState {
    sidebar_open: bool,
    selected_tab: usize,
}

#[derive(MemoryState, Default)]
struct SessionState {
    user_id: Option<u64>,
    permissions: Vec<String>,
}

#[derive(PersistedState, Default)]
#[table("user_preferences")]  // Optional: defaults to snake_case struct name
struct UserPrefs {
    #[key]  // Primary key for persistence
    user_id: u64,
    theme: String,
    notifications_enabled: bool,
}
```

### Handler Macros

```rust
// Runs entirely on client - no server round-trip
#[local_handler]
fn toggle_sidebar(state: &mut UiState) {
    state.sidebar_open = !state.sidebar_open;
}

// Current behavior - server memory
#[handler]
fn login(state: &mut SessionState) {
    state.user_id = Some(12345);
}

// Auto-persists after mutation
#[persisted_handler]
fn update_theme(state: &mut UserPrefs) {
    state.theme = "dark".to_string();
}
```

### Renderer Macros

```rust
// Re-renders on client without server
#[local_renderer]
fn render_sidebar(state: &UiState) -> ElementBuilder {
    el(El::Div)
        .class(if state.sidebar_open { "open" } else { "closed" })
}

// Current behavior
#[renderer]
fn render_user(state: &SessionState) -> ElementBuilder {
    // ...
}

// Subscribes to DB changes, re-renders on update
#[persisted_renderer]
fn render_prefs(state: &UserPrefs) -> ElementBuilder {
    // ...
}
```

### Pros
- Clear separation of concerns
- Explicit about where state lives
- Each macro can have specialized behavior

### Cons
- Three sets of macros to learn
- Mixing state types in one component requires care
- More code in macro crate

---

## Solution B: Unified Trait with Storage Attribute

### API

```rust
use rwire::State;

#[derive(State, Default)]
#[storage(local)]
struct UiState {
    sidebar_open: bool,
}

#[derive(State, Default)]
#[storage(memory)]  // Default if omitted
struct SessionState {
    user_id: Option<u64>,
}

#[derive(State)]
#[storage(persisted, table = "user_prefs", key = "user_id")]
struct UserPrefs {
    user_id: u64,
    theme: String,
}
```

### Handler/Renderer (Unchanged)

```rust
// Macro infers storage from state type
#[handler]
fn toggle_sidebar(state: &mut UiState) {  // Compiles to local handler
    state.sidebar_open = !state.sidebar_open;
}

#[handler]
fn login(state: &mut SessionState) {  // Server memory handler
    state.user_id = Some(12345);
}

#[handler]
fn update_theme(state: &mut UserPrefs) {  // Auto-persisted handler
    state.theme = "dark".to_string();
}
```

### Pros
- Single `#[handler]` macro works for all types
- Storage is a property of the type, not the handler
- Easier migration: just change the derive

### Cons
- Magic: handler behavior depends on state type
- Harder to see at a glance where code runs
- Requires type information flow through macros

---

## Solution C: Generic Wrapper Types

### API

```rust
use rwire::{Local, Memory, Persisted};

#[derive(Default)]
struct Counter {
    count: i32,
}

// Storage determined by wrapper
type LocalCounter = Local<Counter>;
type MemoryCounter = Memory<Counter>;
type PersistedCounter = Persisted<Counter, "counters", "id">;
```

### Usage

```rust
#[handler]
fn increment(state: &mut Local<Counter>) {
    state.count += 1;  // Deref coercion
}

#[handler]
fn increment_server(state: &mut Memory<Counter>) {
    state.count += 1;
}

#[handler]
fn increment_db(state: &mut Persisted<Counter, "counters", "id">) {
    state.count += 1;
}
```

### Pros
- Explicit in function signature where it runs
- Reuse same struct with different storage
- Type system enforces correct usage

### Cons
- Verbose signatures
- Const generics for table/key names are awkward
- Can't easily switch storage without changing all handlers

---

## Recommended: Solution B (Unified with Storage Attribute)

**Rationale:**
1. **Simplest mental model**: State type determines everything
2. **Minimal API surface**: One derive, one handler macro, one renderer macro
3. **Compile-time inference**: Macro can look up storage from type
4. **Easy migration path**: Change derive attribute, handlers auto-adapt

---

## Implementation Plan

### Phase 1: Local State (Client-Side)

**Goal:** UI state that updates without server round-trip.

#### 1.1 Protocol Changes

The protocol already has `BIND_LOCAL` opcode. Currently unused. We'll use it for local handlers.

```rust
// opcodes.rs - already exists
pub const BIND_LOCAL: u8 = 0x30;     // Local handler (client-only)
pub const BIND_REMOTE: u8 = 0x31;    // Remote handler (server round-trip)
```

New opcode for local state initialization:
```rust
pub const INIT_LOCAL_STATE: u8 = 0x40;  // [INIT_LOCAL_STATE, state_type_id, serialized_default]
```

#### 1.2 Macro Changes

**`#[derive(State)]` with storage attribute:**

```rust
// rwire-macros/src/lib.rs

#[proc_macro_derive(State, attributes(storage, table, key))]
pub fn derive_state(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Parse #[storage(...)] attribute
    let storage = parse_storage_attr(&input.attrs);

    match storage {
        Storage::Local => generate_local_state(&input),
        Storage::Memory => generate_memory_state(&input),
        Storage::Persisted { table, key } => generate_persisted_state(&input, table, key),
    }
}
```

**Local state generates:**
- `impl LocalState for T {}`
- Serialization code (for sending default to client)
- Type ID constant for client-side lookup

#### 1.3 Client Runtime Changes

The JS runtime needs to:
1. Maintain local state storage: `Map<TypeId, State>`
2. Execute local handlers without WebSocket
3. Re-render local synced elements

```javascript
// Capsule runtime addition
const localStates = new Map();
const localHandlers = new Map();  // handler_idx -> function

function handleLocalEvent(handlerIdx, targetRef, payload) {
    const handler = localHandlers.get(handlerIdx);
    if (handler) {
        const stateType = handler.stateType;
        let state = localStates.get(stateType);
        if (!state) {
            state = handler.createDefault();
            localStates.set(stateType, state);
        }
        handler.fn(state);
        rerenderLocalSynced(stateType);
    }
}
```

#### 1.4 Build-Time Handler Compilation

For local handlers, we need to compile Rust logic to client-side execution:

**Option A: Interpret at Runtime**
- Serialize handler as simple AST
- Client interprets mutations
- Limited to simple operations

**Option B: Generate JS from Handler**
- Macro generates equivalent JS
- Full expressiveness for supported patterns
- Limited to supported operations

**Recommendation:** Start with Option A (interpreted), upgrade to B as needed.

Simple interpreted format:
```rust
// For: fn increment(state: &mut Counter) { state.count += 1; }
// Generate:
HandlerSpec {
    state_type: "Counter",
    ops: vec![
        Op::FieldAdd { field: "count", value: 1 },
    ],
}
```

---

### Phase 2: Memory State (Current Behavior)

**Goal:** Preserve current `ClientState` behavior, rename to `MemoryState`.

#### 2.1 Rename and Deprecate

```rust
// state.rs

/// Memory-resident state (per-connection, server-side).
pub trait MemoryState: Default + Send + Sync + 'static {}

// Backwards compatibility
#[deprecated(note = "Use #[derive(State)] with #[storage(memory)] instead")]
pub trait ClientState: MemoryState {}
```

#### 2.2 Default Storage

When `#[storage(...)]` is omitted, default to memory:

```rust
#[derive(State, Default)]  // Implicitly #[storage(memory)]
struct Counter { count: i32 }
```

---

### Phase 3: Persisted State (Database-Backed)

**Goal:** State that auto-persists to database with same reactive API.

#### 3.1 Schema Generation

At compile time, generate SQL schema from struct:

```rust
#[derive(State)]
#[storage(persisted, table = "todos")]
struct Todo {
    #[key]
    id: u64,
    title: String,
    completed: bool,
    #[indexed]
    user_id: u64,
}
```

Generates:
```sql
CREATE TABLE IF NOT EXISTS todos (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    completed INTEGER NOT NULL DEFAULT 0,
    user_id INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_todos_user_id ON todos(user_id);
```

#### 3.2 Auto-Wiring

**Build-time (`build.rs`):**
1. Scan for `#[storage(persisted)]` types
2. Generate migration SQL
3. Embed in binary or write to file

**Runtime (server startup):**
1. Auto-connect to configured database
2. Run migrations
3. Provide connection pool to handlers

```rust
// Server configuration
Server::bind("127.0.0.1:8080")?
    .database("sqlite://app.db")  // Auto-wiring point
    .root(build_app)
    .run()
    .await
```

#### 3.3 Handler Integration

Persisted handlers get a database context:

```rust
#[handler]
fn add_todo(state: &mut Todo) {  // Macro detects Persisted type
    state.title = "New todo".to_string();
    // After handler returns, framework auto-saves
}

// Macro expands to something like:
fn add_todo() -> HandlerFn {
    HandlerFn::new_persisted::<Todo>(|state, db| {
        state.title = "New todo".to_string();
        // Auto-generated: db.save(state)?;
    })
}
```

#### 3.4 Reactive Subscriptions

When a persisted state changes (even from another connection), subscribed renderers update:

```rust
#[renderer]
fn render_todos(state: &Todos) -> ElementBuilder {
    // This re-renders when ANY connection modifies Todos
}
```

Implementation:
1. Each connection subscribes to types it renders
2. On mutation, notify all subscribers
3. Push updates via WebSocket

---

## File Changes Summary

| File | Changes |
|------|---------|
| `rwire-macros/src/lib.rs` | New `#[derive(State)]`, storage parsing, handler inference |
| `rwire/src/state.rs` | `LocalState`, `MemoryState`, `PersistedState` traits |
| `rwire/src/local.rs` | **New:** Local state serialization, handler specs |
| `rwire/src/persist.rs` | **New:** Database abstraction, schema generation |
| `rwire/src/builder.rs` | Storage-aware handler binding (BIND_LOCAL vs BIND_REMOTE) |
| `rwire/src/server.rs` | Database connection, subscription management |
| `rwire/src/capsule_gen.rs` | Local state runtime, local handler execution |
| `rwire/src/protocol/opcodes.rs` | `INIT_LOCAL_STATE` opcode |
| `rwire/build.rs` | **New:** Schema generation, migration embedding |

---

## API Summary (Final)

```rust
use rwire::{State, el, El};

// === Local State (Client-Side) ===
#[derive(State, Default)]
#[storage(local)]
struct UiState {
    sidebar_open: bool,
}

#[handler]
fn toggle_sidebar(state: &mut UiState) {
    state.sidebar_open = !state.sidebar_open;
}

// === Memory State (Server RAM) ===
#[derive(State, Default)]
#[storage(memory)]  // or just omit for default
struct Session {
    user_id: Option<u64>,
}

#[handler]
fn set_user(state: &mut Session) {
    state.user_id = Some(123);
}

// === Persisted State (Database) ===
#[derive(State)]
#[storage(persisted, table = "notes")]
struct Note {
    #[key]
    id: u64,
    content: String,
}

#[handler]
fn update_note(state: &mut Note) {
    state.content = "Updated!".to_string();
    // Auto-saved after handler returns
}

// === Renderers work the same for all ===
#[renderer]
fn render_ui(state: &UiState) -> ElementBuilder { /* ... */ }

#[renderer]
fn render_session(state: &Session) -> ElementBuilder { /* ... */ }

#[renderer]
fn render_note(state: &Note) -> ElementBuilder { /* ... */ }
```

---

## Open Questions

1. **Local handler complexity**: How much Rust can we compile to client-side execution?
   - Start simple (field mutations only)
   - Expand based on need

2. **Persisted state identity**: How to identify which record to load?
   - Option A: Key from URL/session
   - Option B: Explicit query in handler
   - Option C: Automatic from `#[key]` field

3. **Cross-connection reactivity**: Should persisted state updates push to ALL connections?
   - Could be opt-in: `#[storage(persisted, broadcast = true)]`

4. **Database choice**: SQLite only, or pluggable?
   - Start with SQLite (embedded, zero-config)
   - Abstract interface for future PostgreSQL/etc.

5. **Offline support**: Should local state persist across page reloads?
   - Could use localStorage/IndexedDB
   - `#[storage(local, persist = true)]`

---

## Implementation Order

1. **Phase 1.1-1.2**: Macro infrastructure (storage attribute, trait generation)
2. **Phase 2**: Rename `ClientState` to `MemoryState`, ensure backwards compat
3. **Phase 1.3-1.4**: Local state runtime and simple interpreted handlers
4. **Phase 3.1-3.2**: Persisted state schema generation and auto-wiring
5. **Phase 3.3-3.4**: Persisted handler integration and subscriptions

Estimated complexity: Phase 1 is the hardest (client-side execution), Phase 2 is trivial, Phase 3 is moderate (database abstraction is well-understood).
