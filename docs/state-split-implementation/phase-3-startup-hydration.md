# Phase 3: Startup Hydration

**Goal**: Load all persisted state from database into shared cache at server startup.

## Overview

```
Server Start
     │
     ▼
┌─────────────────────────────────────┐
│  1. Create SharedServerState        │
│  2. Connect to SQLite               │
│  3. Query all tables                │
│  4. Deserialize into shared_cache   │
│  5. Start accepting connections     │
└─────────────────────────────────────┘
```

## New File: `rwire/src/persist.rs`

```rust
//! Persistence traits and helpers for rwire.

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Trait for state types that can be persisted to database.
pub trait Persistable: Send + Sync + 'static {
    /// Table name for this state type.
    fn table_name() -> &'static str;

    /// SQL statements to create tables.
    fn schema() -> &'static [&'static str];

    /// Load state from database rows.
    fn load_from_db(
        conn: &rusqlite::Connection,
        key: &str,
    ) -> Option<Box<dyn Any + Send + Sync>>;

    /// Save state to database.
    fn save_to_db(
        conn: &rusqlite::Connection,
        key: &str,
        state: &dyn Any,
    ) -> Result<(), rusqlite::Error>;
}

/// Registry of persistable types (populated by macros).
pub struct PersistRegistry {
    types: HashMap<&'static str, PersistableType>,
}

pub struct PersistableType {
    pub table_name: &'static str,
    pub schema: &'static [&'static str],
    pub type_id: TypeId,
    pub load_fn: fn(&rusqlite::Connection, &str) -> Option<Box<dyn Any + Send + Sync>>,
    pub save_fn: fn(&rusqlite::Connection, &str, &dyn Any) -> Result<(), rusqlite::Error>,
}

impl PersistRegistry {
    pub fn new() -> Self {
        Self { types: HashMap::new() }
    }

    pub fn register(&mut self, persistable: PersistableType) {
        self.types.insert(persistable.table_name, persistable);
    }

    pub fn get(&self, table_name: &str) -> Option<&PersistableType> {
        self.types.get(table_name)
    }

    pub fn all(&self) -> impl Iterator<Item = &PersistableType> {
        self.types.values()
    }
}

// Global registry using inventory crate
inventory::collect!(PersistableType);

/// Get the global persist registry.
pub fn persist_registry() -> PersistRegistry {
    let mut registry = PersistRegistry::new();
    for p in inventory::iter::<PersistableType> {
        registry.register(p.clone());
    }
    registry
}
```

## Hydration Logic: `rwire/src/server.rs`

```rust
impl SharedServerState {
    /// Hydrate shared cache from database at startup.
    pub fn hydrate(&self) -> Result<(), Box<dyn std::error::Error>> {
        let store = match &self.store {
            Some(s) => s,
            None => return Ok(()), // No store configured
        };

        let conn = store.connection()?;
        let registry = persist_registry();

        // Ensure all schemas exist
        for persistable in registry.all() {
            for sql in persistable.schema {
                conn.execute(sql, [])?;
            }
        }

        // Load all persisted state
        for persistable in registry.all() {
            self.hydrate_table(&conn, persistable)?;
        }

        Ok(())
    }

    fn hydrate_table(
        &self,
        conn: &rusqlite::Connection,
        persistable: &PersistableType,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Query all keys from main table
        let key_field = "session_id"; // TODO: Get from persistable
        let sql = format!(
            "SELECT {} FROM {}",
            key_field,
            persistable.table_name
        );

        let mut stmt = conn.prepare(&sql)?;
        let keys: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        // Load each state into cache
        let mut cache = self.shared_cache.write().unwrap();
        for key in keys {
            let full_key = format!("{}:{}", persistable.table_name, key);
            if let Some(state) = (persistable.load_fn)(conn, &key) {
                cache.insert(full_key, state);
            }
        }

        Ok(())
    }
}
```

## Generated Load Function

The `#[derive(State)]` macro generates the load function:

```rust
// For TodoState, generates:
impl Persistable for TodoState {
    fn table_name() -> &'static str { "todos" }

    fn schema() -> &'static [&'static str] { &Self::SCHEMA }

    fn load_from_db(
        conn: &rusqlite::Connection,
        key: &str,
    ) -> Option<Box<dyn Any + Send + Sync>> {
        // Load main table row
        let mut stmt = conn.prepare(
            "SELECT filter FROM todos WHERE session_id = ?"
        ).ok()?;

        let filter: i32 = stmt.query_row([key], |row| row.get(0)).ok()?;

        // Load child table rows
        let mut items_stmt = conn.prepare(
            "SELECT text, done FROM todos__items WHERE _parent = ? ORDER BY _idx"
        ).ok()?;

        let items: Vec<TodoItem> = items_stmt
            .query_map([key], |row| {
                Ok(TodoItem {
                    text: row.get(0)?,
                    done: row.get::<_, i32>(1)? != 0,
                })
            })
            .ok()?
            .filter_map(|r| r.ok())
            .collect();

        Some(Box::new(TodoState {
            session_id: key.to_string(),
            filter: Filter::from_i32(filter),
            items,
        }))
    }

    fn save_to_db(
        conn: &rusqlite::Connection,
        key: &str,
        state: &dyn Any,
    ) -> Result<(), rusqlite::Error> {
        let state = state.downcast_ref::<TodoState>()
            .ok_or(rusqlite::Error::InvalidParameterName("wrong type".into()))?;

        // Upsert main table
        conn.execute(
            "INSERT INTO todos (session_id, filter)
             VALUES (?1, ?2)
             ON CONFLICT(session_id) DO UPDATE SET filter = excluded.filter",
            rusqlite::params![key, state.filter as i32],
        )?;

        // Replace child table
        conn.execute(
            "DELETE FROM todos__items WHERE _parent = ?",
            [key],
        )?;

        for (idx, item) in state.items.iter().enumerate() {
            conn.execute(
                "INSERT INTO todos__items (_parent, _idx, text, done)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![key, idx as i32, &item.text, item.done as i32],
            )?;
        }

        Ok(())
    }
}

// Register with inventory
inventory::submit! {
    PersistableType {
        table_name: "todos",
        schema: TodoState::SCHEMA,
        type_id: std::any::TypeId::of::<TodoState>(),
        load_fn: TodoState::load_from_db,
        save_fn: TodoState::save_to_db,
    }
}
```

## Server Startup

```rust
impl<F> ServerWithRoot<F>
where
    F: Fn() -> ElementBuilder + Send + Sync + Clone + 'static,
{
    pub async fn run(self) -> Result<(), std::io::Error> {
        // Create shared state
        let shared = SharedServerState::new(self.store, self.persist_interval);

        // Hydrate from database
        if let Err(e) = shared.hydrate() {
            eprintln!("WARNING: Failed to hydrate from database: {}", e);
            eprintln!("Starting with empty state.");
        }

        // Start background persist task
        let persist_shared = shared.clone();
        async_std::task::spawn(async move {
            persist_task(persist_shared).await;
        });

        // Start server
        run_server(self.addr, self.root, shared).await
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_hydrate_empty_db() {
        let conn = Connection::open_in_memory().unwrap();

        // Create schema
        conn.execute(
            "CREATE TABLE todos (session_id TEXT PRIMARY KEY, filter INTEGER)",
            [],
        ).unwrap();

        let shared = SharedServerState::new(Some(Arc::new(SqliteStore::from_conn(conn))), Duration::from_millis(100));
        shared.hydrate().unwrap();

        assert!(shared.shared_cache.read().unwrap().is_empty());
    }

    #[test]
    fn test_hydrate_with_data() {
        let conn = Connection::open_in_memory().unwrap();

        // Create schema and insert data
        conn.execute(
            "CREATE TABLE todos (session_id TEXT PRIMARY KEY, filter INTEGER)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO todos (session_id, filter) VALUES ('abc', 1)",
            [],
        ).unwrap();

        // Register persistable type
        // ... (done via inventory in real code)

        let shared = SharedServerState::new(/* ... */);
        shared.hydrate().unwrap();

        assert!(shared.shared_cache.read().unwrap().contains_key("todos:abc"));
    }
}
```

## Checklist

- [ ] Create `rwire/src/persist.rs` with `Persistable` trait
- [ ] Create `PersistRegistry` for type registration
- [ ] Add `inventory` crate for compile-time registration
- [ ] Implement `SharedServerState::hydrate()`
- [ ] Generate `load_from_db()` in `#[derive(State)]` macro
- [ ] Ensure schema is created before loading
- [ ] Handle missing tables gracefully
- [ ] Add unit tests
- [ ] Run `cargo clippy`
