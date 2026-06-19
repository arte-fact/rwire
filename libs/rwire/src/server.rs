//! WebSocket server for rwire with stateful client connections.
//!
//! Single port serving both:
//! - HTTP GET / → capsule HTML
//! - WebSocket upgrade → binary DOM protocol with state management

use async_std::future::timeout;
use async_std::net::{TcpListener, TcpStream};
use async_std::task;
use async_tungstenite::accept_async;
use async_tungstenite::tungstenite::Message;
use futures::prelude::*;
use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::net::{AddrParseError, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::builder::{
    build_synced_update_with_known_symbols, extract_renderers, BuildContext, ElementBuilder,
    SyncedElement,
};
use crate::capsule;
use crate::capsule_gen::{self, CapsuleConfig};
use crate::protocol::ClientEvent;
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
    pub fn cache_session(
        &self,
        session_id: &str,
        states: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    ) {
        if states.is_empty() {
            return;
        }
        self.session_state_cache
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(
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
}

/// Server with root element configured, ready to run.
pub struct ServerWithRoot<F> {
    addr: SocketAddr,
    persist_interval: Duration,
    root: F,
    shared: Option<Arc<SharedServerState>>,
    capsule_config: Option<CapsuleConfig>,
    route_handler: Option<HandlerFn>,
    router: Option<crate::router::Router>,
    theme_provider: Option<ThemeProvider>,
    auth: Option<AuthGate>,
}

impl Server {
    /// Start building a server bound to the given address.
    pub fn bind(addr: &str) -> Result<ServerBuilder, AddrParseError> {
        Ok(ServerBuilder {
            addr: addr.parse()?,
            persist_interval: Duration::from_millis(100),
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
            route_handler: None,
            router: None,
            theme_provider: None,
            auth: None,
        }
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

    /// Register a router for view tree-shaking.
    ///
    /// At startup, all registered view functions are called with default params
    /// to discover element types, style tokens, events, and attributes. This
    /// eliminates the need for `extra_elements` / `extra_styles` in the capsule
    /// config.
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

        // Generate capsule - styled if config provided, basic otherwise
        let capsule = if let Some(config) = self.capsule_config {
            // If theme provider is set, override config theme with initial theme
            let config = if let Some(ref theme) = initial_theme {
                config.theme(theme.clone())
            } else {
                config
            };

            // The capsule's static CSS only needs composite classes + globals;
            // utility/pseudo/breakpoint rules (.u/.h/.b) are delivered lazily over
            // the wire (STYLE_DEF), and the small u8 enum maps are shipped whole.
            // So only the composite table and client-action flag feed the config.
            let composite_css = ctx.composite_table().generate_css();
            let config = config
                .has_client_actions(ctx.has_client_actions())
                .with_composite_css(composite_css);

            // Generate CSS and embed in capsule HTML <style> tag.
            let css = capsule_gen::generate_capsule_css(&config);
            capsule_gen::generate_styled_capsule(&config, &css)
        } else {
            capsule_gen::generate_capsule()
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
    tokens: std::sync::Mutex<HashMap<String, std::time::Instant>>,
}

impl AuthGate {
    fn new(user: String, password: String) -> Self {
        Self {
            user,
            password,
            brand: None,
            tokens: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Whether the request carries a valid, unexpired session cookie.
    fn has_session(&self, request: &str) -> bool {
        let Some(token) = cookie_value(request, AUTH_COOKIE) else {
            return false;
        };
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
            let token = generate_token();
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

/// Return the body that follows the header terminator, if present.
fn request_body(request: &str) -> &str {
    request.split_once("\r\n\r\n").map_or("", |(_, body)| body)
}

/// Extract Cookie header value from HTTP request.
fn extract_cookie_from_request(request: &str) -> Option<String> {
    for line in request.lines() {
        if line.len() >= 7 && line[..7].eq_ignore_ascii_case("cookie:") {
            return Some(line[7..].trim().to_string());
        }
    }
    None
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
                println!("[{peer_addr}] WebSocket rejected (no session)");
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

    // Extract session ID from cookie, or generate new one
    let (session_id, is_new_session) =
        if let Some(cookie_value) = extract_cookie_from_request(&peek_str) {
            if let Some(sid) = SessionId::from_cookie(&cookie_value) {
                println!("[{}] Found session: {}", peer_addr, sid);
                (sid, false)
            } else {
                let sid = SessionId::generate();
                println!("[{}] New session (no valid cookie): {}", peer_addr, sid);
                (sid, true)
            }
        } else {
            let sid = SessionId::generate();
            println!("[{}] New session: {}", peer_addr, sid);
            (sid, true)
        };

    // Check if this is a WebSocket upgrade request
    if capsule::is_websocket_upgrade(&peek_str) {
        println!("[{}] WebSocket connection", peer_addr);
        match accept_async(stream).await {
            Ok(ws_stream) => {
                if let Err(e) = handle_websocket(
                    ws_stream,
                    peer_addr,
                    root,
                    shared,
                    session_id,
                    route_handler,
                    initial_theme,
                    composite_table,
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
        if let Err(e) = capsule::serve(stream, &capsule, Some(&session_id), is_new_session).await {
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
        }
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

#[allow(clippy::too_many_arguments)]
async fn handle_websocket<F>(
    ws_stream: async_tungstenite::WebSocketStream<TcpStream>,
    peer_addr: SocketAddr,
    root: Arc<F>,
    shared: Arc<SharedServerState>,
    session_id: SessionId,
    route_handler: Option<Arc<HandlerFn>>,
    initial_theme: Option<Arc<crate::theme::Theme>>,
    composite_table: Arc<crate::style_groups::CompositeTable>,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    F: Fn() -> ElementBuilder + Send + Sync + 'static,
{
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
    let initial_dom = {
        let mut ctx = BuildContext::new();

        // Acquire read lock on shared cache
        let cache_guard = shared
            .shared_cache
            .read()
            .map_err(|_| "shared cache lock poisoned")?;

        // Build states_map with connection state, then override with shared cache
        let mut states_map: HashMap<TypeId, &(dyn Any + Send + Sync)> = conn_state
            .states
            .iter()
            .map(|(k, v)| (*k, v.as_ref()))
            .collect();

        // Override memory state with shared/persisted instances from the cache.
        for (tid, key) in shared_persisted_keys(
            &conn_state.handlers,
            &conn_state.synced_elements,
            &conn_state.session_id,
        ) {
            if let Some(state) = cache_guard.get(&key) {
                states_map.insert(tid, state.as_ref());
            }
        }

        // Reuse the startup composite table so that composite IDs match
        // the CSS baked into the capsule (different DOM state would produce
        // different ID assignments if we re-analyzed here).
        ctx.set_composite_table((*composite_table).clone());

        if states_map.is_empty() {
            // No states available, use placeholder
            ctx.collect_symbols(&root_element, &placeholder_state);
            ctx.emit(&root_element, &placeholder_state);
        } else {
            // Use multi-state methods to render all synced elements correctly
            ctx.collect_symbols_multi(&root_element, &states_map);
            ctx.emit_multi(&root_element);
        }

        // Drop cache_guard before continuing (it's automatically dropped at end of scope)
        drop(cache_guard);

        // Re-extract handlers and synced elements (they should be the same)
        conn_state.handlers = ctx.handlers().clone();
        conn_state.synced_elements = ctx.take_synced_elements();
        // Capture sent symbols for incremental updates
        conn_state.sent_symbols = ctx.take_symbol_map();

        // Prepend STYLE_DEF for the styles this initial render uses (lazy CSS):
        // the capsule ships only global CSS; class rules arrive over the wire.
        ctx.finish_with_style_defs(&mut conn_state.sent_css)
    };

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
                    let update = {
                        let mut states_map: HashMap<TypeId, &(dyn Any + Send + Sync)> = conn_state
                            .states
                            .iter()
                            .map(|(k, v)| (*k, v.as_ref()))
                            .collect();

                        // Keyed (persisted) updates read the authoritative copy
                        // from the shared cache; keyless (global) updates render
                        // from this connection's own memory state.
                        let cache_guard = shared
                            .shared_cache
                            .read()
                            .map_err(|_| "shared cache lock poisoned")?;
                        if !key.is_empty() {
                            if let Some(state) = cache_guard.get(&key) {
                                states_map.insert(state_type_id, state.as_ref());
                            }
                        }

                        build_synced_update_with_known_symbols(
                            &conn_state.synced_elements,
                            &states_map,
                            &mut conn_state.handlers,
                            changes,
                            Some(&mut conn_state.sent_symbols),
                            Some(state_type_id),
                            Some(&mut conn_state.synced_hashes),
                            Some(&mut conn_state.sent_css),
                        )
                    };

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

                        // Resolve where this handler's state lives: connection-
                        // local (memory) or the shared cache (persisted/shared).
                        let state_type_id = handler.state_type_id();
                        let cache_key = shared_cache_key(
                            handler.storage_type(),
                            handler.table_name(),
                            &conn_state.session_id,
                        );

                        if let Some(key) = &cache_key {
                            // Shared/persisted: get from shared cache, execute, write back
                            let mut cache = shared
                                .shared_cache
                                .write()
                                .map_err(|_| "shared cache lock poisoned")?;
                            let state = cache
                                .entry(key.clone())
                                .or_insert_with(|| handler.create_state());
                            handler.call_with_context(state.as_mut(), &ctx);
                            drop(cache);

                            // Only persisted state is queued for DB writes.
                            if handler.storage_type() == StorageType::Persisted {
                                shared.mark_dirty(key);
                            }

                            // Notify other connections sharing this state
                            // (cross-tab for persisted, fan-out for shared).
                            shared.broadcast(
                                key,
                                state_type_id,
                                handler.changes(),
                                conn_state.connection_id,
                            );

                            // Subscribe this connection to updates for this key
                            if !conn_state.subscribed_keys.contains(key) {
                                shared.subscribe(conn_state.connection_id, key);
                                conn_state.subscribed_keys.insert(key.clone());
                            }
                        } else {
                            // Memory state: use connection-local state
                            conn_state.ensure_state_initialized_for(&handler);
                            if let Some(state) = conn_state.get_state_mut(state_type_id) {
                                handler.call_with_context(state, &ctx);
                            }
                        }

                        // Re-render synced elements using multi-state support
                        // Build states map that includes both local and shared state
                        let changes = handler.changes();

                        // Build the update bytes within a block to limit lock scope
                        let update = {
                            // Acquire read lock on shared cache (if needed)
                            let cache_guard = if cache_key.is_some() {
                                Some(
                                    shared
                                        .shared_cache
                                        .read()
                                        .map_err(|_| "shared cache lock poisoned")?,
                                )
                            } else {
                                None
                            };

                            let mut states_map: HashMap<TypeId, &(dyn Any + Send + Sync)> =
                                conn_state
                                    .states
                                    .iter()
                                    .map(|(k, v)| (*k, v.as_ref()))
                                    .collect();

                            // For persisted state, add the shared state to the map
                            if let (Some(key), Some(ref cache)) = (&cache_key, &cache_guard) {
                                if let Some(state) = cache.get(key) {
                                    states_map.insert(state_type_id, state.as_ref());
                                }
                            }

                            // Use incremental symbols - pass known symbols and update them
                            // TypeId filter: only re-render synced elements for this handler's state type
                            // Hash dedup: skip emission if rendered output is identical to last send
                            build_synced_update_with_known_symbols(
                                &conn_state.synced_elements,
                                &states_map,
                                &mut conn_state.handlers,
                                changes,
                                Some(&mut conn_state.sent_symbols),
                                Some(state_type_id),
                                Some(&mut conn_state.synced_hashes),
                                Some(&mut conn_state.sent_css),
                            )
                            // cache_guard dropped here at end of block
                        };

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
                if let Some(path) = text.strip_prefix('R') {
                    // Built-in router: update CurrentRoute so the outlet re-renders the
                    // matched view (the view's own renderers update via Phase B).
                    if crate::router::installed_router().is_some() {
                        let route_tid = TypeId::of::<crate::router::CurrentRoute>();
                        conn_state.states.entry(route_tid).or_insert_with(|| {
                            Box::new(crate::router::CurrentRoute::default())
                        });
                        if let Some(state) = conn_state.get_state_mut(route_tid) {
                            if let Some(route) =
                                state.downcast_mut::<crate::router::CurrentRoute>()
                            {
                                route.set_path(path);
                            }
                        }
                        let update = {
                            let states_map: HashMap<TypeId, &(dyn Any + Send + Sync)> = conn_state
                                .states
                                .iter()
                                .map(|(k, v)| (*k, v.as_ref()))
                                .collect();
                            build_synced_update_with_known_symbols(
                                &conn_state.synced_elements,
                                &states_map,
                                &mut conn_state.handlers,
                                ChangeSet::all(),
                                Some(&mut conn_state.sent_symbols),
                                Some(route_tid),
                                Some(&mut conn_state.synced_hashes),
                                Some(&mut conn_state.sent_css),
                            )
                        };
                        if !update.is_empty() {
                            write.send(Message::Binary(update.to_vec())).await?;
                        }
                    }
                    if let Some(ref handler) = route_handler {
                        println!("[{}] Route: {}", peer_addr, path);

                        let ctx = EventContext::from_text(path);
                        let state_type_id = handler.state_type_id();

                        // Ensure state is initialized for the route handler
                        conn_state.ensure_state_initialized_for(handler);
                        if let Some(state) = conn_state.get_state_mut(state_type_id) {
                            handler.call_with_context(state, &ctx);
                        }

                        // Re-render synced elements
                        let changes = handler.changes();
                        let update = {
                            let states_map: HashMap<TypeId, &(dyn Any + Send + Sync)> = conn_state
                                .states
                                .iter()
                                .map(|(k, v)| (*k, v.as_ref()))
                                .collect();

                            build_synced_update_with_known_symbols(
                                &conn_state.synced_elements,
                                &states_map,
                                &mut conn_state.handlers,
                                changes,
                                Some(&mut conn_state.sent_symbols),
                                Some(state_type_id),
                                Some(&mut conn_state.synced_hashes),
                                Some(&mut conn_state.sent_css),
                            )
                        };

                        if !update.is_empty() {
                            write.send(Message::Binary(update.to_vec())).await?;
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
    use super::{request_body, request_line, url_decode, AuthGate};

    #[test]
    fn url_decode_handles_plus_and_percent() {
        assert_eq!(url_decode("hello"), "hello");
        assert_eq!(url_decode("a+b"), "a b");
        assert_eq!(url_decode("p%40ss%2Fword"), "p@ss/word");
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
