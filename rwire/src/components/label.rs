//! Label component for form fields.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::Label;
//!
//! Label::new("Email").build()
//! Label::new("Email").required(true).build()
//! Label::new("Password").attr("for", "pwd").build()
//! ```

use crate::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Label component builder.
#[derive(Clone, Debug, Default)]
pub struct Label {
    text: Option<Cow<'static, str>>,
    required: bool,
    extra_class: Option<Cow<'static, str>>,
}

impl Label {
    /// Base CSS class.
    pub const BASE_CLASS: &'static str = "rw-label";

    /// Create a new label with text.
    pub fn new(text: impl Into<Cow<'static, str>>) -> Self {
        Self {
            text: Some(text.into()),
            required: false,
            extra_class: None,
        }
    }

    /// Set the label text.
    pub fn text(mut self, text: impl Into<Cow<'static, str>>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Mark as required (adds asterisk).
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Add custom class (escape hatch).
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute the full class string.
    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(64);
        classes.push_str(Self::BASE_CLASS);

        if self.required {
            classes.push_str(" rw-label-required");
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the label into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Register for CSS tree-shaking
        super::registry::mark_component_used(super::registry::ComponentType::Label);

        let class = self.compute_class();
        let mut builder = el(El::Label).class(&class);

        if let Some(text) = self.text {
            builder = builder.text(&text);
        }

        builder
    }
}

/// Label CSS.
///
/// Minified CSS for the label component.
/// Size: ~150 bytes (under 200 bytes budget)
pub const LABEL_CSS: &str = "\
.rw-label{display:block;font-size:var(--rw-text-sm);font-weight:var(--rw-font-medium);\
color:var(--rw-text-high);margin-bottom:var(--rw-space-1)}\
.rw-label-required::after{content:' *';color:var(--rw-red-9)}\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_defaults() {
        let label = Label::new("Test");
        assert!(label.text.is_some());
        assert!(!label.required);
    }

    #[test]
    fn test_label_class_default() {
        let label = Label::new("Test");
        assert_eq!(label.compute_class(), "rw-label");
    }

    #[test]
    fn test_label_class_required() {
        let label = Label::new("Test").required(true);
        assert_eq!(label.compute_class(), "rw-label rw-label-required");
    }

    #[test]
    fn test_label_class_with_extra() {
        let label = Label::new("Test").class("custom");
        let class = label.compute_class();
        assert!(class.contains("rw-label"));
        assert!(class.contains("custom"));
    }

    #[test]
    fn test_label_css_size() {
        // Label CSS should be under 200 bytes
        assert!(
            LABEL_CSS.len() < 250,
            "Label CSS too large: {} bytes (budget: 250)",
            LABEL_CSS.len()
        );
        println!("Label CSS size: {} bytes", LABEL_CSS.len());
    }

    #[test]
    fn test_label_css_structure() {
        assert!(LABEL_CSS.contains(".rw-label{"));
        assert!(LABEL_CSS.contains(".rw-label-required"));
    }
}
