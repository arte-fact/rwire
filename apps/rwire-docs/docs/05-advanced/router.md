---
title: Router
description: Client-side routing without page reloads
order: 1
---
# Router

rwire gives you URL-based navigation without full page reloads: route changes travel over the
existing WebSocket, the browser updates the URL bar, and the server sends the new content.

There are **two routing models**. Pick one — mixing them breaks navigation.

| Model | You write | Best for |
|-------|-----------|----------|
| **A. Router + `outlet()`** | `.routes(Router::new()…)` and an `outlet()` in your shell | true SPA view-swapping; the framework owns the swap |
| **B. `on_route` + root re-render** | `.on_route(handler)` that updates your own state; your `root` renderer rebuilds the page | when one renderer already owns the whole layout |

> ⚠️ Installing a `Router` (`.routes(...)`) **without** an `outlet()` in the tree freezes the
> page: navigation updates the route but nothing re-renders. Likewise, don't add `.routes(...)`
> to an app that drives navigation through `.on_route` + a root renderer.

## Model A — Router + `outlet()`

Register views with `.routes(...)`, put a persistent shell in `.root(...)`, and drop an
`outlet()` where the matched view should render. The framework swaps the view on every
navigation via the built-in `CurrentRoute` state — you don't write an `on_route` handler.

```rust
use rwire::router::{Router, outlet, CurrentRoute};

Server::bind("0.0.0.0:9000")?
    .root(shell)
    .routes(
        Router::new()
            .page("/", |_| home())
            .page("/about", |_| about())
            .page("/users/:id", |p| user(p.get("id").unwrap_or("0")))
            .page("/docs/*", |p| doc_page(p.wildcard().unwrap_or("")))
            .not_found(|_| not_found()),
    )
    .run()
    .await

/// The persistent layout: the nav stays mounted; `outlet()` swaps per route.
fn shell() -> ElementBuilder {
    el(El::Div).append([nav(), outlet()])
}
```

`CurrentRoute` is a framework-provided state you can read in any renderer — to highlight the
active link or pull a `:param`:

```rust
#[renderer]
fn nav(route: &CurrentRoute) -> ElementBuilder {
    let active = route.path();          // current URL path
    let id = route.param("id");         // Option<String> for the matched :id, if any
    // …render links, marking `active` …
}
```

### Route patterns

```rust
// Literal — exact match
.page("/about", |_| about())

// Named parameter — captured by key
.page("/users/:id", |p| user(p.get("id").unwrap_or("0")))

// Wildcard — everything after the prefix
.page("/docs/*", |p| doc_page(p.wildcard().unwrap_or("")))
```

Named parameters and wildcards combine: `/api/:version/*` matches `/api/v2/users/42` with
`version="v2"` and wildcard `"users/42"`.

## Model B — `on_route` + root re-render

No router, no `outlet()`. Your `root` renderer owns the whole page and reads a path from your own
state; `.on_route(handler)` updates that state, and the root re-renders. This is what the docs and
design-system sites use.

```rust
#[derive(State, Default)]
#[storage(memory)]
struct AppState { current_path: String }

#[handler]
fn on_route_change(state: &mut AppState, ctx: &EventContext) {
    if let Some(path) = ctx.text() {
        state.current_path = path.to_string();
    }
}

#[renderer]
fn root(state: &AppState) -> ElementBuilder {
    // build the whole page from state.current_path …
}

// Server: .root(root).on_route(on_route_change())   // no .routes(...)
```

## Navigation with `Link::to`

Both models navigate with `Link::to()`. It renders an `<a data-route>`; the runtime intercepts
the click to avoid a full page reload:

```rust
use rwire::router::Link;

Link::to("/about", "About Us")

Link::to_with_content("/settings",
    el(El::Span).st([St::DisplayFlex, St::ItemsCenter, St::GapSm]).append([
        icon(Icon::Settings),
        el(El::Span).text("Settings"),
    ]))
```

Never use raw `el(El::A).attr("href", "/path")` for internal navigation — it triggers a full page
reload and a new WebSocket connection.

## Server-initiated navigation

A handler can navigate without a click via `EventContext`:

```rust
#[handler]
fn save(state: &mut AppState, ctx: &EventContext) {
    // … persist …
    ctx.navigate("/dashboard");       // push a new history entry + route
    // ctx.replace_route("/dashboard"); // or replace the current entry
}
```

## Browser history

The runtime integrates with the History API:

- `Link::to()` clicks push an entry with `history.pushState`
- Back/forward fire `popstate`, which sends a route message to the server
- On load, if the URL isn't `/`, the runtime sends the current path as the initial route

So bookmarks, history, and the back button all work as expected.

## Capsule coverage across routes

You don't declare anything per route. The capsule ships just the runtime; the element/event/
attribute **name maps** and the utility **CSS** are both delivered lazily over the WebSocket as
each route first renders (deduped per connection). Every page gets exactly what it needs
regardless of which routes exist. See [Capsule size](/docs/05-advanced/tree-shaking).
