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
    /// (texture tables, sprite tables, font tables, color tables, cached images).
    ///
    /// Has access to the freshly initialized state for sending state-dependent
    /// setup data (e.g., minimap terrain).
    ///
    /// Default implementation does nothing. Override to send asset manifests.
    fn setup(&self, state: &Self::State, buf: &mut CanvasBuffer) {
        let _ = (state, buf);
    }
}

use crate::scene::Scene;

/// Extended trait for retained-mode scene rendering.
///
/// Games implement this instead of `GameLoop` to use the retained sprite system.
/// The framework automatically diffs scene state against the client view and
/// sends minimal opcodes.
pub trait SceneLoop: Send + Sync + 'static {
    /// The per-connection game state type.
    type State: Send + 'static;

    /// Create initial game state for a new connection.
    fn init(&self) -> Self::State;

    /// Called once at connection start to set up layers, tilemaps, minimap,
    /// and initial sprite population.
    fn setup_scene(&self, state: &mut Self::State, scene: &mut Scene, buf: &mut CanvasBuffer);

    /// Advance game state by one tick.
    fn tick(&self, state: &mut Self::State, input: &InputState, dt: f32);

    /// Update the scene graph to reflect current game state.
    /// Create/move/delete sprites as needed. The framework diffs this
    /// against the client view and sends only changes.
    fn update_scene(&self, state: &Self::State, scene: &mut Scene);

    /// Camera position for this frame: (cx, cy, zoom).
    /// Zoom is in natural units (1.0 = no zoom).
    fn camera(&self, state: &Self::State) -> (f32, f32, f32);

    /// Immediate-mode overlay rendering (HUD, menus, HP bars).
    /// These commands are sent every frame in screen space.
    fn render_overlay(&self, state: &Self::State, buf: &mut CanvasBuffer);
}
