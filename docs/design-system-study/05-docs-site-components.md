# 05 — Docs Site Components

Components needed for a full documentation website built with rwire.

## Page Structure

A documentation site has four primary page types, each with specific component requirements:

```
┌─────────────────────────────────────────────────┐
│  Header (logo, nav links, theme toggle, search) │
├──────────┬──────────────────────────┬───────────┤
│          │                          │           │
│ Sidebar  │     Main Content         │   TOC     │
│ (nav)    │     (markdown)           │  (right)  │
│          │                          │           │
│ Getting  │  # Page Title            │  Title    │
│ Started  │                          │  Section1 │
│  Install │  Paragraph text with     │  Section2 │
│  Quick.. │  `inline code` and       │  Section3 │
│ Compnts  │  [links](/path).         │           │
│  Button  │                          │           │
│  Input   │  ```rust                 │           │
│  ...     │  fn example() {}         │           │
│          │  ```                     │           │
│          │                          │           │
│          │  > Blockquote callout    │           │
│          │                          │           │
│          │  | Col1 | Col2 |         │           │
│          │  |------|------|         │           │
│          │  | data | data |         │           │
│          │                          │           │
├──────────┴──────────────────────────┴───────────┤
│  Footer (links, copyright)                      │
└─────────────────────────────────────────────────┘
```

## Component Usage by Page Type

### 1. Landing Page
| Component | Usage |
|-----------|-------|
| AppShell | Page layout (header only, no sidebar) |
| Container | Max-width content wrapper |
| Stack | Section layout |
| Text | Headings, body text |
| Button | CTA buttons ("Get Started", "View on GitHub") |
| Card | Feature cards |
| Code | Inline code in feature descriptions |
| Badge | Version badge, status indicators |
| Link | Navigation links |

### 2. Documentation Page
| Component | Usage |
|-----------|-------|
| AppShell | Header + sidebar + main |
| DocsSidebar | Left navigation |
| TableOfContents | Right heading nav |
| Prose | Markdown content rendering |
| Code/CodeBlock | Inline and block code |
| Blockquote | Callouts and notes |
| Table | Data tables in docs |
| Text | Headings within sidebar/header |
| Link | Cross-references |
| Breadcrumb | Page path |
| Tabs | Code examples in multiple languages |

### 3. Component Showcase Page
| Component | Usage |
|-----------|-------|
| All docs page components | Same layout |
| Card | Component preview containers |
| Stack | Example layouts |
| (Every component) | Live preview of each component |
| Code | Source code for each example |
| Table | Props/API documentation table |

### 4. Search Results Page
| Component | Usage |
|-----------|-------|
| AppShell | Page layout |
| Input | Search input (type=search) |
| Stack | Results list |
| Card | Individual result cards |
| Text | Result title, snippet |
| Link | Result links |
| Badge | Result category badges |
| Skeleton | Loading state |
| Pagination | Result pages |

## New Components Specification

### AppShell

The page scaffolding component. Uses CSS Grid for layout.

```rust
AppShell::new()
    .header(
        Stack::row().gap(Gap::Md).justify(StackJustify::Between).children([
            Link::new("/").text("rwire").build(),
            Stack::row().gap(Gap::Sm).children([
                Link::new("/docs").text("Docs").build(),
                Link::new("/components").text("Components").build(),
                ThemeToggle::new().build(),
            ]).build(),
        ]).build()
    )
    .sidebar(docs_sidebar)
    .main(page_content)
    .build()
```

**CSS structure:**
```css
.rw-shell {
    display: grid;
    grid-template-rows: auto 1fr;
    grid-template-columns: var(--shell-sidebar-w, 260px) 1fr;
    grid-template-areas:
        "header header"
        "sidebar main";
    min-height: 100vh;
}
.rw-shell-header {
    grid-area: header;
    position: sticky;
    top: 0;
    z-index: 1100;
    border-bottom: 1px solid var(--rw-border);
    padding: 0 var(--rw-space-4);
    height: var(--shell-header-h, 56px);
    display: flex;
    align-items: center;
}
.rw-shell-sidebar {
    grid-area: sidebar;
    border-right: 1px solid var(--rw-border);
    overflow-y: auto;
    position: sticky;
    top: var(--shell-header-h, 56px);
    height: calc(100vh - var(--shell-header-h, 56px));
    padding: var(--rw-space-4);
}
.rw-shell-main {
    grid-area: main;
    padding: var(--rw-space-6);
    min-width: 0;
}
/* No sidebar variant */
.rw-shell-no-sidebar {
    grid-template-columns: 1fr;
    grid-template-areas: "header" "main";
}
```

**Responsive**: On screens < 768px, sidebar collapses. A hamburger toggle reveals it as an overlay.

---

### DocsSidebar

Navigation tree with collapsible sections.

```rust
DocsSidebar::new()
    .section(SidebarSection::new("Getting Started")
        .link("/docs/install", "Installation")
        .link("/docs/quickstart", "Quick Start"))
    .section(SidebarSection::new("Core")
        .link("/docs/architecture", "Architecture")
        .link("/docs/protocol", "Binary Protocol")
        .link("/docs/state", "State Management"))
    .section(SidebarSection::new("Components")
        .link("/docs/components/button", "Button")
        .link("/docs/components/input", "Input")
        .link("/docs/components/stack", "Stack"))
    .active_path("/docs/quickstart")
    .build()
```

**CSS structure:**
```css
.rw-sidebar { display: flex; flex-direction: column; gap: var(--rw-space-1); }
.rw-sidebar-section { margin-bottom: var(--rw-space-2); }
.rw-sidebar-heading {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--rw-text-muted);
    padding: var(--rw-space-1) var(--rw-space-2);
}
.rw-sidebar-link {
    display: block;
    padding: var(--rw-space-1) var(--rw-space-2) var(--rw-space-1) var(--rw-space-4);
    color: var(--rw-text);
    text-decoration: none;
    border-radius: var(--rw-radius);
    font-size: 0.875rem;
}
.rw-sidebar-link:hover { background: var(--rw-bg-hover); }
.rw-sidebar-link-active {
    background: var(--rw-bg-active);
    color: var(--rw-primary);
    font-weight: 500;
}
```

---

### TableOfContents

Right-side heading navigation for the current page.

```rust
TableOfContents::new()
    .heading(2, "Installation", "#installation")
    .heading(2, "Quick Start", "#quick-start")
    .heading(3, "Counter Example", "#counter-example")
    .heading(3, "Todo Example", "#todo-example")
    .heading(2, "Next Steps", "#next-steps")
    .build()
```

**CSS structure:**
```css
.rw-toc {
    position: sticky;
    top: calc(var(--shell-header-h, 56px) + var(--rw-space-4));
    max-height: calc(100vh - var(--shell-header-h, 56px) - var(--rw-space-8));
    overflow-y: auto;
    font-size: 0.8125rem;
}
.rw-toc-link {
    display: block;
    padding: var(--rw-space-1) 0;
    color: var(--rw-text-muted);
    text-decoration: none;
    border-left: 2px solid transparent;
    padding-left: var(--rw-space-3);
}
.rw-toc-link:hover { color: var(--rw-text); }
.rw-toc-link-active {
    color: var(--rw-primary);
    border-left-color: var(--rw-primary);
}
.rw-toc-h3 { padding-left: var(--rw-space-6); }
```

**Extended layout with TOC**: When AppShell includes a TOC, the grid becomes three columns:
```css
.rw-shell-with-toc {
    grid-template-columns: var(--shell-sidebar-w, 260px) 1fr var(--shell-toc-w, 200px);
    grid-template-areas: "header header header" "sidebar main toc";
}
```

---

### Prose

Container that applies typography styles to rendered markdown content.

```rust
Prose::new()
    .child(render_markdown(markdown_string))
    .build()
```

**CSS structure** (abbreviated, showing key styles):
```css
.rw-prose { line-height: 1.7; color: var(--rw-text); max-width: 65ch; }
.rw-prose h1 { font-size: 2rem; font-weight: 700; margin: 2rem 0 1rem; }
.rw-prose h2 { font-size: 1.5rem; font-weight: 600; margin: 1.75rem 0 0.75rem; border-bottom: 1px solid var(--rw-border); padding-bottom: 0.5rem; }
.rw-prose h3 { font-size: 1.25rem; font-weight: 600; margin: 1.5rem 0 0.5rem; }
.rw-prose p { margin: 0 0 1rem; }
.rw-prose a { color: var(--rw-primary); text-decoration: underline; }
.rw-prose code { background: var(--rw-bg-subtle); padding: 0.125rem 0.375rem; border-radius: 3px; font-size: 0.875em; }
.rw-prose pre { background: var(--rw-bg-code); padding: var(--rw-space-4); border-radius: var(--rw-radius); overflow-x: auto; margin: 0 0 1rem; }
.rw-prose pre code { background: none; padding: 0; font-size: 0.875rem; }
.rw-prose blockquote { border-left: 3px solid var(--rw-border); padding-left: var(--rw-space-4); color: var(--rw-text-muted); margin: 0 0 1rem; }
.rw-prose ul,.rw-prose ol { padding-left: 1.5rem; margin: 0 0 1rem; }
.rw-prose li { margin: 0.25rem 0; }
.rw-prose table { width: 100%; border-collapse: collapse; margin: 0 0 1rem; }
.rw-prose th,.rw-prose td { padding: 0.5rem; border: 1px solid var(--rw-border); text-align: left; }
.rw-prose th { font-weight: 600; background: var(--rw-bg-subtle); }
.rw-prose img { max-width: 100%; height: auto; border-radius: var(--rw-radius); }
.rw-prose hr { border: none; border-top: 1px solid var(--rw-border); margin: 2rem 0; }
```

---

### Code/CodeBlock

```rust
// Inline
Code::inline("ElementBuilder").build()

// Block
Code::block(r#"
fn main() {
    let app = counter::app();
    rwire::serve(app).run();
}
"#).language("rust").build()
```

**CSS structure:**
```css
.rw-code {
    background: var(--rw-bg-subtle);
    padding: 0.125rem 0.375rem;
    border-radius: 3px;
    font-family: var(--rw-font-mono);
    font-size: 0.875em;
}
.rw-code-block {
    display: block;
    background: var(--rw-bg-code);
    padding: var(--rw-space-4);
    border-radius: var(--rw-radius);
    overflow-x: auto;
    font-family: var(--rw-font-mono);
    font-size: 0.875rem;
    line-height: 1.6;
    margin: 0 0 1rem;
}
.rw-code-lang {
    display: block;
    font-size: 0.75rem;
    color: var(--rw-text-muted);
    margin-bottom: var(--rw-space-1);
    text-transform: uppercase;
}
```

---

### Accordion

```rust
Accordion::new()
    .item(AccordionItem::new("What is rwire?")
        .content(el(El::P).text("A server-rendered UI framework...")))
    .item(AccordionItem::new("How does the binary protocol work?")
        .content(el(El::P).text("Single-byte opcodes followed by...")))
    .build()
```

**CSS structure:**
```css
.rw-accordion { display: flex; flex-direction: column; }
.rw-accordion-item { border-bottom: 1px solid var(--rw-border); }
.rw-accordion-trigger {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: var(--rw-space-3) 0;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.9375rem;
    font-weight: 500;
    color: var(--rw-text);
    text-align: left;
}
.rw-accordion-trigger::after {
    content: "+";
    font-size: 1.25rem;
    transition: transform 0.2s;
}
.rw-accordion-open > .rw-accordion-trigger::after {
    content: "−";
}
.rw-accordion-content {
    max-height: 0;
    overflow: hidden;
    transition: max-height 0.2s;
}
.rw-accordion-open > .rw-accordion-content {
    max-height: 1000px;
    padding-bottom: var(--rw-space-3);
}
```

**Interaction**: Local handler toggles `.rw-accordion-open` class on the item. `allow_multiple` controls whether opening one closes others.

---

### Skeleton

```rust
Skeleton::text().build()
Skeleton::text().lines(3).build()
Skeleton::rect().height(200).build()
```

**CSS structure:**
```css
.rw-skeleton {
    background: var(--rw-bg-subtle);
    border-radius: var(--rw-radius);
    animation: rw-skeleton-pulse 1.5s ease-in-out infinite;
}
.rw-skeleton-text { height: 1rem; margin-bottom: 0.5rem; }
.rw-skeleton-text:last-child { width: 60%; }
.rw-skeleton-rect { height: 200px; }
.rw-skeleton-circle { border-radius: 50%; }
@keyframes rw-skeleton-pulse {
    0%,100% { opacity: 1; }
    50% { opacity: 0.4; }
}
```

---

### Blockquote

```rust
Blockquote::new("Server-rendered UI means the browser is just a thin display layer.").build()
```

**CSS structure:**
```css
.rw-blockquote {
    border-left: 3px solid var(--rw-primary);
    padding: var(--rw-space-3) var(--rw-space-4);
    margin: 0 0 1rem;
    color: var(--rw-text-muted);
    font-style: italic;
    background: var(--rw-bg-subtle);
    border-radius: 0 var(--rw-radius) var(--rw-radius) 0;
}
```

## Existing Components Used by Docs Site

These components already exist and will be used extensively:

| Component | Docs Site Usage |
|-----------|----------------|
| Button | CTA buttons, copy-to-clipboard, navigation |
| Input (search) | Docs search bar |
| Text | Headings in header/sidebar |
| Link | All navigation |
| Badge | Version indicators, component status |
| Card | Feature showcases, component previews |
| Stack | Layout composition everywhere |
| Container | Max-width content wrappers |
| Tabs | Multi-language code examples |
| Breadcrumb | Page location indicator |
| ThemeToggle | Light/dark mode switch |
| Table | API documentation tables |
| Spinner | Loading states |
| Divider | Section separators |
| Pagination | Search results, long component lists |

## Component Count Summary

| Category | Existing | New (Tier 1) | Total |
|----------|----------|-------------|-------|
| Used from current library | 15 | — | 15 |
| New for docs site | — | 8 | 8 |
| **Total unique components** | | | **23** |
