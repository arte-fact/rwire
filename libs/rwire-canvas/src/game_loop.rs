//! GameLoop trait for server-side game logic.
//!
//! Implement this trait to define your game's init/tick/render cycle.
//! The canvas server calls these methods in a fixed-interval loop per connection.

use crate::input::InputState;
use crate::protocol::encoder::CanvasBuffer;

/// Trait for server-side game logic.
///
/// Each connected client gets its own state instance and runs an independent
/// game loop at the configured tick rate.
pub trait GameLoop: Send + Sync + 'static {
    /// The per-connection game state type.
    type State: Send + 'static;

    /// Create initial game state for a new connection.
    fn init(&self) -> Self::State;

    /// Advance game state by one tick.
    ///
    /// Called at the configured tick rate (default 20Hz = 50ms).
    /// `dt` is the time step in seconds (typically 0.05).
    fn tick(&self, state: &mut Self::State, input: &InputState, dt: f32);

    /// Render the current game state into canvas draw commands.
    ///
    /// Called once per tick after `tick()`. The produced bytes are sent
    /// to the client over WebSocket.
    fn render(&self, state: &Self::State, buf: &mut CanvasBuffer);

    /// Called once at connection start to send initial setup data
    /// (texture tables, sprite tables, font tables, color tables).
    ///
    /// Default implementation does nothing. Override to send asset manifests.
    fn setup(&self, buf: &mut CanvasBuffer) {
        let _ = buf;
    }
}
