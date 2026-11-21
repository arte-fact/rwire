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

use crate::parser::{parse_markdown_with_options, ParseOptions};
use crate::prose::ProseSize;

/// A simple component for embedding markdown content.
pub struct Markdown {
    content: Cow<'static, str>,
    size: Option<ProseSize>,
    full_width: bool,
    remote_images: bool,
}

impl Markdown {
    /// Create a new markdown component from content.
    pub fn new(content: impl Into<Cow<'static, str>>) -> Self {
        Self {
            content: content.into(),
            size: None,
            full_width: false,
            remote_images: false,
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

    /// Load images from another origin (`https://…`, `//…`). Off by default:
    /// a remote image tells its server who reads the page and when, so it is
    /// rendered as its alt text. Turn it on for trusted content only.
    pub fn remote_images(mut self, on: bool) -> Self {
        self.remote_images = on;
        self
    }

    /// Build the markdown into an ElementBuilder tree, honouring the configured prose
    /// `size` (default `Base`), `full_width` and `remote_images`.
    pub fn build(self) -> ElementBuilder {
        let opts = ParseOptions {
            size: self.size.unwrap_or(ProseSize::Base),
            full_width: self.full_width,
            remote_images: self.remote_images,
        };
        parse_markdown_with_options(&self.content, opts).content
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
    fn remote_images_are_opt_in() {
        let md = "![a](https://x.example/y.png)";
        assert!(!Markdown::new(md).build().to_static_html().contains("<img"));
        let on = Markdown::new(md)
            .remote_images(true)
            .build()
            .to_static_html();
        assert!(on.contains("src=\"https://x.example/y.png\""), "{on}");
    }

    #[test]
    fn test_markdown_build() {
        // Should not panic
        let _el = Markdown::new("# Title\n\nSome **bold** text.").build();
    }
}
