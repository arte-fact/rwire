# 06 — Markdown Pipeline

Architecture for parsing `.md` folders into rwire pages.

## Overview

The docs site needs to render markdown files as rwire pages. This requires:

1. **Parsing**: CommonMark markdown → structured data
2. **Frontmatter**: YAML metadata (title, description, order)
3. **Assembly**: Structured data → ElementBuilder tree
4. **Routing**: URL paths → markdown files
5. **Search**: Full-text index across all docs

## Architecture

```
docs/
  getting-started/
    _meta.yml          ← section ordering
    install.md         ← individual pages
    quickstart.md
  components/
    _meta.yml
    button.md
    input.md

    ┌──────────────┐
    │  .md files   │
    └──────┬───────┘
           │ startup: read all .md files
           ▼
    ┌──────────────┐     ┌──────────────┐
    │  Frontmatter │     │  pulldown-   │
    │  Parser      │     │  cmark       │
    └──────┬───────┘     └──────┬───────┘
           │                    │
           ▼                    ▼
    ┌──────────────┐     ┌──────────────┐
    │  DocPage     │     │  Event       │
    │  metadata    │     │  Stream      │
    └──────┬───────┘     └──────┬───────┘
           │                    │
           └────────┬───────────┘
                    ▼
             ┌─────────────┐
             │  Stack-based │
             │  Builder     │
             │  Assembly    │
             └──────┬──────┘
                    ▼
             ┌─────────────┐
             │ Prose::new() │
             │ .child(tree) │
             └──────┬──────┘
                    ▼
             ┌─────────────┐
             │ AppShell     │
             │ + Sidebar    │
             │ + TOC        │
             └─────────────┘
```

## Dependency

**pulldown-cmark** — The standard Rust CommonMark parser.

```toml
[dependencies]
pulldown-cmark = "0.12"
```

Why pulldown-cmark:
- CommonMark-compliant (consistent rendering)
- Zero-copy parsing (fast, low allocation)
- Event-stream API (natural fit for stack-based ElementBuilder assembly)
- Mature (7+ years, widely used in Rust ecosystem)
- Supports: tables, footnotes, strikethrough, task lists
- ~150KB added to binary (acceptable for docs feature)

## Module Structure

```
rwire/src/docs/
  mod.rs              ← Public API: render_markdown(), DocSite, DocPage
  parser.rs           ← pulldown-cmark → ElementBuilder conversion
  frontmatter.rs      ← YAML frontmatter extraction
  search.rs           ← Full-text search index
```

## Frontmatter Format

Each markdown file starts with YAML frontmatter:

```markdown
---
title: Installation
description: How to install rwire
order: 1
---

# Installation

Install rwire by adding it to your Cargo.toml...
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `title` | String | Yes | Page title (used in sidebar, browser tab) |
| `description` | String | No | Meta description for search |
| `order` | u32 | No | Sort order within section (default: alphabetical) |

### Section Meta Files

`_meta.yml` controls section-level configuration:

```yaml
title: Getting Started
order: 1
collapsed: false
```

## Frontmatter Parser

```rust
// rwire/src/docs/frontmatter.rs

pub struct Frontmatter {
    pub title: String,
    pub description: Option<String>,
    pub order: Option<u32>,
}

/// Extract frontmatter from markdown content.
/// Returns (frontmatter, remaining_content).
pub fn parse_frontmatter(content: &str) -> (Option<Frontmatter>, &str) {
    // Look for opening "---" at start of file
    // Find closing "---"
    // Parse YAML between them
    // Return remaining content after closing "---"
}
```

**Implementation notes:**
- No YAML dependency needed — frontmatter fields are simple enough for hand-parsing (`key: value` lines)
- Alternatively, use `serde_yaml` if already in the dependency tree
- Empty or missing frontmatter returns `None` — page title falls back to first `# heading`

## Markdown → ElementBuilder Parser

The core transformation: pulldown-cmark events → ElementBuilder tree.

### pulldown-cmark Event Stream

pulldown-cmark emits events like:

```rust
Start(Heading { level: H2 })
Text("Installation")
End(Heading { level: H2 })
Start(Paragraph)
Text("Run ")
Start(CodeInline)
Text("cargo add rwire")
End(CodeInline)
Text(" to install.")
End(Paragraph)
Start(CodeBlock(Fenced("rust")))
Text("fn main() {}\n")
End(CodeBlock(Fenced("rust")))
```

### Stack-Based Assembly

```rust
// rwire/src/docs/parser.rs

use pulldown_cmark::{Parser, Event, Tag};
use crate::{el, El, ElementBuilder};

pub fn render_markdown(markdown: &str) -> ElementBuilder {
    let parser = Parser::new_ext(markdown, pulldown_cmark::Options::all());
    let mut stack: Vec<ElementBuilder> = vec![el(El::Div)];
    let mut headings: Vec<TocEntry> = vec![];

    for event in parser {
        match event {
            Event::Start(tag) => {
                let element = match tag {
                    Tag::Heading { level, .. } => {
                        match level {
                            HeadingLevel::H1 => el(El::H1),
                            HeadingLevel::H2 => el(El::H2),
                            HeadingLevel::H3 => el(El::H3),
                            _ => el(El::P),
                        }
                    }
                    Tag::Paragraph => el(El::P),
                    Tag::BlockQuote(_) => el(El::Blockquote),
                    Tag::CodeBlock(kind) => {
                        let pre = el(El::Pre);
                        // language extracted from kind for class
                        pre
                    }
                    Tag::List(Some(_)) => el(El::Ol),
                    Tag::List(None) => el(El::Ul),
                    Tag::Item => el(El::Li),
                    Tag::Table(_) => el(El::Table),
                    Tag::TableHead => el(El::Thead),
                    Tag::TableRow => el(El::Tr),
                    Tag::TableCell => el(El::Td), // or Th if in thead
                    Tag::Emphasis => el(El::Em),
                    Tag::Strong => el(El::Strong),
                    Tag::Link { dest_url, .. } => {
                        el(El::A).attr("href", &dest_url)
                    }
                    Tag::Image { dest_url, .. } => {
                        el(El::Img).attr("src", &dest_url)
                    }
                    _ => el(El::Span),
                };
                stack.push(element);
            }
            Event::End(_) => {
                if let Some(completed) = stack.pop() {
                    if let Some(parent) = stack.last_mut() {
                        *parent = std::mem::take(parent).append([completed]);
                    }
                }
            }
            Event::Text(text) => {
                if let Some(current) = stack.last_mut() {
                    *current = std::mem::take(current).text(&text);
                }
            }
            Event::Code(code) => {
                let code_el = el(El::Code).text(&code);
                if let Some(current) = stack.last_mut() {
                    *current = std::mem::take(current).append([code_el]);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                // Append space or <br>
            }
            Event::Rule => {
                let hr = el(El::Hr);
                if let Some(current) = stack.last_mut() {
                    *current = std::mem::take(current).append([hr]);
                }
            }
            _ => {}
        }
    }

    // Return root element
    stack.into_iter().next().unwrap_or_else(|| el(El::Div))
}
```

### Heading Extraction for TOC

During parsing, collect headings for TableOfContents:

```rust
pub struct TocEntry {
    pub level: u8,        // 1, 2, or 3
    pub text: String,     // "Installation"
    pub anchor: String,   // "installation"
}

pub struct ParseResult {
    pub content: ElementBuilder,
    pub headings: Vec<TocEntry>,
}

pub fn parse_markdown(markdown: &str) -> ParseResult {
    // Same as render_markdown but also collects headings
    // Heading anchors generated by slugifying text:
    //   "Quick Start" → "quick-start"
    //   "API Reference" → "api-reference"
}
```

### Anchor Generation

Heading IDs enable linking from TableOfContents:

```rust
fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
        // collapse repeated dashes
}
```

The parser adds `id` attributes to heading elements:
```rust
el(El::H2).attr("id", &anchor).text(&heading_text)
```

## Routing

Use rwire's existing Router for docs pages.

```rust
use rwire::Router;

let mut router = Router::new();

// Static routes
router.route("/", landing_page);

// Doc routes — pattern matching
router.route("/docs/:section/:page", doc_page_handler);
router.route("/docs/:page", doc_page_handler);

// Component showcase
router.route("/components/:name", component_page_handler);
```

### Route → File Resolution

```rust
fn resolve_doc_path(section: &str, page: &str) -> PathBuf {
    // /docs/getting-started/install → docs/getting-started/install.md
    // /docs/architecture → docs/architecture.md
    PathBuf::from("docs").join(section).join(format!("{page}.md"))
}
```

### Navigation

Client-side navigation uses rwire's existing router:
- Sidebar links trigger route changes
- Server re-renders the page with new markdown content
- Only the main content area updates (synced region)

## Search

Server-side full-text search. Natural fit for rwire's server-owns-state model.

### Index Structure

```rust
// rwire/src/docs/search.rs

pub struct SearchIndex {
    entries: Vec<SearchEntry>,
}

pub struct SearchEntry {
    pub path: String,           // "/docs/getting-started/install"
    pub title: String,          // "Installation"
    pub description: String,    // "How to install rwire"
    pub content: String,        // Full text content (stripped of markdown)
    pub section: String,        // "Getting Started"
    pub headings: Vec<String>,  // ["Prerequisites", "Cargo Install", ...]
}

pub struct SearchResult {
    pub path: String,
    pub title: String,
    pub snippet: String,       // Context around match
    pub section: String,
}

impl SearchIndex {
    /// Build index from all doc pages at startup
    pub fn build(pages: &[DocPage]) -> Self { ... }

    /// Search with simple substring matching
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        // Case-insensitive substring search across title, description, content
        // Score: title match > heading match > content match
        // Return surrounding context as snippet
    }
}
```

### Search UX

```
User types in search input
  → debounced input event (BIND_DEBOUNCED, 300ms)
  → server searches index
  → server renders results as Stack of Cards
  → client updates search results region
```

This is server-side search with no client JS — the search index lives in memory on the server, queries go over WebSocket, results are rendered as DOM opcodes.

### Why Not Client-Side Search?

| Approach | Pros | Cons |
|----------|------|------|
| Server-side (chosen) | Zero JS, natural rwire pattern, no index download | Server round-trip per query |
| Client-side (lunr.js) | Instant results | Requires JS index download (~50-200KB), breaks rwire philosophy |

The debounced input event means search feels responsive despite the round-trip. For a documentation site with <1000 pages, server-side substring matching is more than fast enough.

## DocSite Public API

```rust
// rwire/src/docs/mod.rs

pub struct DocSite {
    pages: Vec<DocPage>,
    sidebar: DocsSidebar,
    search_index: SearchIndex,
}

pub struct DocPage {
    pub path: String,
    pub title: String,
    pub description: Option<String>,
    pub section: String,
    pub content: String,        // raw markdown
    pub order: u32,
}

impl DocSite {
    /// Load all .md files from a directory tree
    pub fn load(docs_dir: &Path) -> Self { ... }

    /// Render a page by path
    pub fn render_page(&self, path: &str) -> ElementBuilder {
        let page = self.find_page(path);
        let result = parse_markdown(&page.content);

        AppShell::new()
            .header(self.render_header())
            .sidebar(self.sidebar.active_path(path).build())
            .main(
                Stack::row().children([
                    Prose::new().child(result.content).build(),
                    TableOfContents::from_headings(&result.headings).build(),
                ]).build()
            )
            .build()
    }

    /// Search docs
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        self.search_index.search(query, 20)
    }
}
```

## File Processing Pipeline

At server startup:

1. Walk `docs/` directory recursively
2. For each `.md` file:
   a. Read file contents
   b. Parse frontmatter → title, description, order
   c. Strip markdown → plain text for search index
3. For each `_meta.yml`:
   a. Parse section title and order
4. Build sidebar from section/page tree
5. Build search index from all pages
6. Register routes for each page

This is a one-time cost at startup. For development, file watching could trigger re-parsing, but that's a future enhancement.

## Performance Considerations

| Metric | Estimate |
|--------|----------|
| Parse time per page | ~0.1ms (pulldown-cmark is fast) |
| 100 pages startup parse | ~10ms |
| Search query (100 pages) | <1ms (substring match) |
| Memory per page (search) | ~2-5KB (text content) |
| 100 pages search index | ~200-500KB |
| WebSocket round-trip | ~5-20ms (local), ~50-100ms (remote) |

All well within acceptable bounds for a documentation site.
