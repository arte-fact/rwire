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
use rwire::{el, icons, El, ElementBuilder, Ev, HandlerSpec, Icon};
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
    /// Bordered, muted text that lifts on hover — between Secondary and Ghost.
    Outline,
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
    icon: Option<Icon>,
    text: Option<Cow<'static, str>>,
    aria_label: Option<Cow<'static, str>>,
    disabled: bool,
    loading: bool,
    full_width: bool,
    extra_class: Option<Cow<'static, str>>,
}

#[rwire::component]
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

    /// Outline button with text.
    pub fn outline(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(ButtonIntent::Outline).text(text)
    }

    /// Destructive button with text.
    pub fn destructive(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(ButtonIntent::Destructive).text(text)
    }

    /// Icon-only button. Renders a square button with no visible text, so it
    /// requires an accessible label (applied as `aria-label`).
    pub fn icon_only(icon: Icon, label: impl Into<Cow<'static, str>>) -> Self {
        Self::new().icon(icon).aria_label(label)
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

    /// Set a leading icon, rendered before the text and sized to the button.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set an accessible label (`aria-label`). Recommended for icon-only buttons,
    /// which have no visible text for assistive technology to announce.
    pub fn aria_label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    // ========================================================================
    // Token computation
    // ========================================================================

    /// Icon edge length (px), scaled to the button size.
    fn icon_px(&self) -> u32 {
        match self.size {
            ButtonSize::Sm => 12,
            ButtonSize::Md => 14,
            ButtonSize::Lg => 16,
        }
    }

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
            // Inherit the page font rather than the UA button default.
            St::FontInheritAll,
        ];

        match self.intent {
            ButtonIntent::Primary => tokens.extend([St::BgPrimary, St::TextOnPrimary]),
            ButtonIntent::Secondary => {
                tokens.extend([St::BgSecondary, St::TextOnSecondary, St::BorderDefault])
            }
            ButtonIntent::Ghost => tokens.extend([St::BgTransparent, St::TextHigh]),
            ButtonIntent::Outline => {
                tokens.extend([St::BgTransparent, St::TextMuted, St::BorderDefault])
            }
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
            ButtonIntent::Secondary => builder.hover([St::BgSecondaryHover, St::BorderEmphasis]),
            ButtonIntent::Ghost => builder.hover([St::BgHover]),
            ButtonIntent::Outline => builder.hover([St::TextHigh, St::BorderEmphasis]),
            ButtonIntent::Destructive => builder.hover([St::BgDestructiveHover]),
        };

        if self.disabled {
            builder = builder.disabled_style([
                St::Opacity50,
                St::CursorNotAllowed,
                St::PointerEventsNone,
            ]);
        }
        if self.loading {
            builder = builder.after([
                St::ContentEmpty,
                St::PositionAbsolute,
                St::W1rem,
                St::H1rem,
                St::Border2,
                St::BorderRTransparent,
                St::RoundedFull,
                St::AnimateSpinFast,
            ]);
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
    pub fn build(mut self) -> ElementBuilder {
        let icon_only = self.icon.is_some() && self.text.is_none();
        // The spinner overlay replaces the icon while loading, so don't render both.
        let show_icon = self.icon.is_some() && !self.loading;
        let icon_px = self.icon_px();

        let mut tokens = self.compute_tokens();
        tokens.extend(self.size_tokens());
        if icon_only {
            // Square it: drop the horizontal padding and let aspect-ratio match
            // the width to the height, centering the glyph.
            tokens.retain(|t| !matches!(t, St::PxSp3 | St::PxMd | St::PxLg));
            tokens.push(St::AspectSquare);
        }
        let mut builder = self.apply_pseudo(el(El::Button).st(tokens));

        // Render the icon and text as element children so the appended icon isn't
        // clobbered by `textContent` (and stays *before* the label). Text-only
        // buttons set `textContent` directly — fewer bytes, no wrapper span.
        let icon = self.icon.take();
        let text = self.text.take();
        if show_icon {
            let icon_el = icons::icon_sized(icon.unwrap(), icon_px);
            builder = match text {
                Some(t) => builder.append([icon_el, el(El::Span).text(&t)]),
                None => builder.append([icon_el]),
            };
        } else if let Some(t) = text {
            builder = builder.text(&t);
        }

        if let Some(label) = self.aria_label {
            builder = builder.at_str(At::AriaLabel, &label);
        }
        if self.disabled {
            builder = builder.bool_attr(At::Disabled);
        }
        if self.loading {
            builder = builder.at(At::AriaBusy, Av::True);
        }
        if let Some(extra) = self.extra_class {
            builder = builder.class(&extra);
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
    fn test_icon_px_scales_with_size() {
        assert_eq!(Button::new().size(ButtonSize::Sm).icon_px(), 12);
        assert_eq!(Button::new().size(ButtonSize::Md).icon_px(), 14);
        assert_eq!(Button::new().size(ButtonSize::Lg).icon_px(), 16);
    }

    #[test]
    fn test_icon_with_text_renders_both_as_children() {
        // The icon must not be clobbered by textContent: icon + label become an
        // <svg> and a <span> child, in that order, with no button-level text.
        let btn = Button::primary("Save").icon(Icon::Check).build();
        assert_eq!(btn.text_content(), None);
        let kids = btn.children();
        assert_eq!(kids.len(), 2);
        assert_eq!(kids[0].el_type(), El::Svg);
        assert_eq!(kids[1].el_type(), El::Span);
        assert_eq!(kids[1].text_content(), Some("Save"));
    }

    #[test]
    fn test_text_only_uses_text_content() {
        // No icon → plain textContent, no wrapper span (fewer bytes).
        let btn = Button::primary("Save").build();
        assert!(btn.children().is_empty());
        assert_eq!(btn.text_content(), Some("Save"));
    }

    #[test]
    fn test_icon_only_is_square_with_label() {
        let btn = Button::icon_only(Icon::ChevronDown, "Expand").build();
        // One icon child, no text, square aspect.
        assert_eq!(btn.children().len(), 1);
        assert_eq!(btn.text_content(), None);
        assert!(btn.get_style_utils().contains(&(St::AspectSquare as u16)));
    }

    #[test]
    fn test_loading_hides_icon() {
        // The spinner overlay replaces the icon, so it isn't rendered while loading.
        let btn = Button::primary("Save")
            .icon(Icon::Check)
            .loading(true)
            .build();
        assert!(btn.children().is_empty());
        assert_eq!(btn.text_content(), Some("Save"));
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
