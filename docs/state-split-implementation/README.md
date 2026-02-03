# State Split Implementation Roadmap

Full implementation of `#[storage(persisted)]` with background persistence and normalized database schema.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              SERVER PROCESS                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  SharedServerState (Arc)                                                     │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  shared_cache: RwLock<HashMap<String, Box<dyn Any>>>                  │  │
│  │    "todos:abc" → TodoState { items: [...] }     ◄── ALL STATE HERE    │  │
│  │                                                                        │  │
│  │  dirty_keys: RwLock<HashSet<String>>            ◄── Pending writes    │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│           │                                          ▲                       │
│           │ background                               │ instant mutation      │
│           ▼                                          │                       │
│  ┌─────────────────┐                    ┌────────────┴───────────┐          │
│  │  Persist Task   │                    │   Connection Handler   │          │
│  │  - debounced    │                    │   1. mutate memory     │          │
│  │  - non-blocking │                    │   2. mark dirty        │          │
│  │  - retry logic  │                    │   3. broadcast         │          │
│  └────────┬────────┘                    │   4. return (instant)  │          │
│           │                             └────────────────────────┘          │
│           ▼                                                                  │
│  ┌─────────────────┐                                                         │
│  │     SQLite      │  ◄── Normalized schema, read at startup only           │
│  │  (normalized)   │                                                         │
│  └─────────────────┘                                                         │
└──────────────────────────────────────────────────────────────────────────────┘
```

## Key Principles

1. **Memory-first**: Persisted state lives in memory, DB is just durable storage
2. **Non-blocking writes**: Handler returns immediately, DB write happens in background
3. **Normalized schema**: Efficient storage with compile-time SQL generation
4. **Eventually consistent**: Small window where DB lags behind memory

## Current State

| Feature | Status |
|---------|--------|
| Local state (client-side) | Complete |
| Memory state (server RAM) | Complete |
| Shared state cache | Not implemented |
| Background persistence | Not implemented |
| Normalized schema generation | Not implemented |
| Cross-connection broadcast | Not implemented |

## Implementation Phases

| # | Phase | Description |
|---|-------|-------------|
| 1 | [Normalized Schema](./phase-1-normalized-schema.md) | Compile-time SQL schema from structs |
| 2 | [Shared State Cache](./phase-2-shared-state-cache.md) | Server-level state for persisted types |
| 3 | [Startup Hydration](./phase-3-startup-hydration.md) | Load persisted state from DB at server start |
| 4 | [Dirty Tracking](./phase-4-dirty-tracking.md) | Mark modified state for persistence |
| 5 | [Background Persist Task](./phase-5-background-persist.md) | Async DB writes with debouncing |
| 6 | [Cross-Connection Broadcast](./phase-6-broadcast.md) | Notify other connections on state change |
| 7 | [Graceful Shutdown](./phase-7-graceful-shutdown.md) | Flush dirty state before exit |
| 8 | [Example Update](./phase-8-example-update.md) | Validate with todo-combined |

## Files to Modify

| File | Changes |
|------|---------|
| `rwire-macros/src/lib.rs` | Schema generation, type mapping |
| `rwire-macros/src/schema_gen.rs` | **NEW**: SQL schema generator |
| `rwire/src/server.rs` | SharedServerState, background task, hydration |
| `rwire/src/persist.rs` | **NEW**: Persistence traits and helpers |
| `rwire/src/sqlite_store.rs` | **NEW**: Normalized schema support |
| `rwire/src/session.rs` | Cookie parsing for session ID |
| `examples/todo-combined/src/main.rs` | Use actual persisted storage |

## New Dependencies

```toml
# rwire/Cargo.toml
async-channel = "2.3"
rusqlite = { version = "0.32", features = ["bundled"] }
```

## Data Flow

```
STARTUP
═══════
  SQLite ──hydrate──► shared_cache

HANDLER (instant, non-blocking)
═══════
  1. shared_cache.write()     ← memory mutation
  2. dirty_keys.insert(key)   ← mark for persist
  3. broadcast(StateChanged)  ← notify other connections
  4. return to client         ← done (~1ms)

BACKGROUND TASK (async)
═══════════════════════
  loop {
      sleep(100ms)
      for key in dirty_keys.drain() {
          state = shared_cache.get(key)
          db.save_normalized(key, state)
      }
  }

SHUTDOWN
════════
  flush all dirty_keys synchronously
```

## Configuration

```rust
Server::bind("127.0.0.1:9000")?
    .store(SqliteStore::new("./app.db")?)
    .persist_interval(Duration::from_millis(100))
    .root(build_app)
    .run()
    .await
```
