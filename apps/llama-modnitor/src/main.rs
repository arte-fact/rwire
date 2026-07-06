//! Node monitor: real-time CPU, memory, and GPU dashboard built on rwire.
//!
//! A background thread polls hardware into a shared snapshot; mutating that
//! shared state notifies connections, so rwire diffs and pushes the changes and
//! every connected browser sees live readings without any client-side polling.

#![warn(clippy::pedantic, clippy::nursery)]

mod convert;
mod cpu;
mod gpu;
mod launcher;
mod mem;
mod modelmeta;
mod models;
mod profiles;
mod snapshot;
mod ui;
mod vram;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use mimalloc::MiMalloc;
use rwire::capsule_gen::CapsuleConfig;
use rwire::theme::Theme;
use rwire::{Server, theme};
use rwire_themes::palettes;

/// mimalloc gives a measurable speedup on allocation-heavy render paths.
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

/// Default bind address (all interfaces; reverse-proxied publicly). Override
/// with the `BIND_ADDR` env var.
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:7778";

/// How often the poller reads hardware. 1s matches typical monitoring cadence
/// and keeps `*-smi` invocation overhead negligible.
const POLL_INTERVAL: Duration = Duration::from_secs(1);

#[theme]
fn app_theme() -> Theme {
    Theme::dark().palette(palettes::nord())
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from a .env file if present (see .env.example).
    // Real environment variables already set take precedence over the file.
    if let Ok(path) = dotenvy::dotenv() {
        println!("Loaded env from {}", path.display());
    }

    // GPU_BACKEND overrides auto-detection: "auto", "rocm", "nvidia", or "none".
    let force = std::env::var("GPU_BACKEND").unwrap_or_else(|_| "auto".to_string());
    let backend = gpu::detect_backend(&force);
    println!("GPU backend: {}", backend.name());

    let mut cpu = cpu::Cpu::new();
    // The per-core bars pick a severity background (BgSuccess/Warning/Error) at
    // runtime from a plain helper. With lazy CSS those rules are delivered over
    // the wire on first use, so no `extra_styles` declaration is needed.
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string());
    let mut server = Server::bind(&bind_addr)?
        .root(ui::app)
        .capsule_config(CapsuleConfig::new())
        .theme(app_theme());

    // Optional auth gate. This binds publicly and launch/stop/kill take real
    // actions, so set AUTH_USER + AUTH_PASSWORD (both non-empty) to require a
    // login (form + session cookie) for the page and the WebSocket; unset =
    // open (local dev).
    match (std::env::var("AUTH_USER"), std::env::var("AUTH_PASSWORD")) {
        (Ok(user), Ok(password)) if !user.is_empty() && !password.is_empty() => {
            server = server.auth(user, password);
            println!("Auth: login required (AUTH_USER/AUTH_PASSWORD set)");
        }
        _ => println!("Auth: OPEN — set AUTH_USER and AUTH_PASSWORD to require login"),
    }

    let shared = server.shared_state();

    // Prime the shared App's hardware snapshot so the first browser to connect
    // sees real readings immediately (selection starts empty).
    // Discover local models once at startup. MODELS_DIR is configurable
    // (default "./models"); a missing dir simply yields no models.
    let models_dir = std::env::var("MODELS_DIR").unwrap_or_else(|_| "./models".to_string());
    let discovered = models::discover(Path::new(&models_dir));
    println!("Models: {} found under {}", discovered.len(), models_dir);

    // Saved node profiles (configurable via NODE_PROFILES_DIR; default ./profiles).
    let saved_profiles = profiles::list();
    println!("Profiles: {} saved", saved_profiles.len());

    shared.update_shared::<snapshot::App>(|app| {
        app.hw = snapshot::collect(&mut cpu, backend.as_ref());
        // Snapshot the device list once for the form's device editor: every GPU
        // plus the host CPU, so a node's devices (and class) can be re-picked.
        let mut devices: Vec<snapshot::DeviceInfo> = app
            .hw
            .gpus
            .iter()
            .map(|g| snapshot::DeviceInfo {
                key: g.key(),
                label: g.name.clone(),
            })
            .collect();
        devices.push(snapshot::DeviceInfo {
            key: snapshot::CPU_KEY.to_string(),
            label: "CPU (host)".to_string(),
        });
        app.devices = devices;
        app.models = discovered;
        app.profiles = saved_profiles;
    });

    // Start the engine supervisor (owns launched child processes).
    launcher::init(Arc::clone(&shared));

    // The poller mutates shared state and notifies connections to re-render;
    // rwire diffs and pushes the changes — no client-side polling, no tick.
    let poll_backend = Arc::clone(&backend);
    let poll_shared = Arc::clone(&shared);
    std::thread::spawn(move || {
        let mut cpu = cpu;
        snapshot::run_poller(&mut cpu, poll_backend.as_ref(), &poll_shared, POLL_INTERVAL);
    });

    server.run().await
}
