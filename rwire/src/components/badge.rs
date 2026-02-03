//! Badge component.
//!
//! Status indicator with color variants.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::Badge;
//!
//! Badge::success("Active").build()
//! Badge::warning("Pending").build()
//! Badge::error("Failed").build()
//! ```

use crate::{el, El, ElementBuilder};
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

    // Convenience constructors

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

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(32);
        classes.push_str("rw-badge");

        match self.intent {
            BadgeIntent::Default => {}
            BadgeIntent::Primary => classes.push_str(" rw-badge-primary"),
            BadgeIntent::Success => classes.push_str(" rw-badge-success"),
            BadgeIntent::Warning => classes.push_str(" rw-badge-warning"),
            BadgeIntent::Error => classes.push_str(" rw-badge-error"),
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the badge into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let class = self.compute_class();
        let mut builder = el(El::Span).class(&class);

        if let Some(text) = self.text {
            builder = builder.text(&text);
        }

        builder
    }
}

/// Badge CSS.
pub const BADGE_CSS: &str = "\
.rw-badge{display:inline-flex;align-items:center;padding:0 var(--rw-space-2);height:1.25rem;\
font-size:var(--rw-text-xs);font-weight:var(--rw-font-medium);border-radius:var(--rw-radius-full);\
background:var(--rw-bg-emphasis);color:var(--rw-text-high)}\
.rw-badge-primary{background:var(--rw-accent-4);color:var(--rw-accent-11)}\
.rw-badge-success{background:var(--rw-green-4);color:var(--rw-green-11)}\
.rw-badge-warning{background:var(--rw-amber-4);color:var(--rw-amber-11)}\
.rw-badge-error{background:var(--rw-red-4);color:var(--rw-red-11)}\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_defaults() {
        let badge = Badge::new();
        assert_eq!(badge.intent, BadgeIntent::Default);
    }

    #[test]
    fn test_badge_class_default() {
        let badge = Badge::new();
        assert_eq!(badge.compute_class(), "rw-badge");
    }

    #[test]
    fn test_badge_class_success() {
        let badge = Badge::success("Active");
        assert_eq!(badge.compute_class(), "rw-badge rw-badge-success");
    }

    #[test]
    fn test_badge_css_size() {
        assert!(BADGE_CSS.len() < 600, "Badge CSS too large: {} bytes", BADGE_CSS.len());
        println!("Badge CSS size: {} bytes", BADGE_CSS.len());
    }
}
