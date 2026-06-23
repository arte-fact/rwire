//! Markdown rendering and documentation site components for rwire.
//!
//! Provides:
//! - Markdown-to-ElementBuilder parsing
//! - Documentation site structure (pages, sections, search)
//! - Components: Prose, DocsSidebar, TableOfContents
//! - Simple `Markdown` component for embedding markdown in any page
//!
//! # Quick Start
//!
//! ```ignore
//! use rwire_markdown::Markdown;
//!
//! Markdown::new("# Hello\n\nSome **bold** text.").build()
//! ```

mod code;
mod frontmatter;
mod highlight;
mod markdown;
mod parser;
mod prose;
mod search;
mod sidebar;
mod site;
mod toc;

pub use code::highlight_code;
pub use frontmatter::Frontmatter;
pub use markdown::Markdown;
pub use parser::{parse_markdown, parse_markdown_with, ParseResult, TocEntry};
pub use prose::{Prose, ProseSize};
pub use search::{SearchIndex, SearchResult};
pub use sidebar::{DocsSidebar, SidebarSection};
pub use site::{DocPage, DocSite};
pub use toc::TableOfContents;
