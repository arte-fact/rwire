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

Registers a `Router` that drives the `outlet()` runtime for URL pattern matching. See the [Router](/docs/05-advanced/router) page for the two navigation models.

### WebSocket origin validation

Browser WebSocket handshakes carry an `Origin` header; the server rejects any
that isn't same-origin with the request `Host` (403, before the upgrade) — the
standard cross-site WebSocket hijacking defense. Non-browser clients without an
`Origin` header are unaffected.

For a legitimate cross-origin setup (a page on another domain connecting to
this app), allow that page's origin explicitly:

```rust
use rwire::ServerConfig;

Server::bind("0.0.0.0:9000")?
    .root(root)
    .config(ServerConfig::new().allow_origin("https://embed.example.com"))
    .run()
    .await
```

Same-origin comparison normalizes default ports (80/443) and is
case-insensitive; behind a reverse proxy it compares against the forwarded
`Host`, which matches the page origin in a standard same-origin deployment.

## Capsule Configuration

```rust
use rwire::capsule_gen::{CapsuleConfig, FontFace};
use rwire::theme::{Theme, ThemeStyle};

let config = CapsuleConfig::new()
    .theme(
        Theme::dark()
            .accent("#5E81AC")
            .style(ThemeStyle::soft())
    )
    .font(FontFace::google("Inter", &[400, 500, 600]));
```

### .theme(Theme)

Sets the visual theme. The `Theme` struct controls mode (light/dark), border radius scale, style preset, color palette, and seed colors:

```rust
use rwire::theme::{Theme, RadiusScale};
use rwire_themes::styles;

Theme::dark()
    .accent("#5E81AC")
    .radius(RadiusScale::Large)
    .style(styles::brutalist())
```

Color configuration methods on `Theme`:

- `.accent(color)` -- Sets accent color from hex or oklch string (auto-generates 12-step scale)
- `.neutral(color)` -- Sets neutral color from hex or oklch string
- `.error(color)` / `.success(color)` / `.warning(color)` -- Sets status colors
- `.palette(ColorPalette)` -- Full palette override (e.g., `rwire_themes::palettes::nord()`)

Apply a named palette with `Theme::dark().palette(rwire_themes::palettes::nord())`. The
`rwire-themes` crate ships nord, indigo, catppuccin, dracula, solarized, gruvbox, tokyo_night,
rose_pine, and one_dark.

### .font(FontFace)

Adds a font to the capsule. Multiple fonts can be added by chaining `.font()` calls:

```rust
CapsuleConfig::new()
    .font(FontFace::google("Inter", &[400, 600]))
    .font(FontFace::google("JetBrains Mono", &[400]))
```

### Static first paint (SSR)

```rust
.capsule_config(CapsuleConfig::new().ssr(true))
```

With `ssr(true)`, the capsule ships the root tree rendered at its **default
state** (synced regions included), plus exactly the utility CSS those classes
need — so crawlers and no-JS clients see real content, and humans get a paint
before the WebSocket connects. The live render replaces it on the first
frame. Current limitation: every path serves the root's static paint (routed
views render after connect).

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
