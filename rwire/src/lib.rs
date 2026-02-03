//! rwire - WebSocket server with binary DOM opcodes and reactive state.
//!
//! This library provides:
//! - A binary protocol for efficient DOM manipulation
//! - WebSocket server infrastructure
//! - HTML capsule serving for the client-side runtime
//! - Fluent builder API for constructing components
//! - Reactive state management with synced elements
//!
//! # Example
//!
//! ```ignore
//! use rwire::{el, El, Ev, Server, ClientState, handler, renderer};
//!
//! #[derive(ClientState, Default)]
//! struct Counter {
//!     count: i32,
//! }
//!
//! #[async_std::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     Server::bind("127.0.0.1:9000")?
//!         .root(build_counter)
//!         .run()
//!         .await
//! }
//!
//! fn build_counter() -> ElementBuilder {
//!     el(El::Div).class("counter").append([
//!         el(El::Button).text("-").on(Ev::Click, decrement),
//!         render_count(),
//!         el(El::Button).text("+").on(Ev::Click, increment),
//!     ])
//! }
//!
//! #[renderer]
//! fn render_count(state: &Counter) -> ElementBuilder {
//!     el(El::Span).text(&state.count.to_string())
//! }
//!
//! #[handler]
//! fn increment(state: &mut Counter) {
//!     state.count += 1;
//! }
//!
//! #[handler]
//! fn decrement(state: &mut Counter) {
//!     state.count -= 1;
//! }
//! ```

pub mod builder;
pub mod capsule;
pub mod capsule_gen;
pub mod config;
pub mod form;
pub mod health;
pub mod item_ref;
pub mod metrics;
pub mod protocol;
pub mod registry;
pub mod router;
pub mod server;
pub mod session;
pub mod state;
pub mod store;
pub mod style;

// Builder API exports
pub use builder::{el, ElementBuilder};

// Item reference exports
pub use item_ref::{ItemRef, IterWithRef};

// State exports
#[allow(deprecated)]
pub use state::ClientState;
pub use state::{
    ChangeSet, EventContext, EventPayload, HandlerFn, HandlerSpec, LocalMutations, LocalState,
    MemoryState, Mutation, PersistedState, RendererDeps, State, StorageType,
};

// Protocol exports
pub use protocol::{ClientEvent, DecodeError, El, Ev, OpcodeBuffer};

// Server exports
pub use server::{BroadcastMsg, Server, SharedServerState};

// Store exports
pub use store::{JsonFileStore, MemoryStore, StateStore, StoreError};

// Config exports
pub use config::ServerConfig;

// Registry exports
pub use registry::{AdmissionError, ConnectionGuard, ConnectionRegistry};

// Health exports
pub use health::{HealthResponse, HealthStatus, ReadyResponse};

// Form exports
pub use form::{Field, FieldType, Form, ValidationRule};

// Router exports
pub use router::{Link, Route, RoutePattern, Router};

// Style exports
pub use style::{ScopedClass, Style};

// Metrics exports
pub use metrics::{Counter, Gauge, Histogram, Metrics};

// Session exports
pub use session::{Session, SessionId};

// Macro re-exports
#[allow(deprecated)]
pub use rwire_macros::ClientState;
pub use rwire_macros::{handler, renderer, State};

// Re-export common types for convenience
pub use bytes::Bytes;
