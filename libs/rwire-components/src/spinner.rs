//! Spinner component.
//!
//! Loading spinner with CSS animation.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Spinner, SpinnerSize};
//!
//! Spinner::new().build()
//!
//! Spinner::new()
//!     .size(SpinnerSize::Lg)
//!     .label("Loading data...")
//!     .build()
//! ```

use rwire::attr_tokens::{At, Av};
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Spinner size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpinnerSize {
    /// Small: 1rem
    Sm,
    /// Medium: 1.5rem (default)
    #[default]
    Md,
    /// Large: 2rem
    Lg,
}

/// Spinner builder.
#[derive(Clone, Debug, Default)]
pub struct Spinner {
    size: SpinnerSize,
    label: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
}

#[rwire::component]
impl Spinner {
    /// Create a new spinner.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the spinner size.
    pub fn size(mut self, size: SpinnerSize) -> Self {
        self.size = size;
        self
    }

    /// Set aria-label.
    pub fn label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the spinner.
    pub fn compute_tokens(&self) -> Vec<St> {
        vec![St::DisplayInlineBlock, St::RoundedFull, St::BorderDefault]
    }

    /// Compute size-specific style tokens.
    fn size_tokens(&self) -> Vec<St> {
        match self.size {
            SpinnerSize::Sm => vec![St::W1rem, St::H1rem, St::Border2, St::BorderRTransparent, St::AnimateSpinFast],
            SpinnerSize::Md => vec![St::W1_5rem, St::H1_5rem, St::Border2, St::BorderRTransparent, St::AnimateSpinFast],
            SpinnerSize::Lg => vec![St::W2rem, St::H2rem, St::Bw3, St::BorderRTransparent, St::AnimateSpinFast],
        }
    }

    /// Build the spinner into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut tokens = self.compute_tokens();
        tokens.extend(self.size_tokens());
        let mut spinner = el(El::Span)
            .st(tokens)
            .at(At::Role, Av::RoleStatus);

        if let Some(label_text) = self.label {
            spinner = spinner.at_str(At::AriaLabel, &label_text);
        } else {
            spinner = spinner.at_str(At::AriaLabel, "Loading");
        }

        if let Some(ref extra) = self.extra_class {
            spinner = spinner.class(extra.as_ref());
        }

        spinner
    }
}

/// Spinner CSS.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_defaults() {
        let spinner = Spinner::new();
        assert_eq!(spinner.size, SpinnerSize::Md);
        assert!(spinner.label.is_none());
    }

    #[test]
    fn test_spinner_default_tokens() {
        let spinner = Spinner::new();
        let tokens = spinner.compute_tokens();
        assert!(tokens.contains(&St::DisplayInlineBlock));
        assert!(tokens.contains(&St::RoundedFull));
        assert!(tokens.contains(&St::BorderDefault));
    }

    #[test]
    fn test_spinner_size_tokens_sm() {
        let spinner = Spinner::new().size(SpinnerSize::Sm);
        let tokens = spinner.size_tokens();
        assert!(tokens.contains(&St::W1rem));
        assert!(tokens.contains(&St::H1rem));
        assert!(tokens.contains(&St::Border2));
    }

    #[test]
    fn test_spinner_size_tokens_md() {
        let spinner = Spinner::new();
        let tokens = spinner.size_tokens();
        assert!(tokens.contains(&St::W1_5rem));
        assert!(tokens.contains(&St::H1_5rem));
    }

    #[test]
    fn test_spinner_size_tokens_lg() {
        let spinner = Spinner::new().size(SpinnerSize::Lg);
        let tokens = spinner.size_tokens();
        assert!(tokens.contains(&St::W2rem));
        assert!(tokens.contains(&St::H2rem));
        assert!(tokens.contains(&St::Bw3));
    }

    #[test]
    fn test_spinner_size_tokens_have_animation() {
        let spinner = Spinner::new();
        let tokens = spinner.size_tokens();
        assert!(tokens.contains(&St::AnimateSpinFast));
    }

}
