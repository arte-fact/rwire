---
title: Navigation Components
description: NavMenu, Breadcrumb, Tabs, Pagination, and routing
order: 3
---
# Navigation Components

## Client-Side Routing with Link

rwire has two `Link` types. For client-side navigation, use `rwire::Link` (from the router module). It creates `<a>` elements with `data-route` so the JS runtime intercepts clicks, updates the URL via `history.pushState`, and sends a route message over WebSocket instead of causing a full page reload.

```rust
use rwire::Link;

// Text link -- client-side route change
Link::to("/docs", "Documentation")

// Link with custom content (icons, badges, etc.)
Link::to_with_content("/settings", settings_icon)
```

For styled external links, use `rwire::components::Link`:

```rust
use rwire::components::Link;

// External link (opens in new tab with rel="noopener noreferrer")
Link::external("https://github.com/example/rwire")
    .text("GitHub")
    .build()

// Internal styled link (no client-side routing -- use rwire::Link for that)
Link::new("/about").text("About").build()
```

### Routing Setup

Register route handlers on the server with `Router`:

```rust
use rwire::Router;

let router = Router::new()
    .page("/", render_home)
    .page("/docs", render_docs)
    .page("/docs/:slug", render_doc_page);
```

---

## NavMenu

Horizontal navigation bar with active-path highlighting.

```rust
use rwire::components::{NavMenu, NavItem};

NavMenu::new()
    .item(NavItem::new("Home", "/"))
    .item(NavItem::new("Docs", "/docs"))
    .item(NavItem::new("API", "/api"))
    .active_path("/docs")
    .build()
```

Active items get a `BgEmphasis` background and `TextDefault` color. Inactive items show `TextMuted` with hover highlighting.

---

## Breadcrumb

Trail of navigation links showing the current location in a hierarchy.

```rust
use rwire::components::Breadcrumb;

Breadcrumb::new()
    .item("Home", Some("/"))
    .item("Products", Some("/products"))
    .item("Laptop", None::<&str>)  // current page, no link
    .build()
```

The last item renders as text (not a link) with `aria-current="page"`. Earlier items are clickable links with a `/` separator generated via CSS `::after`.

---

## Tabs

Tab navigation with content panels. Only the active panel renders.

```rust
use rwire::components::{Tabs, Tab};

Tabs::new()
    .tab(Tab::new("Overview", overview_content.build()))
    .tab(Tab::new("Settings", settings_content.build()))
    .tab(Tab::new("History", history_content.build()))
    .active(0)
    .build()
```

Tab buttons use `role="tab"` and panels use `role="tabpanel"`. The active tab shows an accent-colored bottom border. Connect `.active()` to server state and use a handler to switch tabs:

```rust
#[handler]
fn switch_tab(state: &mut AppState, ctx: &EventContext) {
    state.active_tab = ctx.value().parse().unwrap_or(0);
}
```

---

## Pagination

Page navigation with Previous/Next buttons and page numbers.

```rust
use rwire::components::Pagination;

Pagination::new()
    .current_page(3)
    .total_pages(10)
    .max_visible(5)
    .build()
```

The component auto-generates page buttons with ellipsis for large ranges. The current page shows an accent background with `aria-current="page"`. Previous and Next buttons are automatically disabled at the boundaries. Pages are 1-indexed.
