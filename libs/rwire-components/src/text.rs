//! Text component.
//!
//! Typography component with semantic variants.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Text, TextVariant};
//!
//! Text::heading1("Welcome").build()
//! Text::body("Regular paragraph text").build()
//! Text::caption("Small helper text").muted().build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Text variant for different typography styles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextVariant {
    /// Body text (default) - base size
    #[default]
    Body,
    /// Small body text
    BodySmall,
    /// Large heading (h1)
    Heading1,
    /// Medium heading (h2)
    Heading2,
    /// Small heading (h3)
    Heading3,
    /// Label text (form labels)
    Label,
    /// Caption text (small helper text)
    Caption,
}

/// Text color variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextColor {
    /// Default text color
    #[default]
    Default,
    /// High contrast text
    High,
    /// Muted/subdued text
    Muted,
    /// Accent color text
    Accent,
    /// Success color text
    Success,
    /// Warning color text
    Warning,
    /// Error color text
    Error,
}

/// Text component builder.
#[derive(Clone, Debug, Default)]
pub struct Text {
    variant: TextVariant,
    color: TextColor,
    content: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
}

#[rwire::component]
impl Text {
    /// Create a new text component.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the text variant.
    pub fn variant(mut self, variant: TextVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the text color.
    pub fn color(mut self, color: TextColor) -> Self {
        self.color = color;
        self
    }

    /// Set the text content.
    pub fn content(mut self, content: impl Into<Cow<'static, str>>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Body text with content.
    pub fn body(content: impl Into<Cow<'static, str>>) -> Self {
        Self::new().variant(TextVariant::Body).content(content)
    }

    /// Small body text with content.
    pub fn body_small(content: impl Into<Cow<'static, str>>) -> Self {
        Self::new().variant(TextVariant::BodySmall).content(content)
    }

    /// H1 heading with content.
    pub fn heading1(content: impl Into<Cow<'static, str>>) -> Self {
        Self::new().variant(TextVariant::Heading1).content(content)
    }

    /// H2 heading with content.
    pub fn heading2(content: impl Into<Cow<'static, str>>) -> Self {
        Self::new().variant(TextVariant::Heading2).content(content)
    }

    /// H3 heading with content.
    pub fn heading3(content: impl Into<Cow<'static, str>>) -> Self {
        Self::new().variant(TextVariant::Heading3).content(content)
    }

    /// Label text with content.
    pub fn label(content: impl Into<Cow<'static, str>>) -> Self {
        Self::new().variant(TextVariant::Label).content(content)
    }

    /// Caption text with content.
    pub fn caption(content: impl Into<Cow<'static, str>>) -> Self {
        Self::new().variant(TextVariant::Caption).content(content)
    }

    /// Set muted color.
    pub fn muted(self) -> Self {
        self.color(TextColor::Muted)
    }

    /// Set accent color.
    pub fn accent(self) -> Self {
        self.color(TextColor::Accent)
    }

    /// Set success color.
    pub fn success(self) -> Self {
        self.color(TextColor::Success)
    }

    /// Set warning color.
    pub fn warning(self) -> Self {
        self.color(TextColor::Warning)
    }

    /// Set error color.
    pub fn error(self) -> Self {
        self.color(TextColor::Error)
    }

    /// Compute style tokens for this text configuration.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = Vec::with_capacity(4);

        match self.variant {
            TextVariant::Body => {
                tokens.push(St::TextDefault);
                tokens.push(St::LeadingNormal);
            }
            TextVariant::BodySmall => {
                tokens.push(St::TextSm);
            }
            TextVariant::Heading1 => {
                tokens.push(St::Text3xl);
                tokens.push(St::FontBold);
                tokens.push(St::LeadingTight);
            }
            TextVariant::Heading2 => {
                tokens.push(St::Text2xl);
                tokens.push(St::FontSemibold);
                tokens.push(St::LeadingTight);
            }
            TextVariant::Heading3 => {
                tokens.push(St::TextXl);
                tokens.push(St::FontSemibold);
                tokens.push(St::LeadingSnug);
            }
            TextVariant::Label => {
                tokens.push(St::TextSm);
                tokens.push(St::FontMedium);
            }
            TextVariant::Caption => {
                tokens.push(St::TextXs);
                tokens.push(St::TextMuted);
            }
        }

        match self.color {
            TextColor::Default => {}
            TextColor::High => tokens.push(St::TextHigh),
            TextColor::Muted => tokens.push(St::TextMuted),
            TextColor::Accent => tokens.push(St::TextAccent),
            TextColor::Success => tokens.push(St::TextSuccess),
            TextColor::Warning => tokens.push(St::TextWarning),
            TextColor::Error => tokens.push(St::TextError),
        }

        tokens
    }

    /// Determine the appropriate HTML element for this variant.
    pub fn element(&self) -> El {
        match self.variant {
            TextVariant::Heading1 => El::H1,
            TextVariant::Heading2 => El::H2,
            TextVariant::Heading3 => El::H3,
            TextVariant::Label => El::Span,
            _ => El::P,
        }
    }

    /// Build the text into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let element = self.element();
        let mut builder = el(element).st(self.compute_tokens());

        if let Some(ref extra) = self.extra_class {
            builder = builder.class(extra.as_ref());
        }

        if let Some(content) = self.content {
            builder = builder.text(&content);
        }

        builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_defaults() {
        let text = Text::new();
        assert_eq!(text.variant, TextVariant::Body);
        assert_eq!(text.color, TextColor::Default);
    }

    #[test]
    fn test_text_body_tokens() {
        let text = Text::new();
        let tokens = text.compute_tokens();
        assert!(tokens.contains(&St::TextDefault));
        assert!(tokens.contains(&St::LeadingNormal));
    }

    #[test]
    fn test_text_heading_tokens() {
        let text = Text::heading1("Title");
        let tokens = text.compute_tokens();
        assert!(tokens.contains(&St::Text3xl));
        assert!(tokens.contains(&St::FontBold));
        assert!(tokens.contains(&St::LeadingTight));
    }

    #[test]
    fn test_text_caption_muted_tokens() {
        let text = Text::caption("Help").muted();
        let tokens = text.compute_tokens();
        assert!(tokens.contains(&St::TextXs));
        assert!(tokens.contains(&St::TextMuted));
    }

    #[test]
    fn test_text_elements() {
        assert_eq!(Text::heading1("").element(), El::H1);
        assert_eq!(Text::heading2("").element(), El::H2);
        assert_eq!(Text::heading3("").element(), El::H3);
        assert_eq!(Text::body("").element(), El::P);
        assert_eq!(Text::label("").element(), El::Span);
    }

}
