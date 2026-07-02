//! Link component.
//!
//! Anchor element with styling for internal and external links.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::Link;
//!
//! Link::new("/about").text("About Us").build()
//! Link::external("https://example.com").text("Example").build()
//! ```

use rwire::attr_tokens::{At, Av};
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Link component builder.
#[derive(Clone, Debug, Default)]
pub struct Link {
    href: Cow<'static, str>,
    external: bool,
    content: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
}

#[rwire::component]
impl Link {
    /// Create a new link with href.
    pub fn new(href: impl Into<Cow<'static, str>>) -> Self {
        Self {
            href: href.into(),
            ..Self::default()
        }
    }

    /// Create an external link (opens in new tab).
    pub fn external(href: impl Into<Cow<'static, str>>) -> Self {
        Self {
            href: href.into(),
            external: true,
            ..Self::default()
        }
    }

    /// Set whether this is an external link.
    pub fn is_external(mut self, external: bool) -> Self {
        self.external = external;
        self
    }

    /// Set the link text content.
    pub fn text(mut self, content: impl Into<Cow<'static, str>>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the link.
    pub fn compute_tokens(&self) -> Vec<St> {
        vec![St::TextAccent, St::NoDecoration, St::CursorPointer]
    }

    /// Build the link into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut builder = el(El::A)
            .st(self.compute_tokens())
            .hover([St::Underline])
            .focus_visible([St::OutlineAccent, St::OutlineOffset2])
            .at_str(At::Href, &self.href);

        if self.external {
            builder = builder
                .at(At::Target, Av::Blank)
                .at(At::Rel, Av::NoopenerNoreferrer);
        }

        if let Some(ref extra) = self.extra_class {
            builder = builder.class(extra.as_ref());
        }

        if let Some(content) = self.content {
            builder = builder.text(&content);
        }

        builder
    }
}

/// Link CSS.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_defaults() {
        let link = Link::new("/about");
        assert_eq!(link.href, "/about");
        assert!(!link.external);
    }

    #[test]
    fn test_link_default_tokens() {
        let link = Link::new("/about");
        let tokens = link.compute_tokens();
        assert!(tokens.contains(&St::TextAccent));
        assert!(tokens.contains(&St::NoDecoration));
        assert!(tokens.contains(&St::CursorPointer));
    }

    #[test]
    fn test_link_pseudo_groups() {
        let link = Link::new("/about").build();
        let groups = link.get_pseudo_groups();
        // Should have hover and focus_visible groups
        assert!(groups.iter().any(|(pc, _)| *pc == 0x00)); // Pc::Hover
        assert!(groups.iter().any(|(pc, _)| *pc == 0x02)); // Pc::FocusVisible
    }

    #[test]
    fn test_link_external_constructor() {
        let link = Link::external("https://example.com");
        assert!(link.external);
    }
}
