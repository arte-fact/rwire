//! WebSocket server for rwire with stateful client connections.
//!
//! Single port serving both:
//! - HTTP GET / → capsule HTML
//! - WebSocket upgrade → binary DOM protocol with state management

use async_std::future::timeout;
use async_std::net::{TcpListener, TcpStream};
use async_std::task;
use async_tungstenite::accept_async_with_config;
use async_tungstenite::tungstenite::protocol::WebSocketConfig;
use async_tungstenite::tungstenite::Message;
use bytes::Bytes;
use futures::prelude::*;
use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::net::{AddrParseError, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::builder::{
    build_synced_update_with_known_symbols, extract_renderers, BuildContext, ClientActionIndices,
    ElementBuilder, SyncedElement,
};
use crate::capsule;
use crate::capsule_gen::{self, CapsuleConfig};
use crate::config::ServerConfig;
use crate::metrics::Metrics;
use crate::protocol::ClientEvent;
use crate::registry::{AdmissionError, ConnectionRegistry};
use crate::session::SessionId;
use crate::state::{ChangeSet, EventContext, HandlerFn, HandlerSpec, State, StorageType};
use crate::theme::ThemeProvider;

// ============================================================================
// Shared Server State
// ============================================================================

/// Message broadcast to connections when shared state changes.
#[derive(Clone, Debug)]
pub enum BroadcastMsg {
    /// State changed, re-render needed.
    StateChanged {
        /// Cache key: "{table}:{session_id}"
        key: String,
        /// TypeId of the state struct
        state_type_id: TypeId,
        /// Which fields changed
        changes: ChangeSet,
    },
}

/// Cached session state for reconnection.
struct CachedSession {
    states: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    cached_at: Instant,
}

/// Upper bound on disconnected-session states retained for reconnection. Caps the
/// memory an attacker can pin by churning connections within the eviction TTL.
const MAX_CACHED_SESSIONS: usize = 10_000;

/// Maximum size of an incoming WebSocket message/frame the server will buffer.
/// Client→server traffic is small (binary events, route strings; the protocol
/// decoder caps a parsed payload at 64KB), so a few hundred KB is generous while
/// far below tungstenite's 64 MiB / 16 MiB defaults. Limits reads only.
const MAX_WS_MESSAGE_SIZE: usize = 256 * 1024;
const MAX_WS_FRAME_SIZE: usize = 256 * 1024;

/// Per-connection inbound-event rate limit (token bucket). Capacity is the burst
/// allowance; refill is the sustained rate. 100/s with a 100-event burst is far
/// above human interaction (clicks, typing, even a 60 fps drag) yet caps a
/// well-formed event flood that would otherwise drive unbounded render/broadcast work.
const EVENT_BUCKET_CAPACITY: f64 = 100.0;
const EVENT_REFILL_PER_SEC: f64 = 100.0;

/// Per-connection ceiling on the interned symbol table (distinct strings sent to
/// the client). The table can't be evicted safely (wire indices are positional),
/// so a connection that exceeds this is dropped rather than allowed to grow without
/// bound. Set far above any realistic app vocabulary; a reconnect resets the table.
const MAX_SENT_SYMBOLS: usize = 50_000;

/// Shared state across all connections.
///
/// This holds the single source of truth for persisted state, allowing
/// multiple connections to share state and receive updates when it changes.
pub struct SharedServerState {
    /// Persisted state cache.
    /// Key format: "{table}:{session_id}"
    pub shared_cache: RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>,

    /// Keys that have been modified and need persistence.
    pub dirty_keys: RwLock<HashSet<String>>,

    /// Subscriptions: which connections are watching which keys.
    pub subscriptions: RwLock<HashMap<String, Vec<u64>>>,

    /// Broadcast channels to notify connections of changes.
    pub broadcast_senders: RwLock<HashMap<u64, async_channel::Sender<BroadcastMsg>>>,

    /// Counter for unique connection IDs.
    next_connection_id: AtomicU64,

    /// Persist interval for background task.
    pub persist_interval: Duration,

    /// Capacity of per-connection broadcast channels. Default: 32.
    pub broadcast_capacity: usize,

    /// Cached session states for reconnection.
    session_state_cache: RwLock<HashMap<String, CachedSession>>,
}

/// Resolve the `shared_cache` key for a state type, or `None` for memory state.
///
/// Persisted state is keyed per session; shared state has one global instance.
/// This is the single source of truth used by every render and mutation path.
fn shared_cache_key(storage: StorageType, table: Option<&str>, session_id: &str) -> Option<String> {
    match storage {
        StorageType::Memory => None,
        StorageType::Persisted => table.map(|t| format!("{t}:{session_id}")),
        StorageType::Shared => table.map(|t| format!("__shared__:{t}")),
    }
}

/// Collect `(TypeId, cache_key)` for every shared/persisted state referenced by
/// these handlers and synced elements. Memory state is omitted (it lives
/// per-connection). Deduplicated; used to drive subscription and state mapping.
fn shared_persisted_keys(
    handlers: &HashMap<u32, HandlerFn>,
    synced: &[SyncedElement],
    session_id: &str,
) -> Vec<(TypeId, String)> {
    let from_handlers = handlers
        .values()
        .map(|h| (h.state_type_id(), h.storage_type(), h.table_name()));
    let from_synced = synced.iter().map(|s| {
        (
            s.state_type_id,
            s.renderer.storage_type(),
            s.renderer.table_name(),
        )
    });

    let mut out: Vec<(TypeId, String)> = Vec::new();
    for (tid, storage, table) in from_handlers.chain(from_synced) {
        if let Some(key) = shared_cache_key(storage, table, session_id) {
            if !out.iter().any(|(t, k)| *t == tid && *k == key) {
                out.push((tid, key));
            }
        }
    }
    out
}

impl SharedServerState {
    /// Create new shared server state with default broadcast capacity (32).
    pub fn new(persist_interval: Duration) -> Arc<Self> {
        Self::with_broadcast_capacity(persist_interval, 32)
    }

    /// Create new shared server state with custom broadcast capacity.
    pub fn with_broadcast_capacity(
        persist_interval: Duration,
        broadcast_capacity: usize,
    ) -> Arc<Self> {
        Arc::new(Self {
            shared_cache: RwLock::new(HashMap::new()),
            dirty_keys: RwLock::new(HashSet::new()),
            subscriptions: RwLock::new(HashMap::new()),
            broadcast_senders: RwLock::new(HashMap::new()),
            next_connection_id: AtomicU64::new(1),
            persist_interval,
            broadcast_capacity,
            session_state_cache: RwLock::new(HashMap::new()),
        })
    }

    /// Mutate a `#[storage(shared)]` state type from outside a connection
    /// (e.g. a background task), then re-render every connection bound to it.
    ///
    /// The instance lives once in `shared_cache`; this locks it, applies `f`,
    /// and broadcasts so all connections diff and push. Handlers mutating shared
    /// state go through the normal handler path instead.
    pub fn update_shared<T: State + Default>(&self, f: impl FnOnce(&mut T)) {
        self.update_shared_changed(ChangeSet::all(), f);
    }

    /// Like [`Self::update_shared`], but broadcasts a specific [`ChangeSet`] so
    /// only renderers whose field dependencies overlap the changed fields
    /// re-render.
    ///
    /// Use this for high-frequency updates that touch a subset of a state's
    /// fields (e.g. a 1 s metrics poll writing only one field) to avoid
    /// re-rendering — and disrupting the inputs of — regions bound to the same
    /// state but reading different fields. Pair with the `FIELD_*` consts the
    /// `State` derive emits, e.g. `ChangeSet::from_fields(&[App::FIELD_HW])`.
    pub fn update_shared_changed<T: State + Default>(
        &self,
        changes: ChangeSet,
        f: impl FnOnce(&mut T),
    ) {
        let key = shared_cache_key(StorageType::Shared, Some(T::TABLE_NAME), "");
        let Some(key) = key else { return };
        {
            let mut cache = self.shared_cache.write().unwrap_or_else(|e| e.into_inner());
            let slot = cache
                .entry(key.clone())
                .or_insert_with(|| Box::new(T::default()) as Box<dyn Any + Send + Sync>);
            if let Some(value) = slot.downcast_mut::<T>() {
                f(value);
            }
        }
        // except_conn_id = 0 is never a real connection id (ids start at 1).
        self.broadcast(&key, TypeId::of::<T>(), changes, 0);
    }

    /// Allocate unique connection ID.
    pub fn next_connection_id(&self) -> u64 {
        self.next_connection_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Check if state exists in cache.
    pub fn has_state(&self, key: &str) -> bool {
        self.shared_cache
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(key)
    }

    /// Insert state into cache (for hydration).
    pub fn insert_state(&self, key: String, state: Box<dyn Any + Send + Sync>) {
        self.shared_cache
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key, state);
    }

    /// Mark a key as dirty (needs persistence).
    pub fn mark_dirty(&self, key: &str) {
        self.dirty_keys
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key.to_string());
    }

    /// Check if any keys are dirty.
    pub fn has_dirty(&self) -> bool {
        !self
            .dirty_keys
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .is_empty()
    }

    /// Get count of dirty keys.
    pub fn dirty_count(&self) -> usize {
        self.dirty_keys
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }

    /// Drain all dirty keys for persistence.
    pub fn drain_dirty(&self) -> Vec<String> {
        let mut dirty = self.dirty_keys.write().unwrap_or_else(|e| e.into_inner());
        dirty.drain().collect()
    }

    /// Register connection's broadcast channel.
    pub fn register_connection(&self, conn_id: u64, sender: async_channel::Sender<BroadcastMsg>) {
        self.broadcast_senders
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(conn_id, sender);
    }

    /// Unregister connection on disconnect.
    pub fn unregister_connection(&self, conn_id: u64) {
        self.broadcast_senders
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&conn_id);

        // Remove from all subscriptions
        let mut subs = self
            .subscriptions
            .write()
            .unwrap_or_else(|e| e.into_inner());
        for conn_ids in subs.values_mut() {
            conn_ids.retain(|&id| id != conn_id);
        }
    }

    /// Subscribe connection to state changes for a key.
    pub fn subscribe(&self, conn_id: u64, key: &str) {
        self.subscriptions
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .entry(key.to_string())
            .or_default()
            .push(conn_id);
    }

    /// Hydrate shared cache from a SqliteStore.
    ///
    /// This loads all persisted state from the database into memory.
    /// Should be called at server startup before accepting connections.
    pub fn hydrate(
        &self,
        store: &crate::persist::SqliteStore,
    ) -> Result<usize, crate::persist::PersistError> {
        // Ensure schemas exist
        store.ensure_schema()?;

        // Load all state into cache
        let states = store.hydrate_all()?;
        let count = states.len();

        let mut cache = self.shared_cache.write().unwrap_or_else(|e| e.into_inner());
        for (key, state) in states {
            cache.insert(key, state);
        }

        Ok(count)
    }

    /// Broadcast state change to subscribed connections.
    pub fn broadcast(
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

        let subs = self.subscriptions.read().unwrap_or_else(|e| e.into_inner());
        let senders = self
            .broadcast_senders
            .read()
            .unwrap_or_else(|e| e.into_inner());

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

    /// Notify every connection that a state type changed, so its renderers
    /// re-evaluate and push a diff.
    ///
    /// Unlike [`Self::broadcast`], this does not require a subscription key. Use
    /// it for process-global state mutated outside a client event (e.g. a
    /// background poller). The `key` of the message is empty; connections render
    /// such updates from their own (memory) state.
    pub fn notify_all(&self, state_type_id: TypeId, changes: ChangeSet) {
        let msg = BroadcastMsg::StateChanged {
            key: String::new(),
            state_type_id,
            changes,
        };
        let senders = self
            .broadcast_senders
            .read()
            .unwrap_or_else(|e| e.into_inner());
        for sender in senders.values() {
            // Non-blocking; drop if a slow connection's channel is full.
            let _ = sender.try_send(msg.clone());
        }
    }

    /// Cache session state on disconnect for later reconnection.
    ///
    /// The cache is bounded to [`MAX_CACHED_SESSIONS`]: if inserting would exceed
    /// it, the oldest entry is evicted first. Without this bound, an attacker
    /// looping connect → disconnect with a fresh cookie each time would accumulate
    /// per-session state for the full 5-minute TTL with no ceiling (memory DoS).
    pub fn cache_session(
        &self,
        session_id: &str,
        states: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    ) {
        if states.is_empty() {
            return;
        }
        let mut cache = self
            .session_state_cache
            .write()
            .unwrap_or_else(|e| e.into_inner());

        // Evict the oldest entry when at capacity (and this is a new session id,
        // so the insert would grow the map rather than replace in place).
        if cache.len() >= MAX_CACHED_SESSIONS && !cache.contains_key(session_id) {
            if let Some(oldest_key) = cache
                .iter()
                .min_by_key(|(_, c)| c.cached_at)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest_key);
            }
        }

        cache.insert(
            session_id.to_string(),
            CachedSession {
                states,
                cached_at: Instant::now(),
            },
        );
    }

    /// Restore cached session state on reconnect (removes from cache).
    pub fn restore_session(
        &self,
        session_id: &str,
    ) -> Option<HashMap<TypeId, Box<dyn Any + Send + Sync>>> {
        self.session_state_cache
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(session_id)
            .map(|c| c.states)
    }

    /// Evict expired sessions older than the given TTL.
    pub fn evict_expired_sessions(&self, ttl: Duration) {
        self.session_state_cache
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|_, cached| cached.cached_at.elapsed() < ttl);
    }

    /// Persist all dirty state to the store.
    ///
    /// This is called by the background persist task and during graceful shutdown.
    /// Returns the number of keys successfully persisted.
    pub fn persist_dirty(
        &self,
        store: &crate::persist::SqliteStore,
    ) -> Result<usize, crate::persist::PersistError> {
        let dirty_keys = self.drain_dirty();
        if dirty_keys.is_empty() {
            return Ok(0);
        }

        let conn = store.connection();
        let conn = conn
            .lock()
            .map_err(|_| crate::persist::PersistError::LockPoisoned)?;

        // Start transaction
        conn.execute("BEGIN TRANSACTION", [])?;

        let mut persisted_count = 0;
        let mut failed_keys = Vec::new();

        for key in &dirty_keys {
            match self.persist_single_state(&conn, store, key) {
                Ok(()) => persisted_count += 1,
                Err(e) => {
                    eprintln!("Failed to persist {}: {}", key, e);
                    failed_keys.push(key.clone());
                }
            }
        }

        if failed_keys.is_empty() {
            conn.execute("COMMIT", [])?;
        } else {
            conn.execute("ROLLBACK", [])?;
            // Re-mark all failed keys as dirty for retry
            for key in failed_keys {
                self.mark_dirty(&key);
            }
        }

        Ok(persisted_count)
    }

    /// Persist a single state to the database.
    fn persist_single_state(
        &self,
        conn: &rusqlite::Connection,
        store: &crate::persist::SqliteStore,
        key: &str,
    ) -> Result<(), crate::persist::PersistError> {
        // Parse table name and session_id from key
        let mut parts = key.splitn(2, ':');
        let table_name = parts.next().ok_or_else(|| {
            crate::persist::PersistError::ConnectionError("Invalid key format".to_string())
        })?;
        let session_id = parts.next().ok_or_else(|| {
            crate::persist::PersistError::ConnectionError("Invalid key format".to_string())
        })?;

        // Get persistable type info from registry
        let registry = store.registry();
        let registry = registry
            .lock()
            .map_err(|_| crate::persist::PersistError::LockPoisoned)?;
        let persistable = registry
            .get_by_table(table_name)
            .ok_or_else(|| crate::persist::PersistError::TypeNotFound(table_name.to_string()))?;

        // Get state from cache
        let cache = self
            .shared_cache
            .read()
            .map_err(|_| crate::persist::PersistError::LockPoisoned)?;
        let state = cache.get(key).ok_or_else(|| {
            crate::persist::PersistError::ConnectionError(format!("State not in cache: {}", key))
        })?;

        // Save to database using the persistable's save function
        (persistable.save_fn)(conn, session_id, &**state)
    }

    /// Flush all dirty state to the store synchronously.
    ///
    /// This is called during graceful shutdown to ensure all state is persisted
    /// before the server exits. It will retry failed keys up to max_attempts times.
    ///
    /// Returns the total number of keys successfully persisted.
    pub fn flush_all_dirty(
        &self,
        store: &crate::persist::SqliteStore,
        max_attempts: u32,
    ) -> Result<usize, crate::persist::PersistError> {
        let mut total_persisted = 0;
        let mut attempts = 0;

        while self.has_dirty() && attempts < max_attempts {
            attempts += 1;

            let dirty_count = self.dirty_count();
            eprintln!(
                "Flushing {} dirty keys (attempt {})...",
                dirty_count, attempts
            );

            match self.persist_dirty(store) {
                Ok(count) => {
                    total_persisted += count;
                    eprintln!("  Persisted {} keys successfully.", count);
                }
                Err(e) => {
                    eprintln!("  Flush error: {}", e);
                }
            }
        }

        if self.has_dirty() {
            let remaining = self.dirty_count();
            eprintln!(
                "WARNING: {} keys could not be persisted after {} attempts",
                remaining, max_attempts
            );
        }

        Ok(total_persisted)
    }
}

/// Background task that persists dirty state to the database.
///
/// This task runs in a loop, sleeping for the configured interval between
/// persistence cycles. Dirty keys are drained and persisted in a single
/// transaction for efficiency.
pub async fn persist_task(shared: Arc<SharedServerState>, store: crate::persist::SqliteStore) {
    loop {
        // Wait for persist interval
        task::sleep(shared.persist_interval).await;

        // Persist dirty state
        match shared.persist_dirty(&store) {
            Ok(count) if count > 0 => {
                // Optionally log: println!("Persisted {} keys", count);
            }
            Ok(_) => {
                // No dirty keys
            }
            Err(e) => {
                eprintln!("Persist task error: {}", e);
            }
        }
    }
}

/// Background task that evicts expired session state caches.
///
/// Runs periodically at half the TTL interval. Sessions older than the TTL
/// are removed to prevent unbounded memory growth.
pub async fn session_eviction_task(shared: Arc<SharedServerState>, ttl: Duration) {
    loop {
        task::sleep(ttl / 2).await;
        shared.evict_expired_sessions(ttl);
    }
}

// ============================================================================
// Server Builder
// ============================================================================

/// Server builder - first step.
pub struct ServerBuilder {
    addr: SocketAddr,
    persist_interval: Duration,
    base_path: Option<String>,
}

/// Server with root element configured, ready to run.
pub struct ServerWithRoot<F> {
    addr: SocketAddr,
    persist_interval: Duration,
    root: F,
    shared: Option<Arc<SharedServerState>>,
    capsule_config: Option<CapsuleConfig>,
    base_path: Option<String>,
    route_handler: Option<HandlerFn>,
    router: Option<crate::router::Router>,
    theme_provider: Option<ThemeProvider>,
    auth: Option<AuthGate>,
    config: ServerConfig,
}

impl Server {
    /// Start building a server bound to the given address.
    pub fn bind(addr: &str) -> Result<ServerBuilder, AddrParseError> {
        Ok(ServerBuilder {
            addr: addr.parse()?,
            persist_interval: Duration::from_millis(100),
            base_path: None,
        })
    }
}

/// Marker type for the Server namespace.
pub struct Server;

impl ServerBuilder {
    /// Configure persist interval (default 100ms).
    pub fn persist_interval(mut self, interval: Duration) -> Self {
        self.persist_interval = interval;
        self
    }

    /// Mount the whole app under a URL prefix (e.g. `/preview/<id>`), set right alongside the bind
    /// address so a deployment reads both from its environment in one place. Applied to the capsule
    /// last (at [`run`](ServerWithRoot::run)), so it survives a later `.capsule_config(...)`; an
    /// empty prefix is a no-op. See [`CapsuleConfig::base_path`](crate::CapsuleConfig::base_path).
    pub fn base_path(mut self, prefix: impl Into<String>) -> Self {
        self.base_path = Some(prefix.into());
        self
    }

    /// Set the root element builder function.
    ///
    /// This function is called once per connection to build the initial DOM.
    /// The returned ElementBuilder can contain synced elements that will
    /// automatically re-render when state changes.
    pub fn root<F>(self, f: F) -> ServerWithRoot<F>
    where
        F: Fn() -> ElementBuilder + Send + Sync + 'static,
    {
        ServerWithRoot {
            addr: self.addr,
            persist_interval: self.persist_interval,
            root: f,
            shared: None,
            capsule_config: None,
            base_path: self.base_path,
            route_handler: None,
            router: None,
            theme_provider: None,
            auth: None,
            config: ServerConfig::default(),
        }
    }
}

/// Fold a builder-level `base_path` into the capsule config as the final step before capsule
/// generation. Applied last so it wins regardless of where `.base_path(...)` sat relative to
/// `.capsule_config(...)`; an empty/`"/"`-only prefix is dropped (a true no-op that leaves the
/// basic capsule intact), and a base with no prior config forces a default styled capsule so the
/// `const BASE` line is still emitted.
fn apply_base_path(
    config: Option<CapsuleConfig>,
    base_path: Option<String>,
) -> Option<CapsuleConfig> {
    let base_path = base_path.filter(|prefix| !prefix.trim().trim_end_matches('/').is_empty());
    match (config, base_path) {
        (Some(config), Some(base)) => Some(config.base_path(base)),
        (None, Some(base)) => Some(CapsuleConfig::default().base_path(base)),
        (config, None) => config,
    }
}

impl<F> ServerWithRoot<F>
where
    F: Fn() -> ElementBuilder + Send + Sync + Clone + 'static,
{
    /// Set a pre-created shared server state.
    ///
    /// Use this when you need to hydrate state from a database before running
    /// the server. This allows persistence to be configured before accepting
    /// connections.
    /// Gate all access — the page *and* the WebSocket — behind a login form.
    ///
    /// Unauthenticated page requests get the login page; a successful `POST
    /// /login` issues a random session cookie that subsequent requests (and the
    /// WebSocket upgrade) must present. `GET /logout` clears it. Off by default;
    /// wire it from env in the application so local development stays open.
    pub fn auth(mut self, user: impl Into<String>, password: impl Into<String>) -> Self {
        self.auth = Some(AuthGate::new(user.into(), password.into()));
        self
    }

    /// Brand/title shown on the login form (with a glyph). Call after [`auth`];
    /// no-op if auth isn't enabled.
    pub fn auth_brand(mut self, brand: impl Into<String>) -> Self {
        if let Some(gate) = self.auth.as_mut() {
            gate.brand = Some(brand.into());
        }
        self
    }

    /// Set a stable **development** session token. When set, login issues this token as the
    /// session cookie and the gate accepts it without consulting the in-memory token map — so it
    /// stays valid across server restarts. A rebuild (e.g. `cargo watch`) then keeps an open tab
    /// logged in (and on its current route, since the client reloads in place) instead of bouncing
    /// it to the login form.
    ///
    /// Intended for local development only — drive it from an env var and leave it unset in
    /// production, where sessions should stay ephemeral. Call after [`auth`]; no-op without it.
    pub fn dev_session(mut self, token: impl Into<String>) -> Self {
        if let Some(gate) = self.auth.as_mut() {
            gate.dev_token = Some(token.into());
        }
        self
    }

    pub fn with_shared_state(mut self, shared: Arc<SharedServerState>) -> Self {
        self.shared = Some(shared);
        self
    }

    /// Set the capsule configuration for styled output.
    ///
    /// This enables the styling system with theme support and component CSS.
    /// The capsule will include tree-shaken CSS for only the components used.
    pub fn capsule_config(mut self, config: CapsuleConfig) -> Self {
        self.capsule_config = Some(config);
        self
    }

    /// Mount the whole app under a URL prefix (e.g. `/preview/<id>`) — see
    /// [`CapsuleConfig::base_path`](crate::CapsuleConfig::base_path). Reachable behind a same-origin
    /// reverse proxy (which strips the prefix) with no server-side path changes. Applied to the
    /// capsule **last** at [`run`](Self::run), so chain order doesn't matter — setting it before or
    /// after `.capsule_config(...)` both work, and it can't be silently clobbered. Empty = no-op.
    pub fn base_path(mut self, prefix: impl Into<String>) -> Self {
        self.base_path = Some(prefix.into());
        self
    }

    /// Configure connection limits and timeouts (admission control).
    ///
    /// Sets the total/per-IP connection caps enforced before each WebSocket
    /// upgrade, and the limits surfaced by the `/health` and `/ready` endpoints.
    /// Defaults to [`ServerConfig::default`] (10k total, 100 per IP).
    pub fn config(mut self, config: ServerConfig) -> Self {
        self.config = config;
        self
    }

    /// Register a handler for client-side route changes.
    ///
    /// When the browser URL changes (via link click or back/forward button),
    /// the client sends a route message to the server. This handler receives
    /// the new path via `ctx.text()` and can update state accordingly.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[handler]
    /// fn on_route(state: &mut AppState, ctx: &EventContext) {
    ///     if let Some(path) = ctx.text() {
    ///         state.current_path = path.to_string();
    ///     }
    /// }
    ///
    /// Server::bind("0.0.0.0:9000")?
    ///     .root(root)
    ///     .on_route(on_route())
    ///     .run()
    ///     .await
    /// ```
    pub fn on_route(mut self, handler: HandlerSpec) -> Self {
        self.route_handler = handler.remote_handler;
        self
    }

    /// Set the theme for this server.
    ///
    /// Accepts a `ThemeProvider` created by the `#[theme]` attribute macro.
    /// The theme is used for:
    /// 1. Default CSS variables in the capsule `<head>` (FOUC prevention)
    /// 2. A built-in synced renderer that outputs a `<style>` element
    /// 3. Per-connection `Theme` state that handlers can mutate via `&mut Theme`
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[theme]
    /// fn app_theme() -> Theme {
    ///     Theme::dark().accent("#5E81AC")
    /// }
    ///
    /// Server::bind("0.0.0.0:9000")?
    ///     .root(app)
    ///     .theme(app_theme)
    ///     .run().await
    /// ```
    pub fn theme(mut self, provider: ThemeProvider) -> Self {
        self.theme_provider = Some(provider);
        self
    }

    /// Register a router that drives the [`outlet`](crate::router::outlet) runtime.
    ///
    /// On every navigation, the framework updates the built-in `CurrentRoute` state and the
    /// `outlet()` in your shell renders the matched view. Place an `outlet()` in the tree passed
    /// to [`root`](Self::root) — installing a router without one leaves navigation with nothing
    /// to re-render. This is an alternative to the `on_route` + root-rerender model; use one or
    /// the other, not both. (Capsule contents are not affected: names and CSS stream lazily over
    /// the wire regardless of routes.)
    pub fn routes(mut self, router: crate::router::Router) -> Self {
        self.router = Some(router);
        self
    }

    /// Get the shared server state, creating it if needed.
    ///
    /// Call this before `run()` to get a reference to the shared state for
    /// hydration and persistence setup. The same instance will be used when
    /// `run()` is called.
    pub fn shared_state(&mut self) -> Arc<SharedServerState> {
        self.shared
            .get_or_insert_with(|| SharedServerState::new(self.persist_interval))
            .clone()
    }

    /// Run the server, accepting connections until shutdown.
    pub async fn run(self) -> Result<(), Box<dyn Error>> {
        let listener = bind_reusable(self.addr)?;

        // Use provided shared state or create new
        let shared = self
            .shared
            .unwrap_or_else(|| SharedServerState::new(self.persist_interval));

        // Install the app's router (if any) so each connection's `outlet()` renders the
        // matched view, and the startup analysis below tree-shakes the "/" view.
        if let Some(router) = self.router {
            crate::router::install_router(std::sync::Arc::new(router));
        }

        // Pre-analyze the root element to determine used types for tree shaking
        let root_element = (self.root)();
        let mut ctx = BuildContext::new();

        // Extract renderers and create default states for proper tree walking
        let renderers = extract_renderers(&root_element);
        let mut default_states: HashMap<TypeId, Box<dyn Any + Send + Sync>> = HashMap::new();
        for renderer in &renderers {
            default_states
                .entry(renderer.state_type_id())
                .or_insert_with(|| renderer.create_default_state());
        }

        // Analyze the root for tree-shaking. Scoped so the (non-Send) cache
        // read guard is dropped before any `.await`, keeping `run()`'s future
        // Send (it may be spawned).
        {
            let shared_cache_guard = shared
                .shared_cache
                .read()
                .unwrap_or_else(|e| e.into_inner());
            let mut states_map: HashMap<TypeId, &(dyn Any + Send + Sync)> = default_states
                .iter()
                .map(|(k, v)| (*k, v.as_ref()))
                .collect();
            // Shared state may have been primed before run(); analyze the real
            // instance so its conditional branches (and their tokens) are walked
            // into the capsule rather than the empty default.
            for r in &renderers {
                if let Some(key) = shared_cache_key(r.storage_type(), r.table_name(), "") {
                    if let Some(state) = shared_cache_guard.get(&key) {
                        states_map.insert(r.state_type_id(), state.as_ref());
                    }
                }
            }

            if states_map.is_empty() {
                // No renderers - use simple collect_symbols with placeholder
                let placeholder: () = ();
                ctx.collect_symbols(&root_element, &placeholder);
                ctx.analyze_style_patterns(&root_element);
                ctx.emit(&root_element, &placeholder);
            } else {
                // Use multi-state collection for proper renderer handling
                ctx.collect_symbols_multi(&root_element, &states_map);
                ctx.analyze_style_patterns(&root_element);
                ctx.emit_multi(&root_element);
            }
        }

        // Resolve initial theme if provider is set
        let initial_theme = self.theme_provider.as_ref().map(|p| p.init());

        // Fold a builder-level base_path into the capsule config as the final step, so it wins
        // regardless of chain order and works with or without a prior `.capsule_config(...)`.
        let capsule_config = apply_base_path(self.capsule_config, self.base_path);

        // Generate capsule - styled if config provided, basic otherwise.
        // Also freeze PWA assets (manifest/sw/icons) keyed to the capsule's hash.
        let (capsule, pwa_assets) = if let Some(config) = capsule_config {
            // If theme provider is set, override config theme with initial theme
            let config = if let Some(ref theme) = initial_theme {
                config.theme(theme.clone())
            } else {
                config
            };

            // The capsule's static CSS only needs composite classes + globals;
            // utility/pseudo/breakpoint rules (.u/.h/.b) are delivered lazily over
            // the wire (STYLE_DEF), and the small u8 enum maps are shipped whole.
            // So only the composite table feeds the config (client actions ride inside the bundle).
            let composite_css = ctx.composite_table().generate_css();
            let config = config.with_composite_css(composite_css);

            // Generate CSS and embed in capsule HTML <style> tag.
            let css = capsule_gen::generate_capsule_css(&config);
            let capsule = capsule_gen::generate_styled_capsule(&config, &css);

            // PWA: version the service-worker cache by the capsule's hash so a new
            // build invalidates the old shell. The head tags are already inlined.
            let pwa_assets = config.pwa.as_ref().map(|p| {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                capsule.hash(&mut h);
                Arc::new(p.freeze_served(&config.theme, h.finish()))
            });
            (capsule, pwa_assets)
        } else {
            (capsule_gen::generate_capsule(), None)
        };

        let capsule_size = capsule.len();
        let capsule = Arc::new(capsule);
        let composite_table = Arc::new(ctx.composite_table().clone());

        println!("Server listening on http://{}", self.addr);
        println!("Capsule: {capsule_size} bytes");

        let root = Arc::new(self.root);
        let route_handler = self.route_handler.map(Arc::new);
        let initial_theme = initial_theme.map(Arc::new);
        let auth = Arc::new(self.auth);
        if auth.is_some() {
            println!("Auth: HTTP Basic enabled");
        }

        // Admission control: a shared registry tracks live connections so the
        // accept loop can reject over-limit clients (total + per-IP) before doing
        // any per-connection work, and the /health and /ready endpoints can report.
        let registry = Arc::new(ConnectionRegistry::new());
        let config = Arc::new(self.config);
        // Prometheus metrics, exported at GET /metrics.
        let metrics = Arc::new(Metrics::new());
        println!(
            "Limits: {} total, {} per IP",
            config.max_connections, config.max_connections_per_ip
        );

        // Spawn session eviction task (5-minute TTL)
        {
            let shared = Arc::clone(&shared);
            task::spawn(session_eviction_task(shared, Duration::from_secs(300)));
        }

        while let Ok((stream, peer_addr)) = listener.accept().await {
            let root = Arc::clone(&root);
            let capsule = Arc::clone(&capsule);
            let shared = Arc::clone(&shared);
            let route_handler = route_handler.clone();
            let initial_theme = initial_theme.clone();
            let composite_table = Arc::clone(&composite_table);
            let auth = Arc::clone(&auth);
            let registry = Arc::clone(&registry);
            let config = Arc::clone(&config);
            let metrics = Arc::clone(&metrics);
            let pwa_assets = pwa_assets.clone();
            task::spawn(async move {
                handle_client(
                    stream,
                    peer_addr,
                    root,
                    capsule,
                    shared,
                    route_handler,
                    initial_theme,
                    composite_table,
                    auth,
                    registry,
                    config,
                    metrics,
                    pwa_assets,
                )
                .await;
            });
        }

        Ok(())
    }
}

/// Bind a TCP listener with `SO_REUSEADDR` set.
///
/// Without this, a quick restart fails with "Address already in use" while
/// sockets from the previous process linger in `TIME_WAIT`. Returns a
/// non-blocking async-std listener.
fn bind_reusable(addr: SocketAddr) -> std::io::Result<TcpListener> {
    use socket2::{Domain, Protocol, Socket, Type};

    let domain = if addr.is_ipv4() {
        Domain::IPV4
    } else {
        Domain::IPV6
    };
    let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;
    socket.set_reuse_address(true)?;
    socket.bind(&addr.into())?;
    // Backlog sized for connection bursts; the OS clamps to its own max.
    socket.listen(1024)?;

    let std_listener: std::net::TcpListener = socket.into();
    std_listener.set_nonblocking(true)?;
    Ok(TcpListener::from(std_listener))
}

/// Auth session cookie name.
const AUTH_COOKIE: &str = "rwire_auth";
/// How long an issued login session stays valid.
const AUTH_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Form-login auth gate: the expected credential plus the set of issued,
/// unexpired session tokens (the cookie value clients present after logging in).
pub(crate) struct AuthGate {
    user: String,
    password: String,
    /// Optional brand/title shown on the login form.
    brand: Option<String>,
    /// A stable development session token. When set, it's accepted regardless of the in-memory
    /// token map (which resets on restart), so a rebuild doesn't log an open tab out. See
    /// [`ServerWithRoot::dev_session`].
    dev_token: Option<String>,
    tokens: std::sync::Mutex<HashMap<String, std::time::Instant>>,
}

impl AuthGate {
    fn new(user: String, password: String) -> Self {
        Self {
            user,
            password,
            brand: None,
            dev_token: None,
            tokens: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Whether the request carries a valid, unexpired session cookie.
    fn has_session(&self, request: &str) -> bool {
        let Some(token) = cookie_value(request, AUTH_COOKIE) else {
            return false;
        };
        // A configured dev session token is always valid: it isn't kept in the in-memory map, so
        // it survives a server restart and an open tab stays logged in across a rebuild.
        if self.dev_token.as_deref() == Some(token.as_str()) {
            return true;
        }
        let mut tokens = self
            .tokens
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        tokens.retain(|_, issued| issued.elapsed() < AUTH_TTL);
        tokens.contains_key(&token)
    }

    /// Validate posted credentials; on success, issue and store a session token.
    /// Both field comparisons run regardless of the first, so timing does not
    /// reveal which field was wrong.
    fn login(&self, body: &str) -> Option<String> {
        let mut user = None;
        let mut password = None;
        for pair in body.split('&') {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            match key {
                "username" => user = Some(url_decode(value)),
                "password" => password = Some(url_decode(value)),
                _ => {}
            }
        }
        let user_ok = ct_eq(user?.as_bytes(), self.user.as_bytes());
        let pass_ok = ct_eq(password?.as_bytes(), self.password.as_bytes());
        if user_ok & pass_ok {
            // In dev-session mode, issue the stable token so the cookie keeps working across
            // restarts; otherwise a fresh random token tracked in memory.
            let token = self.dev_token.clone().unwrap_or_else(generate_token);
            self.tokens
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert(token.clone(), std::time::Instant::now());
            Some(token)
        } else {
            None
        }
    }

    /// Invalidate the session presented by this request, if any.
    fn logout(&self, request: &str) {
        if let Some(token) = cookie_value(request, AUTH_COOKIE) {
            self.tokens
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&token);
        }
    }
}

/// Constant-time byte equality (avoids leaking the credential via timing).
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Extract a cookie value by name from a raw HTTP request.
fn cookie_value(request: &str, name: &str) -> Option<String> {
    let header = extract_cookie_from_request(request)?;
    for part in header.split(';') {
        if let Some(rest) = part.trim().strip_prefix(name) {
            if let Some(value) = rest.strip_prefix('=') {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

/// Percent-decode an `application/x-www-form-urlencoded` field value.
fn url_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = char::from(bytes[i + 1]).to_digit(16);
                let lo = char::from(bytes[i + 2]).to_digit(16);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    out.push((hi * 16 + lo) as u8);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Mint a 256-bit random session token (hex) from the OS CSPRNG. Falls back to a
/// weaker time-based token only if `/dev/urandom` is unavailable.
fn generate_token() -> String {
    use std::fmt::Write as _;
    use std::io::Read as _;
    let mut buf = [0u8; 32];
    if std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut buf))
        .is_ok()
    {
        let mut token = String::with_capacity(64);
        for byte in buf {
            let _ = write!(token, "{byte:02x}");
        }
        token
    } else {
        crate::session::SessionId::generate().as_str().to_string()
    }
}

/// Parse the method and path from the request line (`"GET /path HTTP/1.1"`).
fn request_line(request: &str) -> (&str, &str) {
    let line = request.lines().next().unwrap_or("");
    let mut parts = line.split(' ');
    (parts.next().unwrap_or(""), parts.next().unwrap_or(""))
}

/// Proxy a matched request to a local upstream port (P2), streaming both directions until either
/// side closes. `head` is the client's already-drained request head; its first line is rewritten to
/// strip `prefix` so the upstream sees a root-relative path. A WebSocket upgrade needs no special
/// handling — once the head is forwarded, frames are just bytes the pump copies each way.
async fn serve_proxy(mut stream: TcpStream, head: Vec<u8>, port: u16, prefix: &str) {
    let mut upstream = match TcpStream::connect(("127.0.0.1", port)).await {
        Ok(upstream) => upstream,
        Err(_) => {
            let _ = crate::health::serve_unavailable(stream, "preview_unreachable").await;
            return;
        }
    };
    let rewritten = rewrite_proxy_head(&head, prefix);
    if upstream.write_all(&rewritten).await.is_err() {
        return;
    }
    let _ = upstream.flush().await;

    // Pump both directions concurrently; the first to reach EOF (upstream closing after an HTTP
    // response, or either side closing a WebSocket) ends the exchange. The inner scope drops both
    // copy futures — releasing their borrows on `stream` — before the final flush.
    {
        let mut client_read = stream.clone();
        let mut upstream_write = upstream.clone();
        let to_upstream = futures::io::copy(&mut client_read, &mut upstream_write);
        let to_client = futures::io::copy(&mut upstream, &mut stream);
        futures::pin_mut!(to_upstream, to_client);
        let _ = futures::future::select(to_upstream, to_client).await;
    }
    let _ = stream.flush().await;
}

/// Rewrite a proxied request's first line to strip the `prefix` path segment, so the upstream app
/// sees a root-relative path (`/preview/<id>/foo` → `/foo`; the bare prefix → `/`). All other bytes
/// (headers, any body already read) are preserved verbatim; a malformed head is passed through.
fn rewrite_proxy_head(head: &[u8], prefix: &str) -> Vec<u8> {
    let Some(eol) = head.windows(2).position(|w| w == b"\r\n") else {
        return head.to_vec();
    };
    let line = String::from_utf8_lossy(&head[..eol]);
    let mut parts = line.splitn(3, ' ');
    let (Some(method), Some(path), Some(version)) = (parts.next(), parts.next(), parts.next())
    else {
        return head.to_vec();
    };
    let stripped = path.strip_prefix(prefix).unwrap_or(path);
    let new_path = if stripped.is_empty() { "/" } else { stripped };
    let mut out = format!("{method} {new_path} {version}").into_bytes();
    out.extend_from_slice(&head[eol..]);
    out
}

/// Return the body that follows the header terminator, if present.
fn request_body(request: &str) -> &str {
    request.split_once("\r\n\r\n").map_or("", |(_, body)| body)
}

/// Extract Cookie header value from HTTP request.
/// Case-insensitive single-header lookup in a raw request head.
fn header_value(request: &str, name: &str) -> Option<String> {
    for line in request.lines().skip(1) {
        if let Some((n, v)) = line.split_once(':') {
            if n.trim().eq_ignore_ascii_case(name) {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

/// Host[:port] equality with default ports (80/443) stripped, case-insensitive.
fn host_eq(a: &str, b: &str) -> bool {
    let norm = |h: &str| {
        let h = h.trim().to_ascii_lowercase();
        h.trim_end_matches(":443")
            .trim_end_matches(":80")
            .to_string()
    };
    !a.trim().is_empty() && norm(a) == norm(b)
}

/// Whether a WebSocket handshake's `Origin` may connect (T2, CSWSH defense).
///
/// Same-origin (the Origin's host[:port] matching the request `Host`) always
/// passes; anything else must be in `allowed` (full origin strings). Requests
/// WITHOUT an Origin header are not routed here — non-browser clients don't
/// send one, and browsers always do.
fn origin_allowed(origin: &str, host: Option<&str>, allowed: &[String]) -> bool {
    let origin = origin.trim().trim_end_matches('/');
    if allowed
        .iter()
        .any(|a| a.trim_end_matches('/').eq_ignore_ascii_case(origin))
    {
        return true;
    }
    let origin_lc = origin.to_ascii_lowercase();
    let origin_host = origin_lc
        .strip_prefix("https://")
        .or_else(|| origin_lc.strip_prefix("http://"))
        .unwrap_or("");
    match host {
        Some(h) => host_eq(origin_host, h),
        None => false,
    }
}

fn extract_cookie_from_request(request: &str) -> Option<String> {
    for line in request.lines() {
        if line.len() >= 7 && line[..7].eq_ignore_ascii_case("cookie:") {
            return Some(line[7..].trim().to_string());
        }
    }
    None
}

/// True if the client-facing connection is HTTPS, per the `X-Forwarded-Proto`
/// header a TLS-terminating proxy sets.
///
/// rwire serves plain HTTP itself, so this header is the signal used to decide
/// whether the session cookie should carry `Secure` (the cookie must not be
/// `Secure` over plain HTTP or the browser drops it). A proxy chain may send a
/// comma-separated list; the first (original client) value is authoritative.
fn forwarded_https(request: &str) -> bool {
    for line in request.lines() {
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("x-forwarded-proto") {
                let proto = value.split(',').next().unwrap_or("").trim();
                return proto.eq_ignore_ascii_case("https");
            }
        }
    }
    false
}

/// Build the capsule config used to render the static login page: the app theme
/// (for `:root` vars) plus the composite classes, so the login uses the design
/// system. Cheap enough to build per login-page render (a rare request).
fn login_capsule_config(
    initial_theme: &Option<Arc<crate::theme::Theme>>,
    composite_table: &crate::style_groups::CompositeTable,
) -> CapsuleConfig {
    let mut config = CapsuleConfig::new();
    if let Some(theme) = initial_theme {
        config = config.theme((**theme).clone());
    }
    config.with_composite_css(composite_table.generate_css())
}

#[allow(clippy::too_many_arguments)]
async fn handle_client<F>(
    mut stream: TcpStream,
    peer_addr: SocketAddr,
    root: Arc<F>,
    capsule: Arc<String>,
    shared: Arc<SharedServerState>,
    route_handler: Option<Arc<HandlerFn>>,
    initial_theme: Option<Arc<crate::theme::Theme>>,
    composite_table: Arc<crate::style_groups::CompositeTable>,
    auth: Arc<Option<AuthGate>>,
    registry: Arc<ConnectionRegistry>,
    config: Arc<ServerConfig>,
    metrics: Arc<Metrics>,
    pwa_assets: Option<Arc<crate::pwa::PwaAssets>>,
) where
    F: Fn() -> ElementBuilder + Send + Sync + 'static,
{
    // Peek at the first bytes to detect request type
    let mut peek_buf = [0u8; 4096];
    let n = match stream.peek(&mut peek_buf).await {
        Ok(n) => n,
        Err(e) => {
            eprintln!("[{}] Failed to peek: {}", peer_addr, e);
            return;
        }
    };

    let peek_str = String::from_utf8_lossy(&peek_buf[..n]);

    // Health/readiness probes: answered before auth and admission so load
    // balancers and orchestrators can reach them without a session and even
    // while the server is at capacity.
    if peek_str.starts_with("GET ") {
        let (_, path) = request_line(&peek_str);
        if path == "/health" || path == "/ready" || path == "/metrics" {
            let mut drain = vec![0u8; n];
            let _ = stream.read_exact(&mut drain).await;
            let max = config.max_connections;
            let result = match path {
                "/health" => crate::health::serve_health(stream, &registry, max).await,
                "/ready" => crate::health::serve_ready(stream, &registry, max).await,
                _ => {
                    // Reflect the registry's live count into the gauge, then export.
                    metrics
                        .active_connections
                        .set(registry.total_connections() as u64);
                    crate::health::serve_metrics(stream, &metrics.to_prometheus()).await
                }
            };
            if let Err(e) = result {
                eprintln!("[{}] {} endpoint error: {}", peer_addr, path, e);
            }
            return;
        }
    }

    // PWA assets (manifest, service worker, icons): served before auth so the
    // browser can fetch them to evaluate installability without a session.
    if let Some(pwa) = &pwa_assets {
        if peek_str.starts_with("GET ") {
            let (_, path) = request_line(&peek_str);
            let served: Option<(&str, &[u8])> = match path {
                "/manifest.webmanifest" => {
                    Some(("application/manifest+json", pwa.manifest.as_bytes()))
                }
                "/sw.js" => Some(("text/javascript", pwa.service_worker.as_bytes())),
                p => pwa
                    .icons
                    .iter()
                    .find(|(ip, _, _)| ip == p)
                    .map(|(_, mime, bytes)| (*mime, bytes.as_ref())),
            };
            if let Some((ctype, bytes)) = served {
                let mut drain = vec![0u8; n];
                let _ = stream.read_exact(&mut drain).await;
                if let Err(e) = crate::health::serve_static(stream, ctype, bytes).await {
                    eprintln!("[{}] {} error: {}", peer_addr, path, e);
                }
                return;
            }
        }
    }

    // Auth gate (page + WebSocket): handle the login lifecycle and reject any
    // request that lacks a valid session cookie.
    if let Some(gate) = auth.as_ref() {
        let (method, path) = request_line(&peek_str);
        let is_ws = capsule::is_websocket_upgrade(&peek_str);

        if method == "POST" && path == "/login" {
            // Drain the request (headers + small form body) before responding.
            let mut buf = vec![0u8; n];
            let _ = stream.read_exact(&mut buf).await;
            let request = String::from_utf8_lossy(&buf);
            if let Some(token) = gate.login(request_body(&request)) {
                println!("[{peer_addr}] login OK");
                let cookie = format!(
                    "{AUTH_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
                    AUTH_TTL.as_secs()
                );
                let _ = capsule::serve_redirect(stream, "/", Some(&cookie)).await;
            } else {
                println!("[{peer_addr}] login failed");
                let config = login_capsule_config(&initial_theme, &composite_table);
                let _ = capsule::serve_login(stream, true, gate.brand.as_deref(), &config).await;
            }
            return;
        }

        if method == "GET" && path == "/logout" {
            gate.logout(&peek_str);
            let mut buf = vec![0u8; n];
            let _ = stream.read_exact(&mut buf).await;
            let expired = format!("{AUTH_COOKIE}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0");
            let _ = capsule::serve_redirect(stream, "/", Some(&expired)).await;
            return;
        }

        if !gate.has_session(&peek_str) {
            if is_ws {
                // Expected and benign: a still-open tab from a previous run (tokens are
                // in-memory and reset on restart, or expired) auto-reconnects its socket.
                // The client probes and reloads to the login form on repeated rejects,
                // so this isn't logged to avoid noise.
                let _ = capsule::serve_unauthorized(stream).await;
            } else {
                let mut buf = vec![0u8; n];
                let _ = stream.read_exact(&mut buf).await;
                let config = login_capsule_config(&initial_theme, &composite_table);
                let _ = capsule::serve_login(stream, false, gate.brand.as_deref(), &config).await;
            }
            return;
        }
        // Valid session: fall through to normal capsule/WebSocket handling.
    }

    // Reverse-proxy gate (after auth, so previews inherit the gate): a matched path forwards to a
    // pooled local port, streaming both directions — a WebSocket upgrade rides through as raw bytes.
    if let Some(resolver) = config.proxy.as_ref() {
        let (_, path) = request_line(&peek_str);
        if let Some((port, prefix)) = resolver.resolve(path) {
            let mut head = vec![0u8; n];
            if stream.read_exact(&mut head).await.is_ok() {
                serve_proxy(stream, head, port, &prefix).await;
            }
            return;
        }
    }

    // Extract the session ID from the cookie, but only trust it if it has the
    // exact format we mint (32 hex chars). A missing, malformed, or crafted value
    // (e.g. one containing `:` to confuse the persisted-state cache key, or an
    // attempt at session fixation) yields a fresh server-generated id instead.
    let (session_id, is_new_session) = match extract_cookie_from_request(&peek_str)
        .and_then(|c| SessionId::from_cookie(&c))
        .filter(|sid| sid.is_valid_format())
    {
        Some(sid) => {
            println!("[{}] Found session: {}", peer_addr, sid);
            (sid, false)
        }
        None => {
            let sid = SessionId::generate();
            println!("[{}] New session: {}", peer_addr, sid);
            (sid, true)
        }
    };

    // Check if this is a WebSocket upgrade request
    if capsule::is_websocket_upgrade(&peek_str) {
        // Origin gate (CSWSH defense): a browser handshake carries an Origin
        // header; reject it unless same-origin with the request Host or in the
        // configured allowlist. Origin-less (non-browser) handshakes pass.
        if let Some(origin) = header_value(&peek_str, "origin") {
            let host = header_value(&peek_str, "host");
            if !origin_allowed(&origin, host.as_deref(), &config.allowed_origins) {
                println!(
                    "[{}] WebSocket rejected: cross_origin ({})",
                    peer_addr, origin
                );
                metrics.connections_rejected.inc();
                let _ = crate::health::serve_forbidden(stream, "cross_origin").await;
                return;
            }
        }
        // Admission control: enforce total and per-IP connection caps before
        // spawning the (long-lived, stateful) WebSocket session. Rejected clients
        // get a 503 instead of an upgrade.
        let ip = peer_addr.ip();
        if let Err(reason) =
            registry.check_admission(ip, config.max_connections, config.max_connections_per_ip)
        {
            let why = match reason {
                AdmissionError::AtCapacity => "at_capacity",
                AdmissionError::TooManyFromIp => "too_many_from_ip",
            };
            println!("[{}] WebSocket rejected: {}", peer_addr, why);
            metrics.connections_rejected.inc();
            let _ = crate::health::serve_unavailable(stream, why).await;
            return;
        }
        // Held for the lifetime of the connection; the guard decrements the
        // registry counts on drop (normal close, error, or panic).
        let _conn_guard = registry.register(ip);
        metrics.connections_total.inc();

        println!("[{}] WebSocket connection", peer_addr);
        // Bound incoming message/frame size so a malicious client can't make the
        // WebSocket layer buffer a huge frame before our 64KB protocol decode runs.
        // These limit reads only (server→client DOM messages are unaffected).
        let ws_config = WebSocketConfig {
            max_message_size: Some(MAX_WS_MESSAGE_SIZE),
            max_frame_size: Some(MAX_WS_FRAME_SIZE),
            ..Default::default()
        };
        match accept_async_with_config(stream, Some(ws_config)).await {
            Ok(ws_stream) => {
                if let Err(e) = handle_websocket(
                    ws_stream,
                    peer_addr,
                    ConnContext {
                        root,
                        shared,
                        session_id,
                        route_handler,
                        initial_theme,
                        composite_table,
                        metrics,
                    },
                )
                .await
                {
                    eprintln!("[{}] Connection error: {}", peer_addr, e);
                }
            }
            Err(e) => {
                eprintln!("[{}] WebSocket handshake failed: {}", peer_addr, e);
            }
        }
        println!("[{}] WebSocket closed", peer_addr);
    } else if peek_str.starts_with("GET ") {
        // Consume the request data first
        let mut request_buf = vec![0u8; n];
        if let Err(e) = stream.read_exact(&mut request_buf).await {
            eprintln!("[{}] Failed to read request: {}", peer_addr, e);
            return;
        }

        // Serve capsule HTML (CSS is embedded in <style> tag)
        println!("[{}] HTTP request - serving capsule", peer_addr);
        // PWA installs need a secure context. Warn once if a non-loopback client is
        // reaching us over plain HTTP while PWA is enabled (localhost is exempt — it's
        // a secure context for installability).
        if pwa_assets.is_some() && !forwarded_https(&peek_str) && !peer_addr.ip().is_loopback() {
            static PWA_TLS_WARNED: std::sync::atomic::AtomicBool =
                std::sync::atomic::AtomicBool::new(false);
            if !PWA_TLS_WARNED.swap(true, Ordering::Relaxed) {
                eprintln!(
                    "[pwa] reachable over plain HTTP from a non-loopback client — installs \
                     require HTTPS. Terminate TLS at a proxy and forward 'X-Forwarded-Proto: https'."
                );
            }
        }
        // Mark the session cookie `Secure` when the client-facing connection is
        // HTTPS (auto-detected from the proxy's X-Forwarded-Proto), or when the
        // config forces it on. Off for plain-HTTP dev so the cookie isn't dropped.
        let secure_cookie = config.secure_cookies || forwarded_https(&peek_str);
        if let Err(e) = capsule::serve(
            stream,
            &capsule,
            Some(&session_id),
            is_new_session,
            secure_cookie,
        )
        .await
        {
            eprintln!("[{}] Failed to serve capsule: {}", peer_addr, e);
        }
    } else {
        eprintln!("[{}] Unknown request type", peer_addr);
    }
}

/// Per-connection state container supporting multiple state types.
struct ConnectionState {
    /// Unique connection ID for this connection.
    connection_id: u64,
    /// Session ID for this connection (used for persisted state keying).
    session_id: String,
    /// State values keyed by TypeId, supporting multiple state types per connection.
    /// Note: For persisted state, the authoritative copy is in SharedServerState.
    states: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    /// Registered event handlers, keyed by stable handler id (grows only with
    /// distinct handlers; reset to the full set on each full render).
    handlers: HashMap<u32, HandlerFn>,
    /// Synced elements that need to re-render on state change.
    synced_elements: Vec<SyncedElement>,
    /// Symbols that have been sent to this client (for incremental symbol updates).
    /// Maps symbol string -> symbol index (0x80+). Uses u32 for varint encoding.
    sent_symbols: HashMap<String, u32>,
    /// Keys this connection is subscribed to (for cleanup on disconnect).
    subscribed_keys: HashSet<String>,
    /// Content hashes of last-sent synced element renders (for render dedup).
    /// Maps synced element ID -> content hash. If identical, skip re-emission.
    synced_hashes: HashMap<u32, u64>,
    /// Class-referenced CSS rules already delivered to this client (lazy CSS).
    /// Each rule is sent once via `STYLE_DEF` the first time it is referenced.
    sent_css: HashSet<crate::style_tokens::StyleKey>,
    /// `(category, code)` name-map entries already delivered to this client (lazy names).
    /// Each is sent once via `MAP_DEF` the first time its code is referenced.
    sent_maps: HashSet<(u8, u8)>,
    /// Target/selector slot indices assigned during the initial render. Synced
    /// updates reuse these to re-bind regenerated elements to the client-side
    /// action state set up by the initial DOM's INIT_TARGET/INIT_SELECTOR opcodes.
    client_actions: ClientActionIndices,
    /// Token-bucket allowance for inbound events (rate limiting). Refilled lazily
    /// based on elapsed time; each processed event/route message consumes one token.
    event_tokens: f64,
    /// When `event_tokens` was last refilled.
    last_event_refill: Instant,
}

impl ConnectionState {
    fn new(connection_id: u64, session_id: SessionId) -> Self {
        Self {
            connection_id,
            session_id: session_id.as_str().to_string(),
            states: HashMap::new(),
            handlers: HashMap::new(),
            synced_elements: Vec::new(),
            sent_symbols: HashMap::new(),
            subscribed_keys: HashSet::new(),
            synced_hashes: HashMap::new(),
            sent_css: HashSet::new(),
            sent_maps: HashSet::new(),
            client_actions: ClientActionIndices::default(),
            // Start with a full bucket so a fresh connection can burst.
            event_tokens: EVENT_BUCKET_CAPACITY,
            last_event_refill: Instant::now(),
        }
    }

    /// Token-bucket rate limit for inbound client messages.
    ///
    /// Returns true if the message is within budget (and consumes a token), false
    /// if it should be dropped. Refills `EVENT_REFILL_PER_SEC` tokens/second up to
    /// `EVENT_BUCKET_CAPACITY`, allowing brief human-speed bursts while capping a
    /// flood of well-formed events (each of which would otherwise run a handler,
    /// re-render synced regions, and possibly broadcast to every connection).
    fn allow_event(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_event_refill).as_secs_f64();
        self.last_event_refill = now;
        self.event_tokens =
            (self.event_tokens + elapsed * EVENT_REFILL_PER_SEC).min(EVENT_BUCKET_CAPACITY);
        if self.event_tokens >= 1.0 {
            self.event_tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Build a synced-update message for `changed_type` after `changes`,
    /// optionally injecting the authoritative shared/persisted copy keyed by
    /// `inject_key` (read from the shared cache). The broadcast path and the
    /// post-handler render paths differ only in that key, so they share this.
    fn build_type_update(
        &mut self,
        shared: &SharedServerState,
        changed_type: TypeId,
        changes: ChangeSet,
        inject_key: Option<&str>,
    ) -> Result<Bytes, Box<dyn Error + Send + Sync>> {
        // Only take the read lock when there's a shared/persisted copy to inject.
        let cache_guard = match inject_key {
            Some(_) => Some(
                shared
                    .shared_cache
                    .read()
                    .map_err(|_| "shared cache lock poisoned")?,
            ),
            None => None,
        };
        let mut states_map: HashMap<TypeId, &(dyn Any + Send + Sync)> =
            self.states.iter().map(|(k, v)| (*k, v.as_ref())).collect();
        if let (Some(key), Some(cache)) = (inject_key, &cache_guard) {
            if let Some(state) = cache.get(key) {
                states_map.insert(changed_type, state.as_ref());
            }
        }
        Ok(build_synced_update_with_known_symbols(
            &self.synced_elements,
            &states_map,
            &mut self.handlers,
            changes,
            Some(&mut self.sent_symbols),
            Some(changed_type),
            Some(&mut self.synced_hashes),
            Some(&mut self.sent_css),
            Some(&mut self.sent_maps),
            None,
            0,
            Some(&self.client_actions),
        ))
    }

    /// Run `handler` against its state. Shared/persisted state executes on (and is
    /// written back to) the shared cache, is marked dirty if persisted, broadcast to
    /// other subscribers, and subscribed to; memory state executes on the connection.
    /// Returns the cache key (Some for shared/persisted) so the caller can render
    /// from the copy the handler just wrote.
    fn dispatch_handler(
        &mut self,
        shared: &SharedServerState,
        handler: &HandlerFn,
        ctx: &EventContext,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        let state_type_id = handler.state_type_id();
        let cache_key = shared_cache_key(
            handler.storage_type(),
            handler.table_name(),
            &self.session_id,
        );
        if let Some(key) = &cache_key {
            // Shared/persisted: execute on the shared cache, then write-back side effects.
            let mut cache = shared
                .shared_cache
                .write()
                .map_err(|_| "shared cache lock poisoned")?;
            let state = cache
                .entry(key.clone())
                .or_insert_with(|| handler.create_state());
            handler.call_with_context(state.as_mut(), ctx);
            drop(cache);

            if handler.storage_type() == StorageType::Persisted {
                shared.mark_dirty(key);
            }
            shared.broadcast(key, state_type_id, handler.changes(), self.connection_id);
            if self.subscribed_keys.insert(key.clone()) {
                shared.subscribe(self.connection_id, key);
            }
        } else {
            // Memory state: connection-local.
            self.ensure_state_initialized_for(handler);
            if let Some(state) = self.get_state_mut(state_type_id) {
                handler.call_with_context(state, ctx);
            }
        }
        Ok(cache_key)
    }

    /// Swap the built-in router's outlet to `path`: update `CurrentRoute`, seed the
    /// matched view's renderer states, prune the previous view's regions, render the
    /// new view (ids floored above any still on the client), then register the new
    /// regions and subscribe any shared/persisted state they read. Returns the update
    /// bytes for the caller to send.
    fn render_route_view_swap(
        &mut self,
        shared: &SharedServerState,
        router: &crate::router::Router,
        path: &str,
    ) -> Result<Bytes, Box<dyn Error + Send + Sync>> {
        let route_tid = TypeId::of::<crate::router::CurrentRoute>();
        self.states
            .entry(route_tid)
            .or_insert_with(|| Box::new(crate::router::CurrentRoute::default()));
        if let Some(route) = self
            .get_state_mut(route_tid)
            .and_then(|s| s.downcast_mut::<crate::router::CurrentRoute>())
        {
            route.set_path(path);
        }

        // Initialize any state the matched view's renderers read, so the outlet can
        // render them inline. `or_insert` preserves state from a prior visit.
        let view = router.build_for_path(path);
        for r in crate::builder::extract_renderers(&view) {
            self.states
                .entry(r.state_type_id())
                .or_insert_with(|| r.create_default_state());
        }

        // Drop the render-hash dedup cache across a view swap: hashes exist only to skip
        // re-sending unchanged content, and after a swap the client's DOM for any reused
        // region id is not guaranteed to match what the hash claims was last sent (a lost
        // or partially-applied update would otherwise be pinned stale forever).
        self.synced_hashes.clear();

        // Prune the previous view's regions (every synced region descended from a
        // CurrentRoute region) so the new view's regions render fresh, not against
        // stale ids.
        let route_regions: HashSet<u32> = self
            .synced_elements
            .iter()
            .filter(|se| se.state_type_id == route_tid)
            .map(|se| se.id)
            .collect();
        let mut stale = HashSet::new();
        let mut frontier: Vec<u32> = route_regions.iter().copied().collect();
        while !frontier.is_empty() {
            let next: Vec<u32> = self
                .synced_elements
                .iter()
                .filter(|se| {
                    se.parent.is_some_and(|p| frontier.contains(&p)) && !stale.contains(&se.id)
                })
                .map(|se| se.id)
                .collect();
            for id in &next {
                stale.insert(*id);
            }
            frontier = next;
        }
        // Floor new ids above every id currently on the client (including the view
        // being pruned), so a swapped-in region never reuses a just-removed id (which
        // the client morph would preserve as a stale span). Computed before the retain.
        let synced_id_floor = self
            .synced_elements
            .iter()
            .map(|se| se.id)
            .max()
            .map(|m| m + 1)
            .unwrap_or(0);
        self.synced_elements.retain(|se| !stale.contains(&se.id));

        let mut discovered: Vec<crate::builder::SyncedElement> = Vec::new();
        let update = {
            let mut states_map: HashMap<TypeId, &(dyn Any + Send + Sync)> =
                self.states.iter().map(|(k, v)| (*k, v.as_ref())).collect();
            // `states` only holds defaults for shared state; the authoritative copy is
            // in the shared cache. Override before rendering so the swapped-in view
            // doesn't paint from empty defaults.
            let cache_guard = shared
                .shared_cache
                .read()
                .map_err(|_| "shared cache lock poisoned")?;
            for (tid, key) in
                shared_persisted_keys(&self.handlers, &self.synced_elements, &self.session_id)
            {
                if let Some(state) = cache_guard.get(&key) {
                    states_map.insert(tid, state.as_ref());
                }
            }
            build_synced_update_with_known_symbols(
                &self.synced_elements,
                &states_map,
                &mut self.handlers,
                ChangeSet::all(),
                Some(&mut self.sent_symbols),
                Some(route_tid),
                Some(&mut self.synced_hashes),
                Some(&mut self.sent_css),
                Some(&mut self.sent_maps),
                Some(&mut discovered),
                synced_id_floor,
                Some(&self.client_actions),
            )
        };

        // Register the new view's regions so later state changes re-render them, and
        // subscribe any shared/persisted state they read.
        for region in discovered {
            if !self.synced_elements.iter().any(|se| se.id == region.id) {
                self.synced_elements.push(region);
            }
        }
        for (_, key) in
            shared_persisted_keys(&self.handlers, &self.synced_elements, &self.session_id)
        {
            if self.subscribed_keys.insert(key.clone()) {
                shared.subscribe(self.connection_id, &key);
            }
        }
        Ok(update)
    }

    /// Render the full initial DOM for this connection: render every region with the
    /// authoritative state (memory from the connection, shared/persisted from the
    /// cache), reusing the startup composite table so composite ids match the capsule
    /// CSS, then capture handlers/synced/symbols/client-actions and prepend lazy CSS.
    fn render_initial_dom(
        &mut self,
        shared: &SharedServerState,
        root_element: &ElementBuilder,
        composite_table: &crate::style_groups::CompositeTable,
    ) -> Result<Bytes, Box<dyn Error + Send + Sync>> {
        let mut ctx = BuildContext::new();

        let cache_guard = shared
            .shared_cache
            .read()
            .map_err(|_| "shared cache lock poisoned")?;
        let mut states_map: HashMap<TypeId, &(dyn Any + Send + Sync)> =
            self.states.iter().map(|(k, v)| (*k, v.as_ref())).collect();
        // Override memory defaults with the shared/persisted instances from the cache.
        for (tid, key) in
            shared_persisted_keys(&self.handlers, &self.synced_elements, &self.session_id)
        {
            if let Some(state) = cache_guard.get(&key) {
                states_map.insert(tid, state.as_ref());
            }
        }

        // Reuse the startup composite table so composite ids match the CSS baked into
        // the capsule (re-analyzing different DOM state would reassign ids).
        ctx.set_composite_table(composite_table.clone());
        if states_map.is_empty() {
            let placeholder: () = ();
            ctx.collect_symbols(root_element, &placeholder);
            ctx.emit(root_element, &placeholder);
        } else {
            ctx.collect_symbols_multi(root_element, &states_map);
            ctx.emit_multi(root_element);
        }
        drop(cache_guard);

        self.handlers = ctx.handlers().clone();
        self.synced_elements = ctx.take_synced_elements();
        self.sent_symbols = ctx.take_symbol_map();
        self.client_actions = ctx.client_action_indices();
        // Prepend STYLE_DEF for the styles this render uses (lazy CSS): the capsule
        // ships only global CSS; class rules arrive over the wire.
        Ok(ctx.finish_with_style_defs(&mut self.sent_css, &mut self.sent_maps))
    }

    /// Ensure state of a given type is initialized using the handler's factory.
    fn ensure_state_initialized_for(&mut self, handler: &HandlerFn) {
        let type_id = handler.state_type_id();
        self.states
            .entry(type_id)
            .or_insert_with(|| handler.create_state());
    }

    /// Initialize all states from the registered handlers and synced elements.
    fn initialize_all_states(&mut self) {
        // Collect unique state types from handlers
        let handlers: Vec<HandlerFn> = self.handlers.values().cloned().collect();
        for handler in &handlers {
            self.ensure_state_initialized_for(handler);
        }
        // Also initialize states from synced element renderers
        // These states might not have handlers but need to exist for rendering
        for synced in &self.synced_elements {
            let type_id = synced.state_type_id;
            self.states
                .entry(type_id)
                .or_insert_with(|| synced.create_default_state());
        }
    }

    /// Get mutable state by TypeId for type-erased access.
    fn get_state_mut(&mut self, type_id: TypeId) -> Option<&mut (dyn Any + Send + Sync)> {
        self.states.get_mut(&type_id).map(|s| s.as_mut())
    }

    /// Take all states out of this connection (for caching on disconnect).
    fn take_states(&mut self) -> HashMap<TypeId, Box<dyn Any + Send + Sync>> {
        std::mem::take(&mut self.states)
    }

    /// Restore states from a cached session.
    fn restore_states(&mut self, cached: HashMap<TypeId, Box<dyn Any + Send + Sync>>) {
        self.states = cached;
    }

    /// Initialize missing states (fill gaps for types not in the cache).
    fn initialize_missing_states(&mut self) {
        let handlers: Vec<HandlerFn> = self.handlers.values().cloned().collect();
        for handler in &handlers {
            let type_id = handler.state_type_id();
            self.states
                .entry(type_id)
                .or_insert_with(|| handler.create_state());
        }
        for synced in &self.synced_elements {
            let type_id = synced.state_type_id;
            self.states
                .entry(type_id)
                .or_insert_with(|| synced.create_default_state());
        }
    }
}

/// RAII guard that unregisters a connection when dropped.
///
/// Ensures cleanup happens even if `handle_websocket` panics.
struct ConnectionGuard {
    shared: Arc<SharedServerState>,
    connection_id: u64,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.shared.unregister_connection(self.connection_id);
    }
}

/// Per-connection context for [`handle_websocket`]: the app root and shared state
/// plus the resolved appearance/routing config, cloned per connection in `run()`.
struct ConnContext<F> {
    root: Arc<F>,
    shared: Arc<SharedServerState>,
    session_id: SessionId,
    route_handler: Option<Arc<HandlerFn>>,
    initial_theme: Option<Arc<crate::theme::Theme>>,
    composite_table: Arc<crate::style_groups::CompositeTable>,
    metrics: Arc<Metrics>,
}

async fn handle_websocket<F>(
    ws_stream: async_tungstenite::WebSocketStream<TcpStream>,
    peer_addr: SocketAddr,
    cx: ConnContext<F>,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    F: Fn() -> ElementBuilder + Send + Sync + 'static,
{
    let ConnContext {
        root,
        shared,
        session_id,
        route_handler,
        initial_theme,
        composite_table,
        metrics,
    } = cx;
    let (mut write, mut read) = ws_stream.split();

    // Allocate connection ID and register broadcast channel. The receiver is
    // consumed in the main loop so background state changes (and cross-tab
    // persisted updates) re-render and push diffs without a client event.
    let connection_id = shared.next_connection_id();
    let (broadcast_tx, broadcast_rx) =
        async_channel::bounded::<BroadcastMsg>(shared.broadcast_capacity);
    shared.register_connection(connection_id, broadcast_tx);

    // RAII guard ensures cleanup on drop (even on panic)
    let _cleanup = ConnectionGuard {
        shared: Arc::clone(&shared),
        connection_id,
    };

    // Create per-connection state with the session ID from cookie
    let mut conn_state = ConnectionState::new(connection_id, session_id);

    // Build the root element, wrapping with theme synced region if theme is configured.
    // Theme synced builder must be a sibling of the root (not a child), because
    // synced elements clear their children on re-render — a child synced element
    // would be destroyed when the parent re-renders.
    let root_element = if initial_theme.is_some() {
        use crate::builder::el;
        use crate::protocol::El;
        use crate::theme::theme_synced_builder;
        el(El::Div).append([root(), theme_synced_builder()])
    } else {
        root()
    };

    // First pass: collect handlers to find the state types
    let mut ctx = BuildContext::new();

    // Use a temporary unit state for the first pass to collect handlers
    let placeholder_state: () = ();
    ctx.collect_symbols(&root_element, &placeholder_state);
    ctx.emit(&root_element, &placeholder_state);

    // Extract handlers
    conn_state.handlers = ctx.handlers().clone();
    conn_state.synced_elements = ctx.take_synced_elements();

    // Pre-populate theme state with initial value (before state initialization)
    if let Some(ref theme) = initial_theme {
        use crate::theme::Theme;
        conn_state
            .states
            .insert(TypeId::of::<Theme>(), Box::new(theme.as_ref().clone()));
    }

    // Seed the built-in CurrentRoute so the outlet renders the initial path ("/");
    // the client's on-connect R<path> corrects it for deep-links.
    if crate::router::installed_router().is_some() {
        conn_state.states.insert(
            TypeId::of::<crate::router::CurrentRoute>(),
            Box::new(crate::router::CurrentRoute::default()),
        );
    }

    // Restore cached session state if available, otherwise initialize fresh
    if let Some(cached) = shared.restore_session(conn_state.session_id.as_str()) {
        println!("[{}] Restored cached session state", peer_addr);
        conn_state.restore_states(cached);
        conn_state.initialize_missing_states();
    } else {
        conn_state.initialize_all_states();
    }

    // Subscribe to every shared/persisted state this connection renders or
    // mutates, so broadcasts (cross-tab persisted writes, or background
    // `update_shared` calls) re-render it. Also lazily create the single shared
    // instance so the first connection doesn't render an empty default.
    {
        let keys = shared_persisted_keys(
            &conn_state.handlers,
            &conn_state.synced_elements,
            &conn_state.session_id,
        );
        let mut cache = shared
            .shared_cache
            .write()
            .map_err(|_| "shared cache lock poisoned")?;
        for (tid, key) in keys {
            if key.starts_with("__shared__:") {
                if let Some(synced) = conn_state
                    .synced_elements
                    .iter()
                    .find(|s| s.state_type_id == tid)
                {
                    cache
                        .entry(key.clone())
                        .or_insert_with(|| synced.renderer.create_default_state());
                }
            }
            if conn_state.subscribed_keys.insert(key.clone()) {
                shared.subscribe(conn_state.connection_id, &key);
            }
        }
    }

    // Now rebuild the DOM with all states available
    // Build a HashMap of all states for multi-state rendering
    // For persisted state, use shared cache; for memory state, use connection state
    let initial_dom = conn_state.render_initial_dom(&shared, &root_element, &composite_table)?;

    // Send initial DOM message (global CSS is in the capsule; class rules are
    // delivered lazily via STYLE_DEF, starting with this initial batch).
    println!(
        "[{}] Sending initial DOM ({} bytes, {} handlers, {} synced, {} state types)",
        peer_addr,
        initial_dom.len(),
        conn_state.handlers.len(),
        conn_state.synced_elements.len(),
        conn_state.states.len()
    );
    write.send(Message::Binary(initial_dom.to_vec())).await?;

    // Handle incoming messages
    let mut consecutive_decode_errors: u32 = 0;
    const MAX_CONSECUTIVE_DECODE_ERRORS: u32 = 10;
    const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
    let mut awaiting_pong = false;
    // Stop racing the broadcast channel if it ever closes (avoids busy-looping
    // on a permanently-ready Err). It stays open for the connection's lifetime.
    let mut broadcast_open = true;

    loop {
        // Symbol-table ceiling (memory DoS guard): `sent_symbols` interns each
        // distinct text/class string sent to this client and is never evicted (the
        // wire indices are positional, so eviction would desync the client table).
        // A connection fed a torrent of unique strings (e.g. echoed user text) would
        // otherwise grow it without bound. Past the cap, drop the connection — a
        // reconnect starts the table fresh. The cap is far above any normal app's
        // string vocabulary, and the inbound rate limit (above) bounds how fast an
        // attacker can approach it.
        if conn_state.sent_symbols.len() > MAX_SENT_SYMBOLS {
            eprintln!(
                "[{}] Symbol table exceeded {} entries, disconnecting",
                peer_addr, MAX_SENT_SYMBOLS
            );
            break;
        }

        // Race the client read against the broadcast channel. A broadcast means
        // some state changed outside this connection's events (a background task
        // via `notify_all`, or another connection's persisted write); re-render
        // the affected state type and push the diff. Per-region hash dedup keeps
        // unchanged renders off the wire.
        let read_result = if broadcast_open {
            use futures::future::{select, Either};
            let read_fut = timeout(HEARTBEAT_INTERVAL, read.next());
            let bcast_fut = broadcast_rx.recv();
            futures::pin_mut!(read_fut, bcast_fut);
            match select(read_fut, bcast_fut).await {
                Either::Left((res, _)) => res,
                Either::Right((Err(_), _)) => {
                    broadcast_open = false;
                    continue;
                }
                Either::Right((
                    Ok(BroadcastMsg::StateChanged {
                        key,
                        state_type_id,
                        changes,
                    }),
                    _,
                )) => {
                    // Keyed (persisted) broadcasts inject the authoritative copy by
                    // key; keyless (global) ones render from this connection's state.
                    let inject = (!key.is_empty()).then_some(key.as_str());
                    let update =
                        conn_state.build_type_update(&shared, state_type_id, changes, inject)?;

                    if !update.is_empty() {
                        write.send(Message::Binary(update.to_vec())).await?;
                    }
                    continue;
                }
            }
        } else {
            timeout(HEARTBEAT_INTERVAL, read.next()).await
        };

        let msg = match read_result {
            Ok(Some(msg)) => {
                awaiting_pong = false; // any message = alive
                metrics.messages_received.inc();
                msg
            }
            Ok(None) => break, // stream ended
            Err(_) => {
                // Timeout — no message in 30s
                if awaiting_pong {
                    println!("[{}] Heartbeat timeout, disconnecting", peer_addr);
                    break;
                }
                if write.send(Message::Ping(vec![])).await.is_err() {
                    break;
                }
                awaiting_pong = true;
                continue;
            }
        };
        match msg {
            Ok(Message::Binary(data)) => match ClientEvent::decode(&data) {
                Ok(event) => {
                    consecutive_decode_errors = 0;

                    // Rate limit: drop well-formed events that exceed the per-connection
                    // token bucket before doing any handler/render/broadcast work. The
                    // client-side debounce is only advisory, so this is enforced here.
                    if !conn_state.allow_event() {
                        continue;
                    }

                    println!(
                        "[{}] Event: handler=0x{:02X} type={} target_ref={}",
                        peer_addr,
                        event.handler_idx,
                        event.event_type_name(),
                        event.target_ref
                    );

                    if let Some(handler) = conn_state.handlers.get(&event.handler_idx).cloned() {
                        // Create EventContext from payload and param_bytes
                        let ctx = EventContext::new_with_params(event.payload, event.param_bytes);
                        let state_type_id = handler.state_type_id();

                        // Execute the handler against its state (shared cache or memory)
                        // and learn whether it was shared/persisted (cache key).
                        let cache_key = conn_state.dispatch_handler(&shared, &handler, &ctx)?;

                        // Re-render this handler's state type (TypeId-filtered; hash
                        // dedup skips identical output). Persisted/shared state is
                        // injected from the cache the handler just wrote.
                        let changes = handler.changes();
                        let update = conn_state.build_type_update(
                            &shared,
                            state_type_id,
                            changes,
                            cache_key.as_deref(),
                        )?;

                        if !update.is_empty() {
                            write.send(Message::Binary(update.to_vec())).await?;
                        }

                        // A handler may have requested a client navigation (e.g. it
                        // created a resource and wants the URL to reflect it). Flush
                        // it after the DOM update so the page is already in the new
                        // state when the URL changes.
                        if let Some(nav) = ctx.take_navigation() {
                            let mut buf = crate::protocol::OpcodeBuffer::new();
                            if nav.replace {
                                buf.route_replace_inline(&nav.url);
                            } else {
                                buf.route_push_inline(&nav.url);
                            }
                            write.send(Message::Binary(buf.finish().to_vec())).await?;
                        }
                    } else {
                        eprintln!(
                            "[{}] No handler registered for index {}",
                            peer_addr, event.handler_idx
                        );
                    }
                }
                Err(e) => {
                    consecutive_decode_errors += 1;
                    eprintln!(
                        "[{}] Failed to decode event: {} ({}/{})",
                        peer_addr, e, consecutive_decode_errors, MAX_CONSECUTIVE_DECODE_ERRORS
                    );
                    if consecutive_decode_errors >= MAX_CONSECUTIVE_DECODE_ERRORS {
                        eprintln!(
                            "[{}] Too many consecutive decode errors, disconnecting",
                            peer_addr
                        );
                        break;
                    }
                }
            },
            Ok(Message::Text(text)) => {
                // Route changes also drive a re-render (and possibly a broadcast), so
                // they draw from the same per-connection rate budget as events.
                if !conn_state.allow_event() {
                    continue;
                }
                if let Some(path) = text.strip_prefix('R') {
                    // Built-in router: update CurrentRoute so the outlet re-renders the
                    // matched view, then reconcile the view's renderer registrations so
                    // its stateful regions stay live (and the prior view's are pruned).
                    if let Some(router) = crate::router::installed_router() {
                        let update = conn_state.render_route_view_swap(&shared, router, path)?;
                        if !update.is_empty() {
                            write.send(Message::Binary(update.to_vec())).await?;
                        }
                    }
                    if let Some(ref handler) = route_handler {
                        println!("[{}] Route: {}", peer_addr, path);

                        let ctx = EventContext::from_text(path);
                        let state_type_id = handler.state_type_id();

                        if crate::router::installed_router().is_some() {
                            // A router owns the route view + nav re-render (above). Run the
                            // app's on_route purely for side-effects (e.g. asking a bridge to
                            // load data) on the connection's state — no shared write, no
                            // broadcast, no re-render. Re-rendering shared renderers here
                            // (the handler's `changes` are conservative) would fight the
                            // router's view swap, leaving the page frozen. Data mutations
                            // should flow through event handlers / the bridge, which
                            // broadcast their own narrow updates.
                            conn_state.ensure_state_initialized_for(handler);
                            if let Some(state) = conn_state.get_state_mut(state_type_id) {
                                handler.call_with_context(state, &ctx);
                            }
                        } else {
                            // No router: dispatch like a regular event handler —
                            // shared/persisted executes on the shared cache (broadcast +
                            // subscribed), memory state on the connection.
                            let cache_key = conn_state.dispatch_handler(&shared, handler, &ctx)?;

                            let changes = handler.changes();
                            let update = conn_state.build_type_update(
                                &shared,
                                state_type_id,
                                changes,
                                cache_key.as_deref(),
                            )?;

                            if !update.is_empty() {
                                write.send(Message::Binary(update.to_vec())).await?;
                            }
                        }
                    }
                } else {
                    println!("[{}] Text message (unexpected): {}", peer_addr, text);
                }
            }
            Ok(Message::Ping(data)) => {
                write.send(Message::Pong(data)).await?;
            }
            Ok(Message::Pong(_)) => {}
            Ok(Message::Close(_)) => {
                break;
            }
            Ok(Message::Frame(_)) => {}
            Err(e) => {
                eprintln!("[{}] Error receiving message: {}", peer_addr, e);
                break;
            }
        }
    }

    // Cache session state for potential reconnection
    let session_states = conn_state.take_states();
    shared.cache_session(&conn_state.session_id, session_states);

    // Cleanup happens automatically via ConnectionGuard drop

    Ok(())
}

#[cfg(test)]
mod auth_tests {
    use super::{request_body, request_line, rewrite_proxy_head, url_decode, AuthGate};

    #[test]
    fn url_decode_handles_plus_and_percent() {
        assert_eq!(url_decode("hello"), "hello");
        assert_eq!(url_decode("a+b"), "a b");
        assert_eq!(url_decode("p%40ss%2Fword"), "p@ss/word");
    }

    #[test]
    fn proxy_head_strips_the_prefix_and_preserves_the_rest() {
        let head = b"GET /preview/ws-abc/dash?x=1 HTTP/1.1\r\nHost: h\r\n\r\n";
        let out = rewrite_proxy_head(head, "/preview/ws-abc");
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "GET /dash?x=1 HTTP/1.1\r\nHost: h\r\n\r\n"
        );
        // The bare mount point becomes root.
        let bare = b"GET /preview/ws-abc HTTP/1.1\r\n\r\n";
        assert!(
            String::from_utf8(rewrite_proxy_head(bare, "/preview/ws-abc"))
                .unwrap()
                .starts_with("GET / HTTP/1.1")
        );
        // A WebSocket upgrade line is rewritten the same way (headers untouched → still upgrades).
        let ws = b"GET /preview/ws-abc/ HTTP/1.1\r\nUpgrade: websocket\r\n\r\n";
        let got = String::from_utf8(rewrite_proxy_head(ws, "/preview/ws-abc")).unwrap();
        assert!(got.starts_with("GET / HTTP/1.1"));
        assert!(got.contains("Upgrade: websocket"));
    }

    #[test]
    fn request_line_and_body_parse() {
        let req = "POST /login HTTP/1.1\r\nHost: x\r\n\r\nusername=a&password=b";
        assert_eq!(request_line(req), ("POST", "/login"));
        assert_eq!(request_body(req), "username=a&password=b");
        assert_eq!(request_line("GET / HTTP/1.1\r\n\r\n"), ("GET", "/"));
    }

    #[test]
    fn login_issues_token_only_for_correct_credentials() {
        let gate = AuthGate::new("admin".to_string(), "secret".to_string());
        assert!(gate.login("username=admin&password=secret").is_some());
        assert!(gate.login("username=admin&password=wrong").is_none());
        assert!(gate.login("username=other&password=secret").is_none());
        assert!(gate.login("password=secret").is_none());
    }

    #[test]
    fn session_valid_only_with_issued_cookie() {
        let gate = AuthGate::new("admin".to_string(), "secret".to_string());
        let token = gate.login("username=admin&password=secret").expect("login");
        let signed = format!("GET / HTTP/1.1\r\nCookie: rwire_auth={token}\r\n\r\n");
        assert!(gate.has_session(&signed));
        assert!(!gate.has_session("GET / HTTP/1.1\r\n\r\n"));
        assert!(!gate.has_session("GET / HTTP/1.1\r\nCookie: rwire_auth=forged\r\n\r\n"));
        // Logout invalidates the token.
        gate.logout(&signed);
        assert!(!gate.has_session(&signed));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_base_path_folds_a_prefix_and_drops_empty() {
        // A prefix with no prior config forces a default (styled) capsule carrying the base.
        assert_eq!(
            apply_base_path(None, Some("/preview/x".to_owned()))
                .unwrap()
                .base_path,
            "/preview/x"
        );
        // A prefix merges into an existing config.
        assert_eq!(
            apply_base_path(Some(CapsuleConfig::new()), Some("/preview/x".to_owned()))
                .unwrap()
                .base_path,
            "/preview/x"
        );
        // Empty / "/"-only / whitespace prefixes are a no-op — no config is conjured.
        assert!(apply_base_path(None, Some(String::new())).is_none());
        assert!(apply_base_path(None, Some("/".to_owned())).is_none());
        assert!(apply_base_path(None, Some("   ".to_owned())).is_none());
        // No prefix leaves the config untouched (present or absent).
        assert!(apply_base_path(None, None).is_none());
        assert!(apply_base_path(Some(CapsuleConfig::new()), None).is_some());
    }

    #[test]
    fn base_path_is_order_independent_on_the_builder() {
        use crate::builder::el;
        use crate::protocol::El;

        // Set base_path BEFORE .root()/.capsule_config() — via ServerBuilder::base_path (alongside
        // bind). The base rides through as its own field, untouched by the later capsule_config.
        let before = Server::bind("127.0.0.1:0")
            .unwrap()
            .base_path("/preview/x")
            .root(|| el(El::Div))
            .capsule_config(CapsuleConfig::new());
        assert_eq!(
            apply_base_path(before.capsule_config, before.base_path)
                .unwrap()
                .base_path,
            "/preview/x"
        );

        // Set base_path AFTER .capsule_config() — via ServerWithRoot::base_path. This is the order
        // the old immediate-merge silently clobbered; now both land the same prefix.
        let after = Server::bind("127.0.0.1:0")
            .unwrap()
            .root(|| el(El::Div))
            .capsule_config(CapsuleConfig::new())
            .base_path("/preview/x");
        assert_eq!(
            apply_base_path(after.capsule_config, after.base_path)
                .unwrap()
                .base_path,
            "/preview/x"
        );
    }

    #[test]
    fn session_cache_is_bounded() {
        // Churning distinct sessions must not grow the reconnect cache without
        // bound (memory DoS guard). Insert past the cap and confirm it holds.
        let shared = SharedServerState::new(Duration::from_millis(100));
        for i in 0..(MAX_CACHED_SESSIONS + 50) {
            let mut states: HashMap<TypeId, Box<dyn Any + Send + Sync>> = HashMap::new();
            states.insert(TypeId::of::<i32>(), Box::new(i as i32));
            shared.cache_session(&format!("sess-{i}"), states);
        }
        let len = shared
            .session_state_cache
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .len();
        assert!(
            len <= MAX_CACHED_SESSIONS,
            "cache grew past cap: {len} > {MAX_CACHED_SESSIONS}"
        );
    }

    #[test]
    fn event_rate_limit_caps_a_flood() {
        // A fresh connection may burst, but a tight flood is capped near the bucket
        // capacity rather than processed unbounded.
        let mut conn = ConnectionState::new(1, SessionId::new("a".repeat(32)));
        let mut allowed = 0usize;
        for _ in 0..10_000 {
            if conn.allow_event() {
                allowed += 1;
            }
        }
        // The initial burst is honored...
        assert!(
            allowed >= EVENT_BUCKET_CAPACITY as usize,
            "expected at least the burst capacity to pass, got {allowed}"
        );
        // ...but the flood is throttled far below the 10k attempts (tiny refill only).
        assert!(
            allowed < EVENT_BUCKET_CAPACITY as usize + 100,
            "flood not capped: {allowed} allowed out of 10000"
        );
    }

    #[derive(Default)]
    struct SharedCounter {
        n: i32,
    }
    impl State for SharedCounter {
        const STORAGE_TYPE: StorageType = StorageType::Shared;
        const TABLE_NAME: &'static str = "shared_counter";
    }

    #[test]
    fn update_shared_mutates_and_broadcasts() {
        let shared = SharedServerState::new(Duration::from_millis(100));
        let (tx, rx) = async_channel::bounded::<BroadcastMsg>(8);
        shared.register_connection(1, tx);
        shared.subscribe(1, "__shared__:shared_counter");

        shared.update_shared::<SharedCounter>(|s| s.n = 42);

        // Subscribed connection is notified.
        let msg = rx.try_recv().unwrap();
        assert!(matches!(
            msg,
            BroadcastMsg::StateChanged { ref key, .. } if key == "__shared__:shared_counter"
        ));

        // The single shared instance was mutated in place.
        let cache = shared.shared_cache.read().unwrap();
        let value = cache
            .get("__shared__:shared_counter")
            .unwrap()
            .downcast_ref::<SharedCounter>()
            .unwrap();
        assert_eq!(value.n, 42);
    }

    #[test]
    fn shared_cache_key_per_storage_type() {
        assert_eq!(
            shared_cache_key(StorageType::Memory, Some("t"), "sess"),
            None
        );
        assert_eq!(
            shared_cache_key(StorageType::Persisted, Some("notes"), "sess"),
            Some("notes:sess".to_string())
        );
        assert_eq!(
            shared_cache_key(StorageType::Shared, Some("metrics"), "sess"),
            Some("__shared__:metrics".to_string())
        );
    }

    #[test]
    fn test_shared_state_new() {
        let shared = SharedServerState::new(Duration::from_millis(100));
        assert!(shared.shared_cache.read().unwrap().is_empty());
        assert!(shared.dirty_keys.read().unwrap().is_empty());
        assert_eq!(shared.persist_interval, Duration::from_millis(100));
    }

    #[test]
    fn test_connection_id_generation() {
        let shared = SharedServerState::new(Duration::from_millis(100));
        let id1 = shared.next_connection_id();
        let id2 = shared.next_connection_id();
        let id3 = shared.next_connection_id();
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[test]
    fn test_dirty_key_tracking() {
        let shared = SharedServerState::new(Duration::from_millis(100));

        assert!(!shared.has_dirty());
        assert_eq!(shared.dirty_count(), 0);

        shared.mark_dirty("key1");
        shared.mark_dirty("key2");

        assert!(shared.has_dirty());
        assert_eq!(shared.dirty_count(), 2);

        let drained = shared.drain_dirty();
        assert_eq!(drained.len(), 2);
        assert!(drained.contains(&"key1".to_string()));
        assert!(drained.contains(&"key2".to_string()));

        assert!(!shared.has_dirty());
        assert_eq!(shared.dirty_count(), 0);
    }

    #[test]
    fn test_state_insertion() {
        let shared = SharedServerState::new(Duration::from_millis(100));

        assert!(!shared.has_state("test:abc"));

        shared.insert_state("test:abc".to_string(), Box::new(42i32));

        assert!(shared.has_state("test:abc"));
    }

    #[test]
    fn test_subscription_management() {
        let shared = SharedServerState::new(Duration::from_millis(100));

        let (tx1, rx1) = async_channel::bounded::<BroadcastMsg>(10);
        let (tx2, rx2) = async_channel::bounded::<BroadcastMsg>(10);

        shared.register_connection(1, tx1);
        shared.register_connection(2, tx2);

        shared.subscribe(1, "key1");
        shared.subscribe(2, "key1");

        // Broadcast from connection 1 - should NOT reach connection 1
        shared.broadcast("key1", TypeId::of::<i32>(), ChangeSet::all(), 1);

        // Connection 1 should NOT receive (sender excluded)
        assert!(rx1.is_empty());

        // Connection 2 SHOULD receive
        assert!(!rx2.is_empty());
        let msg = rx2.try_recv().unwrap();
        assert!(matches!(msg, BroadcastMsg::StateChanged { key, .. } if key == "key1"));
    }

    #[test]
    fn test_broadcast_ignores_unsubscribed() {
        let shared = SharedServerState::new(Duration::from_millis(100));

        let (tx1, rx1) = async_channel::bounded::<BroadcastMsg>(10);
        let (tx2, rx2) = async_channel::bounded::<BroadcastMsg>(10);

        shared.register_connection(1, tx1);
        shared.register_connection(2, tx2);

        // Only connection 1 subscribes
        shared.subscribe(1, "key1");

        // Broadcast from connection 1
        shared.broadcast("key1", TypeId::of::<i32>(), ChangeSet::all(), 1);

        // Neither should receive (1 is sender, 2 not subscribed)
        assert!(rx1.is_empty());
        assert!(rx2.is_empty());
    }

    #[test]
    fn test_unregister_cleans_subscriptions() {
        let shared = SharedServerState::new(Duration::from_millis(100));

        let (tx, _rx) = async_channel::bounded::<BroadcastMsg>(10);
        shared.register_connection(1, tx);

        shared.subscribe(1, "key1");
        shared.subscribe(1, "key2");

        {
            let subs = shared.subscriptions.read().unwrap();
            assert!(subs.get("key1").is_some_and(|v| v.contains(&1)));
            assert!(subs.get("key2").is_some_and(|v| v.contains(&1)));
        }

        shared.unregister_connection(1);

        {
            let subs = shared.subscriptions.read().unwrap();
            assert!(!subs.get("key1").is_some_and(|v| v.contains(&1)));
            assert!(!subs.get("key2").is_some_and(|v| v.contains(&1)));
        }
    }

    #[test]
    fn test_broadcast_drops_when_channel_full() {
        let shared = SharedServerState::new(Duration::from_millis(100));

        // Small channel size
        let (tx, rx) = async_channel::bounded::<BroadcastMsg>(2);
        shared.register_connection(1, tx);
        shared.subscribe(1, "key");

        // Fill channel (broadcast from connection 0, so connection 1 receives)
        shared.broadcast("key", TypeId::of::<i32>(), ChangeSet::all(), 0);
        shared.broadcast("key", TypeId::of::<i32>(), ChangeSet::all(), 0);

        // Third should be dropped (no panic, no block)
        shared.broadcast("key", TypeId::of::<i32>(), ChangeSet::all(), 0);

        assert_eq!(rx.len(), 2);
    }

    #[test]
    fn test_persist_dirty_with_registered_type() {
        use crate::persist::{PersistableType, SqliteStore};

        let store = SqliteStore::memory().unwrap();

        // Register a simple persistable type
        let persistable = PersistableType {
            table_name: "counters",
            schema: &["CREATE TABLE IF NOT EXISTS counters (id TEXT PRIMARY KEY, value INTEGER NOT NULL DEFAULT 0)"],
            type_id: TypeId::of::<i32>(),
            key_field: "id",
            load_fn: |conn, key| {
                use crate::persist::PersistError;
                use rusqlite::Error as SqliteError;
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
                use crate::persist::PersistError;
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

        let shared = SharedServerState::new(Duration::from_millis(100));

        // Insert state into cache
        shared.insert_state("counters:test1".to_string(), Box::new(42i32));
        shared.mark_dirty("counters:test1");

        // Persist dirty state
        let count = shared.persist_dirty(&store).unwrap();
        assert_eq!(count, 1);

        // Verify no longer dirty
        assert!(!shared.has_dirty());

        // Verify in database
        let conn = store.connection();
        let conn = conn.lock().unwrap();
        let value: i32 = conn
            .query_row("SELECT value FROM counters WHERE id = 'test1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_persist_dirty_empty() {
        use crate::persist::SqliteStore;

        let store = SqliteStore::memory().unwrap();
        let shared = SharedServerState::new(Duration::from_millis(100));

        // No dirty keys
        let count = shared.persist_dirty(&store).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_extract_cookie_from_request() {
        // Standard case
        let request = "GET / HTTP/1.1\r\nHost: localhost\r\nCookie: rwire_sid=abc123\r\n\r\n";
        let cookie = super::extract_cookie_from_request(request);
        assert_eq!(cookie, Some("rwire_sid=abc123".to_string()));

        // Case-insensitive
        let request = "GET / HTTP/1.1\r\nhost: localhost\r\ncookie: rwire_sid=def456\r\n\r\n";
        let cookie = super::extract_cookie_from_request(request);
        assert_eq!(cookie, Some("rwire_sid=def456".to_string()));

        // No cookie header
        let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let cookie = super::extract_cookie_from_request(request);
        assert_eq!(cookie, None);

        // Multiple cookies
        let request = "GET / HTTP/1.1\r\nCookie: foo=bar; rwire_sid=xyz789; other=value\r\n\r\n";
        let cookie = super::extract_cookie_from_request(request);
        assert_eq!(
            cookie,
            Some("foo=bar; rwire_sid=xyz789; other=value".to_string())
        );
    }

    #[test]
    fn test_forwarded_https() {
        // Proxy signals HTTPS → Secure cookie.
        assert!(super::forwarded_https(
            "GET / HTTP/1.1\r\nX-Forwarded-Proto: https\r\n\r\n"
        ));
        // Case-insensitive header name and value.
        assert!(super::forwarded_https(
            "GET / HTTP/1.1\r\nx-forwarded-proto: HTTPS\r\n\r\n"
        ));
        // Proxy chain: first value is authoritative.
        assert!(super::forwarded_https(
            "GET / HTTP/1.1\r\nX-Forwarded-Proto: https, http\r\n\r\n"
        ));
        // Plain HTTP (header absent or http) → not secure.
        assert!(!super::forwarded_https(
            "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n"
        ));
        assert!(!super::forwarded_https(
            "GET / HTTP/1.1\r\nX-Forwarded-Proto: http\r\n\r\n"
        ));
    }

    #[test]
    fn origin_gate_same_origin_passes() {
        assert!(origin_allowed(
            "http://localhost:9000",
            Some("localhost:9000"),
            &[]
        ));
        assert!(origin_allowed(
            "https://app.example.com",
            Some("app.example.com"),
            &[]
        ));
        // default ports normalize
        assert!(origin_allowed(
            "https://app.example.com",
            Some("app.example.com:443"),
            &[]
        ));
        assert!(origin_allowed(
            "HTTP://LOCALHOST:9000",
            Some("localhost:9000"),
            &[]
        ));
    }

    #[test]
    fn origin_gate_cross_origin_rejected() {
        assert!(!origin_allowed(
            "http://evil.example",
            Some("localhost:9000"),
            &[]
        ));
        // same host, different explicit port = different origin
        assert!(!origin_allowed(
            "http://localhost:3000",
            Some("localhost:9000"),
            &[]
        ));
        assert!(!origin_allowed("null", Some("localhost:9000"), &[]));
        assert!(!origin_allowed(
            "chrome-extension://abc",
            Some("localhost:9000"),
            &[]
        ));
        assert!(!origin_allowed("http://localhost:9000", None, &[]));
    }

    #[test]
    fn origin_gate_allowlist_passes_cross_origin() {
        let allowed = vec!["https://embed.example.com".to_string()];
        assert!(origin_allowed(
            "https://embed.example.com",
            Some("api.other.com"),
            &allowed
        ));
        assert!(origin_allowed(
            "https://embed.example.com/",
            Some("api.other.com"),
            &allowed
        ));
        assert!(!origin_allowed(
            "https://evil.example.com",
            Some("api.other.com"),
            &allowed
        ));
    }

    #[test]
    fn test_flush_all_dirty() {
        use crate::persist::{PersistableType, SqliteStore};

        let store = SqliteStore::memory().unwrap();

        let persistable = PersistableType {
            table_name: "counters",
            schema: &["CREATE TABLE IF NOT EXISTS counters (id TEXT PRIMARY KEY, value INTEGER NOT NULL DEFAULT 0)"],
            type_id: TypeId::of::<i32>(),
            key_field: "id",
            load_fn: |conn, key| {
                use crate::persist::PersistError;
                use rusqlite::Error as SqliteError;
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
                use crate::persist::PersistError;
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

        let shared = SharedServerState::new(Duration::from_millis(100));

        // Insert multiple states
        shared.insert_state("counters:a".to_string(), Box::new(1i32));
        shared.insert_state("counters:b".to_string(), Box::new(2i32));
        shared.insert_state("counters:c".to_string(), Box::new(3i32));

        shared.mark_dirty("counters:a");
        shared.mark_dirty("counters:b");
        shared.mark_dirty("counters:c");

        // Flush all dirty
        let count = shared.flush_all_dirty(&store, 3).unwrap();
        assert_eq!(count, 3);

        // Verify all persisted
        assert!(!shared.has_dirty());

        // Verify in database
        let conn = store.connection();
        let conn = conn.lock().unwrap();
        let total: i32 = conn
            .query_row("SELECT COUNT(*) FROM counters", [], |row| row.get(0))
            .unwrap();
        assert_eq!(total, 3);
    }
}
