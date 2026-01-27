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
pub mod protocol;
pub mod server;
pub mod state;

// Builder API exports
pub use builder::{el, ElementBuilder};

// State exports
pub use state::{ClientState, HandlerFn};

// Protocol exports
pub use protocol::{ClientEvent, DecodeError, El, Ev, OpcodeBuffer};

// Server exports
pub use server::Server;

// Macro re-exports
pub use rwire_macros::{handler, renderer, ClientState};

// Re-export common types for convenience
pub use bytes::Bytes;
