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
use std::any::Any;
use std::error::Error;
use std::net::{AddrParseError, SocketAddr};
use std::sync::Arc;

use crate::builder::{build_synced_update, BuildContext, ElementBuilder, SyncedElement};
use crate::capsule;
use crate::capsule_gen;
use crate::protocol::ClientEvent;
use crate::state::HandlerFn;

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
        Ok(ServerBuilder { addr: addr.parse()? })
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
        let capsule = capsule_gen::generate_capsule(ctx.used_elements(), ctx.used_events());
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

/// Per-connection state container.
struct ConnectionState {
    /// The actual state value, type-erased.
    state: Box<dyn Any + Send + Sync>,
    /// Registered event handlers.
    handlers: Vec<HandlerFn>,
    /// Synced elements that need to re-render on state change.
    synced_elements: Vec<SyncedElement>,
}

impl ConnectionState {
    fn new() -> Self {
        Self {
            state: Box::new(()),
            handlers: Vec::new(),
            synced_elements: Vec::new(),
        }
    }

    /// Check if state is initialized (not placeholder).
    fn is_state_initialized(&self) -> bool {
        self.state.downcast_ref::<()>().is_none()
    }

    /// Initialize state using a handler's factory if not already initialized.
    fn ensure_state_initialized(&mut self) {
        if !self.is_state_initialized() {
            if let Some(first_handler) = self.handlers.first() {
                self.state = first_handler.create_state();
            }
        }
    }

    /// Get state as Any for type-erased access.
    fn state_as_any(&self) -> &dyn Any {
        self.state.as_ref()
    }

    /// Get mutable state as Any for type-erased access.
    fn state_as_any_mut(&mut self) -> &mut dyn Any {
        self.state.as_mut()
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

    // First pass: collect handlers to find the state type
    let mut ctx = BuildContext::new();

    // Use a temporary unit state for the first pass to collect handlers
    let placeholder_state: () = ();
    ctx.collect_symbols(&root_element, &placeholder_state);
    ctx.emit(&root_element, &placeholder_state);

    // Extract handlers
    conn_state.handlers = ctx.handlers().to_vec();
    conn_state.synced_elements = ctx.take_synced_elements();

    // Initialize state from the first handler's factory
    conn_state.ensure_state_initialized();

    // Now rebuild the DOM with the actual state
    let mut ctx = BuildContext::new();
    ctx.collect_symbols(&root_element, conn_state.state_as_any());
    ctx.emit(&root_element, conn_state.state_as_any());

    // Re-extract handlers and synced elements (they should be the same)
    conn_state.handlers = ctx.handlers().to_vec();
    conn_state.synced_elements = ctx.take_synced_elements();

    let initial_dom = ctx.finish();

    println!(
        "[{}] Sending initial DOM ({} bytes, {} handlers, {} synced)",
        peer_addr,
        initial_dom.len(),
        conn_state.handlers.len(),
        conn_state.synced_elements.len()
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

                        // Ensure state is initialized
                        conn_state.ensure_state_initialized();

                        // Call the handler
                        handler.call(conn_state.state_as_any_mut());

                        // Re-render synced elements
                        let update =
                            build_synced_update(&conn_state.synced_elements, conn_state.state_as_any());
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
