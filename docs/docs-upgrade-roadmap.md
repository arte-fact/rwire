# Docs Site Upgrade Roadmap

Three areas of work to bring the docs-site example to production quality:
visual polish, SVG rendering, and client-side routing.

---

## 1. Visual Upgrade

### Problem

The docs-site renders correctly but looks "off" compared to professional
documentation sites (Tailwind CSS, Next.js, Vue, Astro, SvelteKit). The content
is unstyled prose, the layout lacks breathing room, and the typography has no
reading-optimized hierarchy.

### Analysis: What Professional Docs Sites Do

Studied five documentation sites and extracted the common visual patterns:

| Signal | Tailwind | Next.js | Vue | Astro | SvelteKit |
|--------|----------|---------|-----|-------|-----------|
| Content max-width | 768px | 672px | 688px | 768px | 720px |
| Prose line-height | 1.75 | 1.7 | 1.7 | 1.75 | 1.65 |
| H2 margin-top | 48px | 48px | 56px | 48px | 40px |
| H3 margin-top | 32px | 32px | 36px | 32px | 28px |
| Body text color | muted (not pure black) | muted | muted | muted | muted |
| Sidebar width | 256px | 240px | 240px | 240px | 256px |
| TOC width | 224px | 200px | 200px | 200px | 200px |
| Code bg | subtle gray | subtle gray | subtle gray | subtle gray | subtle gray |
| Code font-size | 0.875rem | 0.875rem | 0.875rem | 0.8125rem | 0.875rem |
| Code border-radius | 8px | 8px | 8px | 8px | 8px |

**Ten key polish signals:**

1. **Constrained content width** (600-768px) for readable line lengths
2. **Generous line-height** (1.65-1.75) on body/prose text
3. **Large vertical gaps before headings** (48-56px before h2, 28-36px before h3)
4. **Muted primary text** (not #000, typically gray-700/gray-800)
5. **Code blocks with background, padding, and rounded corners**
6. **Inline code with subtle background pill** (bg + px-1.5 + rounded)
7. **Three-column layout** (sidebar 240-256px | content 600-768px | TOC 200-224px)
8. **Sticky sidebar and TOC** (position: sticky; top: offset)
9. **Active TOC highlighting** synced to scroll position
10. **Smooth hover transitions** on all interactive elements (150-200ms)

### Current State

The docs-site uses `AppShell` for layout, `St` tokens for styling, and the
`Prose` component for markdown rendering. What's missing:

- **No max-width on content area** - prose fills available space, lines too long
- **No optimized prose typography** - default line-height, no heading spacing
- **TOC is static** - no sticky positioning, no scroll-synced highlighting
- **Sidebar not sticky** - scrolls with page
- **Code blocks are minimal** - no language headers, no rounded corners
- **No inline code styling** - backtick code blends with surrounding text

### Implementation Plan

#### Phase 1: Prose Typography (St tokens)

Add style tokens for prose-optimized reading:

```rust
// style_tokens.rs - new tokens
St::MaxWProse      = 0x2C0,  // max-width: 65ch (~700px)
St::LeadingProse    = 0x2C1,  // line-height: 1.75
St::ProseHeadingMt  = 0x2C2,  // margin-top: 3rem (before h2/h3)
St::ProseParaMb     = 0x2C3,  // margin-bottom: 1.25rem
```

Apply in the `docs` module's markdown renderer so all docs pages inherit
the correct typography without manual token application.

#### Phase 2: Layout Polish

- Constrain content area: wrap doc page content in `el(El::Div).st([St::MaxWProse])`
- Make sidebar sticky: add `St::Sticky` + `St::Top0` tokens
- Make TOC sticky: same sticky treatment, offset for header height
- Widen sidebar to 256px (currently uses `attr("style", "width:200px")`)

#### Phase 3: Code Block Styling

Enhance the `Code` component with:

- Background color token: `St::BgCode` (subtle gray, theme-aware)
- Padding: `St::P4` or similar
- Rounded corners: `St::RoundedLg` (8px)
- Font-size: `St::TextSm` (0.875rem)
- Language label header (optional, via data attribute)
- Inline code: `St::BgCode`, `St::PxXs`, `St::RoundedSm`, `St::TextSm`

#### Phase 4: Interactive Polish

- Active TOC item highlighting (requires scroll position tracking - future)
- Smooth hover transitions on sidebar items (St::TransitionColors already exists)
- Breadcrumb separator styling (muted color, smaller font)

### Priority

Phase 1-2 are the highest impact. Phase 3 makes code-heavy docs readable.
Phase 4 is nice-to-have polish.

---

## 2. SVG Rendering

### Problem

SVG elements don't render in the browser. The `El::Svg` and `El::Path` element
types exist in the protocol but the JS runtime creates them with
`document.createElement()` instead of `document.createElementNS()`.

### Root Cause

**capsule_gen.rs line 78** - the CREATE opcode handler:

```javascript
else if(o===O.C){r.push(document.createElement(E[d[i++]]||'div'))}
```

`document.createElement('svg')` creates an HTML element in the HTML namespace.
SVG elements must be in the SVG namespace (`http://www.w3.org/2000/svg`) to
render. The browser silently creates a non-rendering HTML element instead of
erroring.

### Existing Infrastructure

**Elements defined (opcodes.rs):**
- `El::Svg` = 0x18
- `El::Path` = 0x19

**SVG attributes tokenized (attr_tokens.rs):**
- `At::Xmlns` (0x40), `At::ViewBox` (0x41), `At::Fill` (0x42)
- `At::Stroke` (0x43), `At::StrokeWidth` (0x44), `At::D` (0x47)
- `At::Width` (0x48), `At::Height` (0x49)
- Plus `Av::SvgNs`, `Av::ViewBox24`, `Av::CurrentColor`, `Av::Round`

**Missing SVG elements:**
- Circle, Line, Polyline, Rect, G (group) are not in the `El` enum

### Implementation Plan

#### Phase 1: Fix createElementNS (single-line fix)

Add an SVG type set to the JS runtime and branch on CREATE:

```javascript
// In the capsule JS runtime, add SVG element set
const SVG_NS='http://www.w3.org/2000/svg';
const SE=new Set([/* svg element type codes */]);

// Replace CREATE handler:
// Before:
else if(o===O.C){r.push(document.createElement(E[d[i++]]||'div'))}
// After:
else if(o===O.C){let t=d[i++];r.push(SE.has(t)?document.createElementNS(SVG_NS,E[t]||'svg'):document.createElement(E[t]||'div'))}
```

The SVG element set (`SE`) should be tree-shaken like other lookup tables -
only include entries for SVG types that are actually used by the application.

#### Phase 2: Add Missing SVG Elements

Add to `El` enum in opcodes.rs:

```rust
El::Circle   = 0x1A,
El::Line     = 0x1B,
El::Polyline = 0x1C,
El::Rect     = 0x1D,
El::G        = 0x1E,
```

Add to `ELEMENT_MAPPINGS` in capsule_gen.rs. All SVG element codes (0x18-0x1E)
should be included in the `SE` set.

#### Phase 3: Add Missing SVG Attributes

Evaluate whether additional SVG attributes are needed:

- `cx`, `cy`, `r` (circle)
- `x1`, `y1`, `x2`, `y2` (line)
- `points` (polyline)
- `rx`, `ry` (rect, rounded corners)
- `transform`
- `opacity`

These can be added to the `At` enum or handled via `.attr("name", "value")`
string escape hatch.

### Priority

Phase 1 is a one-line fix that unblocks all SVG rendering. Phase 2 adds element
coverage. Phase 3 is incremental as needed.

---

## 3. Virtual Router

### Problem

The docs-site uses click handlers with `.data("path", ...)` for navigation.
This works but:

- Browser URL never changes (always shows `http://127.0.0.1:9000`)
- Back/forward buttons don't work
- Direct URL access to a page isn't possible
- Bookmarking doesn't work
- No shareable links

### Existing Infrastructure (60% complete)

**Client-side (JS runtime in capsule_gen.rs):**

Route opcodes are handled:
```javascript
// Lines 97-98: ROUTE_PUSH and ROUTE_REPLACE
else if(o===O.RU){let[k,l]=rv(d,i);i+=l;history.pushState(null,'',s[k])}
else if(o===O.RR){let[k,l]=rv(d,i);i+=l;history.replaceState(null,'',s[k])}
```

Navigation interception exists:
```javascript
// Line 112-113: click intercept + popstate
document.addEventListener('click',e=>{let a=e.target.closest('a[data-route]');if(a){...w.send('R'+href)}});
window.addEventListener('popstate',()=>{w.send('R'+location.pathname)});
```

**Server-side Rust:**

Router module fully implemented (router.rs, 295 lines):
- `RoutePattern` with literal, parameter (`:id`), and wildcard segments
- `Router::new().route("/docs/:section/:page", builder).not_found(builder)`
- `Link::to(href, text)` helper (creates `<a>` with `data-route`)

Protocol opcodes defined (opcodes.rs):
- `ROUTE_PUSH` = 0x70
- `ROUTE_REPLACE` = 0x71

Encoder methods exist (encoder.rs):
- `buf.route_push(url_symbol)`
- `buf.route_replace(url_symbol)`

**What's Missing:**

1. **Server discards route messages** - server.rs line 987-989 logs
   `"Text message (unexpected)"` and ignores route messages from the client
2. **No initial URL hydration** - when a client connects, the server doesn't
   know what URL they navigated to
3. **No route-to-state integration** - no way to map a URL to application state
4. **ROUTE_PUSH never sent** - server never sends route opcodes back to client
   to update the URL bar after navigation

### Implementation Plan

#### Phase 1: Server Route Message Handling

Parse route messages in server.rs:

```rust
// In the WebSocket message handler:
Ok(Message::Text(text)) => {
    if let Some(path) = text.strip_prefix('R') {
        // Route navigation from client
        // Update state with new path, trigger re-render
    }
}
```

This requires a way to invoke a "route changed" handler, similar to event
handlers but triggered by URL changes instead of DOM events.

#### Phase 2: Initial URL Hydration

When a client first connects via WebSocket, send the initial URL:

Option A: Client sends route on connect
```javascript
// After WebSocket opens, send current URL
w.addEventListener('open', () => {
    w.send(new TextEncoder().encode('R' + location.pathname));
});
```

Option B: Server reads URL from HTTP upgrade request
The WebSocket upgrade request contains the full URL. The server can extract
the path and use it for initial rendering.

**Recommendation:** Option A is simpler and doesn't require HTTP header parsing.
The client already has the code to send 'R'+pathname, just needs to also send
it on connect.

#### Phase 3: Route-State Integration

Define a route handler pattern that maps URLs to state updates:

```rust
// In the docs-site example:
Server::bind("0.0.0.0:9000")?
    .root(root)
    .on_route(|state: &mut DocState, path: &str| {
        state.current_path = path.to_string();
        state.searching = false;
        state.search_query.clear();
    })
    .run()
    .await
```

Or integrate with the existing Router struct:

```rust
let router = Router::new()
    .route("/docs/:section/:page", |state, params| {
        state.current_path = format!("/docs/{}/{}", params["section"], params["page"]);
    })
    .route("/", |state, _| {
        state.current_path.clear();
    });
```

#### Phase 4: Server-Side URL Push

After navigation via click handler, the server should send ROUTE_PUSH back
to update the browser URL bar:

```rust
// In the navigate_to handler or post-render hook:
buf.route_push(url_symbol);  // Tells browser to pushState
```

This keeps the URL bar in sync with server-side state without the client
needing to know the URL structure.

#### Phase 5: Link Component

Replace manual `.data("path", ...)` patterns with the existing `Link::to()`:

```rust
// Before (current):
el(El::Div)
    .data("path", "/docs/getting-started/install")
    .on(Ev::Click, navigate_to())

// After:
Link::to("/docs/getting-started/install", "Installation")
```

The `Link::to()` already creates `<a data-route>` elements that the JS runtime
intercepts. This gives semantic HTML, keyboard navigation, and right-click
"open in new tab" for free.

### Data Flow (Complete)

```
1. User clicks Link or types URL
2. JS intercepts → sends 'R/docs/getting-started/install'
3. Server parses route message
4. Server updates state (current_path = "/docs/getting-started/install")
5. Server re-renders affected regions
6. Server sends DOM diff + ROUTE_PUSH opcode
7. Browser applies DOM changes + history.pushState()
8. URL bar shows /docs/getting-started/install
9. Back button → popstate → sends 'R/previous-path' → repeat from step 3
```

### Priority

Phase 1-2 enable basic URL routing. Phase 3 provides the ergonomic API.
Phase 4-5 complete the developer experience.

---

## Roadmap Summary

| Phase | Area | Effort | Impact |
|-------|------|--------|--------|
| SVG Phase 1 | createElementNS fix | Small (1 line) | Unblocks all SVG |
| Visual Phase 1 | Prose typography tokens | Small | Major readability improvement |
| Visual Phase 2 | Layout polish (sticky, widths) | Small | Professional three-column feel |
| Router Phase 1 | Server route message parsing | Medium | URLs start changing |
| Router Phase 2 | Initial URL hydration | Small | Direct URL access works |
| SVG Phase 2 | Additional SVG elements | Small | Full SVG element coverage |
| Visual Phase 3 | Code block styling | Medium | Code-heavy docs look good |
| Router Phase 3 | Route-state integration API | Medium | Ergonomic route handlers |
| Router Phase 4 | Server-side URL push | Small | URL bar stays in sync |
| Router Phase 5 | Link component adoption | Small | Semantic navigation |
| Visual Phase 4 | Interactive polish (scroll TOC) | Large | Nice-to-have |
| SVG Phase 3 | Additional SVG attributes | Small | As needed |

**Recommended execution order:** SVG Phase 1 first (trivial fix), then Visual
Phases 1-2 (immediate visual improvement), then Router Phases 1-3 (functional
URLs), then remaining phases as needed.
