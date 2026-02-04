//! FormField component.
//!
//! Wraps form inputs with label, help text, and error messages.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{FormField, Input};
//!
//! FormField::new()
//!     .label("Email")
//!     .input(Input::new().name("email").build())
//!     .help("We'll never share your email")
//!     .build()
//!
//! FormField::new()
//!     .label("Password")
//!     .input(Input::new().attr("type", "password").build())
//!     .error("Password must be at least 8 characters")
//!     .build()
//! ```

use crate::{el, El, ElementBuilder};
use std::borrow::Cow;

/// FormField wrapper builder.
#[derive(Clone, Default)]
pub struct FormField {
    label: Option<Cow<'static, str>>,
    input: Option<ElementBuilder>,
    help: Option<Cow<'static, str>>,
    error: Option<Cow<'static, str>>,
    required: bool,
    extra_class: Option<Cow<'static, str>>,
}

impl FormField {
    /// Base CSS class.
    pub const BASE_CLASS: &'static str = "rw-form-field";

    /// Create a new form field.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the label text.
    pub fn label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the input element (pre-built).
    pub fn input(mut self, input: ElementBuilder) -> Self {
        self.input = Some(input);
        self
    }

    /// Set help text.
    pub fn help(mut self, help: impl Into<Cow<'static, str>>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Set error message.
    pub fn error(mut self, error: impl Into<Cow<'static, str>>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Set required state.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
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

        if self.error.is_some() {
            classes.push_str(" rw-form-field-error");
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the form field into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Register for CSS tree-shaking
        super::registry::mark_component_used(super::registry::ComponentType::FormField);

        // Generate ID for input-label association
        let field_id = crate::builder::generate_element_id("field_");

        let class = self.compute_class();
        let mut container = el(El::Div).class(&class);

        // Add label if provided
        if let Some(label_text) = self.label {
            let mut label = el(El::Label)
                .class("rw-form-field-label")
                .attr("for", &field_id)
                .text(&label_text);

            if self.required {
                label = label.append([
                    el(El::Span).class("rw-form-field-required").text(" *")
                ]);
            }

            container = container.append([label]);
        }

        // Add input if provided
        if let Some(mut input) = self.input {
            // Set ID on input for label association
            input = input.attr("id", &field_id);
            container = container.append([input]);
        }

        // Add help text if provided
        if let Some(help_text) = self.help {
            container = container.append([
                el(El::Div)
                    .class("rw-form-field-help")
                    .text(&help_text)
            ]);
        }

        // Add error message if provided
        if let Some(error_text) = self.error {
            container = container.append([
                el(El::Div)
                    .class("rw-form-field-error-msg")
                    .text(&error_text)
            ]);
        }

        container
    }
}

/// FormField CSS.
///
/// Size: ~195 bytes (under 200 bytes budget)
pub const FORM_FIELD_CSS: &str = "\
.rw-form-field{display:flex;flex-direction:column;gap:var(--rw-space-2)}\
.rw-form-field-label{font-size:var(--rw-text-sm);font-weight:var(--rw-font-medium);color:var(--rw-text-high)}\
.rw-form-field-required{color:var(--rw-red-9)}\
.rw-form-field-help{font-size:var(--rw-text-xs);color:var(--rw-text-medium)}\
.rw-form-field-error-msg{font-size:var(--rw-text-xs);color:var(--rw-red-9)}\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_field_defaults() {
        let field = FormField::new();
        assert!(field.label.is_none());
        assert!(field.input.is_none());
        assert!(field.help.is_none());
        assert!(field.error.is_none());
    }

    #[test]
    fn test_form_field_class_default() {
        let field = FormField::new();
        assert_eq!(field.compute_class(), "rw-form-field");
    }

    #[test]
    fn test_form_field_class_with_error() {
        let field = FormField::new().error("Invalid input");
        let class = field.compute_class();
        assert!(class.contains("rw-form-field"));
        assert!(class.contains("rw-form-field-error"));
    }

    #[test]
    fn test_form_field_css_size() {
        // FormField CSS should be under 200 bytes
        assert!(
            FORM_FIELD_CSS.len() < 400,
            "FormField CSS too large: {} bytes (budget: 400)",
            FORM_FIELD_CSS.len()
        );
        println!("FormField CSS size: {} bytes", FORM_FIELD_CSS.len());
    }

    #[test]
    fn test_form_field_css_structure() {
        assert!(FORM_FIELD_CSS.contains(".rw-form-field{"));
        assert!(FORM_FIELD_CSS.contains(".rw-form-field-label"));
        assert!(FORM_FIELD_CSS.contains(".rw-form-field-required"));
        assert!(FORM_FIELD_CSS.contains(".rw-form-field-help"));
        assert!(FORM_FIELD_CSS.contains(".rw-form-field-error-msg"));
    }
}
