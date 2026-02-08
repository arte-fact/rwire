# Phase 5: Polish & Launch

**Goal**: Final polish — examples gallery, responsive testing, performance audit, accessibility, and launch readiness.

**Depends on**: Phase 3 (landing page) + Phase 4 (documentation)

---

## Step 5.1 — Examples Gallery

An `/examples` page showcasing interactive examples visitors can explore.

### Page Design

A grid of example cards, each showing:
- Title (e.g., "Counter", "Todo List", "Form Validation")
- Short description
- Technology tags (e.g., "State", "ItemRef", "Local Mutations")
- Lines of code count
- Link to source code on GitHub

### Examples to Feature

| Example | Description | Key Concepts |
|---------|-------------|-------------|
| Counter | Increment/decrement with live display | State, handlers, renderers |
| Todo List | Add, toggle, filter, delete items | ItemRef, lists, filtering |
| Documentation Site | This site itself | Router, AppShell, Markdown, search |
| Theme Showcase | Toggle themes and styles live | ThemeMode, ThemeStyle, palettes |
| Form Validation | Input with validation rules | FormField, validation, error states |

### Code Display

Each example card links to a detail page showing:
1. The complete source code (syntax highlighted)
2. Explanation of key patterns used
3. "Run this example" instructions

---

## Step 5.2 — Responsive Testing

Systematic testing across viewport sizes.

### Breakpoints to Test

| Viewport | Device Class | Key Checks |
|----------|-------------|------------|
| 375px | Mobile (iPhone SE) | Single column, readable text, touch targets 44px+ |
| 428px | Mobile (iPhone 14) | Same as above, slightly more breathing room |
| 768px | Tablet portrait | 2-column grids, sidebar collapses |
| 1024px | Tablet landscape / small laptop | Full layout begins |
| 1440px | Desktop | Full multi-column layout, TOC visible |
| 1920px | Large desktop | Content max-width prevents excessive line length |

### Responsive Fixes to Anticipate

- **Hero**: Text size scales down (`St::Text5xl` → `St::Text3xl` on mobile)
- **Feature grid**: 3 columns → 2 → 1 as viewport narrows
- **Stats row**: 4 columns → 2×2 grid → single column
- **Comparison table**: Horizontal scroll on mobile
- **Sidebar**: Hidden on mobile, accessible via hamburger menu (Drawer)
- **TOC**: Hidden below 1024px
- **Footer**: Columns collapse to vertical stack
- **Code blocks**: Horizontal scroll, no wrapping

### Implementation

Use `@media` queries in the capsule CSS for responsive overrides. The existing Grid component with `St::GridColsAuto` handles most cases automatically.

For layout-level responsiveness (sidebar show/hide), the AppShell component already handles this.

---

## Step 5.3 — Performance Audit

Verify the website lives up to rwire's performance claims.

### Metrics to Measure

| Metric | Target | How to Measure |
|--------|--------|---------------|
| Capsule JS size | < 4KB gzipped | `wc -c` on generated capsule |
| Initial page load | < 10KB total (HTML + JS + CSS + WS) | Playwright network requests |
| Time to interactive | < 100ms | Performance API in browser |
| WebSocket message size | < 1KB for page transitions | Playwright network capture |
| Lighthouse Performance | 95+ | Chrome Lighthouse |
| Lighthouse Accessibility | 95+ | Chrome Lighthouse |

### Performance Testing Script

```bash
# Build in release mode
cargo build --release -p website

# Start server
./target/release/website &

# Measure capsule size
curl -s http://127.0.0.1:9000 | wc -c

# Run Lighthouse (if available)
lighthouse http://127.0.0.1:9000 --output=json
```

---

## Step 5.4 — Accessibility Audit

Ensure the website is usable by everyone.

### Checklist

- [ ] All images have alt text
- [ ] All interactive elements are keyboard accessible
- [ ] Focus indicators visible on all focusable elements
- [ ] Color contrast ratio ≥ 4.5:1 for text (WCAG AA)
- [ ] Color contrast ratio ≥ 3:1 for large text
- [ ] Skip-to-content link present
- [ ] ARIA landmarks: header, nav, main, footer
- [ ] Code blocks have appropriate `role` and labels
- [ ] Theme toggle is keyboard accessible
- [ ] Search input has visible label or aria-label
- [ ] Heading hierarchy is logical (h1 → h2 → h3, no skips)

### Nord Palette Contrast Check

| Pair | Light Mode | Dark Mode | Passes AA? |
|------|-----------|-----------|------------|
| Text on bg | #2E3440 on #ECEFF4 | #ECEFF4 on #2E3440 | Yes (12.6:1) |
| Muted on bg | #4C566A on #ECEFF4 | #D8DEE9 on #2E3440 | Yes (7.1:1) |
| Accent on bg | #5E81AC on #ECEFF4 | #88C0D0 on #2E3440 | Check needed |
| Accent on card | #5E81AC on #E5E9F0 | #88C0D0 on #3B4252 | Check needed |

---

## Step 5.5 — SEO & Meta

Even though rwire is server-driven via WebSocket, the initial HTML capsule can include meta tags for search engines and social sharing.

### Meta Tags

```html
<title>rwire — Server-side UI with a binary protocol</title>
<meta name="description" content="A Rust framework for building server-driven web UIs with a binary protocol, ~1.5KB JS runtime, and 51 production-ready components.">
<meta name="keywords" content="rust, web framework, server-driven, binary protocol, websocket, ui components">
<meta property="og:title" content="rwire — Server-side UI with a binary protocol">
<meta property="og:description" content="Build web UIs entirely in Rust. Binary protocol, ~1.5KB runtime, 580+ style tokens.">
<meta property="og:type" content="website">
<meta name="twitter:card" content="summary_large_image">
```

### Implementation

Add meta tag support to `CapsuleConfig`:

```rust
CapsuleConfig::new()
    .title("rwire — Server-side UI with a binary protocol")
    .description("Build web UIs entirely in Rust...")
    .meta("og:title", "rwire — ...")
```

---

## Step 5.6 — Playwright E2E Tests

Automated visual and functional tests for the website.

### Test Scenarios

```
test_landing_page_renders
  - Navigate to /
  - Verify hero heading visible
  - Verify install command visible
  - Verify 6 feature cards visible
  - Screenshot comparison (light + dark)

test_docs_navigation
  - Navigate to /docs/getting-started/install
  - Verify sidebar shows active state
  - Verify TOC renders
  - Click next page → verify navigation
  - Click sidebar link → verify page change

test_theme_toggle
  - Navigate to /
  - Click theme toggle
  - Verify data-theme changes to "dark"
  - Verify visual change (screenshot comparison)
  - Click again → back to light

test_search
  - Navigate to /docs
  - Type in search box
  - Verify search results appear
  - Click result → verify navigation

test_copy_button
  - Navigate to / (landing page)
  - Click copy button on install command
  - Verify clipboard contains "cargo add rwire"

test_mobile_layout
  - Set viewport to 375px
  - Navigate to /
  - Verify single-column layout
  - Navigate to /docs → verify sidebar is hidden
  - Open hamburger menu → verify sidebar appears

test_external_links
  - Find GitHub link
  - Verify target="_blank" attribute
  - Verify href points to correct URL
```

---

## Step 5.7 — Final Review & Launch

### Pre-Launch Checklist

- [ ] All phases (1-5) verification checklists passed
- [ ] `cargo clippy --workspace` — zero warnings
- [ ] `cargo test --workspace` — all pass
- [ ] Playwright E2E tests pass
- [ ] Light mode: visual review of all pages
- [ ] Dark mode: visual review of all pages
- [ ] Mobile: visual review at 375px
- [ ] Tablet: visual review at 768px
- [ ] Performance: capsule < 4KB, initial load < 10KB
- [ ] Accessibility: WCAG AA compliance
- [ ] All documentation pages have content
- [ ] All internal links work
- [ ] All external links have `target="_blank"`
- [ ] README updated with website link
- [ ] Comparative study linked from landing page

### Launch

1. Merge all website code to `main`
2. Deploy (hosting TBD — any server that can run the Rust binary)
3. Update project README with website link
4. Announce on relevant channels

---

## Success Criteria

The website is ready when:

1. A developer can go from "what is rwire?" to "running their first app" in under 5 minutes
2. The landing page communicates rwire's key differentiators with real numbers
3. The site itself demonstrates rwire's capabilities (binary protocol, minimal JS, theme switching)
4. All content is accurate and up-to-date with the current codebase
5. The site works well on mobile, tablet, and desktop
6. Light and dark modes both look polished
