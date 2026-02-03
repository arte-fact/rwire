# Phase 2: Shared State Cache

**Goal**: Move persisted state from per-connection to server-level shared cache.

## Overview

Currently, each connection has its own copy of state. For persisted state, we need a single source of truth that all connections share.

```
BEFORE (per-connection):
┌─────────────┐  ┌─────────────┐
│ Connection1 │  │ Connection2 │
│ state: {...}│  │ state: {...}│  ← Different copies!
└─────────────┘  └─────────────┘

AFTER (shared):
┌────────────────────────────┐
│     SharedServerState      │
│  cache: "todos:abc" → {...}│  ← Single source of truth
└────────────────────────────┘
       ▲              ▲
       │              │
┌──────┴──┐    ┌──────┴──┐
│ Conn 1  │    │ Conn 2  │
└─────────┘    └─────────┘
```

## New Types: `rwire/src/server.rs`

```rust
use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};
use async_channel::Sender;

/// Shared state across all connections.
pub struct SharedServerState {
    /// Persisted state cache.
    /// Key format: "{table}:{session_id}"
    pub shared_cache: RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>,

    /// Keys that have been modified and need persistence.
    pub dirty_keys: RwLock<HashSet<String>>,

    /// Subscriptions: which connections are watching which keys.
    pub subscriptions: RwLock<HashMap<String, Vec<u64>>>,

    /// Broadcast channels to notify connections of changes.
    pub broadcast_senders: RwLock<HashMap<u64, Sender<BroadcastMsg>>>,

    /// Counter for unique connection IDs.
    next_connection_id: AtomicU64,

    /// Optional database store for persistence.
    pub store: Option<Arc<dyn StateStore + Send + Sync>>,

    /// Persist interval for background task.
    pub persist_interval: std::time::Duration,
}

/// Message broadcast to connections when shared state changes.
#[derive(Clone, Debug)]
pub enum BroadcastMsg {
    /// State changed, re-render needed.
    StateChanged {
        key: String,
        state_type_id: TypeId,
        changes: ChangeSet,
    },
}

impl SharedServerState {
    /// Create new shared state.
    pub fn new(
        store: Option<Arc<dyn StateStore + Send + Sync>>,
        persist_interval: std::time::Duration,
    ) -> Arc<Self> {
        Arc::new(Self {
            shared_cache: RwLock::new(HashMap::new()),
            dirty_keys: RwLock::new(HashSet::new()),
            subscriptions: RwLock::new(HashMap::new()),
            broadcast_senders: RwLock::new(HashMap::new()),
            next_connection_id: AtomicU64::new(1),
            store,
            persist_interval,
        })
    }

    /// Allocate unique connection ID.
    pub fn next_connection_id(&self) -> u64 {
        self.next_connection_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Get state from cache, creating default if missing.
    pub fn get_or_create<S: Default + Send + Sync + 'static>(
        &self,
        key: &str,
    ) -> Box<dyn Any + Send + Sync> {
        let mut cache = self.shared_cache.write().unwrap();
        if !cache.contains_key(key) {
            cache.insert(key.to_string(), Box::new(S::default()));
        }
        // Clone for connection's use
        // Note: Requires Clone implementation or Arc wrapper
        cache.get(key).unwrap().clone()
    }

    /// Update state in cache and mark dirty.
    pub fn update<S: Send + Sync + 'static>(
        &self,
        key: &str,
        state: S,
        changes: ChangeSet,
        from_connection: u64,
    ) {
        // Update cache
        {
            let mut cache = self.shared_cache.write().unwrap();
            cache.insert(key.to_string(), Box::new(state));
        }

        // Mark dirty for background persistence
        {
            let mut dirty = self.dirty_keys.write().unwrap();
            dirty.insert(key.to_string());
        }

        // Broadcast to other connections
        self.broadcast(key, TypeId::of::<S>(), changes, from_connection);
    }

    /// Register connection's broadcast channel.
    pub fn register_connection(&self, conn_id: u64, sender: Sender<BroadcastMsg>) {
        self.broadcast_senders.write().unwrap().insert(conn_id, sender);
    }

    /// Unregister connection on disconnect.
    pub fn unregister_connection(&self, conn_id: u64) {
        self.broadcast_senders.write().unwrap().remove(&conn_id);

        // Remove from all subscriptions
        let mut subs = self.subscriptions.write().unwrap();
        for conn_ids in subs.values_mut() {
            conn_ids.retain(|&id| id != conn_id);
        }
    }

    /// Subscribe connection to state changes for a key.
    pub fn subscribe(&self, conn_id: u64, key: &str) {
        self.subscriptions
            .write()
            .unwrap()
            .entry(key.to_string())
            .or_default()
            .push(conn_id);
    }

    /// Broadcast state change to subscribed connections.
    fn broadcast(
        &self,
        key: &str,
        state_type_id: TypeId,
        changes: ChangeSet,
        except_conn_id: u64,
    ) {
        let msg = BroadcastMsg::StateChanged {
            key: key.to_string(),
            state_type_id,
            changes,
        };

        let subs = self.subscriptions.read().unwrap();
        let senders = self.broadcast_senders.read().unwrap();

        if let Some(conn_ids) = subs.get(key) {
            for &conn_id in conn_ids {
                if conn_id != except_conn_id {
                    if let Some(sender) = senders.get(&conn_id) {
                        // Non-blocking send, drop if channel full
                        let _ = sender.try_send(msg.clone());
                    }
                }
            }
        }
    }
}
```

## Update ConnectionState

```rust
/// Per-connection state (only for memory state now).
struct ConnectionState {
    /// Unique connection ID.
    connection_id: u64,

    /// Session ID from cookie.
    session_id: SessionId,

    /// Memory-only state (not persisted).
    memory_states: HashMap<TypeId, Box<dyn Any + Send + Sync>>,

    /// Keys this connection uses from shared cache.
    subscribed_keys: HashSet<String>,

    /// Handlers registered for this connection.
    handlers: Vec<HandlerFn>,

    /// Synced elements for re-rendering.
    synced_elements: Vec<SyncedElement>,

    /// Symbol table for string interning.
    sent_symbols: HashMap<String, u8>,

    /// Receiver for broadcast messages.
    broadcast_rx: async_channel::Receiver<BroadcastMsg>,
}

impl ConnectionState {
    fn new(
        connection_id: u64,
        session_id: SessionId,
        broadcast_rx: async_channel::Receiver<BroadcastMsg>,
    ) -> Self {
        Self {
            connection_id,
            session_id,
            memory_states: HashMap::new(),
            subscribed_keys: HashSet::new(),
            handlers: Vec::new(),
            synced_elements: Vec::new(),
            sent_symbols: HashMap::new(),
            broadcast_rx,
        }
    }

    /// Get state reference, checking shared cache for persisted types.
    fn get_state<'a>(
        &'a self,
        type_id: TypeId,
        storage_type: StorageType,
        key: &str,
        shared: &'a SharedServerState,
    ) -> Option<&'a Box<dyn Any + Send + Sync>> {
        match storage_type {
            StorageType::Memory => self.memory_states.get(&type_id),
            StorageType::Persisted => {
                shared.shared_cache.read().unwrap().get(key)
            }
            StorageType::Local => None, // Local is client-side only
        }
    }
}
```

## Server Builder Update

```rust
pub struct Server {
    addr: SocketAddr,
    store: Option<Arc<dyn StateStore + Send + Sync>>,
    persist_interval: std::time::Duration,
}

impl Server {
    pub fn bind(addr: &str) -> Result<Self, std::io::Error> {
        Ok(Self {
            addr: addr.parse().map_err(|e| std::io::Error::new(
                std::io::ErrorKind::InvalidInput, e
            ))?,
            store: None,
            persist_interval: std::time::Duration::from_millis(100),
        })
    }

    /// Configure state store for persistence.
    pub fn store<S: StateStore + Send + Sync + 'static>(mut self, store: S) -> Self {
        self.store = Some(Arc::new(store));
        self
    }

    /// Configure persist interval (default 100ms).
    pub fn persist_interval(mut self, interval: std::time::Duration) -> Self {
        self.persist_interval = interval;
        self
    }

    pub fn root<F>(self, root: F) -> ServerWithRoot<F> {
        ServerWithRoot {
            addr: self.addr,
            store: self.store,
            persist_interval: self.persist_interval,
            root,
        }
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default, Clone)]
    struct TestState { count: i32 }

    #[test]
    fn test_shared_cache_get_or_create() {
        let shared = SharedServerState::new(None, Duration::from_millis(100));

        // First access creates default
        let state = shared.get_or_create::<TestState>("test:abc");
        let typed = state.downcast_ref::<TestState>().unwrap();
        assert_eq!(typed.count, 0);
    }

    #[test]
    fn test_shared_cache_update_marks_dirty() {
        let shared = SharedServerState::new(None, Duration::from_millis(100));

        shared.update("test:abc", TestState { count: 42 }, ChangeSet::all(), 1);

        assert!(shared.dirty_keys.read().unwrap().contains("test:abc"));
    }

    #[test]
    fn test_subscription_broadcast() {
        let shared = SharedServerState::new(None, Duration::from_millis(100));

        let (tx1, rx1) = async_channel::bounded(10);
        let (tx2, rx2) = async_channel::bounded(10);

        shared.register_connection(1, tx1);
        shared.register_connection(2, tx2);

        shared.subscribe(1, "test:key");
        shared.subscribe(2, "test:key");

        // Update from connection 1
        shared.update("test:key", TestState { count: 1 }, ChangeSet::all(), 1);

        // Connection 1 should NOT receive (sender excluded)
        assert!(rx1.is_empty());

        // Connection 2 SHOULD receive
        assert!(!rx2.is_empty());
    }

    #[test]
    fn test_unregister_cleans_subscriptions() {
        let shared = SharedServerState::new(None, Duration::from_millis(100));

        let (tx, _rx) = async_channel::bounded(10);
        shared.register_connection(1, tx);
        shared.subscribe(1, "key1");
        shared.subscribe(1, "key2");

        shared.unregister_connection(1);

        let subs = shared.subscriptions.read().unwrap();
        assert!(!subs.get("key1").map_or(false, |v| v.contains(&1)));
        assert!(!subs.get("key2").map_or(false, |v| v.contains(&1)));
    }
}
```

## Checklist

- [ ] Create `SharedServerState` struct
- [ ] Add `shared_cache` for persisted state
- [ ] Add `dirty_keys` for tracking modifications
- [ ] Add subscription management
- [ ] Add broadcast mechanism
- [ ] Update `ConnectionState` to use shared cache
- [ ] Update `Server` builder with store and interval config
- [ ] Add unit tests
- [ ] Run `cargo clippy`
