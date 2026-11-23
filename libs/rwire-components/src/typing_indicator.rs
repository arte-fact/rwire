//! TypingIndicator component.
//!
//! The "someone is writing" cue: three pulsing dots (the global `rw-pulse`
//! keyframes) with an optional muted label ("claw is typing…"). Rendered by
//! `ChatTranscript` as the writing-state row; usable standalone anywhere a
//! quiet in-progress cue fits.

use std::borrow::Cow;

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder, Tone};

/// TypingIndicator builder.
#[derive(Debug, Default)]
pub struct TypingIndicator {
    label: Option<Cow<'static, str>>,
    tone: Option<Tone>,
}

impl TypingIndicator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Muted label after the dots (e.g. `"claw is typing…"`).
    pub fn label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Colour the dots and the label with this tone (`Tone::Success` while an
    /// agent generates). The pulsation itself is cut globally under
    /// `prefers-reduced-motion`.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }

    fn dot(tone: Option<Tone>) -> ElementBuilder {
        let mut dot = el(El::Span).st([St::DisplayInlineBlock, St::RoundedFull, St::AnimatePulse]);
        // Themed dot colour: the tone's, else the muted ground.
        dot = match tone {
            Some(tone) => dot.st([tone.dot(), St::BgCurrentColor]),
            None => dot.st([St::BgMuted]),
        };
        dot.st([St::SqDot04])
    }

    /// Build into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut row = el(El::Div)
            .st([St::DisplayFlex, St::ItemsCenter, St::GapXs])
            .append([
                Self::dot(self.tone),
                Self::dot(self.tone),
                Self::dot(self.tone),
            ]);
        if let Some(label) = self.label {
            let mut label_tokens = vec![St::TextXs];
            label_tokens.push(match self.tone {
                Some(tone) => tone.ink(),
                None => St::TextMuted,
            });
            row = row.append([el(El::Span).st(label_tokens).text(label.as_ref())]);
        }
        row
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tone_colours_the_dots_and_the_label() {
        let t = TypingIndicator::new()
            .label("claude travaille…")
            .tone(Tone::Success)
            .build();
        let dot = &t.children()[0];
        assert!(dot
            .get_style_utils()
            .contains(&Tone::Success.dot().as_u16()));
        assert!(dot.get_style_utils().contains(&St::BgCurrentColor.as_u16()));
        assert!(dot.get_style_utils().contains(&St::SqDot04.as_u16()));
        let label = &t.children()[3];
        assert!(label
            .get_style_utils()
            .contains(&Tone::Success.ink().as_u16()));
        let html = t.to_static_html();
        assert!(!html.contains("style="), "no inline style: {html}");
        // Default stays muted.
        let plain = TypingIndicator::new().build();
        assert!(plain.children()[0]
            .get_style_utils()
            .contains(&St::BgMuted.as_u16()));
    }

    #[test]
    fn renders_three_dots_and_optional_label() {
        let plain = TypingIndicator::new().build();
        assert_eq!(plain.children().len(), 3);
        let labeled = TypingIndicator::new().label("claw is typing…").build();
        assert_eq!(labeled.children().len(), 4);
    }
}
