//! Button component with variants.
//!
//! # Variants
//!
//! - **Intent**: Primary (default), Secondary, Ghost, Destructive
//! - **Size**: Sm, Md (default), Lg
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Button, ButtonIntent, ButtonSize};
//!
//! // Convenience constructors
//! Button::primary("Save").build()
//! Button::secondary("Cancel").build()
//! Button::destructive("Delete").build()
//!
//! // Full configuration
//! Button::new()
//!     .intent(ButtonIntent::Ghost)
//!     .size(ButtonSize::Sm)
//!     .text("More options")
//!     .disabled(true)
//!     .build()
//! ```

use rwire::attr_tokens::{At, Av};
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder, Ev, HandlerSpec};
use std::borrow::Cow;

/// Button visual intent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonIntent {
    /// Primary action (solid accent color)
    #[default]
    Primary,
    /// Secondary action (subtle background, border)
    Secondary,
    /// Ghost button (transparent, text only)
    Ghost,
    /// Destructive action (red)
    Destructive,
}

/// Button size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    /// Small: 28px height
    Sm,
    /// Medium: 36px height (default)
    #[default]
    Md,
    /// Large: 44px height
    Lg,
}

/// Button component builder.
#[derive(Clone, Debug, Default)]
pub struct Button {
    intent: ButtonIntent,
    size: ButtonSize,
    text: Option<Cow<'static, str>>,
    disabled: bool,
    loading: bool,
    full_width: bool,
    extra_class: Option<Cow<'static, str>>,
}

impl Button {
    /// Create a new button with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    // ========================================================================
    // Convenience constructors
    // ========================================================================

    /// Primary button with text.
    pub fn primary(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().text(text)
    }

    /// Secondary button with text.
    pub fn secondary(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(ButtonIntent::Secondary).text(text)
    }

    /// Ghost button with text.
    pub fn ghost(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(ButtonIntent::Ghost).text(text)
    }

    /// Destructive button with text.
    pub fn destructive(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(ButtonIntent::Destructive).text(text)
    }

    // ========================================================================
    // Fluent setters
    // ========================================================================

    /// Set the button intent.
    pub fn intent(mut self, intent: ButtonIntent) -> Self {
        self.intent = intent;
        self
    }

    /// Set the button size.
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Set the button text.
    pub fn text(mut self, text: impl Into<Cow<'static, str>>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Set disabled state.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set loading state.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Set full width.
    pub fn full_width(mut self, full: bool) -> Self {
        self.full_width = full;
        self
    }

    /// Add custom class (escape hatch).
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    // ========================================================================
    // Token computation
    // ========================================================================

    /// Compute style tokens for this button configuration.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![
            St::DisplayInlineFlex,
            St::ItemsCenter,
            St::JustifyCenter,
            St::GapSm,
            St::FontMedium,
            St::RoundedMd,
            St::BorderTransparent,
            St::CursorPointer,
            St::TransTheme,
            St::TextSm,
        ];

        match self.intent {
            ButtonIntent::Primary => tokens.extend([St::BgPrimary, St::TextOnPrimary]),
            ButtonIntent::Secondary => {
                tokens.extend([St::BgSecondary, St::TextOnSecondary, St::BorderDefault])
            }
            ButtonIntent::Ghost => tokens.extend([St::BgTransparent, St::TextHigh]),
            ButtonIntent::Destructive => tokens.extend([St::BgDestructive, St::TextOnDestructive]),
        }

        match self.size {
            ButtonSize::Sm => {
                tokens.retain(|t| !matches!(t, St::GapSm | St::TextSm));
                tokens.extend([St::TextXs, St::GapXs]);
            }
            ButtonSize::Md => {} // defaults
            ButtonSize::Lg => {
                tokens.retain(|t| !matches!(t, St::GapSm | St::TextSm));
                tokens.extend([St::TextBase, St::GapMd]);
            }
        }

        if self.loading {
            tokens.extend([St::PositionRelative, St::TextTransparent]);
        }
        if self.full_width {
            tokens.push(St::WFull);
        }

        tokens
    }

    /// Apply pseudo-class styles to the builder based on button configuration.
    fn apply_pseudo(&self, mut builder: ElementBuilder) -> ElementBuilder {
        builder = builder.focus_visible([St::RingTheme]);

        builder = match self.intent {
            ButtonIntent::Primary => builder.hover([St::BgPrimaryHover, St::GlowTheme]),
            ButtonIntent::Secondary => {
                builder.hover([St::BgSecondaryHover, St::BorderEmphasis])
            }
            ButtonIntent::Ghost => builder.hover([St::BgHover]),
            ButtonIntent::Destructive => builder.hover([St::BgDestructiveHover]),
        };

        if self.disabled {
            builder = builder.disabled_style([St::Opacity50, St::CursorNotAllowed, St::PointerEventsNone]);
        }
        if self.loading {
            builder = builder.after([St::ContentEmpty, St::PositionAbsolute, St::W1rem, St::H1rem, St::Border2, St::BorderRTransparent, St::RoundedFull, St::AnimateSpinFast]);
        }

        builder
    }

    /// Compute size-specific style tokens.
    fn size_tokens(&self) -> Vec<St> {
        match self.size {
            ButtonSize::Sm => vec![St::H1_75rem, St::Py0, St::PxSp3],
            ButtonSize::Md => vec![St::H2_25rem, St::Py0, St::PxMd],
            ButtonSize::Lg => vec![St::H2_75rem, St::Py0, St::PxLg],
        }
    }

    // ========================================================================
    // Build
    // ========================================================================

    /// Build the button into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut tokens = self.compute_tokens();
        tokens.extend(self.size_tokens());
        let mut builder = self.apply_pseudo(
            el(El::Button).st(tokens)
        );

        if let Some(text) = self.text {
            builder = builder.text(&text);
        }

        if self.disabled {
            builder = builder.bool_attr(At::Disabled);
        }

        if self.loading {
            builder = builder.at(At::AriaBusy, Av::True);
        }

        if let Some(ref extra) = self.extra_class {
            builder = builder.class(extra.as_ref());
        }

        builder
    }

    /// Build with click handler.
    pub fn on_click(self, handler: HandlerSpec) -> ElementBuilder {
        self.build().on(Ev::Click, handler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_defaults() {
        let btn = Button::new();
        assert_eq!(btn.intent, ButtonIntent::Primary);
        assert_eq!(btn.size, ButtonSize::Md);
        assert!(!btn.disabled);
    }

    #[test]
    fn test_button_primary_tokens() {
        let btn = Button::primary("Save");
        let tokens = btn.compute_tokens();
        assert!(tokens.contains(&St::BgPrimary));
        assert!(tokens.contains(&St::TextOnPrimary));
        assert!(tokens.contains(&St::DisplayInlineFlex));
    }

    #[test]
    fn test_button_secondary_tokens() {
        let btn = Button::secondary("Cancel");
        let tokens = btn.compute_tokens();
        assert!(tokens.contains(&St::BgSecondary));
        assert!(tokens.contains(&St::TextOnSecondary));
        assert!(tokens.contains(&St::BorderDefault));
    }

    #[test]
    fn test_button_destructive_tokens() {
        let btn = Button::destructive("Delete");
        let tokens = btn.compute_tokens();
        assert!(tokens.contains(&St::BgDestructive));
        assert!(tokens.contains(&St::TextOnDestructive));
    }

    #[test]
    fn test_button_primary_pseudo() {
        let btn = Button::primary("Save").build();
        let groups = btn.get_pseudo_groups();
        assert!(!groups.is_empty());
    }

    #[test]
    fn test_button_disabled_pseudo() {
        let btn = Button::primary("Save").disabled(true).build();
        let groups = btn.get_pseudo_groups();
        // Should have disabled_style group
        assert!(groups.iter().any(|(pc, _)| *pc == 0x04)); // Pc::Disabled
    }

    #[test]
    fn test_button_loading_tokens() {
        let btn = Button::primary("Save").loading(true);
        let tokens = btn.compute_tokens();
        assert!(tokens.contains(&St::PositionRelative));
        assert!(tokens.contains(&St::TextTransparent));
        let built = btn.build();
        let groups = built.get_pseudo_groups();
        // Should have after group for spinner
        assert!(groups.iter().any(|(pc, _)| *pc == 0x08)); // Pc::After
    }

    #[test]
    fn test_button_size_tokens() {
        let btn = Button::new().size(ButtonSize::Sm);
        let tokens = btn.size_tokens();
        assert_eq!(tokens, vec![St::H1_75rem, St::Py0, St::PxSp3]);

        let btn = Button::new().size(ButtonSize::Md);
        let tokens = btn.size_tokens();
        assert_eq!(tokens, vec![St::H2_25rem, St::Py0, St::PxMd]);

        let btn = Button::new().size(ButtonSize::Lg);
        let tokens = btn.size_tokens();
        assert_eq!(tokens, vec![St::H2_75rem, St::Py0, St::PxLg]);
    }

}
