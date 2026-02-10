# Phase 4: Documentation Hub

**Goal**: Enhance the existing docs-site with comprehensive content covering all of rwire's features.

**Depends on**: Phase 1 (tokens for layout improvements)

**Note**: This phase can proceed in parallel with Phase 3 (landing page) since they share only the Phase 1 foundation.

---

## Current State

The docs-site (`examples/docs-site/`) already works with:
- AppShell layout (header + sidebar + main content)
- Markdown rendering via `DocSite::load()` and `parse_markdown()`
- Sidebar navigation with active state
- Table of contents (right-side sticky)
- Search with real-time filtering
- Breadcrumb navigation
- Theme toggle (light/dark) and ThemeStyle switching
- Client-side routing via `Link::to()` and `on_route`

### What Needs Enhancement

1. **Content** — The current docs are sparse. Need comprehensive guides.
2. **Code examples** — More inline code with syntax highlighting.
3. **Navigation** — Previous/Next page links at bottom of each doc page.
4. **Footer** — Add the Footer component from Phase 2.

---

## Documentation Structure

### Section 1: Getting Started

| Page | Path | Content |
|------|------|---------|
| Installation | `/docs/getting-started/install` | Prerequisites, `cargo add rwire`, first project setup |
| Quick Start | `/docs/getting-started/quick-start` | Counter app walkthrough, step by step |
| Project Structure | `/docs/getting-started/project-structure` | Crate layout, `main.rs` anatomy, server startup |

**Installation page content outline**:
```markdown
# Installation

## Prerequisites
- Rust 1.75+ (edition 2021)
- A modern browser (Chrome, Firefox, Safari, Edge)

## Create a New Project
$ cargo new my-app
$ cd my-app
$ cargo add rwire

## Hello World
[full code example]

## Run It
$ cargo run
# Open http://127.0.0.1:9000

## What Just Happened?
[explain: server started, browser connects via WebSocket,
 capsule JS (~1.5KB) loads, binary opcodes render the UI]
```

### Section 2: Core Concepts

| Page | Path | Content |
|------|------|---------|
| State | `/docs/core-concepts/state` | #[derive(State)], storage types, fields, defaults |
| Handlers | `/docs/core-concepts/handlers` | #[handler], EventContext, ctx.text(), mutations |
| Renderers | `/docs/core-concepts/renderers` | #[renderer], reactive re-rendering, synced regions |
| Elements | `/docs/core-concepts/elements` | el(), El enum, ElementBuilder API, nesting |
| Events | `/docs/core-concepts/events` | Ev enum, on(), on_ref(), debounced, local handlers |
| Binary Protocol | `/docs/core-concepts/protocol` | Opcodes, symbol table, varint, wire format |

**State page content outline**:
```markdown
# State

All application state lives on the server as a Rust struct.

## Defining State
#[derive(State, Default)]
#[storage(memory)]
struct AppState { ... }

## Storage Types
- `memory` — Server RAM, lost on restart
- `persisted` — Survives restart (JSON file store)
- `local` — Client-side only (no round-trip)

## Accessing State
Handlers receive `&mut State`, renderers receive `&State`.
No global state, no context providers, no prop drilling.
```

### Section 3: Components

| Page | Path | Content |
|------|------|---------|
| Overview | `/docs/components/overview` | Component philosophy, builder pattern, St tokens |
| Layout | `/docs/components/layout` | Stack, Grid, Container, Spacer, Divider, AppShell |
| Navigation | `/docs/components/navigation` | NavMenu, Breadcrumb, Pagination, Tabs, Link |
| Data Display | `/docs/components/data-display` | Card, Badge, Table, Code, Prose, Timeline, Stat |
| Forms | `/docs/components/forms` | Button, Input, Select, Checkbox, Radio, Switch |
| Feedback | `/docs/components/feedback` | Alert, Toast, Spinner, Progress, Skeleton, Modal |

Each component page follows the pattern:
1. **Import** — `use rwire_components::*;`
2. **Basic usage** — Simplest example
3. **Variants** — Size, color, style options
4. **With events** — Interactive example
5. **Tokens** — Style tokens used by this component

### Section 4: Theming

| Page | Path | Content |
|------|------|---------|
| Style Tokens | `/docs/theming/tokens` | St enum, how tokens work, varint encoding |
| Color Palettes | `/docs/theming/palettes` | ColorPalette, Nord, custom palettes |
| Dark Mode | `/docs/theming/dark-mode` | ThemeMode, ThemeToggle, data-theme attribute |
| ThemeStyle | `/docs/theming/theme-styles` | Default, Soft, Brutalist, Minimal presets |
| Custom Themes | `/docs/theming/custom` | Creating custom palettes, extending tokens |

### Section 5: Advanced

| Page | Path | Content |
|------|------|---------|
| Router | `/docs/advanced/router` | Link::to(), on_route, pushState, back/forward |
| ItemRef | `/docs/advanced/item-ref` | Dynamic lists, iter_with_ref(), type-safe binding |
| Local Mutations | `/docs/advanced/local-mutations` | #[storage(local)], client-side execution |
| Tree Shaking | `/docs/advanced/tree-shaking` | BuildContext, symbol collection, capsule generation |
| Configuration | `/docs/advanced/config` | CapsuleConfig, FontFace, Server::bind options |

---

## Content Writing Guidelines

1. **Start with "why"** — Each page opens with the problem it solves.
2. **Show code first** — Lead with a code example, then explain.
3. **Keep it short** — Prefer 200-400 words per page. Developers skim.
4. **Real examples** — Use examples from the counter, todolist, and docs-site apps.
5. **Link to source** — Reference actual file paths in the codebase.

---

## Step 4.1 — Write Getting Started Content

Write the 3 Getting Started pages as markdown files in `examples/docs-site/docs/getting-started/`.

Priority: This is the most important documentation. A developer's first 5 minutes with rwire.

---

## Step 4.2 — Write Core Concepts Content

Write the 6 Core Concepts pages. These are the "reference manual" — thorough but not exhaustive.

---

## Step 4.3 — Write Component Guides

Write the 6 Components section pages. Each page covers multiple related components.

---

## Step 4.4 — Write Theming Guides

Write the 5 Theming pages. Include visual examples of light/dark mode and ThemeStyle presets.

---

## Step 4.5 — Write Advanced Guides

Write the 5 Advanced pages. These target developers who've built their first app and want to go deeper.

---

## Step 4.6 — Add Previous/Next Navigation

Add `Pagination` component at the bottom of each doc page, linking to the previous and next page in the section.

```rust
// At bottom of doc page
Pagination::new()
    .prev("Installation", "/docs/getting-started/install")
    .next("Quick Start", "/docs/getting-started/quick-start")
    .build()
```

---

## Step 4.7 — Add Footer

Add the `Footer` component (from Phase 2) to the AppShell layout so it appears on every page.

---

## Verification Checklist

- [ ] All 25 documentation pages render without errors
- [ ] Sidebar shows correct active state for each page
- [ ] TOC generates correct anchor links for each page
- [ ] Search finds content across all pages
- [ ] Previous/Next navigation works at bottom of each page
- [ ] Code examples are syntax-highlighted
- [ ] Footer appears on all pages
- [ ] Light/dark mode works across all content
- [ ] No broken internal links
- [ ] `cargo clippy --workspace` — zero warnings
- [ ] `cargo test --workspace` — all pass
