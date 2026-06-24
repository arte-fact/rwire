//! Prose component.
//!
//! Typography container that provides sensible spacing and sizing
//! for rich text content (markdown output, documentation, etc.).
//!
//! # Example
//!
//! ```ignore
//! use rwire_markdown::Prose;
//!
//! Prose::new()
//!     .child(el(El::H1).text("Introduction"))
//!     .child(el(El::P).text("Welcome to the documentation."))
//!     .build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Prose size variant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProseSize {
    /// Smaller text (--rw-text-sm base)
    Sm,
    /// Default text (--rw-text-base base)
    #[default]
    Base,
    /// Larger text (--rw-text-lg base)
    Lg,
}

/// Prose builder.
#[derive(Clone, Default)]
pub struct Prose {
    size: ProseSize,
    max_width: bool,
    extra_class: Option<Cow<'static, str>>,
    children: Vec<ElementBuilder>,
}

impl Prose {
    /// Create a new prose container.
    pub fn new() -> Self {
        Self {
            max_width: true,
            ..Self::default()
        }
    }

    /// Set the text size.
    pub fn size(mut self, size: ProseSize) -> Self {
        self.size = size;
        self
    }

    /// Disable max-width constraint (full width).
    pub fn full_width(mut self) -> Self {
        self.max_width = false;
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

    /// Add multiple children.
    pub fn children(mut self, children: impl IntoIterator<Item = ElementBuilder>) -> Self {
        self.children.extend(children);
        self
    }

    /// Compute style tokens for the prose container.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![St::LeadingRelaxedProse, St::TextDefault, St::SpaceYMd];

        match self.size {
            ProseSize::Sm => tokens.push(St::TextSm),
            ProseSize::Base => tokens.push(St::TextBase),
            ProseSize::Lg => tokens.push(St::TextLg),
        }

        if self.max_width {
            tokens.push(St::MaxWProse);
        }

        tokens
    }

    /// Build the prose container into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut builder = el(El::Div).st(self.compute_tokens());

        if let Some(ref extra) = self.extra_class {
            builder = builder.class(extra.as_ref());
        }

        for child in self.children {
            builder = builder.append([child]);
        }

        builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prose_defaults() {
        let prose = Prose::new();
        assert_eq!(prose.size, ProseSize::Base);
        assert!(prose.max_width);
        assert!(prose.children.is_empty());
    }

    #[test]
    fn test_prose_tokens() {
        let prose = Prose::new();
        let tokens = prose.compute_tokens();
        assert!(tokens.contains(&St::LeadingRelaxedProse));
        assert!(tokens.contains(&St::TextDefault));
        assert!(tokens.contains(&St::SpaceYMd));
        assert!(tokens.contains(&St::TextBase));
        assert!(tokens.contains(&St::MaxWProse));
    }

    #[test]
    fn test_prose_sm() {
        let prose = Prose::new().size(ProseSize::Sm);
        let tokens = prose.compute_tokens();
        assert!(tokens.contains(&St::TextSm));
    }

    #[test]
    fn test_prose_lg() {
        let prose = Prose::new().size(ProseSize::Lg);
        let tokens = prose.compute_tokens();
        assert!(tokens.contains(&St::TextLg));
    }

    #[test]
    fn test_prose_full_width() {
        let prose = Prose::new().full_width();
        let tokens = prose.compute_tokens();
        assert!(!tokens.contains(&St::MaxWProse));
    }

    #[test]
    fn test_prose_with_children() {
        let prose = Prose::new()
            .child(el(El::P).text("Hello"))
            .child(el(El::P).text("World"));
        assert_eq!(prose.children.len(), 2);
    }
}
