# Phase 8: Example Update

**Goal**: Update todo-combined example to use actual persisted storage with SqliteStore.

## Changes to `examples/todo-combined/src/main.rs`

### Before

```rust
/// Persisted state for durable todo items.
///
/// This state is saved to a JSON file and survives server restarts.
/// For simplicity, we use a single global file - in production you'd
/// use session IDs for per-user storage.
#[derive(State, Default, Clone, Serialize, Deserialize)]
#[storage(memory)] // Using memory storage with manual persistence hooks
struct PersistedTodoState {
    items: Vec<PersistedTodoItem>,
}

// In main():
Server::bind("127.0.0.1:9000")?.root(build_app).run().await
```

### After

```rust
use rwire::{
    el, handler, renderer, El, ElementBuilder, Ev, EventContext, IterWithRef, Server, State,
    SqliteStore,  // NEW: Import SqliteStore
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ============================================================================
// Persisted State - SQLite-backed, survives restart
// ============================================================================

/// Persisted state for durable todo items.
///
/// This state lives in memory for instant access, with background persistence
/// to SQLite. The database is only read at server startup (hydration) and
/// written asynchronously after state changes (non-blocking).
///
/// Features:
/// - Instant handler execution (memory-only mutation)
/// - Automatic persistence (background task)
/// - Cross-tab synchronization (broadcast to other connections)
/// - Survives server restarts (SQLite storage)
#[derive(State, Default, Clone, Serialize, Deserialize)]
#[storage(persisted, table = "todos")]
struct PersistedTodoState {
    #[key]
    session_id: String,
    items: Vec<PersistedTodoItem>,
}

/// A persisted todo item.
#[derive(Clone, Default, Serialize, Deserialize)]
struct PersistedTodoItem {
    text: String,
    done: bool,
}

// Handlers are unchanged - they work exactly the same!
// The only difference is invisible: background persistence happens automatically.

#[handler]
fn add_persisted_item(state: &mut PersistedTodoState, ctx: &EventContext) {
    let text = ctx
        .field("todo")
        .or_else(|| ctx.text())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    if let Some(text) = text {
        state.items.push(PersistedTodoItem {
            text: text.to_string(),
            done: false,
        });
    }
    // State mutation complete - returns immediately
    // Background task will persist to SQLite later
}

#[handler]
fn toggle_persisted_item(state: &mut PersistedTodoState, ctx: &EventContext) {
    if let Some(item_ref) = ctx.item_ref::<PersistedTodoItem>() {
        if let Some(item) = item_ref.get_mut(&mut state.items) {
            item.done = !item.done;
        }
    }
    // Instant response, background persistence
}

#[handler]
fn delete_persisted_item(state: &mut PersistedTodoState, ctx: &EventContext) {
    if let Some(idx) = ctx.item_index() {
        if idx < state.items.len() {
            state.items.remove(idx);
        }
    }
}

#[handler]
fn clear_persisted_completed(state: &mut PersistedTodoState) {
    state.items.retain(|item| !item.done);
}

// ============================================================================
// UI Components - Updated description
// ============================================================================

fn build_persisted_column() -> ElementBuilder {
    el(El::Div).class("column persisted").append([
        el(El::H2).text("Persisted State"),
        el(El::P)
            .class("description")
            .text("SQLite-backed, survives restart, syncs across tabs"),  // UPDATED
        // ... rest unchanged ...
    ])
}

// ============================================================================
// Main - Now with SqliteStore
// ============================================================================

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("rwire Server - Todo Combined Demo");
    println!("==================================");
    println!();
    println!("Demonstrating three storage types:");
    println!("  - Local:     Client-side, instant UI response");
    println!("  - Memory:    Server-side, lost on disconnect");
    println!("  - Persisted: Memory + SQLite background sync");
    println!();
    println!("Persisted state features:");
    println!("  - Instant handler execution (memory-only)");
    println!("  - Background persistence (non-blocking)");
    println!("  - Cross-tab synchronization");
    println!("  - Survives server restart");
    println!();
    println!("Open http://127.0.0.1:9000 in your browser");
    println!("Try opening multiple tabs to see cross-tab sync!");
    println!();

    // Create SQLite store
    let store = SqliteStore::new("./todo-combined.db")?;

    Server::bind("127.0.0.1:9000")?
        .store(store)
        .persist_interval(Duration::from_millis(100))  // Flush every 100ms
        .root(build_app)
        .run()
        .await
}
```

## Testing Scenarios

### 1. Basic Persistence

```bash
# Start server
cargo run -p todo-combined

# In browser: Add items to "Persisted" column
# Stop server (Ctrl+C) - should see "Flushing state..."
# Restart server
cargo run -p todo-combined

# Items should still be there!
```

### 2. Cross-Tab Synchronization

```bash
# Start server
cargo run -p todo-combined

# Open Tab 1: http://127.0.0.1:9000
# Open Tab 2: http://127.0.0.1:9000

# In Tab 1: Add item to "Persisted" column
# Watch Tab 2: Item appears automatically!

# In Tab 2: Toggle the item
# Watch Tab 1: Toggle state updates!
```

### 3. Instant Response (No Latency)

```bash
# In browser console, measure click-to-update time:
# - Local state: ~0ms (client-side)
# - Memory state: ~5-20ms (WebSocket round-trip)
# - Persisted state: ~5-20ms (same as memory!)

# Persisted state has NO additional latency because
# mutation happens in memory, persistence is background
```

### 4. Graceful Shutdown

```bash
cargo run -p todo-combined

# Add several items rapidly
# Immediately press Ctrl+C

# Should see:
# Shutdown signal received, flushing state...
# Flushing 1 dirty keys (attempt 1)...
#   Flushed 1 keys successfully.
# Shutdown complete.

# Restart and verify all items persisted
```

### 5. Database Inspection

```bash
sqlite3 ./todo-combined.db

sqlite> .schema
CREATE TABLE todos (
    session_id TEXT PRIMARY KEY,
    filter INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE todos__items (
    _parent TEXT NOT NULL,
    _idx INTEGER NOT NULL,
    text TEXT NOT NULL,
    done INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (_parent, _idx)
);

sqlite> SELECT * FROM todos;
a1b2c3d4|0

sqlite> SELECT * FROM todos__items;
a1b2c3d4|0|Buy milk|0
a1b2c3d4|1|Call mom|1

sqlite> .quit
```

### 6. Multi-Session Isolation

```bash
# Open browser in normal mode (Tab 1)
# Open browser in incognito mode (Tab 2)

# Each gets a different session cookie
# Items in one session don't appear in the other
# Each session has independent persisted state
```

## Expected Console Output

```
rwire Server - Todo Combined Demo
==================================

Demonstrating three storage types:
  - Local:     Client-side, instant UI response
  - Memory:    Server-side, lost on disconnect
  - Persisted: Memory + SQLite background sync

Persisted state features:
  - Instant handler execution (memory-only)
  - Background persistence (non-blocking)
  - Cross-tab synchronization
  - Survives server restart

Open http://127.0.0.1:9000 in your browser
Try opening multiple tabs to see cross-tab sync!

Hydrating state from database...
  Loaded 2 keys from todos table.
Server listening on 127.0.0.1:9000
```

## Troubleshooting

### Items not persisting after restart

1. Check graceful shutdown message appeared
2. Check database file exists: `ls -la ./todo-combined.db`
3. Check database has data: `sqlite3 ./todo-combined.db "SELECT * FROM todos"`
4. Check for persist errors in console

### Cross-tab sync not working

1. Ensure both tabs have same session cookie
2. Check browser console for WebSocket errors
3. Verify both connections are subscribed (add debug logging)

### Slow response after adding persistence

This shouldn't happen - persisted state should have same latency as memory state. If it's slow:

1. Check that `.store()` is configured (background persist, not sync)
2. Check persist_interval isn't too short (causing lock contention)
3. Profile with `cargo flamegraph` if needed

## Checklist

- [ ] Update `PersistedTodoState` to `#[storage(persisted)]`
- [ ] Add `#[key]` attribute to session_id field
- [ ] Add `SqliteStore` import
- [ ] Configure `.store()` and `.persist_interval()` in main
- [ ] Update UI description text
- [ ] Test persistence across restarts
- [ ] Test cross-tab synchronization
- [ ] Test graceful shutdown
- [ ] Verify database schema
- [ ] Test multi-session isolation
- [ ] Run `cargo clippy --workspace`
- [ ] Run `cargo test --workspace`
