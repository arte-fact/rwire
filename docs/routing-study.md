# Routing Study: Pulsar vs rwire

Analysis of Pulsar's routing API and what rwire should adopt.

## Pulsar's Routing Architecture

### Route Enum + Derive Macro

Pulsar's central idea: routes are an **enum**, each variant declares its path pattern and render function:

```rust
#[derive(Clone, Default, Routes, UserState)]
#[routes(shell = app_shell)]
pub enum AppRoute {
    #[default]
    #[route("/", render = home_content)]
    Home,

    #[route("/features", render = features_content)]
    Features,

    #[route("/users/:id", render = user_profile)]
    UserProfile { id: u64 },
}
```

The `#[derive(Routes)]` macro generates:
- `to_path()` - variant -> URL string
- `from_path()` - URL string -> variant (with typed param extraction)
- `link()` - create `<a data-route>` for a variant
- `build_router()` - register all routes with the Router
- `render_content()` - dispatch to the right render function

### Shell + Content Separation

The `shell` wraps all page content:

```rust
fn app_shell(content: ReactiveNode) -> ReactiveNode {
    main_el()
        .child(navigation())
        .child(content)  // <-- route-specific content goes here
        .child(footer())
}
```

Page render functions produce just the content area:

```rust
fn home_content() -> NodeBuilder { div().child("Home") }
fn features_content() -> NodeBuilder { div().child("Features") }
fn user_profile(id: u64) -> NodeBuilder { div().child(format!("User {id}")) }
```

### What's Good

1. **All render functions are known at compile time** - the derive macro sees every `render = fn_name`
2. **Type-safe route params** - `UserProfile { id: u64 }` extracts and parses `:id` automatically
3. **Bidirectional** - `to_path()` and `from_path()` are auto-generated inverses
4. **Single source of truth** - routes, paths, params, and render functions in one enum
5. **No manual routing logic** - no `if path == "/docs" { ... } else { ... }` in the root renderer

### What's Overengineered for rwire

1. **Derive macro complexity** - 600+ lines of proc macro for `#[derive(Routes)]`; rwire can do this at runtime
2. **Separate `build_router()` call** - Pulsar builds a Router struct, but rwire's tree-shaking needs happen at startup, not at route-match time
3. **Enum variants as routes** - forces awkward patterns when route state goes beyond just params (e.g., search query, sidebar state)

## rwire's Current Routing

### What Works

- `Link::to(href, text)` creates semantic `<a data-route="">` elements
- JS intercepts clicks, updates URL via History API, sends `R+path` to server
- `on_route()` handler receives path changes
- Back/forward works via popstate
- Initial URL hydrates on WebSocket connect

### The Tree-Shaking Problem

Tree-shaking calls `root()` once with default state. Only elements/styles from that single code path make it into the capsule. Dynamic paths (markdown pages, active sidebar, search results) use tokens that are **never discovered**.

Current workaround:
```rust
CapsuleConfig::new()
    .extra_elements(&[El::Pre, El::Code, ...])  // manual
    .extra_styles(&[St::BgCode, St::TextLeft, ...])  // 32+ tokens
```

This is fragile and defeats the purpose of tree-shaking.

### The Fundamental Issue

The root function has conditional branches:

```rust
fn root(state: &DocState) -> ElementBuilder {
    if state.searching {
        build_search_results(...)  // never called with default state
    } else if state.current_path.is_empty() {
        build_landing_page(...)    // only this runs at startup
    } else {
        build_doc_page(...)        // never called with default state
    }
}
```

Tree-shaking only sees the landing page path. **The fix: tell the framework about ALL view functions.**

## Proposed Design: Route-Aware Tree-Shaking

### Core Idea

Routes register view functions. At startup, tree-shaking calls ALL of them to discover the full set of elements, styles, and events.

### API

```rust
Server::bind("0.0.0.0:9000")?
    .routes(
        Router::page("/", build_landing_page)
            .page("/docs/*", build_doc_page)
            .page("/search", build_search_results)
            .not_found(build_not_found)
    )
    .shell(build_shell)     // wraps page content (header, sidebar, footer)
    .on_route(on_route)     // state mutation handler (existing)
    .capsule_config(config) // theme, etc.
    .run()
    .await
```

### How It Works

1. **Startup**: server calls `build_shell(placeholder)` to tree-shake the layout
2. **Startup**: server calls EACH page view function to tree-shake content
3. **Union** of all discovered elements/styles/events goes into the capsule
4. **No more `extra_elements`/`extra_styles`** - they're discovered automatically
5. **Runtime**: when route changes, server calls matched view + shell, diffs, sends update

### View Functions

View functions take `RouteParams` and return `ElementBuilder`:

```rust
fn build_landing_page(_params: &RouteParams) -> ElementBuilder {
    Stack::column().gap(Gap::Xl).children([
        el(El::H1).text("rwire Documentation"),
        // ...
    ]).build()
}

fn build_doc_page(params: &RouteParams) -> ElementBuilder {
    let path = params.wildcard().unwrap_or("");
    let site = DocSite::load(DOCS_DIR);
    let page = site.page(path).unwrap();
    let parsed = parse_markdown(&page.markdown);
    parsed.content
}
```

### Shell Function

The shell wraps page content:

```rust
fn build_shell(content: ElementBuilder) -> ElementBuilder {
    el(El::Div)
        .st([St::BgApp, St::TextDefault, St::MinHScreen])
        .append([
            AppShell::new()
                .header(build_header())
                .sidebar(build_sidebar())
                .main(content)
                .build(),
        ])
}
```

### Tree-Shaking at Startup

```
For each registered page view:
  1. Call view(dummy_params) -> ElementBuilder
  2. Walk tree, collect used El/Ev/St/At/Av tokens
  3. Add to global used_* sets

Call shell(placeholder) -> ElementBuilder
Walk tree, collect tokens

Union all sets -> generate capsule
```

Because every view is called during tree-shaking, every token is discovered. The `extra_*` escape hatches become unnecessary.

### Route Matching + State Integration

The Router matches paths and extracts params:

```rust
Router::page("/", handler)              // exact
Router::page("/docs/*", handler)        // wildcard → params.wildcard()
Router::page("/users/:id", handler)     // param → params.get("id")
```

When a route message arrives (`R+/docs/architecture/protocol`):
1. Router matches pattern, extracts params
2. Calls view function with params → ElementBuilder
3. Wraps in shell → full page ElementBuilder
4. Diffs against previous render, sends binary update

### What Changes from Current Code

| Current | Proposed |
|---------|----------|
| `Server.root(root)` | `Server.shell(shell).routes(Router::page(...))` |
| `on_route` handler mutates state, root re-renders | Router dispatches to correct view, diffs |
| Manual `if path == "..." { ... }` in root | Router pattern matching |
| `extra_elements`, `extra_styles` | Automatic: all views tree-shaken |
| `RouteParams` not used | Typed param extraction from URL |

### Migration Path for docs-site

Before:
```rust
#[renderer]
fn root(state: &DocState) -> ElementBuilder {
    let content = if state.searching {
        build_search_results(&site, &state.search_query)
    } else if state.current_path.is_empty() {
        build_landing_page(&site)
    } else {
        build_doc_page(&site, &state.current_path)
    };
    build_shell(content)
}
```

After:
```rust
Server::bind("0.0.0.0:9000")?
    .shell(build_shell)
    .routes(
        Router::page("/", build_landing_page)
            .page("/docs/*", build_doc_page)
    )
    .on_search(build_search_results) // special: search is state-driven
    .run()
    .await
```

## Pulsar Features to Adopt

### 1. Pattern Matching with Typed Params (priority: high)

Pulsar's `RouteParams` with `.get("id")`, `.wildcard()`, typed extraction via `FromParam` trait. rwire already has `RoutePattern` in router.rs but it's unused. Wire it in.

### 2. Shell + Content Separation (priority: high)

Pulsar's `#[routes(shell = fn)]` cleanly separates layout from content. rwire should adopt this — the shell renders once, only content area updates on navigation. This also enables proper tree-shaking since the shell and ALL content views are analyzed.

### 3. Bidirectional Route Resolution (priority: medium)

Pulsar's `to_path()` and `from_path()` are generated inverses. rwire's `Link::to()` already takes explicit paths, so this is less critical, but useful for programmatic navigation.

### 4. Server-Initiated Navigation (priority: medium)

rwire already has `ROUTE_PUSH` and `ROUTE_REPLACE` opcodes defined but never sent. Useful for redirects after form submission, auth guards, etc.

## Pulsar Features to Skip

### 1. Derive Macro for Routes
Too much complexity for the gain. Runtime router builder is simpler and more flexible.

### 2. Route Enum
Awkward when route state goes beyond URL params. rwire's state model (server-side, per-session) doesn't need this.

### 3. API Routes in Router
rwire is WebSocket-only. HTTP API routes are a different concern.

## Implementation Order

1. **Router integration** - Wire existing Router into Server, match route messages against patterns
2. **Shell + view architecture** - `Server.shell(fn).routes(Router::page(...))`
3. **Tree-shaking all views** - At startup, call every registered view to discover tokens
4. **Remove `extra_*`** - Once tree-shaking covers all views, delete the workarounds
5. **Server-initiated navigation** - Send ROUTE_PUSH/ROUTE_REPLACE from handlers
6. **Typed params** - `FromParam` trait, `params.extract::<u64>("id")`
