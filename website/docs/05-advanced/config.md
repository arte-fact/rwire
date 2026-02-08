---
title: Configuration
description: Server and capsule configuration options
order: 5
---
# Configuration

rwire configuration happens at two levels: the server (bind address, routes, persistence) and the capsule (theme, fonts, tree-shaking). Both use builder APIs.

## Server Configuration

```rust
use rwire::server::Server;
use rwire::router::Router;

Server::bind("0.0.0.0:9000")?
    .root(root)
    .on_route(handle_route())
    .routes(
        Router::new()
            .page("/", |_| build_home())
            .page("/docs/*", |p| build_docs(p))
    )
    .run()
    .await
```

### Server::bind

Sets the TCP address and port. Use `"0.0.0.0:9000"` to accept connections from any interface, or `"127.0.0.1:9000"` for localhost only.

```rust
Server::bind("0.0.0.0:9000")?   // all interfaces
Server::bind("127.0.0.1:3000")? // localhost only
```

### .root(fn)

The root function returns the top-level `ElementBuilder`. It is called once per WebSocket connection to build the initial DOM:

```rust
fn root() -> ElementBuilder {
    el(El::Div).id("app").append([
        nav(),
        render_content(),
        footer(),
    ])
}
```

### .on_route(handler)

Registers a handler that fires on every client-side navigation. The handler receives the new path via `ctx.text()`:

```rust
#[handler]
fn handle_route(state: &mut AppState, ctx: &EventContext) {
    if let Some(path) = ctx.text() {
        state.current_path = path.to_string();
    }
}
```

### .routes(router)

Registers a `Router` for URL pattern matching and automatic tree shaking. See the [Router](/docs/advanced/router) page for details.

## Capsule Configuration

```rust
use rwire::capsule_gen::{CapsuleConfig, FontFace};
use rwire::theme::{Theme, AccentColor, ThemeStyle};
use rwire::tokens::ColorPalette;

let config = CapsuleConfig::new()
    .theme(
        Theme::dark()
            .with_accent(AccentColor::Green)
            .with_style(ThemeStyle::Soft)
    )
    .palette(ColorPalette::nord())
    .font(FontFace::google("Inter", &[400, 500, 600]))
    .has_local_handlers(true);
```

### .theme(Theme)

Sets the visual theme. The `Theme` struct controls mode (light/dark), accent color, border radius scale, and style preset:

```rust
Theme::dark()
    .with_accent(AccentColor::Amber)
    .with_radius(RadiusScale::Large)
    .with_style(ThemeStyle::Brutalist)
```

### .palette(ColorPalette)

Overrides the default Oklch color scales with a custom palette. Use `ColorPalette::nord()` for the built-in Nord preset, or build a custom palette with `ColorScale::from_hex()`.

### .font(FontFace)

Adds a font to the capsule. Multiple fonts can be added by chaining `.font()` calls:

```rust
CapsuleConfig::new()
    .font(FontFace::google("Inter", &[400, 600]))
    .font(FontFace::google("JetBrains Mono", &[400]))
```

### .has_local_handlers(bool)

When `true`, includes the local mutation interpreter (~150 bytes) in the capsule JS. Required when using `#[storage(local)]` state.

## Health Check Endpoints

rwire provides built-in HTTP health check endpoints:

| Endpoint | Purpose |
|----------|---------|
| `/health` | Liveness check -- returns 200 if the server process is running |
| `/ready` | Readiness check -- returns 200 when the server can accept connections |

These are served over the same port as the WebSocket connection and are useful for load balancer and Kubernetes health probes.

## Session Management

Each WebSocket connection gets a unique session ID. Sessions are used to:

- Route events to the correct connection
- Scope server-side state per client
- Track symbol tables (string interning is per-session)

Session IDs are generated server-side and communicated via cookies. No session configuration is needed for typical applications.
