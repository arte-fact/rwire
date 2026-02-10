//! Badge component.
//!
//! Status indicator with color variants.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::Badge;
//!
//! Badge::success("Active").build()
//! Badge::warning("Pending").build()
//! Badge::error("Failed").build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Badge intent/color.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BadgeIntent {
    /// Neutral/default badge
    #[default]
    Default,
    /// Primary accent color
    Primary,
    /// Success (green)
    Success,
    /// Warning (amber)
    Warning,
    /// Error (red)
    Error,
}

/// Badge builder.
#[derive(Clone, Debug, Default)]
pub struct Badge {
    intent: BadgeIntent,
    text: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
}

impl Badge {
    /// Create a new badge.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the badge intent.
    pub fn intent(mut self, intent: BadgeIntent) -> Self {
        self.intent = intent;
        self
    }

    /// Set the badge text.
    pub fn text(mut self, text: impl Into<Cow<'static, str>>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Default badge with text.
    pub fn default_badge(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().text(text)
    }

    /// Primary badge with text.
    pub fn primary(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(BadgeIntent::Primary).text(text)
    }

    /// Success badge with text.
    pub fn success(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(BadgeIntent::Success).text(text)
    }

    /// Warning badge with text.
    pub fn warning(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(BadgeIntent::Warning).text(text)
    }

    /// Error badge with text.
    pub fn error(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(BadgeIntent::Error).text(text)
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for this badge configuration.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![
            St::DisplayInlineFlex,
            St::ItemsCenter,
            St::PxSm,
            St::TextXs,
            St::FontMedium,
            St::RoundedFull,
        ];

        match self.intent {
            BadgeIntent::Default => {
                tokens.push(St::BgEmphasis);
                tokens.push(St::TextHigh);
            }
            BadgeIntent::Primary => {
                tokens.push(St::BgPrimarySubtle);
                tokens.push(St::TextOnPrimarySubtle);
            }
            BadgeIntent::Success => {
                tokens.push(St::BgGreen4);
                tokens.push(St::TextGreen11);
            }
            BadgeIntent::Warning => {
                tokens.push(St::BgAmber4);
                tokens.push(St::TextAmber11);
            }
            BadgeIntent::Error => {
                tokens.push(St::BgRed4);
                tokens.push(St::TextRed11);
            }
        }

        tokens
    }

    /// Build the badge into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut builder = el(El::Span).st(self.compute_tokens());

        if let Some(ref extra) = self.extra_class {
            builder = builder.class(extra.as_ref());
        }

        if let Some(text) = self.text {
            builder = builder.text(&text);
        }

        builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_defaults() {
        let badge = Badge::new();
        assert_eq!(badge.intent, BadgeIntent::Default);
    }

    #[test]
    fn test_badge_default_tokens() {
        let badge = Badge::new();
        let tokens = badge.compute_tokens();
        assert!(tokens.contains(&St::DisplayInlineFlex));
        assert!(tokens.contains(&St::BgEmphasis));
        assert!(tokens.contains(&St::TextHigh));
    }

    #[test]
    fn test_badge_success_tokens() {
        let badge = Badge::success("Active");
        let tokens = badge.compute_tokens();
        assert!(tokens.contains(&St::BgGreen4));
        assert!(tokens.contains(&St::TextGreen11));
    }

}
