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
pub mod persist;
pub mod protocol;
pub mod registry;
pub mod router;
pub mod server;
pub mod session;
pub mod state;
pub mod store;
pub mod style;
pub mod theme;
pub mod tokens;
pub mod variants;
pub mod components;

// Builder API exports
pub use builder::{el, ElementBuilder};

// Item reference exports
pub use item_ref::{ItemRef, IterWithRef};

// State exports
#[allow(deprecated)]
pub use state::ClientState;
pub use state::{
    get_local_state_default_json, register_local_state_default, ChangeSet, EventContext,
    EventPayload, HandlerFn, HandlerSpec, LocalMutations, LocalState, LocalStateJson, MemoryState,
    Mutation, PersistedState, RendererDeps, State, StorageType,
};

// Protocol exports
pub use protocol::{ClientEvent, DecodeError, El, Ev, OpcodeBuffer};

// Server exports
pub use server::{persist_task, BroadcastMsg, Server, SharedServerState};

// Store exports
pub use store::{JsonFileStore, MemoryStore, StateStore, StoreError};

// Persist exports
pub use persist::{PersistError, PersistRegistry, PersistableType, SqliteStore};

// Re-export rusqlite for use in load_fn/save_fn implementations
pub use rusqlite;

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

// Token exports
pub use tokens::{color, font_size, font_weight, line_height, radius, shadow, space, transition};
pub use tokens::css::{generate_primitive_css, minify_css};

// Theme exports
pub use theme::{
    generate_accent_css, generate_base_css, generate_radius_css, generate_semantic_css,
    generate_theme_css, AccentColor, RadiusScale, Theme, ThemeMode,
};

// Variant exports
pub use variants::{Variant, VariantComponent};

// Component exports
pub use components::{generate_components_css, Button, ButtonIntent, ButtonSize};

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
