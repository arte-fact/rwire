//! Blockquote component.
//!
//! Styled blockquote for quotes and callouts.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::Blockquote;
//!
//! Blockquote::new("rwire is a server-rendered framework.").build()
//!
//! Blockquote::new("Note: This is experimental.")
//!     .cite("https://rwire.dev/docs")
//!     .build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Blockquote builder.
#[derive(Clone, Default)]
pub struct Blockquote {
    content: Cow<'static, str>,
    cite: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
    children: Vec<ElementBuilder>,
}

impl Blockquote {
    /// Create a new blockquote with text content.
    pub fn new(content: impl Into<Cow<'static, str>>) -> Self {
        Self {
            content: content.into(),
            ..Self::default()
        }
    }

    /// Create an empty blockquote for custom child elements.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Set the citation URL.
    pub fn cite(mut self, url: impl Into<Cow<'static, str>>) -> Self {
        self.cite = Some(url.into());
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Add a child element.
    pub fn child(mut self, child: ElementBuilder) -> Self {
        self.children.push(child);
        self
    }

    /// Compute style tokens for the blockquote.
    pub fn compute_tokens() -> Vec<St> {
        vec![
            St::BorderL3Accent,
            St::PlLg,
            St::Italic,
            St::TextMuted,
            St::MyMd,
        ]
    }

    /// Build the blockquote into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut bq = el(El::Blockquote).st(Self::compute_tokens());

        if let Some(ref extra) = self.extra_class {
            bq = bq.class(extra.as_ref());
        }

        if let Some(ref cite_url) = self.cite {
            bq = bq.attr("cite", cite_url.as_ref());
        }

        if !self.content.is_empty() {
            bq = bq.append([
                el(El::P).st([St::M0]).text(&self.content)
            ]);
        }

        for child in self.children {
            bq = bq.append([child]);
        }

        bq
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blockquote_new() {
        let bq = Blockquote::new("Hello world");
        assert_eq!(bq.content.as_ref(), "Hello world");
        assert!(bq.cite.is_none());
    }

    #[test]
    fn test_blockquote_with_cite() {
        let bq = Blockquote::new("Quote").cite("https://example.com");
        assert_eq!(bq.cite.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn test_blockquote_tokens() {
        let tokens = Blockquote::compute_tokens();
        assert!(tokens.contains(&St::BorderL3Accent));
        assert!(tokens.contains(&St::PlLg));
        assert!(tokens.contains(&St::Italic));
        assert!(tokens.contains(&St::TextMuted));
        assert!(tokens.contains(&St::MyMd));
    }
}
