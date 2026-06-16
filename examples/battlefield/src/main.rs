//! The Battlefield — server-side real-time tactical strategy game.
//!
//! All game logic runs on the server. Canvas draw commands are streamed
//! to the browser over WebSocket via the rwire-canvas binary protocol.

use rwire_canvas::{CanvasBuffer, CanvasServer, Scene, SceneLoop, InputState};

mod autotile;
mod combat;
mod flowfield;
mod grid;
mod mapgen;
mod particle;
mod render;
mod rng;
mod sprites;
mod state;
mod unit;

use state::GameState;

struct BattlefieldGame;

impl SceneLoop for BattlefieldGame {
    type State = GameState;

    fn init(&self) -> GameState {
        GameState::new()
    }

    fn setup_scene(&self, state: &mut Self::State, _scene: &mut Scene, buf: &mut CanvasBuffer) {
        buf.texture_table(&sprites::texture_table());
        buf.font_table(&[
            "bold 14px sans-serif",   // 0: UI labels
            "bold 11px monospace",    // 1: zone labels
            "bold 52px sans-serif",   // 2: title
            "bold 20px sans-serif",   // 3: buttons
            "13px monospace",         // 4: hints
            "bold 56px sans-serif",   // 5: death/victory title
        ]);
        let mut sprite_rects = sprites::decoration_sprite_table();
        sprite_rects.extend(sprites::unit_sprite_table());
        buf.sprite_table(&sprite_rects);
        render::send_minimap_base(state, buf);
        render::send_terrain_layer(state, buf);
        // send_decoration_sprites returns tree sprite IDs for per-frame alpha updates
        // We can't mutate state here (it's &Self::State), so we store in a separate init
        render::send_decoration_sprites(state, buf);
    }

    fn tick(&self, state: &mut GameState, input: &InputState, dt: f32) {
        state.tick(input, dt);
    }

    fn update_scene(&self, state: &Self::State, scene: &mut Scene) {
        render::update_unit_sprites(state, scene);
    }

    fn camera(&self, state: &Self::State) -> (f32, f32, f32) {
        (state.camera_x, state.camera_y, state.camera_zoom)
    }

    fn render_overlay(&self, state: &Self::State, buf: &mut CanvasBuffer) {
        render::render_overlay(state, buf);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("The Battlefield");
    println!("Open http://0.0.0.0:9000 in your browser");

    CanvasServer::bind("0.0.0.0:9000")?
        .canvas_size(960, 640)
        .tick_rate(60)
        .title("The Battlefield")
        .static_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/"))
        .run_scene(BattlefieldGame)
        .await
}
