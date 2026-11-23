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
//!     .title("Read-only preview")
//!     .build()
//! ```
//!
//! # Attributes go on the `Button`, not on what `on_click` returns
//!
//! `on_click` / `submit` wrap the button in an action-reflection region that has
//! no box (`display:contents`). Set `title`, `data`, `aria_label`, `class`,
//! `hover` or any other attribute (`attr`) on the `Button` **before** it: chained
//! after, they would land on the wrapper and never show. Debug builds panic at
//! such a call.

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
    /// Danger text on a neutral hairline, the border turning danger on hover
    /// (an "Interrupt" button in a top bar).
    DangerGhost,
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
    title: Option<Cow<'static, str>>,
    /// Other attributes for the button element (see [`Button::attr`]).
    attrs: Vec<(String, String)>,
    disabled: bool,
    loading: bool,
    full_width: bool,
    extra_class: Option<Cow<'static, str>>,
    /// Extra hover style tokens applied to the built button, composing with the intent's own
    /// hover treatment (e.g. a destructive text tint on a ghost icon button). Kept on the
    /// builder — an `ElementBuilder::hover` chained after `on_click` would land on the
    /// action-reflection wrapper, which is `display:contents` and has no box to hover.
    hover_extra: Vec<St>,
    /// `data-*` attributes applied to the built button — the payload the click handler reads
    /// via `ctx.data(...)`. Kept on the builder for the same reason as `hover_extra`: chained
    /// after `on_click` they'd land on the reflection wrapper, which the client's click payload
    /// collection never reads (it looks at the bound element, walking up only for `data-id`).
    data: Vec<(String, String)>,
    /// When set, `on_click` skips action reflection and binds a plain click. Inverted so the
    /// derived `Default` (false) keeps reflection **on** — the intended default.
    reflect_off: bool,
    /// Below this breakpoint the text label becomes screen-reader-only (the
    /// icon stays visible alone).
    label_hidden_below: Option<rwire::style_tokens::Bp>,
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

    /// Below `bp`, the text label is only read by screen readers (sr-only
    /// class, not `display:none`); the icon stays visible alone.
    pub fn label_hidden_below(mut self, bp: rwire::style_tokens::Bp) -> Self {
        self.label_hidden_below = Some(bp);
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

    /// Add hover style tokens on top of the intent's own hover treatment — e.g. a destructive
    /// text tint on a ghost icon button. Set here (not via `ElementBuilder::hover` chained
    /// after `on_click`), so the tokens land on the button element rather than its action
    /// reflection wrapper, which is `display:contents` and has no box to hover.
    pub fn hover(mut self, tokens: impl IntoIterator<Item = St>) -> Self {
        self.hover_extra.extend(tokens);
        self
    }

    /// Set a `data-*` attribute on the built button — the payload its click handler reads via
    /// `ctx.data(key)`. Set here (not via `ElementBuilder::data` chained after `on_click`), so
    /// the attribute lands on the button element rather than its action-reflection wrapper,
    /// which the client's click payload collection never reads.
    pub fn data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.data.push((key.into(), value.into()));
        self
    }

    /// Set an accessible label (`aria-label`). Recommended for icon-only buttons,
    /// which have no visible text for assistive technology to announce.
    pub fn aria_label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// The native tooltip (`title`). It shows on a disabled button too, so it
    /// is the place to say why the button is disabled.
    pub fn title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Any other attribute, set on the button element itself (unlike an
    /// `ElementBuilder::attr` chained after `on_click`, which would land on
    /// the action-reflection wrapper).
    pub fn attr(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attrs.push((key.into(), value.into()));
        self
    }

    // ========================================================================
    // Token computation
    // ========================================================================

    /// An icon and no text.
    fn is_icon_only(&self) -> bool {
        self.icon.is_some() && self.text.is_none()
    }

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
        ];

        match self.intent {
            ButtonIntent::Primary => tokens.extend([St::BgPrimary, St::TextOnPrimary]),
            ButtonIntent::Secondary => {
                tokens.extend([St::BgSecondary, St::TextOnSecondary, St::BorderDefault])
            }
            // An icon-only ghost rests on nothing, in the soft ink (a toolbar's icons).
            ButtonIntent::Ghost if self.is_icon_only() => {
                tokens.extend([St::BgTransparent, St::TextMuted, St::BorderGhost])
            }
            ButtonIntent::Ghost => tokens.extend([St::BgTransparent, St::TextHigh]),
            ButtonIntent::Outline => {
                tokens.extend([St::BgTransparent, St::TextMuted, St::BorderDefault])
            }
            ButtonIntent::Destructive => tokens.extend([St::BgDestructive, St::TextOnDestructive]),
            // Danger text on a neutral hairline; the border turns danger on hover.
            ButtonIntent::DangerGhost => {
                tokens.extend([St::BgTransparent, St::TextError, St::BorderDefault])
            }
        }

        match self.size {
            ButtonSize::Xs | ButtonSize::Sm => {
                tokens.retain(|t| !matches!(t, St::GapSm | St::TextSm));
                tokens.extend([St::TextXs, St::GapXs]);
            }
            // The theme's density sets the default size's text (`--Qzt`).
            ButtonSize::Md => {
                tokens.retain(|t| *t != St::TextSm);
                tokens.push(St::TextControl);
            }
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

        // A disabled button keeps its pointer events, so its tooltip shows (the
        // `disabled` attribute already blocks the click); it gets no hover
        // treatment instead.
        builder = if self.disabled {
            builder.disabled_style([St::Opacity50, St::CursorNotAllowed])
        } else {
            match self.intent {
                ButtonIntent::Primary => builder.hover([St::BgPrimaryHover, St::GlowTheme]),
                ButtonIntent::Secondary => {
                    builder.hover([St::BgSecondaryHover, St::BorderEmphasis])
                }
                // …and on hover takes the raised ground, the hairline and the ink.
                ButtonIntent::Ghost if self.is_icon_only() => {
                    builder.hover([St::BgSurfaceRaised, St::BorderDefault, St::TextDefault])
                }
                ButtonIntent::Ghost => builder.hover([St::BgHover]),
                ButtonIntent::Outline => builder.hover([St::TextHigh, St::BorderEmphasis]),
                ButtonIntent::Destructive => builder.hover([St::BgDestructiveHover]),
                ButtonIntent::DangerGhost => builder.hover([St::BorderError]),
            }
        };

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
            ButtonSize::Md => vec![St::HControl, St::Py0, St::PxMd],
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
            // At the default size the theme's density sizes the square (`--Qzi`).
            if self.size == ButtonSize::Md {
                tokens.retain(|t| *t != St::HControl);
                tokens.push(St::SqIconButton);
            }
        }
        let mut builder = self.apply_pseudo(el(El::Button).st(tokens));
        if !self.hover_extra.is_empty() && !self.disabled {
            builder = builder.hover(self.hover_extra.clone());
        }
        for (key, value) in &self.data {
            builder = builder.data(key, value);
        }

        // Render the icon and text as element children so the appended icon isn't
        // clobbered by `textContent` (and stays *before* the label). Text-only
        // buttons set `textContent` directly — fewer bytes, no wrapper span.
        let icon = self.icon.take();
        let text = self.text.take();
        if show_icon {
            let mut icon_el = icons::icon_sized(icon.unwrap(), icon_px);
            if icon_only && self.size == ButtonSize::Md {
                // …and its glyph (`--Qzg`), 14 px otherwise.
                icon_el = icon_el.st([St::GlyphIconButton]);
            }
            builder = match text {
                Some(t) => {
                    let label = el(El::Span).text(&t);
                    let label = match self.label_hidden_below {
                        Some(bp) => label.st([St::sr_only_below(bp)]),
                        None => label,
                    };
                    builder.append([icon_el, label])
                }
                None => builder.append([icon_el]),
            };
        } else if let Some(t) = text {
            if self.loading {
                // Keep the label's box (constant footprint) but invisible, so the centred spinner
                // reads over it and the button doesn't shrink to the ring.
                builder = builder.append([el(El::Span).st([St::Opacity0]).text(&t)]);
            } else if let Some(bp) = self.label_hidden_below {
                // Label sr-only below `bp` (the icon stays visible alone).
                builder = builder.append([el(El::Span).st([St::sr_only_below(bp)]).text(&t)]);
            } else {
                builder = builder.text(&t);
            }
        }

        if let Some(label) = self.aria_label {
            builder = builder.at_str(At::AriaLabel, &label);
        }
        if let Some(title) = self.title {
            builder = builder.at_str(At::Title, &title);
        }
        for (key, value) in &self.attrs {
            builder = builder.attr(key, value);
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

    /// Build as the submit button of the `<form id=form_id>` it lives outside of.
    pub fn submits(self, form_id: &str) -> ElementBuilder {
        self.build().at_str(At::Form, form_id)
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

    /// Build as a form-submit button (`type="submit"`) that reflects the **form's** submit action.
    ///
    /// A submit button owns no event — the surrounding `<form>` does — so pass the same
    /// `HandlerSpec` the form binds (`el(El::Form).on(Ev::Submit, save())` + `.submit(&save())`).
    /// The button then mirrors that action's lifecycle exactly like [`Button::on_click`]: an
    /// in-place spinner while `Loading`, destructive with the message on a hover tooltip when
    /// `Errored`. No event is bound to the button itself; clicking it submits the form natively,
    /// which is also the retry.
    pub fn submit(self, action: &HandlerSpec) -> ElementBuilder {
        if self.reflect_off {
            return self.build().at(At::Type, Av::Submit);
        }
        let handler_id = action.handler_id;
        let param = action.param_bytes.clone().unwrap_or_default();
        let (idle_btn, loading_btn, err_btn) = (self.clone(), self.clone(), self);
        reflect_action(
            handler_id,
            param,
            move || idle_btn.clone().build().at(At::Type, Av::Submit),
            move || {
                loading_btn
                    .clone()
                    .loading(true)
                    .disabled(true)
                    .build()
                    .at(At::Type, Av::Submit)
            },
            move |message| {
                Tooltip::new(message.to_string())
                    .child(
                        err_btn
                            .clone()
                            .intent(ButtonIntent::Destructive)
                            .build()
                            .at(At::Type, Av::Submit),
                    )
                    .build()
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn danger_ghost_is_danger_text_on_a_neutral_hairline_turning_danger_on_hover() {
        let btn = Button::new()
            .intent(ButtonIntent::DangerGhost)
            .text("Interrompre");
        let tokens = btn.compute_tokens();
        assert!(tokens.contains(&St::BgTransparent), "{tokens:?}");
        assert!(tokens.contains(&St::TextError), "{tokens:?}");
        assert!(tokens.contains(&St::BorderDefault), "{tokens:?}");
        let built = btn.build();
        let html = built.to_static_html();
        assert!(!html.contains("style="), "{html}");
        // The hover treatment turns the border danger.
        let hover: Vec<u16> = built
            .get_pseudo_groups()
            .iter()
            .filter(|(pc, _)| *pc == rwire::style_tokens::Pc::Hover.as_u8())
            .flat_map(|(_, c)| c.iter().copied())
            .collect();
        assert!(hover.contains(&St::BorderError.as_u16()), "{hover:?}");
    }

    #[test]
    fn label_hidden_below_makes_the_label_sr_only_below_the_breakpoint() {
        let btn = Button::new()
            .icon(Icon::Stop)
            .text("Interrompre")
            .label_hidden_below(rwire::style_tokens::Bp::Sm)
            .build();
        let html = btn.to_static_html();
        assert!(
            html.contains("Interrompre"),
            "the label stays in the DOM: {html}"
        );
        // The label span carries the sr-only-below-sm token (not display:none).
        let label = btn
            .children()
            .iter()
            .find(|c| c.text_content() == Some("Interrompre"))
            .expect("label span");
        assert!(label
            .get_style_utils()
            .contains(&St::sr_only_below(rwire::style_tokens::Bp::Sm).as_u16()));
        assert!(
            !html.contains("display:none"),
            "sr-only, not display:none: {html}"
        );
    }

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
    fn a_disabled_button_keeps_its_tooltip_and_loses_its_hover() {
        let btn = Button::primary("Merge")
            .disabled(true)
            .title("Read-only preview")
            .build();
        let html = btn.to_static_html();
        assert!(html.contains("title=\"Read-only preview\""), "{html}");
        assert!(html.contains("disabled"), "{html}");
        let groups = btn.get_pseudo_groups();
        let disabled = groups.iter().find(|(pc, _)| *pc == 0x04).unwrap();
        // The tooltip needs pointer events: no pointer-events:none any more.
        assert!(!disabled.1.contains(&St::PointerEventsNone.as_u16()));
        assert!(disabled.1.contains(&St::CursorNotAllowed.as_u16()));
        assert!(
            !groups.iter().any(|(pc, _)| *pc == 0x00),
            "no hover look on a disabled button"
        );
        // An enabled button still hovers.
        let live = Button::primary("Merge").build();
        assert!(live.get_pseudo_groups().iter().any(|(pc, _)| *pc == 0x00));
    }

    #[test]
    fn title_and_attrs_reach_the_button_through_on_click() {
        #[derive(Default)]
        struct S;
        impl rwire::MemoryState for S {}
        fn noop(_: &mut S) {}
        let spec = HandlerSpec::memory::<S>(noop);
        let html = Button::primary("Merge")
            .title("Merge the branch")
            .attr("data-testid", "merge")
            .on_click(spec)
            .to_static_html();
        // The reflection wrapper renders the idle button inside it.
        let button = &html[html.find("<button").unwrap()..];
        assert!(button.contains("title=\"Merge the branch\""), "{html}");
        assert!(button.contains("data-testid=\"merge\""), "{html}");
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "which has no box")]
    fn an_attribute_chained_after_on_click_fails_loudly() {
        #[derive(Default)]
        struct S;
        impl rwire::MemoryState for S {}
        fn noop(_: &mut S) {}
        let _ = Button::primary("Merge")
            .on_click(HandlerSpec::memory::<S>(noop))
            .attr("title", "lost on the wrapper");
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
    fn test_submit_reflects_the_forms_action_without_binding_events() {
        use rwire::ChangeSet;

        #[derive(rwire::State, Default)]
        #[storage(memory)]
        struct S;
        fn noop(_s: &mut S) {}
        let spec = || HandlerSpec::from_fn_with_changes::<S>(noop, ChangeSet::all());

        // reflect(false): a plain type=submit button, no events (the form owns the submit).
        let plain = Button::outline("Save").reflect(false).submit(&spec());
        assert!(plain.events().is_empty());
        assert!(plain.to_static_html().contains("type=\"submit\""));

        // Default: wrapped in a display:contents reflection region keyed to the form's action.
        let reflected = Button::outline("Save").submit(&spec());
        assert!(reflected
            .get_style_utils()
            .contains(&(St::DisplayContents as u16)));
    }

    #[test]
    fn test_data_and_hover_land_on_the_button_not_the_wrapper() {
        use rwire::ChangeSet;

        #[derive(rwire::State, Default)]
        #[storage(memory)]
        struct S;
        fn noop(_s: &mut S) {}
        let spec = HandlerSpec::from_fn_with_changes::<S>(noop, ChangeSet::all());

        // The payload must sit on the <button> element itself: the client's click payload
        // collection reads the bound element's dataset, never the reflection wrapper's.
        let plain = Button::secondary("Edit")
            .data("name", "k1")
            .hover([St::TextError])
            .reflect(false)
            .on_click(spec);
        assert_eq!(
            plain.attributes().iter().find(|(k, _)| k == "data-name"),
            Some(&("data-name".to_string(), "k1".to_string()))
        );
        assert!(plain
            .get_pseudo_groups()
            .iter()
            .any(|(_, tokens)| tokens.contains(&(St::TextError as u16))));
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
    fn a_ghost_icon_button_rests_on_nothing_and_raises_on_hover() {
        let btn = Button::icon_only(Icon::Settings, "Settings")
            .intent(ButtonIntent::Ghost)
            .build();
        let rest = btn.get_style_utils();
        for st in [St::BgTransparent, St::TextMuted, St::BorderGhost] {
            assert!(rest.contains(&st.as_u16()), "{st:?} at rest");
        }
        assert!(!rest.contains(&St::TextHigh.as_u16()));
        let hover: Vec<u16> = btn
            .get_pseudo_groups()
            .iter()
            .filter(|(pc, _)| *pc == 0x00)
            .flat_map(|(_, c)| c.iter().copied())
            .collect();
        for st in [St::BgSurfaceRaised, St::BorderDefault, St::TextDefault] {
            assert!(hover.contains(&st.as_u16()), "{st:?} on hover");
        }
        // A ghost button with text is unchanged.
        let text = Button::ghost("Cancel").build();
        assert!(text.get_style_utils().contains(&St::TextHigh.as_u16()));
        assert!(!text.get_style_utils().contains(&St::BorderGhost.as_u16()));
        let hover: Vec<u16> = text
            .get_pseudo_groups()
            .iter()
            .flat_map(|(_, c)| c.iter().copied())
            .collect();
        assert!(hover.contains(&St::BgHover.as_u16()));
        assert!(!hover.contains(&St::BgSurfaceRaised.as_u16()));
    }

    #[test]
    fn an_icon_button_follows_the_density_hooks_at_the_default_size() {
        let btn = Button::icon_only(Icon::Settings, "Settings").build();
        let utils = btn.get_style_utils();
        assert!(utils.contains(&St::SqIconButton.as_u16()));
        assert!(
            !utils.contains(&St::HControl.as_u16()),
            "the square sets the height"
        );
        let glyph = btn.children()[0].get_style_utils();
        assert!(glyph.contains(&St::GlyphIconButton.as_u16()));
        // Other sizes, and buttons with text, keep their own dimensions.
        let sm = Button::icon_only(Icon::Settings, "Settings")
            .size(ButtonSize::Sm)
            .build();
        assert!(!sm.get_style_utils().contains(&St::SqIconButton.as_u16()));
        let text = Button::primary("Save").icon(Icon::Check).build();
        assert!(!text.get_style_utils().contains(&St::SqIconButton.as_u16()));
        assert!(!text.children()[0]
            .get_style_utils()
            .contains(&St::GlyphIconButton.as_u16()));
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
        assert_eq!(tokens, vec![St::HControl, St::Py0, St::PxMd]);

        let btn = Button::new().size(ButtonSize::Lg);
        let tokens = btn.size_tokens();
        assert_eq!(tokens, vec![St::H2_75rem, St::Py0, St::PxLg]);
    }
}
