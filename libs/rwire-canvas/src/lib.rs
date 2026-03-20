//! rwire-canvas: Binary Canvas 2D opcode protocol for real-time rendering.
//!
//! This crate provides a compact binary protocol for streaming Canvas 2D
//! draw commands from a server to a browser over WebSocket. It's designed
//! for games and real-time visualizations running on the rwire architecture.
//!
//! # Architecture
//!
//! ```text
//! Server (Rust, 20Hz)                     Browser (JS ~3KB, 60fps)
//! ┌──────────────────────┐                ┌──────────────────────┐
//! │ GameLoop::tick()     │  ──binary──>   │ Canvas 2D Executor   │
//! │ GameLoop::render()   │  ──frames──>   │ Entity Interpolation │
//! │ CanvasBuffer encoder │  <──input───   │ Input @ 20Hz         │
//! └──────────────────────┘                └──────────────────────┘
//! ```
//!
//! # Example
//!
//! ```ignore
//! use rwire_canvas::{CanvasServer, GameLoop, CanvasBuffer, InputState};
//!
//! struct MyGame;
//!
//! impl GameLoop for MyGame {
//!     type State = u32; // simple counter
//!
//!     fn init(&self) -> u32 { 0 }
//!
//!     fn tick(&self, state: &mut u32, input: &InputState, _dt: f32) {
//!         *state += 1;
//!     }
//!
//!     fn render(&self, state: &u32, buf: &mut CanvasBuffer) {
//!         buf.clear()
//!            .set_fill_rgb(255, 255, 255)
//!            .fill_text(100, 100, &format!("Tick: {state}"));
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     CanvasServer::bind("127.0.0.1:9000").unwrap()
//!         .canvas_size(960, 640)
//!         .run(MyGame)
//!         .await
//!         .unwrap();
//! }
//! ```

pub mod capsule;
pub mod color;
pub mod game_loop;
pub mod input;
pub mod protocol;
pub mod runtime;
pub mod server;
pub mod sprite;

pub use color::Color;
pub use game_loop::GameLoop;
pub use input::InputState;
pub use protocol::encoder::CanvasBuffer;
pub use server::CanvasServer;
pub use sprite::{SpriteId, SpriteRect, SpriteSheet, TextureId};
