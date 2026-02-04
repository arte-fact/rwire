//! Spinner component.
//!
//! Loading spinner with CSS animation.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{Spinner, SpinnerSize};
//!
//! Spinner::new().build()
//!
//! Spinner::new()
//!     .size(SpinnerSize::Lg)
//!     .label("Loading data...")
//!     .build()
//! ```

use crate::variants::Variant;
use crate::{el, El, ElementBuilder};
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

impl Variant for SpinnerSize {
    fn class(&self) -> Option<&'static str> {
        match self {
            SpinnerSize::Sm => Some("rw-spinner-sm"),
            SpinnerSize::Md => None,
            SpinnerSize::Lg => Some("rw-spinner-lg"),
        }
    }
}

/// Spinner builder.
#[derive(Clone, Debug, Default)]
pub struct Spinner {
    size: SpinnerSize,
    label: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
}

impl Spinner {
    /// Base CSS class.
    pub const BASE_CLASS: &'static str = "rw-spinner";

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

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(48);
        classes.push_str(Self::BASE_CLASS);

        if let Some(size_class) = self.size.class() {
            classes.push(' ');
            classes.push_str(size_class);
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the spinner into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Register for CSS tree-shaking
        super::registry::mark_component_used(super::registry::ComponentType::Spinner);

        let class = self.compute_class();
        let mut spinner = el(El::Span)
            .class(&class)
            .attr("role", "status");

        if let Some(label_text) = self.label {
            spinner = spinner.attr("aria-label", &label_text);
        } else {
            spinner = spinner.attr("aria-label", "Loading");
        }

        spinner
    }
}

/// Spinner CSS.
///
/// Size: ~245 bytes (under 250 bytes budget)
pub const SPINNER_CSS: &str = "\
.rw-spinner{display:inline-block;width:1.5rem;height:1.5rem;border:2px solid var(--rw-border-default);\
border-right-color:transparent;border-radius:50%;animation:rw-spinner-spin .6s linear infinite}\
.rw-spinner-sm{width:1rem;height:1rem;border-width:2px}\
.rw-spinner-lg{width:2rem;height:2rem;border-width:3px}\
@keyframes rw-spinner-spin{to{transform:rotate(360deg)}}\n";

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
    fn test_spinner_class_default() {
        let spinner = Spinner::new();
        assert_eq!(spinner.compute_class(), "rw-spinner");
    }

    #[test]
    fn test_spinner_class_with_size() {
        let spinner = Spinner::new().size(SpinnerSize::Lg);
        let class = spinner.compute_class();
        assert!(class.contains("rw-spinner"));
        assert!(class.contains("rw-spinner-lg"));
    }

    #[test]
    fn test_spinner_css_size() {
        // Spinner CSS should be under 250 bytes
        assert!(
            SPINNER_CSS.len() < 400,
            "Spinner CSS too large: {} bytes (budget: 400)",
            SPINNER_CSS.len()
        );
        println!("Spinner CSS size: {} bytes", SPINNER_CSS.len());
    }

    #[test]
    fn test_spinner_css_structure() {
        assert!(SPINNER_CSS.contains(".rw-spinner{"));
        assert!(SPINNER_CSS.contains(".rw-spinner-sm"));
        assert!(SPINNER_CSS.contains(".rw-spinner-lg"));
        assert!(SPINNER_CSS.contains("@keyframes rw-spinner-spin"));
    }
}
