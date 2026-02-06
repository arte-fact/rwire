# 04 — Gap Analysis

Missing components organized by category with priority tiers, CSS budget estimates, and implementation notes.

## Priority Tier Definitions

| Tier | Criteria | Timeline |
|------|----------|----------|
| **Tier 1** | Required for the docs site | Phase 1 — build these first |
| **Tier 2** | Component parity with shadcn/ui | Phase 2 — round out the library |
| **Tier 3** | Nice-to-have, low demand | Phase 3 — add on request |

## Tier 1 — Required for Docs Site

These components are directly needed to build a documentation website with rwire.

### Accordion (~400B CSS)

**Purpose**: Collapsible content sections. Essential for sidebar navigation (collapsible doc categories) and FAQ pages.

**API sketch:**
```rust
Accordion::new()
    .item(AccordionItem::new("Getting Started")
        .content(el(El::Div).text("...")))
    .item(AccordionItem::new("API Reference")
        .content(el(El::Div).text("...")))
    .allow_multiple(true)   // multiple sections open at once
    .build()
```

**CSS**: `.rw-accordion`, `.rw-accordion-item`, `.rw-accordion-trigger`, `.rw-accordion-content`, `.rw-accordion-open`
**Protocol**: No new element types needed. Uses `div`, `button`, local handler for toggle.
**Interaction**: Local handler toggles `.rw-accordion-open` class. CSS handles show/hide transition.

---

### Code / CodeBlock (~350B CSS)

**Purpose**: Inline code and code blocks for documentation. Every docs page will have code examples.

**API sketch:**
```rust
// Inline code
Code::inline("let x = 42").build()

// Block code
Code::block("fn main() {\n    println!(\"Hello\");\n}")
    .language("rust")
    .build()
```

**CSS**: `.rw-code`, `.rw-code-block`, optional `.rw-code-lang-*` for language labels
**Protocol**: Needs `El::Code` and `El::Pre` element types.
**Note**: Syntax highlighting is a future enhancement. Initial version uses monospace font with background color. Server-side highlighting (via `syntect` or similar) could emit colored `<span>` elements later.

---

### Blockquote (~150B CSS)

**Purpose**: Styled blockquotes for markdown content. Used in documentation for callouts and quotes.

**API sketch:**
```rust
Blockquote::new("rwire is a server-rendered framework.").build()
Blockquote::new("Note: This is experimental.")
    .cite("https://rwire.dev/docs")
    .build()
```

**CSS**: `.rw-blockquote` — left border, italic text, muted color
**Protocol**: Needs `El::Blockquote` element type.

---

### Prose (~800B CSS)

**Purpose**: Typography styles for rendered markdown/HTML content. Applies sensible defaults to headings, paragraphs, lists, code blocks, links, and tables within a container.

**API sketch:**
```rust
Prose::new()
    .child(markdown_to_elements(content))
    .build()
```

**CSS**: `.rw-prose` container that styles descendant elements:
- `h1-h3`: sizes, margins, font-weight
- `p`: line-height, margins
- `a`: link color, underline
- `ul/ol`: list-style, padding
- `code`: inline code background
- `pre code`: block code styling
- `blockquote`: border, padding
- `table`: borders, padding
- `hr`: margin, color
- `img`: max-width: 100%

**Note**: This is the most CSS-heavy Tier 1 component because it must style ~15 descendant element types. The 800B budget is tight but achievable with minified CSS and shared property values.

---

### Skeleton (~200B CSS)

**Purpose**: Loading placeholder that shows content shape before data arrives. Useful for docs pages loading markdown content.

**API sketch:**
```rust
Skeleton::text().build()          // single line placeholder
Skeleton::text().lines(3).build() // multi-line placeholder
Skeleton::circle().build()        // avatar placeholder
Skeleton::rect().build()          // card/image placeholder
```

**CSS**: `.rw-skeleton` with shimmer animation (`@keyframes`), shape variants
**Protocol**: No new element types. Uses `div` with CSS classes.

---

### AppShell (~500B CSS)

**Purpose**: Full-page layout with header, optional sidebar, and main content area. Every page of the docs site uses this structure.

**API sketch:**
```rust
AppShell::new()
    .header(header_content)
    .sidebar(sidebar_content)     // optional
    .main(main_content)
    .sidebar_width(280)           // px, default 260
    .header_height(60)            // px, default 56
    .build()
```

**CSS**: `.rw-shell`, `.rw-shell-header`, `.rw-shell-sidebar`, `.rw-shell-main`
**Layout**: CSS Grid with `grid-template-areas` for responsive layout. Sidebar collapses on mobile.
**Protocol**: Needs `El::Main` and `El::Aside` element types for semantic HTML.

---

### DocsSidebar (~350B CSS)

**Purpose**: Navigation sidebar with collapsible sections and active state. Specific to documentation sites.

**API sketch:**
```rust
DocsSidebar::new()
    .section(SidebarSection::new("Getting Started")
        .link("/docs/install", "Installation")
        .link("/docs/quickstart", "Quick Start")
        .link("/docs/concepts", "Core Concepts"))
    .section(SidebarSection::new("Components")
        .link("/docs/components/button", "Button")
        .link("/docs/components/input", "Input"))
    .active_path("/docs/quickstart")
    .build()
```

**CSS**: `.rw-sidebar`, `.rw-sidebar-section`, `.rw-sidebar-link`, `.rw-sidebar-link-active`
**Dependency**: Builds on Accordion for collapsible sections and Link for navigation items.

---

### TableOfContents (~250B CSS)

**Purpose**: In-page heading navigation. Shows headings extracted from current page content with scroll-spy active state.

**API sketch:**
```rust
TableOfContents::new()
    .heading(1, "Introduction", "#introduction")
    .heading(2, "Installation", "#installation")
    .heading(2, "Usage", "#usage")
    .heading(3, "Basic Example", "#basic-example")
    .build()
```

**CSS**: `.rw-toc`, `.rw-toc-link`, `.rw-toc-h2` (indented), `.rw-toc-h3` (more indented), `.rw-toc-active`
**Protocol**: Needs `Ev::Scroll` event type for future scroll-spy. Initial version uses click-to-scroll without active tracking.

---

### Tier 1 Total

| Component | CSS Budget |
|-----------|-----------|
| Accordion | ~400B |
| Code/CodeBlock | ~350B |
| Blockquote | ~150B |
| Prose | ~800B |
| Skeleton | ~200B |
| AppShell | ~500B |
| DocsSidebar | ~350B |
| TableOfContents | ~250B |
| **Total** | **~3,000B** |

**Running total**: 12,500B (existing) + 3,000B = **~15,500B**

## Tier 2 — Component Parity

These bring rwire to feature parity with shadcn/ui for general-purpose applications.

### Tooltip (~350B CSS)

**Purpose**: Hover/focus information popover. Useful for icon buttons, truncated text, and help hints.

**API sketch:**
```rust
Tooltip::new("Delete this item")
    .child(Button::new().icon(Icon::Trash).build())
    .position(TooltipPosition::Top)
    .build()
```

**Interaction**: CSS-only using `:hover`/`:focus-visible` + absolute positioning. No JS needed.
**CSS**: `.rw-tooltip`, `.rw-tooltip-top`, `.rw-tooltip-bottom`, etc.

---

### Drawer (~500B CSS)

**Purpose**: Slide-in panel from edge of screen. Used for mobile navigation and detail panels.

**API sketch:**
```rust
Drawer::new()
    .title("Navigation")
    .content(sidebar_content)
    .position(DrawerPosition::Left)
    .open(is_open)
    .on_close(close_handler)
    .build()
```

**CSS**: `.rw-drawer`, `.rw-drawer-left`, `.rw-drawer-right`, `.rw-drawer-backdrop`, `.rw-drawer-open`
**Note**: Shares overlay patterns with Modal (backdrop, z-index, focus trap).

---

### DropdownMenu (~400B CSS)

**Purpose**: Action menu triggered by a button. Common for "more actions" patterns.

**API sketch:**
```rust
DropdownMenu::new()
    .trigger(Button::secondary("Actions").build())
    .item("Edit", edit_handler)
    .item("Delete", delete_handler)
    .divider()
    .item("Export", export_handler)
    .build()
```

**CSS**: `.rw-dropdown`, `.rw-dropdown-item`, `.rw-dropdown-divider`, `.rw-dropdown-open`
**Interaction**: Toggle open/close via local handler. Positioning below trigger via CSS.

---

### Toast (~400B CSS)

**Purpose**: Transient notification messages. Appear briefly, auto-dismiss.

**API sketch:**
```rust
// Server-side: push toast message to client
ctx.toast(Toast::success("Changes saved"));
ctx.toast(Toast::error("Failed to delete item").duration_ms(5000));
```

**CSS**: `.rw-toast`, `.rw-toast-container`, `.rw-toast-success`, `.rw-toast-error`, `@keyframes rw-toast-in`, `@keyframes rw-toast-out`
**Note**: Natural fit for rwire — server pushes a temporary element, client auto-removes after timeout via local timer.

---

### Tier 2 Total

| Component | CSS Budget |
|-----------|-----------|
| Tooltip | ~350B |
| Drawer | ~500B |
| DropdownMenu | ~400B |
| Toast | ~400B |
| **Total** | **~1,650B** |

**Running total**: 15,500B + 1,650B = **~17,150B**

## Tier 3 — Nice-to-Have

Lower priority components for future consideration.

| Component | CSS Est. | Use Case |
|-----------|---------|----------|
| Image | ~150B | Image with loading/error states |
| Kbd | ~100B | Keyboard shortcut display (`⌘K`) |
| Tag/Chip | ~250B | Removable labels/filters |
| EmptyState | ~200B | Placeholder for empty lists |
| Timeline | ~300B | Vertical event timeline |
| Stat | ~200B | Metric display (value + label + trend) |
| NavigationMenu | ~400B | Top-level nav bar with dropdowns |
| Slider | ~400B | Range input control |
| Stepper | ~350B | Multi-step progress indicator |
| AvatarGroup | ~150B | Stacked avatar display |
| **Total** | **~2,500B** | |

**Grand total (all tiers)**: 12,500 + 3,000 + 1,650 + 2,500 = **~19,650B (~19.2KB)**

This keeps the full library under the **20KB CSS budget** target.

## Protocol Extensions Summary

### New Element Types Needed

| Element | Hex | Tier | Used By |
|---------|-----|------|---------|
| `pre` | 0x1D | 1 | Code/CodeBlock |
| `code` | 0x1E | 1 | Code/CodeBlock, Prose, Kbd |
| `blockquote` | 0x1F | 1 | Blockquote, Prose |
| `strong` | 0x20 | 1 | Prose (bold text in markdown) |
| `em` | 0x21 | 1 | Prose (italic text in markdown) |
| `img` | 0x22 | 1 | Prose (images in markdown) |
| `table` | 0x23 | 1 | Prose (tables in markdown) |
| `thead` | 0x24 | 1 | Prose |
| `tbody` | 0x25 | 1 | Prose |
| `tr` | 0x26 | 1 | Prose |
| `th` | 0x27 | 1 | Prose |
| `td` | 0x28 | 1 | Prose |
| `aside` | 0x29 | 1 | AppShell sidebar |
| `main` | 0x2A | 1 | AppShell main content |

**Total: 14 new element types** (all Tier 1 requirements)

### New Event Types Needed

| Event | Hex | Tier | Used By |
|-------|-----|------|---------|
| `scroll` | 0x0D | 1 | TableOfContents scroll-spy |

**Total: 1 new event type**

## Dependency Graph

```
Tier 1:
  Accordion  ←─ DocsSidebar (uses Accordion for collapsible sections)
  Code       ←─ Prose (styles code blocks)
  Blockquote ←─ Prose (styles blockquotes)
  Prose      ←─ Markdown pipeline (wraps rendered markdown)
  AppShell   ←─ Docs site layout
  Skeleton   ←─ Docs page loading state

Tier 2:
  Modal (existing) ←─ Drawer (shares overlay patterns)
  DropdownMenu     ─  independent
  Tooltip          ─  independent
  Toast            ─  independent (new server push mechanism)
```

Build order for Tier 1:
1. Protocol extensions (new element types + event)
2. Accordion, Code, Blockquote (no dependencies)
3. Prose (depends on Code, Blockquote CSS classes)
4. Skeleton (independent)
5. AppShell (independent, uses new `main`/`aside` elements)
6. DocsSidebar (depends on Accordion)
7. TableOfContents (independent, scroll event is optional)
