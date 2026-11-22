//! `Tone`: the color role of a component, shared by components that take one
//! (badges, avatars, status dots, chat authors).
//!
//! A tone names what a color *means*, and each tone resolves to the theme's
//! tokens: a tinted [`ground`](Tone::ground) and the [`ink`](Tone::ink) that
//! reads on it at AA, a [`solid`](Tone::solid) fill and the text
//! [`on_solid`](Tone::on_solid) it, and a [`border`](Tone::border).
//!
//! ```
//! use rwire::{St, Tone};
//!
//! // A badge for a runtime (an identity, no state): categorical color 2.
//! let badge = [Tone::Cat(2).ground(), Tone::Cat(2).ink()];
//! assert_eq!(badge, [St::BgCat2Subtle, St::TextCat2]);
//! // An error pill.
//! assert_eq!(Tone::Danger.ink(), St::TextError);
//! ```

use crate::style_tokens::St;

/// The color role of a component. `Cat(n)` (1 to 6) is a categorical color
/// of the palette ([`ColorPalette::categorical`](crate::tokens::ColorPalette::categorical)):
/// an identity with no meaning of state. Out-of-range `n` wraps (0 → 6, 7 → 1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Tone {
    /// Gray: no state, or idle.
    #[default]
    Neutral,
    /// The brand color.
    Accent,
    /// Information.
    Info,
    /// Success, or a live process.
    Success,
    /// Needs attention.
    Warning,
    /// Error or destructive.
    Danger,
    /// Categorical color `n` (1 to 6).
    Cat(u8),
}

impl Tone {
    /// `Cat(n)` as an index 1 to 6.
    fn cat(n: u8) -> usize {
        (usize::from(n) + 5) % 6 + 1
    }

    /// The tinted ground (a pill, a selected row, an avatar).
    pub fn ground(self) -> St {
        match self {
            Tone::Neutral => St::BgNeutralSubtle,
            Tone::Accent => St::BgPrimarySubtle,
            Tone::Info => St::BgInfoSubtle,
            Tone::Success => St::BgSuccessSubtle,
            Tone::Warning => St::BgWarningSubtle,
            Tone::Danger => St::BgErrorSubtle,
            Tone::Cat(n) => [
                St::BgCat1Subtle,
                St::BgCat2Subtle,
                St::BgCat3Subtle,
                St::BgCat4Subtle,
                St::BgCat5Subtle,
                St::BgCat6Subtle,
            ][Self::cat(n) - 1],
        }
    }

    /// The text that reads at AA on [`Tone::ground`]. For `Neutral`,
    /// `Success`, `Warning`, `Danger` and `Cat`, also on the plain grounds
    /// (app, subtle, surface, raised surface).
    pub fn ink(self) -> St {
        match self {
            Tone::Neutral => St::TextNeutral,
            Tone::Accent => St::TextOnPrimarySubtle,
            Tone::Info => St::TextOnInfoSubtle,
            Tone::Success => St::TextSuccess,
            Tone::Warning => St::TextWarning,
            Tone::Danger => St::TextError,
            Tone::Cat(n) => [
                St::TextCat1,
                St::TextCat2,
                St::TextCat3,
                St::TextCat4,
                St::TextCat5,
                St::TextCat6,
            ][Self::cat(n) - 1],
        }
    }

    /// The solid fill (a dot, an avatar, a strong badge).
    pub fn solid(self) -> St {
        match self {
            Tone::Neutral => St::BgEmphasis,
            Tone::Accent => St::BgPrimary,
            Tone::Info => St::BgAccent,
            Tone::Success => St::BgSuccess,
            Tone::Warning => St::BgWarning,
            Tone::Danger => St::BgError,
            Tone::Cat(n) => [
                St::BgCat1,
                St::BgCat2,
                St::BgCat3,
                St::BgCat4,
                St::BgCat5,
                St::BgCat6,
            ][Self::cat(n) - 1],
        }
    }

    /// The text on [`Tone::solid`].
    pub fn on_solid(self) -> St {
        match self {
            Tone::Neutral => St::TextHigh,
            Tone::Accent => St::TextOnPrimary,
            Tone::Info => St::TextOnAccent,
            Tone::Success => St::TextOnSuccess,
            Tone::Warning => St::TextOnWarning,
            Tone::Danger => St::TextOnError,
            Tone::Cat(n) => [
                St::TextOnCat1,
                St::TextOnCat2,
                St::TextOnCat3,
                St::TextOnCat4,
                St::TextOnCat5,
                St::TextOnCat6,
            ][Self::cat(n) - 1],
        }
    }

    /// A mark on [`Tone::solid`] at 3:1 (the WCAG threshold for graphics):
    /// for a logotype or an `aria-hidden` decorative glyph, **never for read
    /// text** ([`Tone::on_solid`] is at 4.5:1). Categorical tones take the
    /// palette's preferred mark (`ColorPalette::categorical_on_solids`); the
    /// others use their on-solid text.
    pub fn on_solid_mark(self) -> St {
        match self {
            Tone::Cat(n) => [
                St::TextCat1Mark,
                St::TextCat2Mark,
                St::TextCat3Mark,
                St::TextCat4Mark,
                St::TextCat5Mark,
                St::TextCat6Mark,
            ][Self::cat(n) - 1],
            other => other.on_solid(),
        }
    }

    /// A 1px border in the tone (an outlined badge).
    pub fn border(self) -> St {
        match self {
            Tone::Neutral => St::BorderDefault,
            Tone::Accent | Tone::Info => St::BorderAccent,
            Tone::Success => St::BorderSuccess,
            Tone::Warning => St::BorderWarning,
            Tone::Danger => St::BorderError,
            Tone::Cat(n) => [
                St::BorderCat1,
                St::BorderCat2,
                St::BorderCat3,
                St::BorderCat4,
                St::BorderCat5,
                St::BorderCat6,
            ][Self::cat(n) - 1],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categorical_tones_wrap_into_one_to_six() {
        assert_eq!(Tone::Cat(1).ground(), St::BgCat1Subtle);
        assert_eq!(Tone::Cat(6).ink(), St::TextCat6);
        assert_eq!(Tone::Cat(7).solid(), St::BgCat1);
        assert_eq!(Tone::Cat(0).border(), St::BorderCat6);
        assert_eq!(Tone::Cat(3).on_solid(), St::TextOnCat3);
    }

    #[test]
    fn every_tone_token_reads_a_variable_the_theme_defines() {
        use crate::theme::{generate_theme_css, Theme, ThemeMode};
        let tones = [
            Tone::Neutral,
            Tone::Accent,
            Tone::Info,
            Tone::Success,
            Tone::Warning,
            Tone::Danger,
        ]
        .into_iter()
        .chain((1..=6).map(Tone::Cat));
        let themes = [
            generate_theme_css(&Theme::light()),
            generate_theme_css(&Theme::dark()),
            generate_theme_css(&Theme::new().mode(ThemeMode::System)),
        ];
        for tone in tones {
            for st in [
                tone.ground(),
                tone.ink(),
                tone.solid(),
                tone.on_solid(),
                tone.on_solid_mark(),
                tone.border(),
            ] {
                let css = st.css();
                let var = css.split("var(--").nth(1).and_then(|v| v.split(')').next());
                let var = var.unwrap_or_else(|| panic!("{st:?} reads no variable: {css}"));
                for theme in &themes {
                    assert!(
                        theme.contains(&format!("--{var}:")),
                        "{tone:?}: --{var} ({st:?}) undefined"
                    );
                }
            }
        }
    }
}
