# Scroll Handling in rwire

Study of current scroll behavior, identified gaps, and implementation plan for proper scroll management across route navigation and anchor links.

## Current Behavior

### How Route Navigation Works

The capsule JS runtime (`capsule_gen.rs` RUNTIME_JS, line 129) intercepts clicks on `<a data-route>` links:

```javascript
document.addEventListener('click', e => {
    let a = e.target.closest('a[data-route]');
    if (a) {
        e.preventDefault();
        let h = a.getAttribute('href');
        history.pushState(null, '', h);
        w.send('R' + h);
    }
});
```

On popstate (browser back/forward, line 130):

```javascript
window.addEventListener('popstate', () => { w.send('R' + location.pathname) });
```

On initial load (line 127):

```javascript
w.onopen = () => { if (location.pathname !== '/') w.send('R' + location.pathname) };
```

### How Anchor Links Work

TOC links (e.g. `<a href="#prerequisites">`) do **NOT** have `data-route`, so they are not intercepted. The browser handles them natively — scrolling to the element with the matching `id`. The docs parser (`docs/parser.rs:248-250`) generates `id` attributes on headings:

```rust
let anchor = slugify(&text);
heading_el = heading_el.attr("id", &anchor);
```

The TOC component (`components/toc.rs:131`) generates plain links:

```rust
el(El::A).at_str(At::Href, &heading.anchor)  // e.g., href="#prerequisites"
```

### Scroll Problems

| Scenario | Expected | Actual |
|----------|----------|--------|
| Click sidebar nav link (doc→doc) | Scroll to top | **Stays at previous scroll position** |
| Click sidebar nav link (landing→doc) | Scroll to top | **Stays at previous scroll position** |
| Browser back/forward | Restore saved scroll | **Stays at current position** |
| Click TOC anchor link (`#section`) | Smooth scroll to heading | Works (native behavior) |
| Direct URL with hash (`/docs/page#section`) | Load page, scroll to heading | **Loads page, no scroll** |
| Cross-page anchor (`/other-page#section`) | Navigate + scroll to heading | **Navigate only, hash ignored** |

Root cause: `e.preventDefault()` in the click handler cancels all default browser behavior, including scroll reset. The server re-renders DOM but sends no scroll instruction. The browser retains its position.

## Implementation Plan

### 1. Scroll-to-Top on Route Navigation

When a `data-route` link is clicked, scroll to the top of the page after the navigation. This is the most common expected behavior for page-to-page navigation.

**Change in `capsule_gen.rs` RUNTIME_JS line 129** — add `scrollTo(0,0)` after pushState:

```javascript
document.addEventListener('click', e => {
    let a = e.target.closest('a[data-route]');
    if (a) {
        e.preventDefault();
        let h = a.getAttribute('href');
        history.pushState(null, '', h);
        w.send('R' + h);
        // Scroll to top on page navigation, or to hash target if present
        let hash = new URL(h, location.origin).hash;
        if (hash) {
            // Defer to allow DOM update from server
            requestAnimationFrame(() => {
                let el = document.getElementById(hash.slice(1));
                if (el) el.scrollIntoView({ behavior: 'smooth' });
            });
        } else {
            scrollTo(0, 0);
        }
    }
});
```

**Byte cost**: ~120 bytes (hash parsing + scrollIntoView fallback)

**Tradeoffs**:
- `scrollTo(0,0)` is immediate; the DOM update from the server arrives after (async WS). This is correct — the page structure typically stays the same (header, sidebar) and only content changes. The user sees the scroll reset immediately.
- For hash targets, we need `requestAnimationFrame` to wait for the DOM update to arrive. Since the element with the target `id` is created by the server's re-render, we may need to retry.

### 2. Anchor Navigation Within Same Page

Already works via native browser behavior (TOC links don't have `data-route`). No changes needed.

To enable smooth scrolling site-wide, add a CSS scroll-behavior rule to the capsule's generated CSS:

```css
html { scroll-behavior: smooth; }
```

**Byte cost**: ~30 bytes in CSS

### 3. Cross-Page Anchor Links

Support links like `Link::to("/docs/page#section", "text")`. Currently the hash is sent to the server as part of the route message but ignored on the client side.

The fix is in the click handler (step 1 above): after `pushState` and `send`, check for a hash fragment and defer scrolling until the new DOM arrives.

**Challenge**: The server may take 5-50ms to respond with the new DOM. The target element doesn't exist yet. Solutions:

**Option A — Retry with MutationObserver** (robust, ~150 bytes):
```javascript
if (hash) {
    let id = hash.slice(1);
    let try_scroll = () => {
        let el = document.getElementById(id);
        if (el) { el.scrollIntoView({ behavior: 'smooth' }); return true; }
        return false;
    };
    if (!try_scroll()) {
        let obs = new MutationObserver(() => {
            if (try_scroll()) obs.disconnect();
        });
        obs.observe(document.body, { childList: true, subtree: true });
        setTimeout(() => obs.disconnect(), 2000); // safety timeout
    }
}
```

**Option B — Simple timeout** (simpler, ~50 bytes):
```javascript
if (hash) {
    setTimeout(() => {
        let el = document.getElementById(hash.slice(1));
        if (el) el.scrollIntoView({ behavior: 'smooth' });
    }, 100);
}
```

Option B is simpler but fragile (100ms may not be enough on slow connections). Option A is robust and self-cleaning. **Recommend Option A**.

### 4. Browser Back/Forward Scroll Restoration

The popstate handler currently sends only `location.pathname`:

```javascript
window.addEventListener('popstate', () => { w.send('R' + location.pathname) });
```

Two improvements:

**a)** Include hash in the route message:
```javascript
window.addEventListener('popstate', () => {
    w.send('R' + location.pathname + location.hash);
});
```

**b)** Let the browser handle scroll restoration natively. Set:
```javascript
if ('scrollRestoration' in history) history.scrollRestoration = 'manual';
```

Then after the DOM update for back/forward, scroll to the hash target (if any) or scroll to top. The browser's built-in `scrollRestoration = 'auto'` won't work because the DOM changes asynchronously via WebSocket, not during the popstate event.

### 5. Initial Load with Hash

The `onopen` handler ignores the hash:

```javascript
w.onopen = () => { if (location.pathname !== '/') w.send('R' + location.pathname) };
```

Fix: include the hash and defer scroll:

```javascript
w.onopen = () => {
    let p = location.pathname;
    if (p !== '/') w.send('R' + p);
    if (location.hash) {
        // Defer until initial DOM is rendered
        let id = location.hash.slice(1);
        let obs = new MutationObserver(() => {
            let el = document.getElementById(id);
            if (el) { el.scrollIntoView(); obs.disconnect(); }
        });
        obs.observe(document.body, { childList: true, subtree: true });
        setTimeout(() => obs.disconnect(), 3000);
    }
};
```

### 6. Server-Side SCROLL_TO Opcode (Optional, Future)

For programmatic scrolling from the server (e.g., "scroll to error field"), add a new opcode:

```rust
pub const SCROLL_TO: u8 = 0x72;
// Format: [SCROLL_TO, ref]
// JS: r[f].scrollIntoView({ behavior: 'smooth', block: 'start' })
```

This is not needed for the navigation use case but would be useful for forms (scroll to first validation error) and other server-driven scroll scenarios.

## Summary of Changes

| File | Change | Bytes |
|------|--------|-------|
| `capsule_gen.rs` (click handler) | Add scroll-to-top + hash scroll with MutationObserver | ~200 |
| `capsule_gen.rs` (popstate) | Include `location.hash` in route message | ~15 |
| `capsule_gen.rs` (onopen) | Include hash + defer scroll to anchor | ~150 |
| `capsule_gen.rs` (top) | `history.scrollRestoration = 'manual'` | ~50 |
| Generated CSS | `html{scroll-behavior:smooth}` | ~30 |
| **Total** | | **~445 bytes** |

The capsule size increase is modest (~450 bytes) for comprehensive scroll handling. For apps that don't use routing, tree-shaking already omits the route handler code entirely, so the cost is zero.

## Testing Checklist

After implementing, verify at all three viewports (375px, 768px, 1280px):

- [ ] Sidebar link click: scrolls to top of new page
- [ ] TOC link click: smooth scrolls to heading on same page
- [ ] Direct URL `/docs/page#section`: loads page, scrolls to heading
- [ ] Cross-page link `/docs/other#section`: navigates and scrolls to heading
- [ ] Browser back: restores previous page, scroll position reasonable
- [ ] Browser forward: restores next page
- [ ] Landing → Docs: scroll resets to top
- [ ] Docs → Landing: scroll resets to top
- [ ] Mobile drawer link: closes drawer, scroll resets to top
