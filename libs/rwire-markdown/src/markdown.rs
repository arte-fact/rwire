//! Simple Markdown component for embedding markdown in any page.
//!
//! # Example
//!
//! ```ignore
//! use rwire_markdown::Markdown;
//!
//! Markdown::new("# Hello\n\nSome **bold** text.").build()
//! ```

use rwire::ElementBuilder;
use std::borrow::Cow;

use crate::parser::parse_markdown_with;
use crate::prose::ProseSize;

/// A simple component for embedding markdown content.
pub struct Markdown {
    content: Cow<'static, str>,
    size: Option<ProseSize>,
    full_width: bool,
}

impl Markdown {
    /// Create a new markdown component from content.
    pub fn new(content: impl Into<Cow<'static, str>>) -> Self {
        Self {
            content: content.into(),
            size: None,
            full_width: false,
        }
    }

    /// Set the prose text size.
    pub fn size(mut self, size: ProseSize) -> Self {
        self.size = Some(size);
        self
    }

    /// Disable max-width constraint (full width).
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    /// Build the markdown into an ElementBuilder tree, honouring the configured prose
    /// `size` (default `Base`) and `full_width`.
    pub fn build(self) -> ElementBuilder {
        let size = self.size.unwrap_or(ProseSize::Base);
        parse_markdown_with(&self.content, size, self.full_width).content
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_new() {
        let md = Markdown::new("# Hello");
        assert_eq!(md.content.as_ref(), "# Hello");
        assert!(md.size.is_none());
        assert!(!md.full_width);
    }

    #[test]
    fn test_markdown_with_options() {
        let md = Markdown::new("text").size(ProseSize::Lg).full_width();
        assert_eq!(md.size, Some(ProseSize::Lg));
        assert!(md.full_width);
    }

    #[test]
    fn test_markdown_build() {
        // Should not panic
        let _el = Markdown::new("# Title\n\nSome **bold** text.").build();
    }
}
