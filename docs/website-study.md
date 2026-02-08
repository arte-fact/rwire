# rwire Website — Design System Study & Gap Analysis

## Research Summary

### Framework Websites Analyzed
1. **Next.js** (nextjs.org) — The gold standard for framework sites
2. **Svelte** (svelte.dev) — Clean, elegant, community-focused
3. **Astro** (astro.build) — Content-driven, performance-focused
4. **Leptos** (leptos.dev) — Rust framework, code-heavy landing
5. **Dioxus** (dioxuslabs.com) — Rust framework, multiplatform focus

### Common Page Types Across All Sites
| Page | Next.js | Svelte | Astro | Leptos | Dioxus |
|------|---------|--------|-------|--------|--------|
| Landing / Hero | Yes | Yes | Yes | Yes | Yes |
| Documentation | Yes | Yes | Yes | Yes | Yes |
| Examples / Playground | Yes | Yes | Yes | Yes | No |
| Blog | Yes | Yes | Yes | No | Yes |
| Showcase / Gallery | Yes | No | Yes (Themes) | No | No |
| API Reference | Yes | Yes | Yes | Yes | Yes |

### Common Component Patterns (Across 5+ Sites)

**Landing Page Components:**
1. **Hero Section** — Large headline, tagline, 1-2 CTA buttons, optional install command
2. **Feature Grid** — 3-4 column grid of feature cards with icon + title + description
3. **Code Example** — Syntax-highlighted code block with copy button, sometimes tabbed
4. **Social Proof / Logo Cloud** — Company logos in a grid showing adoption
5. **Testimonials** — Quote cards with author attribution
6. **Install/Getting Started** — Terminal-style command display with copy-to-clipboard
7. **CTA Banner** — Full-width call-to-action at bottom of page

**Documentation Components:**
1. **Sidebar Navigation** — Collapsible sections with active state (HAVE: DocsSidebar)
2. **Table of Contents** — Right-side sticky TOC (HAVE: TableOfContents)
3. **Prose/Markdown** — Rendered markdown content (HAVE: Prose)
4. **Code Blocks** — Syntax highlighted with copy button (HAVE: Code, but no copy button)
5. **Search** — Global search with keyboard shortcut (HAVE: basic search)
6. **Breadcrumbs** — Page path navigation (HAVE: Breadcrumb)
7. **Pagination** — Previous/Next page navigation at bottom (HAVE: Pagination)
8. **Version Switcher** — Dropdown for different versions

**Shared Components:**
1. **Header/Navbar** — Logo + nav links + search + theme toggle (PARTIALLY HAVE)
2. **Footer** — Multi-column link grid + social links + newsletter (DON'T HAVE)
3. **Theme Toggle** — Light/dark mode switcher (HAVE: ThemeToggle)
4. **Mobile Menu** — Hamburger → slide-out drawer (HAVE: Drawer)

---

## Gap Analysis: rwire Component Inventory vs. Requirements

### Current Inventory (51 components)
**Layout:** Stack, Container, Spacer, Divider, AppShell
**Navigation:** NavMenu, Breadcrumb, Pagination, Tabs, Link
**Data Display:** Card, Badge, Tag, Table, List, Code, Prose, Timeline, Stat, Avatar, AvatarGroup, Image, Blockquote, Kbd
**Forms:** Button, Input, Textarea, Select, Checkbox, Radio, Switch, Slider, FormField, Label
**Feedback:** Alert, Toast, Spinner, Progress, Skeleton, EmptyState, Tooltip
**Overlay:** Modal, Drawer, Dropdown
**Docs:** TableOfContents, DocsSidebar, Accordion
**Other:** ThemeToggle, Stepper, Text

### Missing Components for Website (Prioritized)

#### P0 — Must Have for MVP Website

1. **Hero** — Landing page hero section
   - Large heading, subtitle text, CTA buttons row, optional code snippet
   - Uses: Stack, Text, Button, Code (composition)
   - **Verdict: Compose from existing components (Stack + heading + buttons). No new component needed.**

2. **Footer** — Multi-column footer with link groups
   - Logo, link columns (Resources, Community, Legal), social icons, copyright
   - Uses: Stack, Link, Divider, icons
   - **NEW COMPONENT NEEDED: `Footer` / `FooterColumn`**

3. **FeatureCard** — Icon + title + description card for feature grids
   - Consistent layout, icon slot, title, short description
   - Uses: Card, Stack, Text, icons
   - **Verdict: Compose from Card + Stack. No new component needed.**

4. **LogoCloud** — Grid of company/partner logos (social proof)
   - Responsive grid of grayscale logos, optionally linking out
   - Uses: Image, Stack/grid layout
   - **NEW COMPONENT NEEDED: `LogoCloud`**

5. **CopyButton** — Copy-to-clipboard button (for install commands & code blocks)
   - Click → copies text → shows "Copied!" feedback → resets
   - Needs: client-local state for feedback, clipboard API
   - **NEW COMPONENT NEEDED: `CopyButton`** (or enhance Code component)

6. **CommandLine** — Terminal-style install command display
   - Monospace text with $ prefix, copy button
   - Uses: Code, CopyButton
   - **Verdict: Can compose from Code + CopyButton. No separate component needed.**

#### P1 — Should Have for Polish

7. **Testimonial** — Quote card with author, role, company, avatar
   - Blockquote + author attribution
   - Uses: Card, Blockquote, Avatar, Text
   - **Verdict: Compose from existing. No new component needed.**

8. **FeatureGrid** — Responsive grid layout component
   - Auto-responsive grid (CSS Grid with auto-fill/minmax)
   - **NEW COMPONENT NEEDED: `Grid`** (generic responsive grid, more flexible than Stack)

9. **SocialLinks** — Row of icon links to GitHub, Discord, X, etc.
   - Consistent icon sizing, hover states, external link behavior
   - Needs new icons: GitHub, Discord, X/Twitter, YouTube, Bluesky
   - **Verdict: Row of icon+links. Need new ICONS, no new component.**

10. **SearchDialog** — Modal-style search with keyboard shortcut (Ctrl+K)
    - Full-screen overlay, text input, result list, keyboard nav
    - Uses: Modal, Input, List
    - **Verdict: Compose from Modal + Input + List. Could be a recipe, not a component.**

#### P2 — Nice to Have

11. **Announcement Banner** — Top-of-page dismissible banner for news/releases
    - Colored strip, text + link, close button
    - Uses: Alert variant or standalone
    - **Verdict: Can use Alert with custom styling or compose. Low priority.**

12. **ThemeShowcase** — Grid of theme/template preview cards
    - Image preview + title + link (like Astro's ecosystem section)
    - Uses: Card, Image, Link
    - **Verdict: Compose from Card + Image. No new component.**

---

## New Components to Build

### 1. `Footer` Component
```rust
Footer::new()
    .logo(el(El::Div).text("rwire"))
    .column(FooterColumn::new("Resources")
        .link("Docs", "/docs")
        .link("Examples", "/examples")
        .link("Blog", "/blog"))
    .column(FooterColumn::new("Community")
        .link("GitHub", "https://github.com/...")
        .link("Discord", "https://discord.gg/..."))
    .social(SocialLink::github("https://github.com/..."))
    .social(SocialLink::discord("https://discord.gg/..."))
    .copyright("2026 rwire")
    .build()
```

### 2. `LogoCloud` Component
```rust
LogoCloud::new()
    .title("Trusted by")
    .logo(LogoItem::new("Company A", "/logos/a.svg"))
    .logo(LogoItem::new("Company B", "/logos/b.svg"))
    .columns(4)
    .grayscale(true)
    .build()
```

### 3. `Grid` Component
```rust
Grid::new()
    .columns(3)           // Fixed column count
    .min_column("280px")  // Or auto-responsive with minmax
    .gap(Gap::Lg)
    .children([card1, card2, card3, ...])
    .build()
```

### 4. `CopyButton` Component
```rust
CopyButton::new("npm install rwire")
    .size(ButtonSize::Sm)
    .build()
```

### 5. New Icons Needed
```rust
// Social/brand icons
Icon::GitHub
Icon::Discord
Icon::Twitter    // or Icon::X
Icon::YouTube
Icon::Bluesky
Icon::Npm        // or Icon::Crate for crates.io
Icon::Terminal

// Additional UI icons
Icon::Clipboard  // for copy button
Icon::ExternalLink // already have Icon::External
```

---

## New Style Tokens Needed

### Grid Layout Tokens
```rust
St::DisplayGrid       // display:grid
St::GridCols1         // grid-template-columns:repeat(1,1fr)
St::GridCols2         // grid-template-columns:repeat(2,1fr)
St::GridCols3         // grid-template-columns:repeat(3,1fr)
St::GridCols4         // grid-template-columns:repeat(4,1fr)
St::GridColsAuto      // grid-template-columns:repeat(auto-fill,minmax(280px,1fr))
```

### Additional Spacing/Typography
```rust
St::Text4xl          // font-size:2.25rem (larger hero headings)
St::Text5xl          // font-size:3rem
St::Text6xl          // font-size:3.75rem
St::LeadingTight     // line-height:1.25
St::LeadingSnug      // line-height:1.375
St::MaxW4xl          // max-width:56rem (for hero content centering)
St::MaxW5xl          // max-width:64rem
St::Grayscale        // filter:grayscale(1) (for logo clouds)
St::OpacityHalf      // opacity:0.5
```

---

## Website Page Structure

### Pages to Build

1. **`/`** — Landing page
   - Hero: "Server-side UI with a binary protocol" + install command
   - Feature grid: 6 key features with icons
   - Code example: Counter in rwire (syntax highlighted)
   - Performance stats: ~1.5KB runtime, binary protocol
   - Getting started section

2. **`/docs/*`** — Documentation (existing docs-site, enhanced)
   - Getting Started / Installation
   - Core Concepts (state, handlers, renderers)
   - Components guide
   - Binary Protocol reference
   - API Reference

3. **`/examples`** — Interactive examples gallery
   - Counter, TodoList, Forms, etc.
   - Code + live preview cards

4. **`/blog`** — (optional, P2)

### Theme Configuration

```rust
CapsuleConfig::new()
    .theme(Theme::default())
    .palette(ColorPalette::nord())
    .font(FontFace::google("Inter", &[400, 500, 600, 700]))
```

Both light and dark mode using Nord colors with accent derived from Nord Frost (nord8 = #88C0D0).

---

## Implementation Order

### Phase 1: New Style Tokens
1. Add grid layout St tokens (DisplayGrid, GridCols1-4, GridColsAuto)
2. Add larger typography tokens (Text4xl-6xl, LeadingTight/Snug)
3. Add utility tokens (Grayscale, OpacityHalf, MaxW4xl/5xl)

### Phase 2: New Components
4. Build `Grid` component
5. Build `Footer` / `FooterColumn` component
6. Build `CopyButton` component (or integrate into Code)
7. Add social/brand icons (GitHub, Discord, X, etc.)
8. Build `LogoCloud` component

### Phase 3: Website
9. Create `examples/website/` crate
10. Build landing page with hero, feature grid, code example, stats
11. Integrate existing docs-site functionality
12. Add footer across all pages
13. Configure Nord theme (light + dark)
14. Test with Playwright

### Phase 4: Polish
15. Add examples gallery page
16. Add CTA sections
17. Responsive testing

## Verification
- `cargo clippy --workspace` — zero warnings
- `cargo test --workspace` — all pass
- Visual testing with Playwright in both light and dark mode
- Mobile viewport testing
