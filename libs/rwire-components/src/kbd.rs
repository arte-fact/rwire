//! Kbd component.
//!
//! Keyboard shortcut display with key-cap styling.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::Kbd;
//!
//! // Single key
//! Kbd::new("K").build()
//!
//! // Shortcut combination
//! Kbd::combo(&["Ctrl", "Shift", "P"]).build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Kbd builder.
#[derive(Clone, Default)]
pub struct Kbd {
    keys: Vec<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
}

#[rwire::component]
impl Kbd {
    /// Create a kbd for a single key.
    pub fn new(key: impl Into<Cow<'static, str>>) -> Self {
        Self {
            keys: vec![key.into()],
            ..Self::default()
        }
    }

    /// Create a kbd for a key combination (e.g., Ctrl+K).
    pub fn combo(keys: &[impl AsRef<str> + Clone]) -> Self {
        Self {
            keys: keys.iter().map(|k| Cow::Owned(k.as_ref().to_string())).collect(),
            ..Self::default()
        }
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for a single key cap.
    pub fn compute_key_tokens() -> Vec<St> {
        vec![
            St::DisplayInlineFlex,
            St::ItemsCenter,
            St::JustifyCenter,
            St::FontMono,
            St::TextXs,
            St::PxXs,
            St::RoundedSm,
            St::KbdBg,
            St::KbdShadow,
            St::MinWKbd,
        ]
    }

    /// Build the kbd into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        if self.keys.len() == 1 {
            let mut key = el(El::Kbd)
                .st(Self::compute_key_tokens())
                .text(&self.keys[0]);

            if let Some(ref extra) = self.extra_class {
                key = key.class(extra.as_ref());
            }

            key
        } else {
            let mut container = el(El::Span)
                .st([St::DisplayInlineFlex, St::ItemsCenter, St::GapXs]);

            if let Some(ref extra) = self.extra_class {
                container = container.class(extra.as_ref());
            }

            for (i, key) in self.keys.iter().enumerate() {
                if i > 0 {
                    container = container.append([
                        el(El::Span).st([St::TextXs, St::TextMuted]).text("+"),
                    ]);
                }
                container = container.append([
                    el(El::Kbd).st(Self::compute_key_tokens()).text(key),
                ]);
            }

            container
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kbd_single() {
        let kbd = Kbd::new("K");
        assert_eq!(kbd.keys.len(), 1);
        assert_eq!(kbd.keys[0].as_ref(), "K");
    }

    #[test]
    fn test_kbd_combo() {
        let kbd = Kbd::combo(&["Ctrl", "K"]);
        assert_eq!(kbd.keys.len(), 2);
    }

    #[test]
    fn test_kbd_key_tokens() {
        let tokens = Kbd::compute_key_tokens();
        assert!(tokens.contains(&St::FontMono));
        assert!(tokens.contains(&St::KbdBg));
        assert!(tokens.contains(&St::KbdShadow));
        assert!(tokens.contains(&St::RoundedSm));
    }
}
