//! Token Counter Example - Phase 1 Static Rendering
//!
//! This example demonstrates loading a YAML app and rendering it
//! using rwire's ElementBuilder. In Phase 1, this is static only -
//! buttons won't function until Phase 2 adds handler execution.

use rwire::builder::ElementBuilder;
use rwire::server::Server;
use rwire_token::{load_yaml, DynamicState, EvalContext, Interpreter};
use std::path::PathBuf;
use std::sync::Arc;

// Store app and state globally for the render function
// In Phase 2, this will be part of TokenRuntime
static mut APP: Option<Arc<rwire_token::AppToken>> = None;
static mut STATE: Option<Arc<DynamicState>> = None;

fn render_from_yaml() -> ElementBuilder {
    // SAFETY: These are set once at startup before server runs
    unsafe {
        let app = APP.as_ref().expect("APP not initialized");
        let state = STATE.as_ref().expect("STATE not initialized");

        let interpreter = Interpreter::new(app, state);
        let ctx = EvalContext::new(state).with_session("static-session");

        interpreter
            .render_page("home", &ctx)
            .expect("Failed to render home page")
    }
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load YAML app
    let yaml_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("app.yaml");
    let app = load_yaml(&yaml_path).expect("Failed to load app.yaml");

    println!("Loaded app: {}", app.version);
    println!(
        "Pages: {:?}",
        app.ui.pages.keys().collect::<Vec<_>>()
    );
    println!(
        "Routes: {:?}",
        app.routes.iter().map(|r| &r.path).collect::<Vec<_>>()
    );

    // Create state
    let state = Arc::new(DynamicState::from_schema(&app.state));
    let app = Arc::new(app);

    // Store for render function
    // SAFETY: Set once before server runs
    unsafe {
        APP = Some(Arc::clone(&app));
        STATE = Some(Arc::clone(&state));
    }

    println!("\nStarting server at http://127.0.0.1:9000");
    println!("Note: Buttons are not functional in Phase 1 (static rendering)");

    // Start server
    Server::bind("127.0.0.1:9000")?
        .root(render_from_yaml)
        .run()
        .await
}
