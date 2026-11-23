//! Chat chrome: the small centered rows of a conversation that are neither
//! messages nor cards — a day divider and a quiet note.
//!
//! Both render as full-width rows in a [`ChatTranscript`](crate::ChatTranscript)
//! via [`ChatItem::row()`](crate::ChatItem::row).

use std::borrow::Cow;

use rwire::attr_tokens::{At, Av};
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};

/// A day divider: `« Hier »`, `« Aujourd'hui »`, or the date — once per day,
/// centered, faint, with a 0.04 em letter spacing, and `role="separator"`.
pub struct ChatDivider;

impl ChatDivider {
    /// A day divider labelled `label`.
    pub fn day(label: impl Into<Cow<'static, str>>) -> ElementBuilder {
        let label = label.into();
        el(El::Div)
            .st([
                St::TextCenter,
                St::Text11_5,
                St::TextFaint,
                St::Tracking04em,
                St::SelectNone,
            ])
            .at(At::Role, Av::RoleSeparator)
            .attr("aria-label", label.as_ref())
            .data("testid", "chat-divider")
            .text(label.as_ref())
    }
}

/// A quiet centered note ("Interrompu", "Début de la conversation",
/// "Pas encore de message.").
pub struct ChatNote;

impl ChatNote {
    /// A note row showing `text`.
    pub fn note(text: impl Into<Cow<'static, str>>) -> ElementBuilder {
        let text = text.into();
        el(El::Div)
            .st([St::TextCenter, St::TextXs, St::TextMuted])
            .data("testid", "chat-note")
            .text(text.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_day_divider_is_centered_faint_and_a_separator() {
        let d = ChatDivider::day("Aujourd'hui");
        let html = d.to_static_html();
        assert!(html.contains("role=\"separator\""), "{html}");
        assert!(html.contains("aria-label=\"Aujourd'hui\""), "{html}");
        assert!(html.contains("data-testid=\"chat-divider\""), "{html}");
        assert!(html.contains("Aujourd'hui"), "{html}");
        let utils = d.get_style_utils();
        for st in [
            St::TextCenter,
            St::Text11_5,
            St::TextFaint,
            St::Tracking04em,
        ] {
            assert!(utils.contains(&st.as_u16()), "{st:?}");
        }
        assert!(!html.contains("style="), "no inline style: {html}");
        assert_eq!(St::Tracking04em.css(), "letter-spacing:0.04em");
    }

    #[test]
    fn the_note_is_centered_muted_and_small() {
        let n = ChatNote::note("Début de la conversation");
        let html = n.to_static_html();
        assert!(html.contains("data-testid=\"chat-note\""), "{html}");
        assert!(html.contains("Début de la conversation"), "{html}");
        let utils = n.get_style_utils();
        for st in [St::TextCenter, St::TextXs, St::TextMuted] {
            assert!(utils.contains(&st.as_u16()), "{st:?}");
        }
    }
}
