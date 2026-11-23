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
use rwire::{el, El, ElementBuilder, Tone};
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

/// Badge corner shape. `Pill` is the classic rounded status badge; `Square` is the sharp
/// outlined chip terminal-styled apps want (born from a real app working around the fixed
/// pill: "the Badge component can't go square").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BadgeShape {
    /// Fully rounded (the classic pill)
    #[default]
    Pill,
    /// Sharp corners
    Square,
}

/// Badge fill. `Solid` is the tinted background; `Outline` is a transparent badge with an
/// intent-colored border and text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BadgeFill {
    /// Tinted background (the default)
    #[default]
    Solid,
    /// Transparent with an intent-colored border
    Outline,
}

/// Badge size. `Xs` is the runtime badge target: 10.5 px mono, 5 px radius.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BadgeSize {
    /// Extra-small: 10.5 px mono, 5 px radius.
    Xs,
    /// Medium (default).
    #[default]
    Md,
}

/// Badge builder.
#[derive(Clone, Debug, Default)]
pub struct Badge {
    intent: BadgeIntent,
    shape: BadgeShape,
    fill: BadgeFill,
    tone: Option<Tone>,
    mono: bool,
    size: BadgeSize,
    text: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
    /// A status badge: a [`StatusDot`](crate::StatusDot) in this tone followed
    /// by the label, on the raised ground with the tone's ink.
    status_dot: Option<Tone>,
    /// Below this breakpoint the label is sr-only (the dot stays visible).
    label_below: Option<rwire::style_tokens::Bp>,
}

#[rwire::component]
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

    /// Set the corner shape (pill by default).
    pub fn shape(mut self, shape: BadgeShape) -> Self {
        self.shape = shape;
        self
    }

    /// Set the fill (solid by default).
    pub fn fill(mut self, fill: BadgeFill) -> Self {
        self.fill = fill;
        self
    }

    /// Set the color tone: `tone.ground()` background, `tone.ink()` text, and
    /// `tone.border()` in the `Outline` fill. Overrides the intent colors.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }

    /// Use the mono font.
    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    /// Set the size (`Xs` is 10.5 px mono with a 5 px radius).
    pub fn size(mut self, size: BadgeSize) -> Self {
        self.size = size;
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

    /// A status badge: a dot in `tone` followed by `label`, on the raised
    /// ground with the tone's ink, 12 px, weight 600, fully rounded (the top
    /// bar's state pill).
    pub fn status(tone: Tone, label: impl Into<Cow<'static, str>>) -> Self {
        Self::new().status_dot(tone).text(label)
    }

    /// The status badge's dot tone (see [`Badge::status`]).
    pub fn status_dot(mut self, tone: Tone) -> Self {
        self.status_dot = Some(tone);
        self
    }

    /// Below `bp`, the label is only read by screen readers (sr-only, not
    /// `display:none`); the dot stays visible alone.
    pub fn label_hidden_below(mut self, bp: rwire::style_tokens::Bp) -> Self {
        self.label_below = Some(bp);
        self
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
        ];
        if self.mono || self.size == BadgeSize::Xs {
            tokens.push(St::FontMono);
        }

        tokens.push(match (self.size, self.shape) {
            // The runtime badge: 10.5 px, 5 px radius.
            (BadgeSize::Xs, _) => St::RoundedBadgeXs,
            (_, BadgeShape::Pill) => St::RoundedFull,
            (_, BadgeShape::Square) => St::Rounded0,
        });
        if self.size == BadgeSize::Xs {
            tokens.retain(|t| *t != St::TextXs);
            tokens.push(St::TextBadgeXs);
        }

        if let Some(tone) = self.tone {
            tokens.push(tone.ground());
            tokens.push(tone.ink());
            if self.fill == BadgeFill::Outline {
                tokens.push(St::BgTransparent);
                tokens.push(tone.border());
            }
            return tokens;
        }

        // A status badge: raised ground, the tone's ink, weight 600, hairline.
        if let Some(tone) = self.status_dot {
            tokens.push(St::BgSurfaceRaised);
            tokens.push(tone.ink());
            tokens.push(St::FontSemibold);
            tokens.push(St::BorderSubtle);
            return tokens;
        }

        match self.fill {
            BadgeFill::Solid => match self.intent {
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
            },
            // Outline: transparent, an intent-colored border and text. The `Border*8`
            // tokens override only border-color, so they layer on `BorderDefault`.
            BadgeFill::Outline => {
                tokens.push(St::BgTransparent);
                tokens.push(St::BorderDefault);
                match self.intent {
                    BadgeIntent::Default => tokens.push(St::TextMuted),
                    BadgeIntent::Primary => {
                        tokens.push(St::BorderAccent);
                        tokens.push(St::TextAccent);
                    }
                    BadgeIntent::Success => {
                        tokens.push(St::BorderGreen8);
                        tokens.push(St::TextGreen11);
                    }
                    BadgeIntent::Warning => {
                        tokens.push(St::BorderAmber8);
                        tokens.push(St::TextAmber11);
                    }
                    BadgeIntent::Error => {
                        tokens.push(St::BorderRed8);
                        tokens.push(St::TextRed11);
                    }
                }
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

        if let Some(tone) = self.status_dot {
            // The status pill: a dot, then the label (sr-only below `label_below`).
            let dot = crate::StatusDot::new().tone(tone).build();
            let label = match self.text {
                Some(text) => {
                    let span = el(El::Span).text(&text);
                    match self.label_below {
                        Some(bp) => span.st([St::sr_only_below(bp)]),
                        None => span,
                    }
                }
                None => el(El::Span),
            };
            return builder.data("testid", "badge-status").append([dot, label]);
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

    #[test]
    fn badge_tone_uses_ground_and_ink() {
        let badge = Badge::new().tone(Tone::Danger);
        let tokens = badge.compute_tokens();
        assert!(tokens.contains(&Tone::Danger.ground()), "{tokens:?}");
        assert!(tokens.contains(&Tone::Danger.ink()), "{tokens:?}");
    }

    #[test]
    fn badge_tone_outline_adds_the_tone_border() {
        let badge = Badge::new().tone(Tone::Success).fill(BadgeFill::Outline);
        let tokens = badge.compute_tokens();
        assert!(tokens.contains(&Tone::Success.border()), "{tokens:?}");
        assert!(tokens.contains(&St::BgTransparent), "{tokens:?}");
        assert!(tokens.contains(&Tone::Success.ink()), "{tokens:?}");
    }

    #[test]
    fn badge_mono_uses_the_mono_font() {
        let tokens = Badge::new().mono(true).compute_tokens();
        assert!(tokens.contains(&St::FontMono), "{tokens:?}");
    }

    #[test]
    fn badge_xs_is_10_5px_mono_with_a_5px_radius() {
        let badge = Badge::new().size(BadgeSize::Xs).text("12");
        let tokens = badge.compute_tokens();
        assert!(tokens.contains(&St::FontMono), "{tokens:?}");
        let html = badge.build().to_static_html();
        // Tokens only: no inline style (an app can still override with `.st()`).
        assert!(!html.contains(" style="), "{html}");
        assert!(tokens.contains(&St::TextBadgeXs), "{tokens:?}");
        assert!(tokens.contains(&St::RoundedBadgeXs), "{tokens:?}");
        assert!(!tokens.contains(&St::TextXs) && !tokens.contains(&St::RoundedFull));
    }

    #[test]
    fn a_status_badge_is_a_toned_dot_plus_label_on_the_raised_ground() {
        let badge = Badge::status(Tone::Success, "Travaille")
            .label_hidden_below(rwire::style_tokens::Bp::Sm)
            .build();
        let html = badge.to_static_html();
        assert!(html.contains("data-testid=\"badge-status\""), "{html}");
        assert!(html.contains("Travaille"), "{html}");
        let utils = badge.get_style_utils();
        for st in [
            St::BgSurfaceRaised,
            Tone::Success.ink(),
            St::FontSemibold,
            St::RoundedFull,
        ] {
            assert!(utils.contains(&st.as_u16()), "{st:?}");
        }
        // The label span is sr-only below sm (kept for screen readers).
        let label = &badge.children()[1];
        assert!(label
            .get_style_utils()
            .contains(&St::sr_only_below(rwire::style_tokens::Bp::Sm).as_u16()));
        assert!(!html.contains("style="), "no inline style: {html}");
    }
}
