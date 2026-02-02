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
use std::collections::HashMap;
use std::error::Error;
use std::net::{AddrParseError, SocketAddr};
use std::sync::Arc;

use crate::builder::{
    build_synced_update_with_known_symbols, BuildContext, ElementBuilder, SyncedElement,
};
use crate::capsule;
use crate::capsule_gen;
use crate::protocol::ClientEvent;
use crate::state::{EventContext, HandlerFn};

/// Server builder - first step.
pub struct ServerBuilder {
    addr: SocketAddr,
}

/// Server with root element configured, ready to run.
pub struct ServerWithRoot<F> {
    addr: SocketAddr,
    root: F,
}

impl Server {
    /// Start building a server bound to the given address.
    pub fn bind(addr: &str) -> Result<ServerBuilder, AddrParseError> {
        Ok(ServerBuilder {
            addr: addr.parse()?,
        })
    }
}

/// Marker type for the Server namespace.
pub struct Server;

impl ServerBuilder {
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
            root: f,
        }
    }
}

impl<F> ServerWithRoot<F>
where
    F: Fn() -> ElementBuilder + Send + Sync + 'static,
{
    /// Run the server, accepting connections until shutdown.
    pub async fn run(self) -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(self.addr).await?;

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
            task::spawn(async move {
                handle_client(stream, peer_addr, root, capsule).await;
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
                if let Err(e) = handle_websocket(ws_stream, peer_addr, root).await {
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
    /// State values keyed by TypeId, supporting multiple state types per connection.
    states: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    /// Registered event handlers with their associated state type.
    handlers: Vec<HandlerFn>,
    /// Synced elements that need to re-render on state change.
    synced_elements: Vec<SyncedElement>,
    /// Symbols that have been sent to this client (for incremental symbol updates).
    /// Maps symbol string -> symbol index (0x80+).
    sent_symbols: HashMap<String, u8>,
}

impl ConnectionState {
    fn new() -> Self {
        Self {
            states: HashMap::new(),
            handlers: Vec::new(),
            synced_elements: Vec::new(),
            sent_symbols: HashMap::new(),
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
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    F: Fn() -> ElementBuilder + Send + Sync + 'static,
{
    let (mut write, mut read) = ws_stream.split();

    // Create per-connection state
    let mut conn_state = ConnectionState::new();

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

    Ok(())
}
