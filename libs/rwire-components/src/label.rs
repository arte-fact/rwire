//! Label component for form fields.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::Label;
//!
//! Label::new("Email").build()
//! Label::new("Email").required(true).build()
//! Label::new("Password").attr("for", "pwd").build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Label component builder.
#[derive(Clone, Debug, Default)]
pub struct Label {
    text: Option<Cow<'static, str>>,
    required: bool,
    extra_class: Option<Cow<'static, str>>,
}

#[rwire::component]
impl Label {
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

    /// Compute style tokens for the label.
    pub fn compute_tokens(&self) -> Vec<St> {
        vec![
            St::DisplayBlock,
            St::TextSm,
            St::FontMedium,
            St::TextHigh,
            St::MbXs,
        ]
    }

    /// Build the label into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut builder = el(El::Label).st(self.compute_tokens());

        if self.required {
            builder = builder.after([St::ContentAsterisk, St::TextError]);
        }

        if let Some(ref extra) = self.extra_class {
            builder = builder.class(extra.as_ref());
        }

        if let Some(text) = self.text {
            builder = builder.text(&text);
        }

        builder
    }
}

/// Label CSS.
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
    fn test_label_default_tokens() {
        let label = Label::new("Test");
        let tokens = label.compute_tokens();
        assert!(tokens.contains(&St::DisplayBlock));
        assert!(tokens.contains(&St::TextSm));
        assert!(tokens.contains(&St::FontMedium));
        assert!(tokens.contains(&St::TextHigh));
        assert!(tokens.contains(&St::MbXs));
    }

    #[test]
    fn test_label_required_pseudo() {
        let label = Label::new("Test").required(true).build();
        let groups = label.get_pseudo_groups();
        // Should have ::after group for asterisk
        assert!(groups.iter().any(|(pc, _)| *pc == 0x08)); // Pc::After
    }

    #[test]
    fn test_label_not_required_no_pseudo() {
        let label = Label::new("Test").build();
        let groups = label.get_pseudo_groups();
        assert!(groups.is_empty());
    }
}
