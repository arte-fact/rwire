# rwire Website — Roadmap

> **Status (2026-07-07): SHIPPED.** The website (`apps/rwire-website/`), docs site
> (`apps/rwire-docs/`), design system (`apps/rwire-design-system/`), and examples
> gallery (`apps/rwire-examples/`) are all built and live in-tree. The foundation
> tokens, the `Grid`/`Footer`/`CopyButton` components, and the full landing page
> (hero, stats, code example, features, surfaces, comparison, CTA) are done.
> This document is kept as the historical plan; the phases below are annotated
> with their actual outcome.

A phased implementation plan for rwire's official website, combining the technical strengths identified in the [comparative study](../comparative-study.md) with the design patterns catalogued in the [website study](../website-study.md).

## Vision

The rwire website is the framework's first impression. It should **demonstrate** rwire's strengths by being itself: a fast, lightweight, server-driven site built entirely with rwire components and the binary protocol. Every page load is a live proof of the framework's claims — sub-2KB runtime, instant rendering, minimal bandwidth.

### Design Principles

1. **Show, don't tell** — The site itself is the demo. Visitors experience the binary protocol firsthand.
2. **Numbers are persuasive** — Surface real metrics (runtime size, bytes per update, connections per GB) prominently.
3. **Code is the hero** — rwire's Rust API is beautiful and concise. Show real code, not marketing abstractions.
4. **Nord palette, light and dark** — Clean, professional aesthetic with seamless theme switching.
5. **Accessible and fast everywhere** — Works on 3G, works with screen readers, works without JS (graceful degradation).

---

## Phase Overview

| Phase | Focus | Deliverables | Depends On | Status |
|-------|-------|-------------|------------|--------|
| [Phase 1](./01-foundation.md) | Design System Foundation | ~15 new style tokens, 7 new icons, Grid component | — | ✅ Shipped (Grid tokens + `Grid` component in `rwire-components`) |
| [Phase 2](./02-components.md) | Website Components | Footer, CopyButton, LogoCloud components | Phase 1 | ✅ Shipped (`Footer`, `CopyButton` in `rwire-components`; LogoCloud not needed) |
| [Phase 3](./03-landing-page.md) | Landing Page | Hero, features, code example, stats, CTA | Phase 2 | ✅ Shipped (`apps/rwire-website/`, 714 lines, 7 sections) |
| [Phase 4](./04-documentation.md) | Documentation Hub | Enhanced docs-site with full content | Phase 1 | ✅ Shipped (`apps/rwire-docs/`, 26-page docs site with sidebar + mobile drawer) |
| [Phase 5](./05-polish.md) | Polish & Launch | Examples gallery, responsive testing, performance audit | Phase 3+4 | ✅ Shipped (`apps/rwire-examples/` gallery); visual/performance audit deferred to post-launch |

---

## Site Map

```
/                          Landing page (hero, features, code, stats)
/docs/                     Documentation hub
/docs/getting-started/     Installation, first app, project structure
/docs/core-concepts/       State, handlers, renderers, binary protocol
/docs/components/          Component catalog with examples
/docs/theming/             Tokens, palettes, ThemeStyle, dark mode
/docs/advanced/            Router, ItemRef, local mutations, tree-shaking
/examples                  Interactive examples gallery
```

## Theme

```rust
CapsuleConfig::new()
    .theme(Theme::default())
    .palette(ColorPalette::nord())
    .font(FontFace::google("Quicksand", &[300, 400]))
```

- **Light mode**: Nord Snow (#ECEFF4) background, Polar Night (#2E3440) text
- **Dark mode**: Polar Night (#2E3440) background, Snow (#ECEFF4) text
- **Accent**: Nord Frost (#88C0D0 — nord8) for links, buttons, highlights
- **Code**: Nord Aurora colors for syntax highlighting

## Quality Gates

Every phase must pass before the next begins:

- `cargo clippy --workspace` — zero warnings
- `cargo test --workspace` — all tests pass
- Playwright visual tests in both light and dark mode
- Mobile viewport check (375px, 768px, 1024px, 1440px)
