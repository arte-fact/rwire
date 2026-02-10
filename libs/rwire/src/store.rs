//! State persistence for rwire.
//!
//! This module provides traits and implementations for persisting state
//! across server restarts and connection drops.
//!
//! # Example
//!
//! ```ignore
//! use rwire::store::{StateStore, JsonFileStore};
//!
//! // Create a file-based store
//! let store = JsonFileStore::new("./data");
//!
//! // Save state
//! store.save("user:123", &user_state)?;
//!
//! // Load state
//! let user: Option<UserState> = store.load("user:123")?;
//! ```

use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Error type for state store operations.
#[derive(Debug)]
pub enum StoreError {
    /// I/O error during file operations.
    Io(io::Error),
    /// JSON serialization/deserialization error.
    Json(serde_json::Error),
    /// Key not found.
    NotFound(String),
    /// Lock acquisition failed.
    LockPoisoned,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Io(e) => write!(f, "I/O error: {}", e),
            StoreError::Json(e) => write!(f, "JSON error: {}", e),
            StoreError::NotFound(key) => write!(f, "Key not found: {}", key),
            StoreError::LockPoisoned => write!(f, "Lock poisoned"),
        }
    }
}

impl Error for StoreError {}

impl From<io::Error> for StoreError {
    fn from(e: io::Error) -> Self {
        StoreError::Io(e)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        StoreError::Json(e)
    }
}

/// Trait for state persistence stores.
///
/// Implementations can use different backends (file, Redis, database, etc.).
pub trait StateStore: Send + Sync {
    /// Save state for a given key.
    fn save<T: Serialize>(&self, key: &str, value: &T) -> Result<(), StoreError>;

    /// Load state for a given key.
    fn load<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, StoreError>;

    /// Delete state for a given key.
    fn delete(&self, key: &str) -> Result<(), StoreError>;

    /// Check if a key exists.
    fn exists(&self, key: &str) -> Result<bool, StoreError>;

    /// List all keys with a given prefix.
    fn list_keys(&self, prefix: &str) -> Result<Vec<String>, StoreError>;
}

/// File-based JSON state store.
///
/// Each key maps to a JSON file in the data directory.
/// Keys containing `/` or `\` are not allowed for security.
#[derive(Clone)]
pub struct JsonFileStore {
    data_dir: PathBuf,
}

impl JsonFileStore {
    /// Create a new file-based store with the given data directory.
    ///
    /// The directory will be created if it doesn't exist.
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self, StoreError> {
        let path = data_dir.as_ref().to_path_buf();
        fs::create_dir_all(&path)?;
        Ok(Self { data_dir: path })
    }

    fn key_to_path(&self, key: &str) -> Result<PathBuf, StoreError> {
        // Security: prevent directory traversal
        if key.contains('/') || key.contains('\\') || key.contains("..") {
            return Err(StoreError::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid key: contains path separators",
            )));
        }
        Ok(self.data_dir.join(format!("{}.json", key)))
    }
}

impl StateStore for JsonFileStore {
    fn save<T: Serialize>(&self, key: &str, value: &T) -> Result<(), StoreError> {
        let path = self.key_to_path(key)?;
        let json = serde_json::to_string_pretty(value)?;
        fs::write(path, json)?;
        Ok(())
    }

    fn load<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, StoreError> {
        let path = self.key_to_path(key)?;
        if !path.exists() {
            return Ok(None);
        }
        let json = fs::read_to_string(path)?;
        let value = serde_json::from_str(&json)?;
        Ok(Some(value))
    }

    fn delete(&self, key: &str) -> Result<(), StoreError> {
        let path = self.key_to_path(key)?;
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    fn exists(&self, key: &str) -> Result<bool, StoreError> {
        let path = self.key_to_path(key)?;
        Ok(path.exists())
    }

    fn list_keys(&self, prefix: &str) -> Result<Vec<String>, StoreError> {
        let mut keys = Vec::new();
        for entry in fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if stem.starts_with(prefix) {
                        keys.push(stem.to_string());
                    }
                }
            }
        }
        Ok(keys)
    }
}

/// In-memory state store for testing.
///
/// State is lost when the process exits.
#[derive(Clone, Default)]
pub struct MemoryStore {
    data: Arc<RwLock<HashMap<String, String>>>,
}

impl MemoryStore {
    /// Create a new in-memory store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl StateStore for MemoryStore {
    fn save<T: Serialize>(&self, key: &str, value: &T) -> Result<(), StoreError> {
        let json = serde_json::to_string(value)?;
        let mut data = self.data.write().map_err(|_| StoreError::LockPoisoned)?;
        data.insert(key.to_string(), json);
        Ok(())
    }

    fn load<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, StoreError> {
        let data = self.data.read().map_err(|_| StoreError::LockPoisoned)?;
        match data.get(key) {
            Some(json) => {
                let value = serde_json::from_str(json)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    fn delete(&self, key: &str) -> Result<(), StoreError> {
        let mut data = self.data.write().map_err(|_| StoreError::LockPoisoned)?;
        data.remove(key);
        Ok(())
    }

    fn exists(&self, key: &str) -> Result<bool, StoreError> {
        let data = self.data.read().map_err(|_| StoreError::LockPoisoned)?;
        Ok(data.contains_key(key))
    }

    fn list_keys(&self, prefix: &str) -> Result<Vec<String>, StoreError> {
        let data = self.data.read().map_err(|_| StoreError::LockPoisoned)?;
        let keys: Vec<String> = data
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestState {
        count: i32,
        name: String,
    }

    #[test]
    fn test_memory_store_save_load() {
        let store = MemoryStore::new();
        let state = TestState {
            count: 42,
            name: "test".to_string(),
        };

        store.save("key1", &state).unwrap();
        let loaded: Option<TestState> = store.load("key1").unwrap();

        assert_eq!(loaded, Some(state));
    }

    #[test]
    fn test_memory_store_not_found() {
        let store = MemoryStore::new();
        let loaded: Option<TestState> = store.load("nonexistent").unwrap();
        assert_eq!(loaded, None);
    }

    #[test]
    fn test_memory_store_delete() {
        let store = MemoryStore::new();
        let state = TestState {
            count: 1,
            name: "x".to_string(),
        };

        store.save("key1", &state).unwrap();
        assert!(store.exists("key1").unwrap());

        store.delete("key1").unwrap();
        assert!(!store.exists("key1").unwrap());
    }

    #[test]
    fn test_memory_store_list_keys() {
        let store = MemoryStore::new();
        let state = TestState {
            count: 1,
            name: "x".to_string(),
        };

        store.save("user:1", &state).unwrap();
        store.save("user:2", &state).unwrap();
        store.save("other:1", &state).unwrap();

        let keys = store.list_keys("user:").unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"user:1".to_string()));
        assert!(keys.contains(&"user:2".to_string()));
    }

    #[test]
    fn test_json_file_store() {
        let temp_dir = std::env::temp_dir().join("rwire_test_store");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up any previous test
        let store = JsonFileStore::new(&temp_dir).unwrap();

        let state = TestState {
            count: 123,
            name: "file_test".to_string(),
        };

        store.save("test_key", &state).unwrap();
        let loaded: Option<TestState> = store.load("test_key").unwrap();
        assert_eq!(loaded, Some(state));

        store.delete("test_key").unwrap();
        assert!(!store.exists("test_key").unwrap());

        // Clean up
        let _ = fs::remove_dir_all(temp_dir);
    }
}
