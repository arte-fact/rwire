---
title: Router
description: Client-side routing without page reloads
order: 1
---
# Router

rwire's router provides URL-based navigation without full page reloads. Route changes happen over the existing WebSocket connection -- the browser updates the URL bar and the server sends a new DOM tree.

```rust
use rwire::router::{Router, Link};
use rwire::server::Server;

Server::bind("0.0.0.0:9000")?
    .root(root)
    .on_route(handle_route())
    .routes(
        Router::new()
            .page("/", |_| build_home())
            .page("/about", |_| build_about())
            .page("/users/:id", |params| {
                let id = params.get("id").unwrap_or("0");
                build_user(id)
            })
            .page("/docs/*", |params| {
                let path = params.wildcard().unwrap_or("");
                build_doc_page(path)
            })
            .not_found(|_| build_404())
    )
    .run()
    .await
```

## Navigation with Link::to

Use `Link::to()` for in-app navigation. It renders an `<a>` tag with `data-route`, which the JS runtime intercepts to prevent a full page reload:

```rust
use rwire::router::Link;

// Simple text link
Link::to("/about", "About Us")

// Link with custom content
Link::to_with_content("/settings",
    el(El::Span).st([St::DisplayFlex, St::ItemsCenter, St::GapSm]).append([
        icon(Icon::Settings),
        el(El::Span).text("Settings"),
    ])
)
```

Never use raw `el(El::A).attr("href", "/path")` for internal navigation -- it causes a full page reload and opens a new WebSocket connection.

## Route Patterns

Three pattern types are supported:

```rust
// Literal -- exact match
.page("/about", |_| build_about())

// Named parameters -- captures as key-value pairs
.page("/users/:id", |params| {
    let id = params.get("id").unwrap_or("0");
    build_user(id)
})

// Wildcard -- captures everything after the prefix
.page("/docs/*", |params| {
    let rest = params.wildcard().unwrap_or("");
    build_doc_page(rest)
})
```

Named parameters and wildcards can be combined: `/api/:version/*` matches `/api/v2/users/42` with `version="v2"` and wildcard `"users/42"`.

## Route Handler

The `on_route` handler fires on every navigation event. The server receives the path as text via `ctx.text()`:

```rust
#[handler]
fn handle_route(state: &mut AppState, ctx: &EventContext) {
    if let Some(path) = ctx.text() {
        state.current_path = path.to_string();
    }
}
```

## Browser History

The JS runtime integrates with the browser history API:

- `Link::to()` clicks push a new entry with `history.pushState`
- Browser back/forward buttons fire `popstate` events, which send a route message to the server
- On page load, if the URL is not `/`, the runtime sends the current path as the initial route

This means bookmarks, browser history, and the back button all work as expected.

## Tree Shaking

The `.routes()` method enables automatic tree shaking. At server startup, every registered view function is called with empty params. This collects all element types, style tokens, and events used across all pages, ensuring the capsule includes everything needed for any route -- without manual `extra_elements` configuration.
