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

### .asset_dir(prefix, dir)

Serves a folder from disk, read on each request, so files added while the
server runs (the images of a new article) are served without a restart:

```rust
Server::bind("0.0.0.0:9000")?
    .root(root)
    .asset_dir("/media", "data/media")         // behind the auth gate
    .public_asset_dir("/brand", "data/brand")  // to every visitor
```

`GET /media/2026/cover.png` reads `data/media/2026/cover.png`.

- **Auth.** With `auth()` or `auth_with()`, an `asset_dir` needs the same
  session as the page; without a gate it is open. `public_asset_dir` is served
  before the gate, to everyone. (Fixed files from `.asset(..)` are always
  served before the gate.)
- **What is refused (404).** `..` in any encoding (`%2e%2e`, double-encoded
  `%252e`, encoded slashes), hidden files (`.env`), NUL and control bytes,
  symbolic links that lead out of the folder, and directories: there is no
  listing. A link that stays inside the folder is followed.
- **Content-Type.** From an allowlist of media extensions (images, video,
  audio, fonts, PDF, plain text, subtitles); anything else, including `.html`,
  `.js` and `.css`, is `application/octet-stream`. Every response carries
  `X-Content-Type-Options: nosniff` and a sandboxing
  `Content-Security-Policy`, so an SVG or HTML file opened directly runs no
  script.
- **HTTP.** `ETag` and `Last-Modified`, answered with `304`;
  `Cache-Control: no-cache` (kept, but revalidated: the file may change);
  a single `Range` (`206`, or `416`), so videos can seek; `HEAD`. Bodies are
  streamed from the file, never read whole into memory.
- **Limit.** Between the containment check and the open, a writer of the
  folder could swap a file for a symbolic link: whoever can write the folder
  is trusted.

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

### .title(text)

```rust
CapsuleConfig::new().title("Office — agents")
```

The page's `<title>` (escaped), in the `ssr(true)` paint too; with a PWA it
replaces the title taken from the PWA name. One title for the whole app: it
doesn't change with client-side navigation (that would need the runtime to set
`document.title` on each route change), so there is no per-route title, not
even in the SSR paint, which would then disagree with the page after the first
navigation.

### .favicon(bytes, content_type)

```rust
CapsuleConfig::new().favicon(include_bytes!("../assets/icon.png"), "image/png")
```

Serves the icon at `/favicon.ico` (before any auth gate, like `.asset(..)`)
and links it from the page's `<head>`. Without it, the browser's request for
`/favicon.ico` gets a short 404. With a PWA, this icon is the page's
`<link rel="icon">`; the PWA icons still serve the manifest and home screen.

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

With `ssr(true)`, every page load ships the root tree rendered for the
**requested route** (the router's `outlet()` renders the matched view), plus
exactly the utility CSS those classes need — so crawlers and no-JS clients see
real content, and humans get a paint before the WebSocket connects. The live
render replaces it on the first frame.

Which state the paint shows:

- `#[storage(shared)]` state: its **current** value, the same for every visitor.
- `#[storage(memory)]` and `#[storage(persisted)]` state: the **default**.
  They belong to a connection or a session, which a plain `GET` doesn't have
  yet; the live render fills them in.

The paint is rendered without a connection: the app's `on_route` handler
doesn't run, and there is no visitor identity. A view that hides shared data
behind connection state (a login flow in `on_route`, a role check on memory
state) would show that data in the paint. Leave such routes out; they are
served with an empty mount and render live only:

```rust
CapsuleConfig::new()
    .ssr(true)
    .ssr_exclude("/account/*")
    .ssr_exclude("/admin")
```

Behind an auth gate (`auth()` or `auth_with()`), the paint is **off** unless
you allow it with `.ssr_behind_auth(true)`: the gate admits the visitor, but
the paint doesn't know who they are, so it is only safe when every
authenticated visitor may see all the shared data it renders. When it is on,
it is rendered after the gate: a visitor who gets the login page instead of
the app gets no static paint either.

With a PWA, the service worker's offline fallback is the `/` page cached at
install, so offline its paint shows the shared state of that time.

## Unknown paths

A path the app has no page for gets a **short `404 Not Found`** (a line of
text, not the 20–30 KB page): `/favicon.ico`, `/robots.txt`, probes.

- With `.routes(router)`: a path no route matches gets the short 404, unless
  the router declares a not-found view (`Router::not_found`); then the page is
  served with status 404 so the view renders.
- Without a router, an app with `.on_route(..)` routes every path itself: the
  page is served everywhere, as before.
- An app with neither has one page, at `/`; any other path gets the short 404.

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
