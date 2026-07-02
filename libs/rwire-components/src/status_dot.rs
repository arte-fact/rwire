//! StatusDot component.
//!
//! A small presence/status indicator: a colored dot, optionally pulsing while something is
//! live, with an optional inline label. The nav-row and "running" glyph every dashboard
//! rebuilds by hand.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{StatusDot, StatusDotIntent};
//!
//! StatusDot::new().intent(StatusDotIntent::Primary).pulse(true).build()   // working
//! StatusDot::new().intent(StatusDotIntent::Primary).label("running").build()
//! StatusDot::new().build()                                                // idle (muted)
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// StatusDot color intent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusDotIntent {
    /// Idle / inactive (muted)
    #[default]
    Muted,
    /// Active on the accent color
    Primary,
    /// Healthy (green)
    Success,
    /// Attention (amber)
    Warning,
    /// Broken (red)
    Error,
}

/// StatusDot builder.
#[derive(Clone, Debug, Default)]
pub struct StatusDot {
    intent: StatusDotIntent,
    pulse: bool,
    label: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
}

impl StatusDot {
    /// Create a muted (idle) dot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the color intent.
    pub fn intent(mut self, intent: StatusDotIntent) -> Self {
        self.intent = intent;
        self
    }

    /// Pulse the dot (something is live).
    pub fn pulse(mut self, pulse: bool) -> Self {
        self.pulse = pulse;
        self
    }

    /// Add an inline label after the dot, tinted to match.
    pub fn label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Add extra CSS classes.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    fn colors(&self) -> (St, St) {
        match self.intent {
            StatusDotIntent::Muted => (St::BgMuted, St::TextMuted),
            StatusDotIntent::Primary => (St::BgAccent, St::TextAccent),
            StatusDotIntent::Success => (St::BgGreen9, St::TextGreen11),
            StatusDotIntent::Warning => (St::BgAmber9, St::TextAmber11),
            StatusDotIntent::Error => (St::BgRed9, St::TextRed11),
        }
    }

    /// Build the dot (and optional label) into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let (bg, text) = self.colors();
        let mut dot = el(El::Span).st([St::WSp2, St::HSp2, St::RoundedFull, bg]);
        if self.pulse {
            dot = dot.st([St::AnimatePulse]);
        }
        let mut builder = match self.label {
            Some(ref label) => el(El::Span)
                .st([
                    St::DisplayInlineFlex,
                    St::ItemsCenter,
                    St::GapSm,
                    St::TextXs,
                    text,
                ])
                .append([dot, el(El::Span).text(label.as_ref())]),
            None => dot,
        };
        if let Some(ref extra) = self.extra_class {
            builder = builder.class(extra.as_ref());
        }
        builder
    }
}
