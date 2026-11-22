//! Simple Markdown component for embedding markdown in any page.
//!
//! # Example
//!
//! ```ignore
//! use rwire_markdown::Markdown;
//!
//! Markdown::new("# Hello\n\nSome **bold** text.").build()
//! ```

use rwire::{ElementBuilder, ProseFace};
use std::borrow::Cow;

use crate::parser::{parse_markdown_with_options, ParseOptions};
use crate::prose::ProseSize;

/// A simple component for embedding markdown content.
pub struct Markdown {
    content: Cow<'static, str>,
    size: Option<ProseSize>,
    full_width: bool,
    remote_images: bool,
    face: Option<ProseFace>,
}

impl Markdown {
    /// Create a new markdown component from content.
    pub fn new(content: impl Into<Cow<'static, str>>) -> Self {
        Self {
            content: content.into(),
            size: None,
            full_width: false,
            remote_images: false,
            face: None,
        }
    }

    /// Set the prose text size.
    pub fn size(mut self, size: ProseSize) -> Self {
        self.size = Some(size);
        self
    }

    /// Set the face the markdown is set in. `ProseFace::Read`: the theme's
    /// reading face (`--Qfr`, 15.5 px / 15 px on a phone, line height 1.6) with
    /// the reading styles for headings, lists, quotes, tables and code blocks
    /// (mono 12.5 px, radius 10, scrolling inside its frame) — a chat answer
    /// needs nothing else. `Ui` (the default): the interface face.
    pub fn face(mut self, face: ProseFace) -> Self {
        self.face = Some(face);
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
    /// `size` (default `Base`), `full_width`, `remote_images` and `face` (default `Ui`).
    pub fn build(self) -> ElementBuilder {
        let opts = ParseOptions {
            size: self.size.unwrap_or(ProseSize::Base),
            full_width: self.full_width,
            remote_images: self.remote_images,
            face: self.face.unwrap_or(ProseFace::Ui),
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

    #[test]
    fn the_read_face_wraps_in_the_reading_face_without_a_width_cap() {
        use rwire::style_tokens::St;
        let md = Markdown::new("# Title\n\nSome **bold** text.\n\n```rust\nlet x = 1;\n```")
            .face(ProseFace::Read)
            .build();
        let utils = md.get_style_utils();
        assert!(utils.contains(&St::FontRead.as_u16()));
        assert!(utils.contains(&St::TextProsePhone.as_u16()));
        assert!(
            !utils.contains(&St::MaxWProse.as_u16()),
            "the column is the cap"
        );
        let sm: Vec<u16> = md
            .get_breakpoint_groups()
            .iter()
            .flat_map(|(_, c)| c.iter().copied())
            .collect();
        assert!(sm.contains(&St::TextProse.as_u16()));
        let html = md.to_static_html();
        assert!(html.contains("data-testid=\"markdown\""), "{html}");
        assert!(html.contains("data-testid=\"markdown-code\""), "{html}");
    }

    #[test]
    fn the_read_face_keeps_the_unsafe_link_and_remote_image_neutralisation() {
        let md =
            "[x](javascript:alert(1)) ![a cat](https://x.example/y.png) [ok](https://example.com)";
        let html = Markdown::new(md)
            .face(ProseFace::Read)
            .build()
            .to_static_html();
        assert!(!html.contains("href=\"javascript:"), "{html}");
        assert!(!html.contains("<img"), "{html}");
        assert!(html.contains("href=\"https://example.com\""), "{html}");
    }
}
