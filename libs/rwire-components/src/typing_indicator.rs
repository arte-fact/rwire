//! TypingIndicator component.
//!
//! The "someone is writing" cue: three pulsing dots (the global `rw-pulse`
//! keyframes) with an optional muted label ("claw is typing…"). Rendered by
//! `ChatTranscript` as the writing-state row; usable standalone anywhere a
//! quiet in-progress cue fits.

use std::borrow::Cow;

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};

/// TypingIndicator builder.
#[derive(Debug, Default)]
pub struct TypingIndicator {
    label: Option<Cow<'static, str>>,
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

    fn dot() -> ElementBuilder {
        el(El::Span)
            .st([
                St::DisplayInlineBlock,
                St::RoundedFull,
                St::BgMuted,
                St::AnimatePulse,
            ])
            // 0.4rem dot — no sizing token this small.
            .style(rwire::style::Style::new().width("0.4rem").height("0.4rem"))
    }

    /// Build into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut row = el(El::Div)
            .st([St::DisplayFlex, St::ItemsCenter, St::GapXs])
            .append([Self::dot(), Self::dot(), Self::dot()]);
        if let Some(label) = self.label {
            row = row.append([el(El::Span)
                .st([St::TextXs, St::TextMuted])
                .text(label.as_ref())]);
        }
        row
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_three_dots_and_optional_label() {
        let plain = TypingIndicator::new().build();
        assert_eq!(plain.children().len(), 3);
        let labeled = TypingIndicator::new().label("claw is typing…").build();
        assert_eq!(labeled.children().len(), 4);
    }
}
