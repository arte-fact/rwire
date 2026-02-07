//! TableOfContents component.
//!
//! In-page heading navigation with indentation by heading level.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::TableOfContents;
//!
//! TableOfContents::new()
//!     .heading(1, "Introduction", "#introduction")
//!     .heading(2, "Installation", "#installation")
//!     .heading(2, "Usage", "#usage")
//!     .heading(3, "Basic Example", "#basic-example")
//!     .build()
//! ```

use crate::attr_tokens::At;
use crate::style_tokens::St;
use crate::{el, El, ElementBuilder};
use std::borrow::Cow;

/// A heading entry for the table of contents.
#[derive(Clone, Debug)]
pub struct TocHeading {
    level: u8,
    text: Cow<'static, str>,
    anchor: Cow<'static, str>,
}

/// TableOfContents builder.
#[derive(Clone, Default)]
pub struct TableOfContents {
    headings: Vec<TocHeading>,
    active_anchor: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
}

impl TableOfContents {
    /// Create a new table of contents.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a heading entry.
    pub fn heading(
        mut self,
        level: u8,
        text: impl Into<Cow<'static, str>>,
        anchor: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.headings.push(TocHeading {
            level,
            text: text.into(),
            anchor: anchor.into(),
        });
        self
    }

    /// Set the currently active anchor (for scroll-spy highlighting).
    pub fn active_anchor(mut self, anchor: impl Into<Cow<'static, str>>) -> Self {
        self.active_anchor = Some(anchor.into());
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the TOC container.
    pub fn compute_tokens() -> Vec<St> {
        vec![St::DisplayFlex, St::FlexCol, St::TextSm, St::PositionSticky, St::Top0]
    }

    /// Compute style tokens for a TOC link.
    pub fn compute_link_tokens() -> Vec<St> {
        vec![
            St::DisplayBlock,
            St::PySm,
            St::TextMuted,
            St::NoDecoration,
            St::TransitionColors,
            St::BorderL,
            St::BorderTransparent,
        ]
    }

    /// Compute style tokens for an active TOC link.
    pub fn compute_active_link_tokens() -> Vec<St> {
        vec![
            St::DisplayBlock,
            St::PySm,
            St::TextAccent12,
            St::NoDecoration,
            St::FontMedium,
            St::BorderL,
            St::BorderColorAccent,
        ]
    }

    /// Build the table of contents into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut nav = el(El::Nav).st(Self::compute_tokens());

        if let Some(ref extra) = self.extra_class {
            nav = nav.class(extra.as_ref());
        }

        let title = el(El::Div)
            .st([St::FontMedium, St::TextDefault, St::PySm, St::MbXs])
            .text("On this page");

        nav = nav.append([title]);

        for heading in &self.headings {
            let is_active = self
                .active_anchor
                .as_deref()
                .is_some_and(|a| a == heading.anchor.as_ref());

            let tokens = if is_active {
                Self::compute_active_link_tokens()
            } else {
                Self::compute_link_tokens()
            };

            let mut link = el(El::A)
                .st(tokens)
                .at_str(At::Href, &heading.anchor)
                .text(&heading.text);

            if !is_active {
                link = link.hover([St::TextDefault]);
            }

            // Indent based on heading level
            link = match heading.level {
                1 => link.st([St::PlSm]),
                2 => link.st([St::PlMd]),
                _ => link.st([St::PlMdIndent]),
            };

            nav = nav.append([link]);
        }

        nav
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toc_new() {
        let toc = TableOfContents::new();
        assert!(toc.headings.is_empty());
        assert!(toc.active_anchor.is_none());
    }

    #[test]
    fn test_toc_with_headings() {
        let toc = TableOfContents::new()
            .heading(1, "Intro", "#intro")
            .heading(2, "Install", "#install")
            .heading(3, "Example", "#example");
        assert_eq!(toc.headings.len(), 3);
        assert_eq!(toc.headings[0].level, 1);
        assert_eq!(toc.headings[1].text.as_ref(), "Install");
    }

    #[test]
    fn test_toc_container_tokens() {
        let tokens = TableOfContents::compute_tokens();
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::FlexCol));
        assert!(tokens.contains(&St::TextSm));
        assert!(tokens.contains(&St::PositionSticky));
    }

    #[test]
    fn test_toc_link_tokens() {
        let tokens = TableOfContents::compute_link_tokens();
        assert!(tokens.contains(&St::TextMuted));
        assert!(tokens.contains(&St::NoDecoration));
        assert!(tokens.contains(&St::BorderL));
    }

    #[test]
    fn test_toc_active_link_tokens() {
        let tokens = TableOfContents::compute_active_link_tokens();
        assert!(tokens.contains(&St::TextAccent12));
        assert!(tokens.contains(&St::FontMedium));
        assert!(tokens.contains(&St::BorderColorAccent));
    }

    #[test]
    fn test_toc_with_active_anchor() {
        let toc = TableOfContents::new()
            .heading(1, "Intro", "#intro")
            .active_anchor("#intro");
        assert_eq!(toc.active_anchor.as_deref(), Some("#intro"));
    }
}
