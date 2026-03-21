//! The Battlefield — server-side real-time tactical strategy game.
//!
//! All game logic runs on the server. Canvas draw commands are streamed
//! to the browser over WebSocket via the rwire-canvas binary protocol.

use rwire_canvas::{CanvasBuffer, CanvasServer, GameLoop, InputState};

mod autotile;
mod combat;
mod flowfield;
mod grid;
mod mapgen;
mod particle;
mod render;
mod sprites;
mod state;
mod unit;

use state::GameState;

struct BattlefieldGame;

impl GameLoop for BattlefieldGame {
    type State = GameState;

    fn init(&self) -> GameState {
        GameState::new()
    }

    fn setup(&self, buf: &mut CanvasBuffer) {
        buf.texture_table(&sprites::texture_table());
        buf.font_table(&[
            "bold 14px sans-serif",   // 0: UI labels
            "bold 11px monospace",    // 1: zone labels
            "bold 52px sans-serif",   // 2: title
            "bold 20px sans-serif",   // 3: buttons
            "13px monospace",         // 4: hints
            "bold 56px sans-serif",   // 5: death/victory title
        ]);
    }

    fn tick(&self, state: &mut GameState, input: &InputState, dt: f32) {
        state.tick(input, dt);
    }

    fn render(&self, state: &GameState, buf: &mut CanvasBuffer) {
        render::render_frame(state, buf);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("The Battlefield");
    println!("Open http://127.0.0.1:9000 in your browser");

    CanvasServer::bind("127.0.0.1:9000")?
        .canvas_size(960, 640)
        .tick_rate(20)
        .title("The Battlefield")
        .static_dir(".")
        .run(BattlefieldGame)
        .await
}
