//! Link component.
//!
//! Anchor element with styling for internal and external links.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::Link;
//!
//! Link::new("/about").text("About Us").build()
//! Link::external("https://example.com").text("Example").build()
//! ```

use crate::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Link component builder.
#[derive(Clone, Debug, Default)]
pub struct Link {
    href: Cow<'static, str>,
    external: bool,
    content: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
}

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

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(32);
        classes.push_str("rw-link");

        if self.external {
            classes.push_str(" rw-link-external");
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the link into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        super::registry::mark_component_used(super::registry::ComponentType::Link);

        let class = self.compute_class();
        let mut builder = el(El::A).class(&class).attr("href", &self.href);

        if self.external {
            builder = builder
                .attr("target", "_blank")
                .attr("rel", "noopener noreferrer");
        }

        if let Some(content) = self.content {
            builder = builder.text(&content);
        }

        builder
    }
}

/// Link CSS.
pub const LINK_CSS: &str = "\
.rw-link{color:var(--rw-accent-11);text-decoration:none;cursor:pointer}\
.rw-link:hover{text-decoration:underline}\
.rw-link:focus-visible{outline:2px solid var(--rw-accent-8);outline-offset:2px}\
.rw-link-external::after{content:\" \\2197\";font-size:0.8em}\n";

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
    fn test_link_class_default() {
        let link = Link::new("/about");
        assert_eq!(link.compute_class(), "rw-link");
    }

    #[test]
    fn test_link_class_external() {
        let link = Link::external("https://example.com");
        assert_eq!(link.compute_class(), "rw-link rw-link-external");
    }

    #[test]
    fn test_link_css_size() {
        assert!(LINK_CSS.len() < 300, "Link CSS too large: {} bytes", LINK_CSS.len());
        println!("Link CSS size: {} bytes", LINK_CSS.len());
    }
}
