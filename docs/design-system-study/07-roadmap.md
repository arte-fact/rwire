# 07 — Roadmap

Phased implementation plan with CSS budgets, dependencies, and verification steps.

## Phase Overview

| Phase | Scope | New CSS | Running Total | Est. Effort |
|-------|-------|---------|---------------|-------------|
| **0** | Protocol extensions | 0B | 12,500B | Small |
| **1** | Foundation components | ~1,900B | 14,400B | Medium |
| **2** | Layout & navigation | ~1,100B | 15,500B | Medium |
| **3** | Markdown pipeline | ~0B (uses Prose) | 15,500B | Large |
| **4** | Tier 2 components | ~1,650B | 17,150B | Medium |
| **5** | Docs site assembly | ~0B (uses existing) | 17,150B | Large |

## Phase 0: Protocol Extensions

Add the element types and event type needed by Tier 1 components.

### Changes

**`rwire/src/protocol/opcodes.rs`** — Add 14 element type constants and 1 event type:

```rust
// New element types
pub const EL_PRE: u8 = 0x1D;
pub const EL_CODE: u8 = 0x1E;
pub const EL_BLOCKQUOTE: u8 = 0x1F;
pub const EL_STRONG: u8 = 0x20;
pub const EL_EM: u8 = 0x21;
pub const EL_IMG: u8 = 0x22;
pub const EL_TABLE: u8 = 0x23;
pub const EL_THEAD: u8 = 0x24;
pub const EL_TBODY: u8 = 0x25;
pub const EL_TR: u8 = 0x26;
pub const EL_TH: u8 = 0x27;
pub const EL_TD: u8 = 0x28;
pub const EL_ASIDE: u8 = 0x29;
pub const EL_MAIN: u8 = 0x2A;

// New event type
pub const EV_SCROLL: u8 = 0x0D;
```

**`El` enum** — Add 14 variants.

**`Ev` enum** — Add `Scroll` variant.

**`rwire/src/capsule_gen.rs`** — Add entries to `ELEMENT_MAPPINGS` and `EVENT_MAPPINGS`:

```rust
// ELEMENT_MAPPINGS additions
(0x1D, "pre"), (0x1E, "code"), (0x1F, "blockquote"),
(0x20, "strong"), (0x21, "em"), (0x22, "img"),
(0x23, "table"), (0x24, "thead"), (0x25, "tbody"),
(0x26, "tr"), (0x27, "th"), (0x28, "td"),
(0x29, "aside"), (0x2A, "main"),

// EVENT_MAPPINGS addition
(0x0D, "scroll"),
```

**`rwire/src/builder.rs`** — Add `.id()` convenience method:

```rust
pub fn id(self, id: &str) -> Self {
    self.attr("id", id)
}
```

Add `.element()` method to support polymorphic rendering for Stack, Card, Container:

```rust
// In ElementBuilder or component builders:
pub fn element(mut self, el: El) -> Self {
    self.element_type = el;
    self
}
```

### Verification
```bash
cargo build --workspace    # zero errors
cargo test --workspace     # all pass
cargo clippy --workspace   # zero warnings
```

---

## Phase 1: Foundation Components

Build the independent Tier 1 components that don't depend on each other.

### 1a. Code/CodeBlock (~350B CSS)

**File**: `rwire/src/components/code.rs`

```rust
pub struct Code { /* inline: bool, language: Option<String>, content: String */ }
impl Code {
    pub fn inline(text: &str) -> Self { ... }
    pub fn block(text: &str) -> Self { ... }
    pub fn language(mut self, lang: &str) -> Self { ... }
    pub fn build(self) -> ElementBuilder { ... }
}
```

**Registry**: Add `ComponentType::Code`

### 1b. Blockquote (~150B CSS)

**File**: `rwire/src/components/blockquote.rs`

```rust
pub struct Blockquote { /* text: String, cite: Option<String> */ }
impl Blockquote {
    pub fn new(text: &str) -> Self { ... }
    pub fn cite(mut self, url: &str) -> Self { ... }
    pub fn child(mut self, el: ElementBuilder) -> Self { ... }
    pub fn build(self) -> ElementBuilder { ... }
}
```

**Registry**: Add `ComponentType::Blockquote`

### 1c. Skeleton (~200B CSS)

**File**: `rwire/src/components/skeleton.rs`

```rust
pub struct Skeleton { /* shape: SkeletonShape, lines: u8, height: Option<u32> */ }
pub enum SkeletonShape { Text, Rect, Circle }
impl Skeleton {
    pub fn text() -> Self { ... }
    pub fn rect() -> Self { ... }
    pub fn circle() -> Self { ... }
    pub fn lines(mut self, n: u8) -> Self { ... }
    pub fn height(mut self, px: u32) -> Self { ... }
    pub fn build(self) -> ElementBuilder { ... }
}
```

**Registry**: Add `ComponentType::Skeleton`

### 1d. Accordion (~400B CSS)

**File**: `rwire/src/components/accordion.rs`

```rust
pub struct Accordion { /* items: Vec<AccordionItem>, allow_multiple: bool */ }
pub struct AccordionItem { /* title: String, content: ElementBuilder */ }
impl Accordion {
    pub fn new() -> Self { ... }
    pub fn item(mut self, item: AccordionItem) -> Self { ... }
    pub fn allow_multiple(mut self, val: bool) -> Self { ... }
    pub fn build(self) -> ElementBuilder { ... }
}
impl AccordionItem {
    pub fn new(title: &str) -> Self { ... }
    pub fn content(mut self, el: ElementBuilder) -> Self { ... }
}
```

**Registry**: Add `ComponentType::Accordion`

### 1e. Prose (~800B CSS)

**File**: `rwire/src/components/prose.rs`

```rust
pub struct Prose { /* children: Vec<ElementBuilder> */ }
impl Prose {
    pub fn new() -> Self { ... }
    pub fn child(mut self, el: ElementBuilder) -> Self { ... }
    pub fn build(self) -> ElementBuilder { ... }
}
```

**Registry**: Add `ComponentType::Prose`

### Phase 1 Verification
```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
```

Verify CSS budgets:
- Code: ≤ 350B
- Blockquote: ≤ 150B
- Skeleton: ≤ 200B
- Accordion: ≤ 400B
- Prose: ≤ 800B
- **Phase 1 total: ≤ 1,900B**

---

## Phase 2: Layout & Navigation Components

Build AppShell, DocsSidebar, and TableOfContents. These depend on Phase 0 (new element types) and Phase 1 (Accordion).

### 2a. AppShell (~500B CSS)

**File**: `rwire/src/components/app_shell.rs`

```rust
pub struct AppShell {
    // header: Option<ElementBuilder>
    // sidebar: Option<ElementBuilder>
    // main: Option<ElementBuilder>
    // sidebar_width: u16
    // header_height: u16
}
impl AppShell {
    pub fn new() -> Self { ... }
    pub fn header(mut self, el: ElementBuilder) -> Self { ... }
    pub fn sidebar(mut self, el: ElementBuilder) -> Self { ... }
    pub fn main(mut self, el: ElementBuilder) -> Self { ... }
    pub fn sidebar_width(mut self, px: u16) -> Self { ... }
    pub fn header_height(mut self, px: u16) -> Self { ... }
    pub fn build(self) -> ElementBuilder { ... }
}
```

**Uses**: `El::Main`, `El::Aside`, `El::Header`, `El::Nav`

### 2b. DocsSidebar (~350B CSS)

**File**: `rwire/src/components/docs_sidebar.rs`

```rust
pub struct DocsSidebar { /* sections: Vec<SidebarSection>, active_path: String */ }
pub struct SidebarSection { /* title: String, links: Vec<SidebarLink> */ }
pub struct SidebarLink { /* path: String, label: String */ }

impl DocsSidebar {
    pub fn new() -> Self { ... }
    pub fn section(mut self, section: SidebarSection) -> Self { ... }
    pub fn active_path(mut self, path: &str) -> Self { ... }
    pub fn build(self) -> ElementBuilder { ... }
}
impl SidebarSection {
    pub fn new(title: &str) -> Self { ... }
    pub fn link(mut self, path: &str, label: &str) -> Self { ... }
}
```

**Depends on**: Accordion (for collapsible sections), Link

### 2c. TableOfContents (~250B CSS)

**File**: `rwire/src/components/toc.rs`

```rust
pub struct TableOfContents { /* headings: Vec<TocHeading> */ }
pub struct TocHeading { /* level: u8, text: String, anchor: String */ }

impl TableOfContents {
    pub fn new() -> Self { ... }
    pub fn heading(mut self, level: u8, text: &str, anchor: &str) -> Self { ... }
    pub fn from_entries(entries: &[TocEntry]) -> Self { ... }
    pub fn build(self) -> ElementBuilder { ... }
}
```

### Phase 2 Verification
```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
```

Verify CSS budgets:
- AppShell: ≤ 500B
- DocsSidebar: ≤ 350B
- TableOfContents: ≤ 250B
- **Phase 2 total: ≤ 1,100B**

**Running total after Phase 2: ~15,500B**

---

## Phase 3: Markdown Pipeline

Build the docs module for parsing markdown into ElementBuilder trees.

### 3a. Frontmatter Parser

**File**: `rwire/src/docs/frontmatter.rs`

```rust
pub struct Frontmatter {
    pub title: String,
    pub description: Option<String>,
    pub order: Option<u32>,
}

pub fn parse_frontmatter(content: &str) -> (Option<Frontmatter>, &str) { ... }
```

### 3b. Markdown Parser

**File**: `rwire/src/docs/parser.rs`

**Dependency**: `pulldown-cmark = "0.12"` (added to Cargo.toml behind a feature flag)

```toml
[features]
docs = ["pulldown-cmark"]

[dependencies]
pulldown-cmark = { version = "0.12", optional = true }
```

```rust
pub struct ParseResult {
    pub content: ElementBuilder,
    pub headings: Vec<TocEntry>,
}

pub struct TocEntry {
    pub level: u8,
    pub text: String,
    pub anchor: String,
}

pub fn parse_markdown(markdown: &str) -> ParseResult { ... }
```

### 3c. Search Index

**File**: `rwire/src/docs/search.rs`

```rust
pub struct SearchIndex { /* entries: Vec<SearchEntry> */ }
pub struct SearchResult { /* path, title, snippet, section */ }

impl SearchIndex {
    pub fn build(pages: &[DocPage]) -> Self { ... }
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> { ... }
}
```

### 3d. Module API

**File**: `rwire/src/docs/mod.rs`

```rust
pub struct DocSite { /* pages, sidebar, search_index */ }
pub struct DocPage { /* path, title, description, section, content, order */ }

impl DocSite {
    pub fn load(docs_dir: &Path) -> Self { ... }
    pub fn render_page(&self, path: &str) -> ElementBuilder { ... }
    pub fn search(&self, query: &str) -> Vec<SearchResult> { ... }
    pub fn sidebar(&self) -> &DocsSidebar { ... }
}
```

### Phase 3 Verification
```bash
cargo build --workspace --features docs
cargo test --workspace --features docs
cargo clippy --workspace --features docs
```

Test with sample markdown files:
- Verify headings, paragraphs, code blocks, lists, tables render correctly
- Verify frontmatter extraction
- Verify search returns expected results
- Verify TOC extraction matches page headings

---

## Phase 4: Tier 2 Components

Add overlay and feedback components for broader library coverage.

### 4a. Tooltip (~350B CSS)

**File**: `rwire/src/components/tooltip.rs`

CSS-only tooltip using `:hover`/`:focus-visible`. No JS required.

### 4b. Drawer (~500B CSS)

**File**: `rwire/src/components/drawer.rs`

Slide-in panel. Shares overlay patterns with Modal.

### 4c. DropdownMenu (~400B CSS)

**File**: `rwire/src/components/dropdown.rs`

Action menu with local handler toggle.

### 4d. Toast (~400B CSS)

**File**: `rwire/src/components/toast.rs`

Server-pushed transient notification with auto-dismiss.

### Phase 4 Verification
```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
```

**Running total after Phase 4: ~17,150B**

---

## Phase 5: Docs Site Assembly

Build the actual documentation site using all components.

### Structure

```
examples/docs-site/
  Cargo.toml
  src/
    main.rs             ← Server setup, routing
    pages/
      landing.rs        ← Landing page
      doc_page.rs       ← Documentation page renderer
      component_page.rs ← Component showcase page
      search_page.rs    ← Search results page
  docs/
    getting-started/
      _meta.yml
      install.md
      quickstart.md
      concepts.md
    architecture/
      _meta.yml
      protocol.md
      state.md
      tree-shaking.md
    components/
      _meta.yml
      button.md
      input.md
      stack.md
      ...
    api/
      _meta.yml
      element-builder.md
      events.md
      state.md
```

### Key Pages

1. **Landing page**: Hero section, feature cards, quick start code, CTA buttons
2. **Doc pages**: Full layout with sidebar, markdown content, TOC
3. **Component pages**: Live preview + source code + API table
4. **Search**: Debounced search input, result cards with snippets

### Phase 5 Verification
- Visual inspection of all page types
- Navigation between pages works
- Search returns relevant results
- Theme toggle works (light/dark)
- Responsive layout (sidebar collapses on mobile)
- All component previews render correctly

---

## CSS Budget Summary

| Phase | Components | CSS Added | Running Total |
|-------|-----------|-----------|---------------|
| Existing | 28 components | — | 12,500B |
| Phase 0 | Protocol only | 0B | 12,500B |
| Phase 1 | Code, Blockquote, Skeleton, Accordion, Prose | 1,900B | 14,400B |
| Phase 2 | AppShell, DocsSidebar, TOC | 1,100B | 15,500B |
| Phase 3 | Markdown pipeline (no new CSS) | 0B | 15,500B |
| Phase 4 | Tooltip, Drawer, Dropdown, Toast | 1,650B | 17,150B |
| Phase 5 | Docs site (uses existing) | 0B | 17,150B |
| **Final** | **36 components** | | **~17.2KB** |

**Well under the 20KB target.** Leaves ~2.8KB headroom for Tier 3 components.

## Dependency Additions

| Dependency | Version | Phase | Reason | Size Impact |
|-----------|---------|-------|--------|-------------|
| pulldown-cmark | 0.12 | 3 | Markdown parsing | ~150KB binary |

**Note**: pulldown-cmark is behind a `docs` feature flag, so it doesn't affect binary size for users who don't use the docs module.

## Critical Files Touched

| File | Phase | Changes |
|------|-------|---------|
| `rwire/src/protocol/opcodes.rs` | 0 | 14 element types + 1 event type |
| `rwire/src/capsule_gen.rs` | 0 | ELEMENT_MAPPINGS + EVENT_MAPPINGS entries |
| `rwire/src/builder.rs` | 0 | `.id()` method, `.element()` for polymorphism |
| `rwire/src/components/mod.rs` | 1-4 | Register new components |
| `rwire/src/components/registry.rs` | 1-4 | Add ComponentType variants |
| `rwire/src/components/stack.rs` | 0 | Add `.element()` polymorphic setter |
| `rwire/src/components/card.rs` | 0 | Add `.element()` polymorphic setter |
| `rwire/src/components/container.rs` | 0 | Add `.element()` polymorphic setter |
| `rwire/src/components/code.rs` | 1 | New file |
| `rwire/src/components/blockquote.rs` | 1 | New file |
| `rwire/src/components/skeleton.rs` | 1 | New file |
| `rwire/src/components/accordion.rs` | 1 | New file |
| `rwire/src/components/prose.rs` | 1 | New file |
| `rwire/src/components/app_shell.rs` | 2 | New file |
| `rwire/src/components/docs_sidebar.rs` | 2 | New file |
| `rwire/src/components/toc.rs` | 2 | New file |
| `rwire/src/components/tooltip.rs` | 4 | New file |
| `rwire/src/components/drawer.rs` | 4 | New file |
| `rwire/src/components/dropdown.rs` | 4 | New file |
| `rwire/src/components/toast.rs` | 4 | New file |
| `rwire/src/docs/mod.rs` | 3 | New file |
| `rwire/src/docs/parser.rs` | 3 | New file |
| `rwire/src/docs/frontmatter.rs` | 3 | New file |
| `rwire/src/docs/search.rs` | 3 | New file |
| `rwire/Cargo.toml` | 3 | Add pulldown-cmark dependency |
