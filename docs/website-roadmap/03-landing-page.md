# Phase 3: Landing Page

**Goal**: Build the landing page for rwire.dev — the first thing visitors see.

**Depends on**: Phase 2 (Footer, CopyButton, Grid)

---

## Design Direction

The landing page follows the pattern observed across Next.js, Svelte, and Astro: a vertical flow of focused sections, each communicating a single message. The tone is **technical and honest** — no marketing fluff, just real numbers and real code.

The page itself is the demo. Every element renders via the binary protocol over WebSocket. The ~1.5KB runtime. The instant updates. The visitor experiences rwire's value proposition by using the site.

---

## Section 1 — Hero

The top of the page. Large heading, one-line description, install command, two CTA buttons.

### Content

```
rwire

Server-side UI with a binary protocol.

$ cargo add rwire

[Get Started →]  [GitHub]
```

### Design

- Heading: `St::Text5xl`, `St::FontBold`, `St::LeadingTight`, `St::TextHigh`
- Subheading: `St::TextXl`, `St::TextMuted`, `St::LeadingRelaxed`
- Install command: monospace block with `$` prefix and CopyButton
- CTAs: Primary button (Get Started → /docs), Ghost button (GitHub → external)
- Vertical centering with generous padding (`St::PyXl` × 2)
- Max width constraint: `St::MaxW4xl`, `St::MxAuto`, `St::TextCenter`

### Technical Notes

- "Get Started" uses `Link::to("/docs/getting-started/install", ...)`
- "GitHub" uses raw `<a href="..." target="_blank">` with `Icon::GitHub`
- Install command uses `CopyButton::new("cargo add rwire")`

---

## Section 2 — Key Numbers

A row of 3-4 stats that immediately establish credibility. Numbers are more persuasive than adjectives.

### Content

| Stat | Value | Label |
|------|-------|-------|
| Runtime | ~1.5KB | JS runtime (tree-shaken) |
| Protocol | 4 bytes | Per text update |
| Components | 51 | Production-ready |
| Connections | 200K+ | Per GB of RAM |

### Design

- 4-column grid using `Grid::new().columns(4)`
- Each stat: large number (`St::Text3xl`, `St::FontBold`, `St::TextAccent`) + label below (`St::TextSm`, `St::TextMuted`)
- Centered, with subtle dividers between stats
- Background: `St::BgSurfaceRaised` card with `St::RoundedLg`

---

## Section 3 — Code Example

Show real rwire code — a complete, runnable counter app. This is the "aha moment" for developers.

### Content

```rust
use rwire::components::*;
use rwire::{el, handler, renderer, El, Server, State};

#[derive(State, Default)]
#[storage(memory)]
struct Counter { count: i32 }

#[handler]
fn increment(state: &mut Counter) {
    state.count += 1;
}

#[renderer]
fn render_count(state: &Counter) -> ElementBuilder {
    Text::heading1(state.count.to_string()).build()
}

fn main() {
    Server::bind("0.0.0.0:9000").unwrap()
        .root(root)
        .run_blocking();
}
```

### Design

- Title: "A complete app in 20 lines" (`St::Text2xl`, `St::FontSemibold`)
- Subtitle: "Define state, write handlers, attach renderers. The macros handle the rest."
- Code block: `Code` component with syntax highlighting, line numbers
- Below code: annotation callouts pointing to key concepts:
  - `#[derive(State)]` → "State is a Rust struct. Typed, owned, serializable."
  - `#[handler]` → "Handlers are plain functions. No ceremony."
  - `#[renderer]` → "Renderers re-run automatically when state changes."

### Layout

Two-column on desktop: code on the left (60%), annotations on the right (40%).
Single column on mobile: code above, annotations below.

---

## Section 4 — Feature Grid

Six feature cards in a 3-column grid, each highlighting a key differentiator.

### Content

| Icon | Title | Description |
|------|-------|-------------|
| `Icon::Zap` (or Lightning) | Binary Protocol | DOM updates in 4 bytes. Not JSON, not HTML fragments — binary opcodes parsed in microseconds. |
| `Icon::Feather` (or Scale) | 1.5KB Runtime | Tree-shaken JS runtime. 20x smaller than LiveView, 200x smaller than Vaadin. Ships in a single TCP packet. |
| `Icon::Shield` | Fully Typed | State, handlers, renderers, components — all Rust. Catch errors at compile time, not in production. |
| `Icon::Cpu` | 200K Connections/GB | Rust async tasks use ~2-5KB per connection. No GC pauses, no JVM warmup, no per-process overhead. |
| `Icon::Palette` (or Brush) | 580+ Style Tokens | CSS encoded as 1-2 byte varint codes. Semantic theming with Nord palette, light/dark mode, ThemeStyle presets. |
| `Icon::Leaf` | Low Carbon | 60x less bandwidth than React SPA. Fewer bytes = less energy across servers, networks, and devices. |

### Design

- Grid: `Grid::new().columns(3).gap(Gap::Lg)`
- Cards: `Card` with `CardShadow::None`, icon + title + description
- Icons: `St::TextAccent`, size 24x24
- Title: `St::FontSemibold`, `St::TextDefault`
- Description: `St::TextSm`, `St::TextMuted`
- Cards have hover state: `St::BgHover` + subtle shadow transition

---

## Section 5 — Comparison Table

A compact table comparing rwire against 3-4 competitors on key metrics. Draws from the comparative study.

### Content

| | rwire | LiveView | Blazor | htmx |
|---|---|---|---|---|
| Client runtime | 1.5KB | 30KB | 200KB | 14KB |
| Wire format | Binary | JSON | JSON | HTML |
| Update cost | 4 bytes | 25 bytes | 100+ bytes | 100+ bytes |
| Memory/conn | 2-5KB | 5-50KB | 250KB | N/A |
| Language | Rust | Elixir | C# | Any |

### Design

- `Table` component with striped rows
- rwire column highlighted with `St::BgAccentSubtle`
- Title: "How rwire compares" (`St::Text2xl`, `St::FontSemibold`)
- Subtitle: "Real numbers, not marketing claims."
- Below table: link to full comparative study

---

## Section 6 — Getting Started CTA

Final call-to-action before the footer. Simple, direct.

### Content

```
Ready to build?

$ cargo add rwire

Read the docs →     Browse examples →
```

### Design

- Background: `St::BgAccentSubtle` for visual contrast
- Heading: `St::Text3xl`, `St::FontBold`, `St::TextCenter`
- Install command with CopyButton (centered)
- Two buttons: primary "Read the docs" + ghost "Browse examples"
- Generous vertical padding

---

## Page Assembly

```rust
fn build_landing_page() -> ElementBuilder {
    Stack::column()
        .gap(Gap::None)  // Sections manage own spacing
        .children([
            section_hero(),
            section_stats(),
            section_code_example(),
            section_features(),
            section_comparison(),
            section_cta(),
        ])
        .build()
}
```

The landing page lives in `examples/website/src/pages/landing.rs` (or inline in `main.rs` if the site is small enough).

---

## Verification Checklist

- [ ] Hero: heading, subtitle, install command with copy, two CTA buttons
- [ ] Stats: 4 numbers render in a row, responsive on mobile
- [ ] Code example: syntax highlighted, readable, annotations visible
- [ ] Feature grid: 6 cards in 3 columns, responsive to 2 → 1 on smaller screens
- [ ] Comparison table: renders with rwire column highlighted
- [ ] CTA: install command with copy, two action buttons
- [ ] Theme toggle works across all sections (light ↔ dark)
- [ ] Navigation: "Get Started" links to `/docs/getting-started/install`
- [ ] External links: GitHub opens in new tab
- [ ] Mobile viewport: all sections readable at 375px
- [ ] `cargo clippy --workspace` — zero warnings
