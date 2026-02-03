//! WebSocket server for rwire with stateful client connections.
//!
//! Single port serving both:
//! - HTTP GET / → capsule HTML
//! - WebSocket upgrade → binary DOM protocol with state management

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
use std::time::Duration;

use crate::builder::{
    build_synced_update_with_known_symbols, BuildContext, ElementBuilder, SyncedElement,
};
use crate::capsule;
use crate::capsule_gen;
use crate::protocol::ClientEvent;
use crate::state::{ChangeSet, EventContext, HandlerFn};

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
}

impl SharedServerState {
    /// Create new shared server state.
    pub fn new(persist_interval: Duration) -> Arc<Self> {
        Arc::new(Self {
            shared_cache: RwLock::new(HashMap::new()),
            dirty_keys: RwLock::new(HashSet::new()),
            subscriptions: RwLock::new(HashMap::new()),
            broadcast_senders: RwLock::new(HashMap::new()),
            next_connection_id: AtomicU64::new(1),
            persist_interval,
        })
    }

    /// Allocate unique connection ID.
    pub fn next_connection_id(&self) -> u64 {
        self.next_connection_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Check if state exists in cache.
    pub fn has_state(&self, key: &str) -> bool {
        self.shared_cache.read().unwrap().contains_key(key)
    }

    /// Insert state into cache (for hydration).
    pub fn insert_state(&self, key: String, state: Box<dyn Any + Send + Sync>) {
        self.shared_cache.write().unwrap().insert(key, state);
    }

    /// Mark a key as dirty (needs persistence).
    pub fn mark_dirty(&self, key: &str) {
        self.dirty_keys.write().unwrap().insert(key.to_string());
    }

    /// Check if any keys are dirty.
    pub fn has_dirty(&self) -> bool {
        !self.dirty_keys.read().unwrap().is_empty()
    }

    /// Get count of dirty keys.
    pub fn dirty_count(&self) -> usize {
        self.dirty_keys.read().unwrap().len()
    }

    /// Drain all dirty keys for persistence.
    pub fn drain_dirty(&self) -> Vec<String> {
        let mut dirty = self.dirty_keys.write().unwrap();
        dirty.drain().collect()
    }

    /// Register connection's broadcast channel.
    pub fn register_connection(&self, conn_id: u64, sender: async_channel::Sender<BroadcastMsg>) {
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

    /// Hydrate shared cache from a SqliteStore.
    ///
    /// This loads all persisted state from the database into memory.
    /// Should be called at server startup before accepting connections.
    pub fn hydrate(&self, store: &crate::persist::SqliteStore) -> Result<usize, crate::persist::PersistError> {
        // Ensure schemas exist
        store.ensure_schema()?;

        // Load all state into cache
        let states = store.hydrate_all()?;
        let count = states.len();

        let mut cache = self.shared_cache.write().unwrap();
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
        }
    }
}

impl<F> ServerWithRoot<F>
where
    F: Fn() -> ElementBuilder + Send + Sync + Clone + 'static,
{
    /// Run the server, accepting connections until shutdown.
    pub async fn run(self) -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(self.addr).await?;

        // Create shared server state
        let shared = SharedServerState::new(self.persist_interval);

        // Pre-analyze the root element to determine used types for tree shaking
        let root_element = (self.root)();
        let mut ctx = BuildContext::new();
        let placeholder: () = ();
        ctx.collect_symbols(&root_element, &placeholder);
        ctx.emit(&root_element, &placeholder);

        // Generate minimal capsule with only used element/event types
        let capsule = capsule_gen::generate_capsule(
            ctx.used_elements(),
            ctx.used_events(),
            ctx.has_local_handlers(),
        );
        let capsule_size = capsule.len();
        let capsule = Arc::new(capsule);

        println!("Server listening on http://{}", self.addr);
        println!(
            "Capsule: {} bytes ({} element types, {} event types)",
            capsule_size,
            ctx.used_elements().len(),
            ctx.used_events().len()
        );

        let root = Arc::new(self.root);

        while let Ok((stream, peer_addr)) = listener.accept().await {
            let root = Arc::clone(&root);
            let capsule = Arc::clone(&capsule);
            let shared = Arc::clone(&shared);
            task::spawn(async move {
                handle_client(stream, peer_addr, root, capsule, shared).await;
            });
        }

        Ok(())
    }
}

async fn handle_client<F>(
    mut stream: TcpStream,
    peer_addr: SocketAddr,
    root: Arc<F>,
    capsule: Arc<String>,
    shared: Arc<SharedServerState>,
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

    // Check if this is a WebSocket upgrade request
    if capsule::is_websocket_upgrade(&peek_str) {
        println!("[{}] WebSocket connection", peer_addr);
        match accept_async(stream).await {
            Ok(ws_stream) => {
                if let Err(e) = handle_websocket(ws_stream, peer_addr, root, shared).await {
                    eprintln!("[{}] Connection error: {}", peer_addr, e);
                }
            }
            Err(e) => {
                eprintln!("[{}] WebSocket handshake failed: {}", peer_addr, e);
            }
        }
        println!("[{}] WebSocket closed", peer_addr);
    } else if peek_str.starts_with("GET ") {
        println!("[{}] HTTP request - serving capsule", peer_addr);

        // Consume the request data first
        let mut request_buf = vec![0u8; n];
        if let Err(e) = stream.read_exact(&mut request_buf).await {
            eprintln!("[{}] Failed to read request: {}", peer_addr, e);
            return;
        }

        if let Err(e) = capsule::serve(stream, &capsule).await {
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
    /// State values keyed by TypeId, supporting multiple state types per connection.
    /// Note: For persisted state, the authoritative copy is in SharedServerState.
    states: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    /// Registered event handlers with their associated state type.
    handlers: Vec<HandlerFn>,
    /// Synced elements that need to re-render on state change.
    synced_elements: Vec<SyncedElement>,
    /// Symbols that have been sent to this client (for incremental symbol updates).
    /// Maps symbol string -> symbol index (0x80+).
    sent_symbols: HashMap<String, u8>,
    /// Keys this connection is subscribed to (for cleanup on disconnect).
    #[allow(dead_code)]
    subscribed_keys: HashSet<String>,
}

impl ConnectionState {
    fn new(connection_id: u64) -> Self {
        Self {
            connection_id,
            states: HashMap::new(),
            handlers: Vec::new(),
            synced_elements: Vec::new(),
            sent_symbols: HashMap::new(),
            subscribed_keys: HashSet::new(),
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
        let handlers: Vec<_> = self.handlers.clone();
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
}

async fn handle_websocket<F>(
    ws_stream: async_tungstenite::WebSocketStream<TcpStream>,
    peer_addr: SocketAddr,
    root: Arc<F>,
    shared: Arc<SharedServerState>,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    F: Fn() -> ElementBuilder + Send + Sync + 'static,
{
    let (mut write, mut read) = ws_stream.split();

    // Allocate connection ID and register broadcast channel
    let connection_id = shared.next_connection_id();
    let (broadcast_tx, _broadcast_rx) = async_channel::bounded::<BroadcastMsg>(32);
    shared.register_connection(connection_id, broadcast_tx);

    // Create per-connection state
    let mut conn_state = ConnectionState::new(connection_id);

    // Build the root element
    let root_element = root();

    // First pass: collect handlers to find the state types
    let mut ctx = BuildContext::new();

    // Use a temporary unit state for the first pass to collect handlers
    let placeholder_state: () = ();
    ctx.collect_symbols(&root_element, &placeholder_state);
    ctx.emit(&root_element, &placeholder_state);

    // Extract handlers
    conn_state.handlers = ctx.handlers().to_vec();
    conn_state.synced_elements = ctx.take_synced_elements();

    // Initialize all state types from handlers and synced elements
    conn_state.initialize_all_states();

    // Now rebuild the DOM with all states available
    // Build a HashMap of all states for multi-state rendering
    let mut ctx = BuildContext::new();
    let states_map: HashMap<TypeId, &(dyn Any + Send + Sync)> = conn_state
        .states
        .iter()
        .map(|(k, v)| (*k, v.as_ref()))
        .collect();

    if states_map.is_empty() {
        // No states available, use placeholder
        ctx.collect_symbols(&root_element, &placeholder_state);
        ctx.emit(&root_element, &placeholder_state);
    } else {
        // Use multi-state methods to render all synced elements correctly
        ctx.collect_symbols_multi(&root_element, &states_map);
        ctx.emit_multi(&root_element, &states_map);
    }

    // Emit local handlers if any
    if ctx.has_local_handlers() {
        ctx.emit_local_handlers();
    }

    // Re-extract handlers and synced elements (they should be the same)
    conn_state.handlers = ctx.handlers().to_vec();
    conn_state.synced_elements = ctx.take_synced_elements();
    // Capture sent symbols for incremental updates
    conn_state.sent_symbols = ctx.take_symbol_map();

    let initial_dom = ctx.finish();

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
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Binary(data)) => match ClientEvent::decode(&data) {
                Ok(event) => {
                    println!(
                        "[{}] Event: handler=0x{:02X} type={} target_ref={}",
                        peer_addr,
                        event.handler_idx,
                        event.event_type_name(),
                        event.target_ref
                    );

                    let handler_idx = event.handler_idx as usize;
                    if handler_idx < conn_state.handlers.len() {
                        // Clone the handler to avoid borrowing issues
                        let handler = conn_state.handlers[handler_idx].clone();

                        // Ensure state for this handler's type is initialized
                        conn_state.ensure_state_initialized_for(&handler);

                        // Create EventContext from payload and param_bytes
                        let ctx = EventContext::new_with_params(event.payload, event.param_bytes);

                        // Get the state for this handler's type
                        let state_type_id = handler.state_type_id();
                        if let Some(state) = conn_state.get_state_mut(state_type_id) {
                            // Call the handler with context
                            handler.call_with_context(state, &ctx);
                        }

                        // Re-render synced elements using multi-state support
                        // Only re-render elements whose dependencies overlap with changed fields
                        let changes = handler.changes();
                        let states_map: HashMap<TypeId, &(dyn Any + Send + Sync)> = conn_state
                            .states
                            .iter()
                            .map(|(k, v)| (*k, v.as_ref()))
                            .collect();
                        // Use incremental symbols - pass known symbols and update them
                        let update = build_synced_update_with_known_symbols(
                            &conn_state.synced_elements,
                            &states_map,
                            &mut conn_state.handlers,
                            changes,
                            Some(&mut conn_state.sent_symbols),
                        );
                        if !update.is_empty() {
                            write.send(Message::Binary(update.to_vec())).await?;
                        }
                    } else {
                        eprintln!(
                            "[{}] No handler registered for index {}",
                            peer_addr, event.handler_idx
                        );
                    }
                }
                Err(e) => {
                    eprintln!("[{}] Failed to decode event: {}", peer_addr, e);
                }
            },
            Ok(Message::Text(text)) => {
                println!("[{}] Text message (unexpected): {}", peer_addr, text);
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

    // Cleanup: unregister connection
    shared.unregister_connection(conn_state.connection_id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
