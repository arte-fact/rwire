//! Text component.
//!
//! Typography component with semantic variants.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{Text, TextVariant};
//!
//! Text::heading1("Welcome").build()
//! Text::body("Regular paragraph text").build()
//! Text::caption("Small helper text").muted().build()
//! ```

use crate::{el, El, ElementBuilder};
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

    // ========================================================================
    // Convenience constructors
    // ========================================================================

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

    // ========================================================================
    // Color shortcuts
    // ========================================================================

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

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(48);
        classes.push_str("rw-text");

        match self.variant {
            TextVariant::Body => {}
            TextVariant::BodySmall => classes.push_str(" rw-text-sm"),
            TextVariant::Heading1 => classes.push_str(" rw-text-h1"),
            TextVariant::Heading2 => classes.push_str(" rw-text-h2"),
            TextVariant::Heading3 => classes.push_str(" rw-text-h3"),
            TextVariant::Label => classes.push_str(" rw-text-label"),
            TextVariant::Caption => classes.push_str(" rw-text-caption"),
        }

        match self.color {
            TextColor::Default => {}
            TextColor::High => classes.push_str(" rw-color-high"),
            TextColor::Muted => classes.push_str(" rw-color-muted"),
            TextColor::Accent => classes.push_str(" rw-color-accent"),
            TextColor::Success => classes.push_str(" rw-color-success"),
            TextColor::Warning => classes.push_str(" rw-color-warning"),
            TextColor::Error => classes.push_str(" rw-color-error"),
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Determine the appropriate HTML element for this variant.
    fn element(&self) -> El {
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
        super::registry::mark_component_used(super::registry::ComponentType::Text);

        let class = self.compute_class();
        let element = self.element();
        let mut builder = el(element).class(&class);

        if let Some(content) = self.content {
            builder = builder.text(&content);
        }

        builder
    }
}

/// Text CSS.
pub const TEXT_CSS: &str = "\
.rw-text{color:var(--rw-text-default);line-height:var(--rw-leading-normal)}\
.rw-text-sm{font-size:var(--rw-text-sm)}\
.rw-text-h1{font-size:var(--rw-text-3xl);font-weight:700;line-height:var(--rw-leading-tight)}\
.rw-text-h2{font-size:var(--rw-text-2xl);font-weight:600;line-height:var(--rw-leading-tight)}\
.rw-text-h3{font-size:var(--rw-text-xl);font-weight:600;line-height:var(--rw-leading-snug)}\
.rw-text-label{font-size:var(--rw-text-sm);font-weight:500}\
.rw-text-caption{font-size:var(--rw-text-xs);color:var(--rw-text-muted)}\
.rw-color-high{color:var(--rw-text-high)}\
.rw-color-muted{color:var(--rw-text-muted)}\
.rw-color-accent{color:var(--rw-accent-11)}\
.rw-color-success{color:var(--rw-success)}\
.rw-color-warning{color:var(--rw-warning)}\
.rw-color-error{color:var(--rw-error)}\n";

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
    fn test_text_class_default() {
        let text = Text::new();
        assert_eq!(text.compute_class(), "rw-text");
    }

    #[test]
    fn test_text_class_heading() {
        let text = Text::heading1("Title");
        assert_eq!(text.compute_class(), "rw-text rw-text-h1");
    }

    #[test]
    fn test_text_class_with_color() {
        let text = Text::caption("Help").muted();
        assert_eq!(text.compute_class(), "rw-text rw-text-caption rw-color-muted");
    }

    #[test]
    fn test_text_elements() {
        assert_eq!(Text::heading1("").element(), El::H1);
        assert_eq!(Text::heading2("").element(), El::H2);
        assert_eq!(Text::heading3("").element(), El::H3);
        assert_eq!(Text::body("").element(), El::P);
        assert_eq!(Text::label("").element(), El::Span);
    }

    #[test]
    fn test_text_css_size() {
        assert!(TEXT_CSS.len() < 800, "Text CSS too large: {} bytes", TEXT_CSS.len());
        println!("Text CSS size: {} bytes", TEXT_CSS.len());
    }
}
