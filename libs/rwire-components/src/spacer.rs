//! Spacer component.
//!
//! Creates space between elements.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Spacer, SpacingSize};
//!
//! Spacer::md().build()  // Medium vertical space
//! Spacer::lg().build()  // Large vertical space
//! Spacer::new(SpacingSize::Xl).horizontal().build()  // Horizontal space
//! ```

use super::divider::SpacingSize;
use rwire::{el, El, ElementBuilder, St};
use std::borrow::Cow;

/// Spacer component builder.
#[derive(Clone, Debug, Default)]
pub struct Spacer {
    size: SpacingSize,
    horizontal: bool,
    extra_class: Option<Cow<'static, str>>,
}

impl Spacer {
    /// Create a new spacer with the given size.
    pub fn new(size: SpacingSize) -> Self {
        Self {
            size,
            ..Self::default()
        }
    }

    /// Create an extra small spacer.
    pub fn xs() -> Self {
        Self::new(SpacingSize::Xs)
    }

    /// Create a small spacer.
    pub fn sm() -> Self {
        Self::new(SpacingSize::Sm)
    }

    /// Create a medium spacer.
    pub fn md() -> Self {
        Self::new(SpacingSize::Md)
    }

    /// Create a large spacer.
    pub fn lg() -> Self {
        Self::new(SpacingSize::Lg)
    }

    /// Create an extra large spacer.
    pub fn xl() -> Self {
        Self::new(SpacingSize::Xl)
    }

    /// Set the spacer size.
    pub fn size(mut self, size: SpacingSize) -> Self {
        self.size = size;
        self
    }

    /// Make this a horizontal spacer (width instead of height).
    pub fn horizontal(mut self) -> Self {
        self.horizontal = true;
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute the style token for this spacer's dimension.
    fn compute_token(&self) -> St {
        match (self.horizontal, self.size) {
            (false, SpacingSize::None) => St::HSp0,
            (false, SpacingSize::Xs) => St::HSp1,
            (false, SpacingSize::Sm) => St::HSp2,
            (false, SpacingSize::Md) => St::HSp4,
            (false, SpacingSize::Lg) => St::HSp6,
            (false, SpacingSize::Xl) => St::HSp8,
            (true, SpacingSize::None) => St::WSp0,
            (true, SpacingSize::Xs) => St::WSp1,
            (true, SpacingSize::Sm) => St::WSp2,
            (true, SpacingSize::Md) => St::WSp4,
            (true, SpacingSize::Lg) => St::WSp6,
            (true, SpacingSize::Xl) => St::WSp8,
        }
    }

    /// Build the spacer into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut builder = el(El::Div).st([self.compute_token()]);

        if let Some(ref extra) = self.extra_class {
            builder = builder.class(extra.as_ref());
        }

        builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spacer_defaults() {
        let spacer = Spacer::md();
        assert_eq!(spacer.size, SpacingSize::Md);
        assert!(!spacer.horizontal);
    }

    #[test]
    fn test_spacer_token_vertical() {
        assert_eq!(Spacer::xs().compute_token(), St::HSp1);
        assert_eq!(Spacer::sm().compute_token(), St::HSp2);
        assert_eq!(Spacer::md().compute_token(), St::HSp4);
        assert_eq!(Spacer::lg().compute_token(), St::HSp6);
        assert_eq!(Spacer::xl().compute_token(), St::HSp8);
    }

    #[test]
    fn test_spacer_token_horizontal() {
        assert_eq!(Spacer::xs().horizontal().compute_token(), St::WSp1);
        assert_eq!(Spacer::sm().horizontal().compute_token(), St::WSp2);
        assert_eq!(Spacer::md().horizontal().compute_token(), St::WSp4);
        assert_eq!(Spacer::lg().horizontal().compute_token(), St::WSp6);
        assert_eq!(Spacer::xl().horizontal().compute_token(), St::WSp8);
    }

}
