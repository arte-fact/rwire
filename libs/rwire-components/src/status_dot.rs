//! StatusDot component.
//!
//! A small presence/status indicator: a colored dot, optionally pulsing while something is
//! live, with an optional halo and an optional inline label. The nav-row and "running" glyph
//! every dashboard rebuilds by hand.
//!
//! The dot's color is its tone's dot color (`Tone::dot`, `--Y?d`): it reaches 3:1, the WCAG
//! threshold for graphics, on every ground a dot sits on (app, surface, sidebar, a hovered
//! or selected row). The pulse stops under `prefers-reduced-motion`.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{StatusDot, StatusDotIntent};
//!
//! StatusDot::new().intent(StatusDotIntent::Success).ring(true).pulse(true).build() // generating
//! StatusDot::new().tone(Tone::Cat(5)).ring(true).build()                         // waiting for a decision
//! StatusDot::new().intent(StatusDotIntent::Hollow).build()                       // inactive
//! StatusDot::new().build()                                                       // idle
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder, Tone};
use std::borrow::Cow;

/// StatusDot color intent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusDotIntent {
    /// Idle (neutral, filled)
    #[default]
    Muted,
    /// Active on the accent color
    Primary,
    /// Healthy, or live (green)
    Success,
    /// Attention (amber)
    Warning,
    /// Broken (red)
    Error,
    /// Inactive: a neutral circle, 1.5 px, not filled
    Hollow,
}

/// StatusDot size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusDotSize {
    /// 9 px (dense lists).
    #[default]
    Sm,
    /// 12 px.
    Md,
}

/// StatusDot builder.
#[derive(Clone, Debug, Default)]
pub struct StatusDot {
    intent: StatusDotIntent,
    tone: Option<Tone>,
    pulse: bool,
    ring: bool,
    size: StatusDotSize,
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

    /// Color the dot by any tone (a categorical one, say), instead of the
    /// intent's. With [`StatusDotIntent::Hollow`] the circle takes it.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }

    /// Pulse the dot (something is live). Stopped under `prefers-reduced-motion`.
    pub fn pulse(mut self, pulse: bool) -> Self {
        self.pulse = pulse;
        self
    }

    /// A 3 px halo at 22 % of the dot's color (generating, waiting).
    pub fn ring(mut self, ring: bool) -> Self {
        self.ring = ring;
        self
    }

    /// Set the size (`Sm`, 9 px, by default).
    pub fn size(mut self, size: StatusDotSize) -> Self {
        self.size = size;
        self
    }

    /// Add an inline label after the dot, in the tone's readable ink.
    pub fn label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Add extra CSS classes.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// The dot's tone: the explicit one, else the intent's.
    fn tone_of(&self) -> Tone {
        self.tone.unwrap_or(match self.intent {
            StatusDotIntent::Muted | StatusDotIntent::Hollow => Tone::Neutral,
            StatusDotIntent::Primary => Tone::Accent,
            StatusDotIntent::Success => Tone::Success,
            StatusDotIntent::Warning => Tone::Warning,
            StatusDotIntent::Error => Tone::Danger,
        })
    }

    /// The dot's own tokens.
    pub fn dot_tokens(&self) -> Vec<St> {
        let tone = self.tone_of();
        let mut tokens = vec![
            match self.size {
                StatusDotSize::Sm => St::SizeDotSm,
                StatusDotSize::Md => St::SizeDotMd,
            },
            St::RoundedFull,
            St::FlexShrink0,
            St::DisplayInlineBlock,
            tone.dot(),
        ];
        tokens.push(if self.intent == StatusDotIntent::Hollow {
            St::DotHollow
        } else {
            St::BgCurrentColor
        });
        if self.ring {
            tokens.push(St::DotRing);
        }
        if self.pulse {
            tokens.push(St::AnimatePulse);
        }
        tokens
    }

    /// Build the dot (and optional label) into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let dot = el(El::Span).st(self.dot_tokens());
        let mut builder = match self.label {
            Some(ref label) => el(El::Span)
                .st([
                    St::DisplayInlineFlex,
                    St::ItemsCenter,
                    St::GapSm,
                    St::TextXs,
                    self.tone_of().ink(),
                ])
                .append([dot, el(El::Span).text(label.as_ref())]),
            None => dot.attr("aria-hidden", "true"),
        };
        if let Some(ref extra) = self.extra_class {
            builder = builder.class(extra.as_ref());
        }
        builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dot_is_drawn_in_its_tones_dot_color() {
        let dot = StatusDot::new().intent(StatusDotIntent::Success).build();
        let utils = dot.get_style_utils();
        for st in [
            St::TextDotSuccess,
            St::BgCurrentColor,
            St::SizeDotSm,
            St::RoundedFull,
        ] {
            assert!(utils.contains(&st.as_u16()), "{st:?}");
        }
        // Decorative without a label.
        assert!(dot.to_static_html().contains("aria-hidden=\"true\""));
    }

    #[test]
    fn ring_hollow_size_and_tone() {
        let ringed = StatusDot::new().tone(Tone::Cat(5)).ring(true).build();
        let utils = ringed.get_style_utils();
        assert!(utils.contains(&St::TextDotCat5.as_u16()));
        assert!(utils.contains(&St::DotRing.as_u16()));
        let hollow = StatusDot::new()
            .intent(StatusDotIntent::Hollow)
            .size(StatusDotSize::Md)
            .build();
        let utils = hollow.get_style_utils();
        assert!(utils.contains(&St::DotHollow.as_u16()));
        assert!(utils.contains(&St::TextDotNeutral.as_u16()));
        assert!(utils.contains(&St::SizeDotMd.as_u16()));
        assert!(!utils.contains(&St::BgCurrentColor.as_u16()));
        assert_eq!(St::SizeDotSm.css(), "width:9px;height:9px");
        assert_eq!(
            St::DotRing.css(),
            "box-shadow:0 0 0 3px color-mix(in srgb,currentColor 22%,transparent)"
        );
        assert_eq!(
            St::DotHollow.css(),
            "border:1.5px solid currentColor;background:transparent"
        );
    }

    #[test]
    fn the_pulse_stops_under_reduced_motion() {
        let live = StatusDot::new().pulse(true).build();
        assert!(live.get_style_utils().contains(&St::AnimatePulse.as_u16()));
        let reduced = rwire::style_tokens::PSEUDO_GLOBAL_CSS;
        let rule = reduced
            .split("@media(prefers-reduced-motion:reduce){")
            .nth(1)
            .unwrap();
        assert!(
            rule.contains(&format!(".u{},", St::AnimatePulse.as_u16())),
            "{rule}"
        );
    }

    #[test]
    fn a_label_takes_the_tones_ink() {
        let html = StatusDot::new()
            .intent(StatusDotIntent::Error)
            .label("failed")
            .build();
        assert!(html.get_style_utils().contains(&St::TextError.as_u16()));
        assert_eq!(html.children().len(), 2);
    }
}
