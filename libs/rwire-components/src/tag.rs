//! Tag/Chip component.
//!
//! Removable label or filter indicator with color variants.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Tag, TagIntent};
//!
//! Tag::new("Rust")
//!     .intent(TagIntent::Primary)
//!     .removable(true)
//!     .on_remove(remove_tag())
//!     .build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder, Ev, HandlerSpec};
use std::borrow::Cow;

/// Tag color variant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TagIntent {
    /// Neutral default.
    #[default]
    Default,
    /// Primary accent.
    Primary,
    /// Success (green).
    Success,
    /// Warning (amber).
    Warning,
    /// Error (red).
    Error,
}

/// Tag builder.
#[derive(Clone, Default)]
pub struct Tag {
    text: Cow<'static, str>,
    intent: TagIntent,
    removable: bool,
    on_remove: Option<HandlerSpec>,
    extra_class: Option<Cow<'static, str>>,
}

impl Tag {
    /// Create a new tag with text.
    pub fn new(text: impl Into<Cow<'static, str>>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    /// Set the tag intent.
    pub fn intent(mut self, intent: TagIntent) -> Self {
        self.intent = intent;
        self
    }

    /// Make the tag removable (shows X button).
    pub fn removable(mut self, removable: bool) -> Self {
        self.removable = removable;
        self
    }

    /// Set the remove handler.
    pub fn on_remove(mut self, handler: HandlerSpec) -> Self {
        self.removable = true;
        self.on_remove = Some(handler);
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the tag.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![
            St::DisplayInlineFlex,
            St::ItemsCenter,
            St::GapXs,
            St::PxSm,
            St::PySm,
            St::TextXs,
            St::FontMedium,
            St::RoundedMd,
        ];

        match self.intent {
            TagIntent::Default => {
                tokens.push(St::BgEmphasis);
                tokens.push(St::TextHigh);
            }
            TagIntent::Primary => {
                tokens.push(St::BgAccent4);
                tokens.push(St::TextAccent11);
            }
            TagIntent::Success => {
                tokens.push(St::BgGreen4);
                tokens.push(St::TextGreen11);
            }
            TagIntent::Warning => {
                tokens.push(St::BgAmber4);
                tokens.push(St::TextAmber11);
            }
            TagIntent::Error => {
                tokens.push(St::BgRed4);
                tokens.push(St::TextRed11);
            }
        }

        tokens
    }

    /// Build the tag into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut tag = el(El::Span).st(self.compute_tokens());

        if let Some(ref extra) = self.extra_class {
            tag = tag.class(extra.as_ref());
        }

        tag = tag.append([el(El::Span).text(&self.text)]);

        if self.removable {
            let mut close_btn = el(El::Button)
                .st([
                    St::DisplayInlineFlex,
                    St::ItemsCenter,
                    St::CursorPointer,
                    St::TextXs,
                    St::Opacity75,
                ])
                .hover([St::Opacity100])
                .text("\u{00d7}"); // × character

            if let Some(handler) = self.on_remove {
                close_btn = close_btn.on(Ev::Click, handler);
            }

            tag = tag.append([close_btn]);
        }

        tag
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_defaults() {
        let tag = Tag::new("Rust");
        assert_eq!(tag.text.as_ref(), "Rust");
        assert_eq!(tag.intent, TagIntent::Default);
        assert!(!tag.removable);
    }

    #[test]
    fn test_tag_default_tokens() {
        let tag = Tag::new("Test");
        let tokens = tag.compute_tokens();
        assert!(tokens.contains(&St::DisplayInlineFlex));
        assert!(tokens.contains(&St::RoundedMd));
        assert!(tokens.contains(&St::BgEmphasis));
    }

    #[test]
    fn test_tag_primary_tokens() {
        let tag = Tag::new("Test").intent(TagIntent::Primary);
        let tokens = tag.compute_tokens();
        assert!(tokens.contains(&St::BgAccent4));
        assert!(tokens.contains(&St::TextAccent11));
    }

    #[test]
    fn test_tag_removable() {
        let tag = Tag::new("Test").removable(true);
        assert!(tag.removable);
    }
}
