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

use crate::tooltip::Tooltip;
use rwire::attr_tokens::{At, Av};
use rwire::style_tokens::St;
use rwire::{el, icons, reflect_action, El, ElementBuilder, Ev, HandlerSpec, Icon, ItemRef};
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
    /// Extra small: 24px height — dense chrome (toolbars, inline forms, list rows)
    Xs,
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
    /// When set, `on_click` skips action reflection and binds a plain click. Inverted so the
    /// derived `Default` (false) keeps reflection **on** — the intended default.
    reflect_off: bool,
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
            ButtonSize::Xs => 12,
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
            ButtonSize::Xs | ButtonSize::Sm => {
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
            // The label is hidden by wrapping it in an opacity:0 span at build time (keeps its
            // box, so the footprint never changes), leaving the button's own colour live — so the
            // ::after ring can tint itself from the foreground via currentColor.
            tokens.push(St::PositionRelative);
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
                // inset:0 + margin:auto centres the fixed-size ring on both axes for any button
                // width; an insetless absolute ::after drifts to where the label used to sit.
                St::Inset0,
                St::MAuto,
                St::W1rem,
                St::H1rem,
                // 2px ring tinted from the button's own foreground (currentColor stays live
                // because the label is hidden with font-size:0, not colour:transparent). The ring
                // colour + spin gap are one bundled token so the gap is cascade-order-proof.
                St::Border2,
                St::SpinnerRingCurrent,
                St::RoundedFull,
                St::AnimateSpinFast,
            ]);
        }

        builder
    }

    /// Compute size-specific style tokens.
    fn size_tokens(&self) -> Vec<St> {
        match self.size {
            ButtonSize::Xs => vec![St::H1_5rem, St::Py0, St::PxSm],
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
            if self.loading {
                // Keep the label's box (constant footprint) but invisible, so the centred spinner
                // reads over it and the button doesn't shrink to the ring.
                builder = builder.append([el(El::Span).st([St::Opacity0]).text(&t)]);
            } else {
                builder = builder.text(&t);
            }
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

    /// Opt this button out of (or back into) automatic action reflection.
    ///
    /// Reflection is **on by default**: a button whose `on_click` handler defers shows a spinner
    /// while the work runs and surfaces its error, with no extra code. Pass `false` for the rare
    /// button that must never reflect (e.g. one already inside a hand-rolled reflecting region).
    pub fn reflect(mut self, enabled: bool) -> Self {
        self.reflect_off = !enabled;
        self
    }

    /// Build with a click handler.
    ///
    /// By default the button is **action-aware**: it reflects the lifecycle of the action its
    /// handler drives (see [`rwire::actions`]). While the action is `Loading` it shows an in-place
    /// spinner (constant footprint); if it fails it turns destructive and carries the message on a
    /// hover tooltip, staying clickable as retry; otherwise it renders normally. A handler that
    /// never defers has no action entry, so the button simply behaves as a plain button — the
    /// reflection is free and complete. Opt out with [`Button::reflect(false)`](Self::reflect).
    pub fn on_click(self, handler: HandlerSpec) -> ElementBuilder {
        if self.reflect_off {
            return self.build().on(Ev::Click, handler);
        }
        let handler_id = handler.handler_id;
        let param = handler.param_bytes.clone().unwrap_or_default();
        let (idle_btn, loading_btn, err_btn) = (self.clone(), self.clone(), self);
        let (idle_handler, err_handler) = (handler.clone(), handler);
        reflect_action(
            handler_id,
            param,
            move || idle_btn.clone().build().on(Ev::Click, idle_handler.clone()),
            move || loading_btn.clone().loading(true).disabled(true).build(),
            move |message| {
                Tooltip::new(message.to_string())
                    .child(
                        err_btn
                            .clone()
                            .intent(ButtonIntent::Destructive)
                            .build()
                            .on(Ev::Click, err_handler.clone()),
                    )
                    .build()
            },
        )
    }

    /// Build with a click handler scoped to a list item — the action-aware equivalent of
    /// [`ElementBuilder::on_ref`](rwire::ElementBuilder::on_ref).
    ///
    /// Encodes `item_ref` into the handler's param bytes, so each item's button reflects its own
    /// action independently: clicking one row's button spins only that row. Otherwise identical to
    /// [`Button::on_click`](Self::on_click).
    pub fn on_click_ref<T: 'static>(
        self,
        handler: HandlerSpec,
        item_ref: ItemRef<T>,
    ) -> ElementBuilder {
        let mut param_bytes = Vec::new();
        item_ref.encode(&mut param_bytes);
        self.on_click(handler.with_param_bytes(param_bytes))
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
        // The button keeps its own colour live (label hidden via an opacity:0 span at build time)
        // so the ring can read the foreground through currentColor.
        assert!(!tokens.contains(&St::TextTransparent));
        let built = btn.build();
        let groups = built.get_pseudo_groups();
        // Should have after group for spinner
        let after = groups
            .iter()
            .find(|(pc, _)| *pc == 0x08) // Pc::After
            .expect("loading button renders an ::after spinner");
        // The ring tints from currentColor (bundled with its spin gap) and is centred via
        // inset:0 + margin:auto so it renders and sits dead-centre on any button width.
        assert!(after.1.contains(&(St::SpinnerRingCurrent as u16)));
        assert!(after.1.contains(&(St::Inset0 as u16)));
        assert!(after.1.contains(&(St::MAuto as u16)));
    }

    #[test]
    fn test_icon_px_scales_with_size() {
        assert_eq!(Button::new().size(ButtonSize::Sm).icon_px(), 12);
        assert_eq!(Button::new().size(ButtonSize::Md).icon_px(), 14);
        assert_eq!(Button::new().size(ButtonSize::Lg).icon_px(), 16);
    }

    #[test]
    fn test_on_click_ref_encodes_param_and_reflects() {
        use rwire::ChangeSet;

        #[derive(rwire::State, Default)]
        #[storage(memory)]
        struct S;
        fn noop(_s: &mut S) {}
        let spec = || HandlerSpec::from_fn_with_changes::<S>(noop, ChangeSet::all());

        let item = ItemRef::<S>::new(3);
        let mut expected = Vec::new();
        item.encode(&mut expected);

        // reflect(false): the click handler carries the encoded item_ref as its param bytes.
        let plain = Button::primary("Run")
            .reflect(false)
            .on_click_ref(spec(), item);
        let events = plain.events();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].1.param_bytes.as_deref(),
            Some(expected.as_slice())
        );

        // Default: wrapped in a display:contents reflection region keyed to that param.
        let reflected = Button::primary("Run").on_click_ref(spec(), ItemRef::<S>::new(3));
        assert!(reflected
            .get_style_utils()
            .contains(&(St::DisplayContents as u16)));
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
        // The spinner overlay replaces the icon, so it isn't rendered while loading. The label is
        // kept — in an invisible span, not textContent — so the button's footprint never shrinks
        // to the ring and the ring can read the live foreground via currentColor.
        let btn = Button::primary("Save")
            .icon(Icon::Check)
            .loading(true)
            .build();
        assert_eq!(btn.text_content(), None);
        let kids = btn.children();
        assert_eq!(kids.len(), 1); // just the hidden label span — no icon
        assert_eq!(kids[0].el_type(), El::Span);
        assert_eq!(kids[0].text_content(), Some("Save"));
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
