//! Divider component.
//!
//! Horizontal or vertical separator line.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Divider, SpacingSize};
//!
//! Divider::horizontal().build()
//! Divider::vertical().build()
//! Divider::horizontal().margin(SpacingSize::Lg).build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Spacing size for margins.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpacingSize {
    /// No spacing
    None,
    /// Extra small
    Xs,
    /// Small
    Sm,
    /// Medium (default)
    #[default]
    Md,
    /// Large
    Lg,
    /// Extra large
    Xl,
}

/// Divider component builder.
#[derive(Clone, Debug, Default)]
pub struct Divider {
    vertical: bool,
    margin: SpacingSize,
    extra_class: Option<Cow<'static, str>>,
}

#[rwire::component]
impl Divider {
    /// Create a new horizontal divider.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a horizontal divider.
    pub fn horizontal() -> Self {
        Self::default()
    }

    /// Create a vertical divider.
    pub fn vertical() -> Self {
        Self {
            vertical: true,
            ..Self::default()
        }
    }

    /// Set whether the divider is vertical.
    pub fn is_vertical(mut self, vertical: bool) -> Self {
        self.vertical = vertical;
        self
    }

    /// Set the margin around the divider.
    pub fn margin(mut self, margin: SpacingSize) -> Self {
        self.margin = margin;
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for this divider configuration.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![St::BorderNone];

        if self.vertical {
            tokens.push(St::BorderLSubtle);
            // Vertical divider needs height:100% and horizontal margin
            // Using style() for width:1px since it's one-off
        } else {
            tokens.push(St::BorderTSubtle);
        }

        match self.margin {
            SpacingSize::None => tokens.push(St::My0),
            SpacingSize::Xs => tokens.push(St::MyXs),
            SpacingSize::Sm => tokens.push(St::MySm),
            SpacingSize::Md => tokens.push(St::MyMd),
            SpacingSize::Lg => tokens.push(St::MyLg),
            SpacingSize::Xl => tokens.push(St::MyXl),
        }

        tokens
    }

    /// Build the divider into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut tokens = self.compute_tokens();
        if self.vertical {
            tokens.extend([St::HFull, St::MxMd]);
        }
        el(El::Hr).st(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_divider_defaults() {
        let divider = Divider::new();
        assert!(!divider.vertical);
        assert_eq!(divider.margin, SpacingSize::Md);
    }

    #[test]
    fn test_divider_default_tokens() {
        let divider = Divider::new();
        let tokens = divider.compute_tokens();
        assert!(tokens.contains(&St::BorderNone));
        assert!(tokens.contains(&St::BorderTSubtle));
        assert!(tokens.contains(&St::MyMd));
    }

    #[test]
    fn test_divider_vertical_tokens() {
        let divider = Divider::vertical().margin(SpacingSize::Lg);
        let tokens = divider.compute_tokens();
        assert!(tokens.contains(&St::BorderLSubtle));
        assert!(tokens.contains(&St::MyLg));
    }

}
