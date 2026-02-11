//! FormField component.
//!
//! Wraps form inputs with label, help text, and error messages.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{FormField, Input};
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

use rwire::attr_tokens::At;
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
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

#[rwire::component]
impl FormField {
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

    /// Compute style tokens for the form field container.
    pub fn compute_tokens(&self) -> Vec<St> {
        vec![St::DisplayFlex, St::FlexCol, St::GapSm]
    }

    /// Build the form field into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Generate ID for input-label association
        let field_id = rwire::builder::generate_element_id("field_");

        let mut container = el(El::Div).st(self.compute_tokens());

        if let Some(ref extra) = self.extra_class {
            container = container.class(extra.as_ref());
        }

        // Add label if provided
        if let Some(label_text) = self.label {
            let mut label = el(El::Label)
                .st([St::TextSm, St::FontMedium, St::TextHigh])
                .at_str(At::For, &field_id)
                .text(&label_text);

            if self.required {
                label = label.append([
                    el(El::Span).st([St::TextError]).text(" *")
                ]);
            }

            container = container.append([label]);
        }

        // Add input if provided
        if let Some(mut input) = self.input {
            // Set ID on input for label association
            input = input.at_str(At::Id, &field_id);
            container = container.append([input]);
        }

        // Add help text if provided
        if let Some(help_text) = self.help {
            container = container.append([
                el(El::Div)
                    .st([St::TextXs, St::TextMedium])
                    .text(&help_text)
            ]);
        }

        // Add error message if provided
        if let Some(error_text) = self.error {
            container = container.append([
                el(El::Div)
                    .st([St::TextXs, St::TextError])
                    .text(&error_text)
            ]);
        }

        container
    }
}

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
    fn test_form_field_tokens() {
        let field = FormField::new();
        let tokens = field.compute_tokens();
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::FlexCol));
        assert!(tokens.contains(&St::GapSm));
    }

}
