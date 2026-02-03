//! Persistence infrastructure for rwire.
//!
//! This module provides SQLite-based persistence for `#[storage(persisted)]` state types.
//! State is hydrated from the database at server startup and persisted asynchronously
//! in the background.

use rusqlite::{Connection, Error as SqliteError};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Error type for persistence operations.
#[derive(Debug)]
pub enum PersistError {
    /// SQLite error.
    Sqlite(SqliteError),
    /// Type not found in registry.
    TypeNotFound(String),
    /// Type mismatch during deserialization.
    TypeMismatch,
    /// Connection error.
    ConnectionError(String),
}

impl std::fmt::Display for PersistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistError::Sqlite(e) => write!(f, "SQLite error: {}", e),
            PersistError::TypeNotFound(name) => write!(f, "Type not found: {}", name),
            PersistError::TypeMismatch => write!(f, "Type mismatch"),
            PersistError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
        }
    }
}

impl std::error::Error for PersistError {}

impl From<SqliteError> for PersistError {
    fn from(e: SqliteError) -> Self {
        PersistError::Sqlite(e)
    }
}

/// Type alias for the load function signature.
pub type LoadFn = fn(&Connection, &str) -> Result<Option<Box<dyn Any + Send + Sync>>, PersistError>;

/// Type alias for the save function signature.
pub type SaveFn = fn(&Connection, &str, &dyn Any) -> Result<(), PersistError>;

/// Type alias for the default state function signature.
pub type DefaultFn = fn() -> Box<dyn Any + Send + Sync>;

/// Information about a persistable state type.
#[derive(Clone)]
pub struct PersistableType {
    /// Table name for this type.
    pub table_name: &'static str,
    /// SQL CREATE TABLE statements.
    pub schema: &'static [&'static str],
    /// TypeId of the state struct.
    pub type_id: TypeId,
    /// Name of the key field (e.g., "session_id").
    pub key_field: &'static str,
    /// Function to load state from database.
    pub load_fn: LoadFn,
    /// Function to save state to database.
    pub save_fn: SaveFn,
    /// Function to create default state.
    pub default_fn: DefaultFn,
}

/// Registry of persistable state types.
#[derive(Default)]
pub struct PersistRegistry {
    /// Types indexed by table name.
    by_table: HashMap<&'static str, PersistableType>,
    /// Types indexed by TypeId.
    by_type_id: HashMap<TypeId, PersistableType>,
}

impl PersistRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a persistable type.
    pub fn register(&mut self, persistable: PersistableType) {
        self.by_type_id.insert(persistable.type_id, persistable.clone());
        self.by_table.insert(persistable.table_name, persistable);
    }

    /// Get persistable type by table name.
    pub fn get_by_table(&self, table_name: &str) -> Option<&PersistableType> {
        self.by_table.get(table_name)
    }

    /// Get persistable type by TypeId.
    pub fn get_by_type_id(&self, type_id: TypeId) -> Option<&PersistableType> {
        self.by_type_id.get(&type_id)
    }

    /// Iterate over all registered types.
    pub fn all(&self) -> impl Iterator<Item = &PersistableType> {
        self.by_table.values()
    }

    /// Check if any types are registered.
    pub fn is_empty(&self) -> bool {
        self.by_table.is_empty()
    }
}

/// SQLite-based state store.
pub struct SqliteStore {
    /// Connection pool (single connection for now, wrapped in mutex for thread safety).
    conn: Arc<Mutex<Connection>>,
    /// Registry of persistable types.
    registry: Arc<Mutex<PersistRegistry>>,
}

impl SqliteStore {
    /// Create a new SQLite store at the given path.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, PersistError> {
        let conn = Connection::open(path)?;
        // Enable WAL mode for better concurrent performance
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            registry: Arc::new(Mutex::new(PersistRegistry::new())),
        })
    }

    /// Create an in-memory SQLite store (useful for testing).
    pub fn memory() -> Result<Self, PersistError> {
        let conn = Connection::open_in_memory()?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            registry: Arc::new(Mutex::new(PersistRegistry::new())),
        })
    }

    /// Register a persistable type.
    pub fn register(&self, persistable: PersistableType) {
        self.registry.lock().unwrap().register(persistable);
    }

    /// Ensure all schemas exist in the database.
    pub fn ensure_schema(&self) -> Result<(), PersistError> {
        let conn = self.conn.lock().unwrap();
        let registry = self.registry.lock().unwrap();

        for persistable in registry.all() {
            for sql in persistable.schema {
                conn.execute(sql, [])?;
            }
        }

        Ok(())
    }

    /// Hydrate all persisted state into the provided cache.
    ///
    /// Returns a HashMap of cache key -> state.
    pub fn hydrate_all(&self) -> Result<HashMap<String, Box<dyn Any + Send + Sync>>, PersistError> {
        let conn = self.conn.lock().unwrap();
        let registry = self.registry.lock().unwrap();
        let mut cache = HashMap::new();

        for persistable in registry.all() {
            // Query all keys from main table
            let sql = format!(
                "SELECT {} FROM {}",
                persistable.key_field, persistable.table_name
            );

            let mut stmt = match conn.prepare(&sql) {
                Ok(stmt) => stmt,
                Err(_) => continue, // Table doesn't exist yet
            };

            let keys: Vec<String> = stmt
                .query_map([], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();

            // Load each state
            for key in keys {
                let full_key = format!("{}:{}", persistable.table_name, key);
                if let Ok(Some(state)) = (persistable.load_fn)(&conn, &key) {
                    cache.insert(full_key, state);
                }
            }
        }

        Ok(cache)
    }

    /// Save a single state to the database.
    pub fn save(&self, key: &str, state: &dyn Any) -> Result<(), PersistError> {
        let conn = self.conn.lock().unwrap();
        let registry = self.registry.lock().unwrap();

        // Parse key format: "table:session_id"
        let parts: Vec<&str> = key.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(PersistError::ConnectionError(format!(
                "Invalid key format: {}",
                key
            )));
        }

        let table_name = parts[0];
        let session_id = parts[1];

        let persistable = registry
            .get_by_table(table_name)
            .ok_or_else(|| PersistError::TypeNotFound(table_name.to_string()))?;

        (persistable.save_fn)(&conn, session_id, state)
    }

    /// Get a reference to the connection (for advanced operations).
    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }

    /// Get a reference to the registry.
    pub fn registry(&self) -> Arc<Mutex<PersistRegistry>> {
        Arc::clone(&self.registry)
    }
}

impl Clone for SqliteStore {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
            registry: Arc::clone(&self.registry),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_store_memory() {
        let store = SqliteStore::memory().unwrap();
        assert!(store.registry.lock().unwrap().is_empty());
    }

    #[test]
    fn test_persist_registry() {
        let mut registry = PersistRegistry::new();
        assert!(registry.is_empty());

        // Create a dummy persistable type
        let persistable = PersistableType {
            table_name: "test_table",
            schema: &["CREATE TABLE IF NOT EXISTS test_table (id TEXT PRIMARY KEY)"],
            type_id: TypeId::of::<i32>(),
            key_field: "id",
            load_fn: |_, _| Ok(None),
            save_fn: |_, _, _| Ok(()),
            default_fn: || Box::new(0i32),
        };

        registry.register(persistable);

        assert!(!registry.is_empty());
        assert!(registry.get_by_table("test_table").is_some());
        assert!(registry.get_by_type_id(TypeId::of::<i32>()).is_some());
    }

    #[test]
    fn test_ensure_schema() {
        let store = SqliteStore::memory().unwrap();

        let persistable = PersistableType {
            table_name: "users",
            schema: &[
                "CREATE TABLE IF NOT EXISTS users (id TEXT PRIMARY KEY, name TEXT NOT NULL)"
            ],
            type_id: TypeId::of::<String>(),
            key_field: "id",
            load_fn: |_, _| Ok(None),
            save_fn: |_, _, _| Ok(()),
            default_fn: || Box::new(String::new()),
        };

        store.register(persistable);
        store.ensure_schema().unwrap();

        // Verify table exists by inserting a row
        let conn = store.conn.lock().unwrap();
        conn.execute("INSERT INTO users (id, name) VALUES ('1', 'test')", [])
            .unwrap();

        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_hydrate_empty() {
        let store = SqliteStore::memory().unwrap();

        let persistable = PersistableType {
            table_name: "empty_table",
            schema: &["CREATE TABLE IF NOT EXISTS empty_table (id TEXT PRIMARY KEY)"],
            type_id: TypeId::of::<()>(),
            key_field: "id",
            load_fn: |_, _| Ok(Some(Box::new(()))),
            save_fn: |_, _, _| Ok(()),
            default_fn: || Box::new(()),
        };

        store.register(persistable);
        store.ensure_schema().unwrap();

        let cache = store.hydrate_all().unwrap();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_hydrate_with_data() {
        let store = SqliteStore::memory().unwrap();

        // Simple test state: just an i32
        let persistable = PersistableType {
            table_name: "counters",
            schema: &["CREATE TABLE IF NOT EXISTS counters (id TEXT PRIMARY KEY, value INTEGER NOT NULL DEFAULT 0)"],
            type_id: TypeId::of::<i32>(),
            key_field: "id",
            load_fn: |conn, key| {
                match conn.query_row(
                    "SELECT value FROM counters WHERE id = ?",
                    [key],
                    |row| row.get::<_, i32>(0),
                ) {
                    Ok(value) => Ok(Some(Box::new(value))),
                    Err(SqliteError::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(PersistError::Sqlite(e)),
                }
            },
            save_fn: |conn, key, state| {
                let value = state.downcast_ref::<i32>().ok_or(PersistError::TypeMismatch)?;
                conn.execute(
                    "INSERT INTO counters (id, value) VALUES (?1, ?2) ON CONFLICT(id) DO UPDATE SET value = excluded.value",
                    rusqlite::params![key, value],
                )?;
                Ok(())
            },
            default_fn: || Box::new(0i32),
        };

        store.register(persistable);
        store.ensure_schema().unwrap();

        // Insert some data directly
        {
            let conn = store.conn.lock().unwrap();
            conn.execute("INSERT INTO counters (id, value) VALUES ('a', 42)", []).unwrap();
            conn.execute("INSERT INTO counters (id, value) VALUES ('b', 100)", []).unwrap();
        }

        let cache = store.hydrate_all().unwrap();
        assert_eq!(cache.len(), 2);

        let value_a = cache.get("counters:a").unwrap().downcast_ref::<i32>().unwrap();
        assert_eq!(*value_a, 42);

        let value_b = cache.get("counters:b").unwrap().downcast_ref::<i32>().unwrap();
        assert_eq!(*value_b, 100);
    }

    #[test]
    fn test_save_state() {
        let store = SqliteStore::memory().unwrap();

        let persistable = PersistableType {
            table_name: "counters",
            schema: &["CREATE TABLE IF NOT EXISTS counters (id TEXT PRIMARY KEY, value INTEGER NOT NULL DEFAULT 0)"],
            type_id: TypeId::of::<i32>(),
            key_field: "id",
            load_fn: |conn, key| {
                match conn.query_row(
                    "SELECT value FROM counters WHERE id = ?",
                    [key],
                    |row| row.get::<_, i32>(0),
                ) {
                    Ok(value) => Ok(Some(Box::new(value))),
                    Err(SqliteError::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(PersistError::Sqlite(e)),
                }
            },
            save_fn: |conn, key, state| {
                let value = state.downcast_ref::<i32>().ok_or(PersistError::TypeMismatch)?;
                conn.execute(
                    "INSERT INTO counters (id, value) VALUES (?1, ?2) ON CONFLICT(id) DO UPDATE SET value = excluded.value",
                    rusqlite::params![key, value],
                )?;
                Ok(())
            },
            default_fn: || Box::new(0i32),
        };

        store.register(persistable);
        store.ensure_schema().unwrap();

        // Save a value
        store.save("counters:test", &99i32).unwrap();

        // Verify it was saved
        let conn = store.conn.lock().unwrap();
        let value: i32 = conn
            .query_row("SELECT value FROM counters WHERE id = 'test'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, 99);
    }
}
