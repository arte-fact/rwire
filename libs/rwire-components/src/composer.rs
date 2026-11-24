//! Composer component.
//!
//! The message-entry bar of a chat surface: an auto-growing textarea where **Enter submits
//! and Shift+Enter inserts a newline** (the runtime's `data-enter-submit` behavior), plus a
//! send button. Pairs with [`ChatScroll`](crate::ChatScroll) as the other half of a chat UI.
//!
//! Two form factors:
//! - the default **pill** — a bordered, shadowed card with the field above an actions row
//!   (an optional hint or tools on the left, Send on the right);
//! - **compact** — **one box**: the field and the send button on a single row, the box
//!   carrying the border and the rounding (no card inside a card, no actions row). The send
//!   button is icon-only there, its label kept as the accessible name; `.send_icon(…)` picks
//!   the glyph and `.send_icon_only(false)` restores the label.
//!
//! An empty field (one showing its placeholder) dims the send button and takes it out
//! of the click path: `[data-send-guard]` plus `:has(:placeholder-shown)`, in pure CSS —
//! no round-trip, no runtime change. A composer with no placeholder cannot be read for
//! emptiness, so the guard needs one.
//!
//! Key the composer by something that changes when a message lands (`.id(...)`) — a fresh
//! element is an empty element, which clears the field without a client opcode.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::Composer;
//!
//! Composer::new()
//!     .id(format!("composer-{}", messages.len()))
//!     .placeholder("Message claw…")
//!     .hint("⏎ send · ⇧⏎ newline")
//!     .on_submit(send_message())
//!     .build()
//! ```
//!
//! A composer that cannot send (a read-only preview) says why up front:
//! `.disabled("Read-only preview: you cannot reply here")` disables the field and the send
//! button, shows the reason in the empty field and as both tooltips, and binds no submit.

use crate::tooltip::Tooltip;
use crate::{Spinner, SpinnerSize};
use rwire::style_tokens::St;
use rwire::{el, icons, reflect_action, At, Av, El, ElementBuilder, Ev, HandlerSpec, Icon, Pc};
use std::borrow::Cow;

/// Composer builder.
#[derive(Debug, Default)]
pub struct Composer {
    id: Option<Cow<'static, str>>,
    name: Option<Cow<'static, str>>,
    placeholder: Option<Cow<'static, str>>,
    hint: Option<Cow<'static, str>>,
    send_label: Option<Cow<'static, str>>,
    on_submit: Option<HandlerSpec>,
    compact: bool,
    max_height: Option<Cow<'static, str>>,
    hidden_fields: Vec<(Cow<'static, str>, Cow<'static, str>)>,
    on_draft: Option<HandlerSpec>,
    /// When set, the send button skips action reflection. Inverted so the derived `Default`
    /// (false) keeps reflection **on** — like [`crate::Button`].
    reflect_off: bool,
    /// Why the composer cannot send, when it cannot (see [`Composer::disabled`]).
    disabled: Option<Cow<'static, str>>,
    /// Glyph on the send button (default `Icon::ArrowUp`).
    send_icon: Option<Icon>,
    /// Icon-only send button. `None` follows the form factor: icon-only in the
    /// compact box, label beside the glyph in the pill.
    icon_only: Option<bool>,
    /// Skip the card's focus highlight. Inverted so the derived `Default`
    /// (false) keeps the highlight **on**, like [`crate::Button`]'s reflection:
    /// the card lights up while the field is focused unless the app says the
    /// surrounding surface already shows it.
    quiet_focus: bool,
}

impl Composer {
    /// Create a composer.
    pub fn new() -> Self {
        Self::default()
    }

    /// The field's element id — key it by something that changes when a message lands, so
    /// the field resets to empty after each send.
    pub fn id(mut self, id: impl Into<Cow<'static, str>>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// The field's form name (default `message`).
    pub fn name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Placeholder text.
    pub fn placeholder(mut self, placeholder: impl Into<Cow<'static, str>>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// A muted hint in the actions row's left slot (e.g. the keyboard convention). Pill
    /// form factor only.
    pub fn hint(mut self, hint: impl Into<Cow<'static, str>>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// The send button's label (default `Send`).
    pub fn send_label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.send_label = Some(label.into());
        self
    }

    /// Glyph on the send button (default `Icon::ArrowUp`). Any icon works —
    /// `ArrowRight` for a classic send, `Check` for a confirm-and-close bar.
    pub fn send_icon(mut self, icon: Icon) -> Self {
        self.send_icon = Some(icon);
        self
    }

    /// Keep the send button to its icon, the label moving to `aria-label` and
    /// `title`. Defaults to the form factor: icon-only in the compact box, label
    /// beside the glyph in the pill.
    pub fn send_icon_only(mut self, on: bool) -> Self {
        self.icon_only = Some(on);
        self
    }

    /// Whether this composer draws an icon-only send button.
    fn sends_icon_only(&self) -> bool {
        self.icon_only.unwrap_or(self.compact)
    }

    /// The form's submit handler — read the message with `ctx.field(name)`.
    pub fn on_submit(mut self, handler: HandlerSpec) -> Self {
        self.on_submit = Some(handler);
        self
    }

    /// Single-row form factor (field beside the send button, no card chrome).
    /// Ride a hidden input along with the submission (e.g. the sender's name
    /// in a chatroom) — form payloads collect every named field.
    pub fn hidden_field(
        mut self,
        name: impl Into<Cow<'static, str>>,
        value: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.hidden_fields.push((name.into(), value.into()));
        self
    }

    /// Fire `handler` (debounced) while the user is composing — the hook for
    /// typing/writing indicators.
    pub fn on_draft(mut self, handler: HandlerSpec) -> Self {
        self.on_draft = Some(handler);
        self
    }

    /// Opt the send button out of automatic action reflection (on by default): when the
    /// `on_submit` handler defers, the send button spins while the work runs and turns
    /// destructive with the message on a tooltip if it fails. See [`crate::Button::reflect`].
    pub fn reflect(mut self, enabled: bool) -> Self {
        self.reflect_off = !enabled;
        self
    }

    /// Disable the whole composer, saying why: the field and the send button are disabled,
    /// the reason shows in the empty field (its placeholder, so it is read before any click)
    /// and as the tooltip of both, and no submit or draft handler is bound.
    pub fn disabled(mut self, reason: impl Into<Cow<'static, str>>) -> Self {
        self.disabled = Some(reason.into());
        self
    }

    /// Light the card up while the field is focused (on by default). Pass
    /// `false` when the field is invisible and the surrounding surface already
    /// says where the focus is: no ring then appears around an empty input.
    pub fn focus_ring(mut self, enabled: bool) -> Self {
        self.quiet_focus = !enabled;
        self
    }

    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    /// Cap the auto-growing field (default `40vh`); it scrolls past the cap.
    pub fn max_height(mut self, max_height: impl Into<Cow<'static, str>>) -> Self {
        self.max_height = Some(max_height.into());
        self
    }

    fn field(&self) -> ElementBuilder {
        let mut field = el(El::Textarea)
            .at_str(At::Name, self.name.as_deref().unwrap_or("message"))
            .at_str(At::Rows, "1")
            .at(At::Autocomplete, Av::Off)
            .data("enter-submit", "1")
            .st([
                St::WFull,
                St::TextDefault,
                // Transparent by default so the field shows its themed parent, never the UA white
                // default. The full mode nests it in a `BgSurface` box; the compact mode borders it
                // directly on the page — without this the compact `<textarea>` renders white.
                St::BgTransparent,
                St::BorderNone,
                St::OutlineNone,
                St::ResizeNone,
                St::OverflowYAuto,
                St::FieldSizingContent,
            ])
            .style(
                rwire::style::Style::new()
                    .set("max-height", self.max_height.as_deref().unwrap_or("40vh")),
            );
        if let Some(ref id) = self.id {
            field = field.id(id.as_ref());
        }
        if let Some(reason) = &self.disabled {
            return field
                .bool_attr(At::Disabled)
                .at_str(At::Placeholder, reason.as_ref())
                .at_str(At::Title, reason.as_ref())
                .st([St::CursorNotAllowed]);
        }
        if let Some(h) = self.on_draft.clone() {
            field = field.on_debounced(Ev::Input, h, 400);
        }
        if let Some(ref placeholder) = self.placeholder {
            field = field.at_str(At::Placeholder, placeholder.as_ref());
        }
        field
    }

    fn send_button(&self) -> ElementBuilder {
        let label = self.send_label.as_deref().unwrap_or("Send").to_owned();
        let (icon, icon_only) = (
            self.send_icon.unwrap_or(Icon::ArrowUp),
            self.sends_icon_only(),
        );
        if let Some(reason) = &self.disabled {
            return send_button_el(&label, SendPhase::Disabled, icon, icon_only)
                .at_str(At::Title, reason.as_ref());
        }
        match &self.on_submit {
            // Reflect the submit action on the send button — same contract as `Button`.
            Some(spec) if !self.reflect_off => {
                let handler_id = spec.handler_id;
                let param = spec.param_bytes.clone().unwrap_or_default();
                let (l_idle, l_load, l_err) = (label.clone(), label.clone(), label);
                reflect_action(
                    handler_id,
                    param,
                    move || send_button_el(&l_idle, SendPhase::Idle, icon, icon_only),
                    move || send_button_el(&l_load, SendPhase::Loading, icon, icon_only),
                    move |message| {
                        Tooltip::new(message.to_string())
                            .child(send_button_el(&l_err, SendPhase::Errored, icon, icon_only))
                            .build()
                    },
                )
            }
            _ => send_button_el(&label, SendPhase::Idle, icon, icon_only),
        }
    }

    /// Build the composer into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut form = el(El::Form);
        // Empty field → dimmed, unclickable send button (see `guard_css`). The
        // disabled composer already has its own reason and a `disabled` button.
        if self.disabled.is_none() {
            form = form.data("send-guard", "");
        }
        if let Some(handler) = self.on_submit.clone().filter(|_| self.disabled.is_none()) {
            form = form.on(Ev::Submit, handler);
        }
        for (name, value) in &self.hidden_fields {
            form = form.append([el(El::Input)
                .at(At::Type, Av::Hidden)
                .at_str(At::Name, name.as_ref())
                .at_str(At::Value, value.as_ref())]);
        }
        if self.compact {
            // One box: the field and the send button share it, and the box — not
            // the field — carries the border, the rounding and the shadow. The
            // field keeps a transparent background and no border of its own, so
            // the row reads as a single control.
            let mut shell = form.st([
                St::DisplayFlex,
                St::ItemsEnd,
                St::GapSm,
                St::MinW0,
                St::BgSurface,
                St::BorderDefault,
                St::RoundedLg,
                St::PSm,
                St::ShadowSm,
            ]);
            if !self.quiet_focus {
                shell = shell.pseudo(Pc::FocusWithin, [St::BorderAccent]);
            }
            // `PySm` gives the field the same height as the 36 px button on one
            // line, so the two centres line up; the row grows from the bottom.
            let field = self.field().st([St::Flex1, St::MinW0, St::PySm]);
            return shell.append([field, self.send_button(), guard_style()]);
        }
        let hint = el(El::Div)
            .st([
                St::DisplayFlex,
                St::ItemsCenter,
                St::GapXs,
                St::TextXs,
                St::TextLow,
            ])
            .append(
                self.hint
                    .as_deref()
                    // The keyboard hint means nothing on a composer that cannot send.
                    .filter(|_| self.disabled.is_none())
                    .map(|hint| el(El::Span).text(hint))
                    .into_iter()
                    .collect::<Vec<_>>(),
            );
        let actions = el(El::Div)
            .st([St::DisplayFlex, St::ItemsCenter, St::JustifyBetween])
            .append([hint, self.send_button()]);
        let field = self
            .field()
            .style(rwire::style::Style::new().set("padding", "0.25rem"));
        form = form.st([
            St::DisplayFlex,
            St::FlexCol,
            St::GapSm,
            St::BgSurface,
            St::BorderDefault,
            St::RoundedLg,
            St::PMd,
            St::ShadowLg,
        ]);
        if !self.quiet_focus {
            form = form.pseudo(Pc::FocusWithin, [St::BorderAccent]);
        }
        form.append([field, actions, guard_style()])
    }
}

/// The empty-field guard: the submit button dims and stops taking clicks while the
/// field shows its placeholder (i.e. holds nothing). Pure CSS — `:has` on the form,
/// no runtime behaviour, no round-trip. The reflection wrapper (`span[data-rwr]`)
/// sits between the form and the button, hence the descendant combinator.
fn guard_style() -> ElementBuilder {
    el(El::Style).text(
        "form[data-send-guard]:has(textarea:placeholder-shown) button[type=submit]{\
opacity:.45;pointer-events:none;cursor:not-allowed}",
    )
}

/// The send button's phases: the three reflected ones, and disabled.
enum SendPhase {
    Idle,
    Loading,
    Errored,
    /// Cannot send: dimmed, `disabled`, keeps pointer events so its tooltip shows.
    Disabled,
}

/// Render the send button in one phase. Constant footprint across all three: `Loading` swaps the
/// arrow for a spinner and blocks pointer events; `Errored` recolours destructive (still a submit,
/// so it retries) — matching `Button`'s reflection.
fn send_button_el(label: &str, phase: SendPhase, icon: Icon, icon_only: bool) -> ElementBuilder {
    let mut tokens = vec![
        St::BorderNone,
        St::DisplayFlex,
        St::ItemsCenter,
        St::JustifyCenter,
        St::FlexShrink0,
    ];
    if icon_only {
        // A round square that reads as a button on its own.
        tokens.extend([St::SqIconButton, St::RoundedFull]);
    } else {
        tokens.extend([St::GapSm, St::PxMd, St::PySm, St::RoundedMd]);
    }
    tokens.push(match phase {
        SendPhase::Disabled => St::CursorNotAllowed,
        _ => St::CursorPointer,
    });
    match phase {
        SendPhase::Idle => tokens.extend([St::BgAccent, St::TextOnAccent]),
        SendPhase::Loading => tokens.extend([
            St::BgAccent,
            St::TextOnAccent,
            St::Opacity50,
            St::PointerEventsNone,
        ]),
        SendPhase::Errored => tokens.extend([St::BgDestructive, St::TextOnDestructive]),
        SendPhase::Disabled => tokens.extend([St::BgAccent, St::TextOnAccent, St::Opacity50]),
    }
    let trailing = match phase {
        SendPhase::Loading => Spinner::new().size(SpinnerSize::Sm).build(),
        _ => icons::icon_sized(icon, if icon_only { 18 } else { 16 }),
    };
    let mut button = el(El::Button)
        .at(At::Type, Av::Submit)
        .at_str(At::AriaLabel, label)
        .at_str(At::Title, label)
        .st(tokens);
    // Icon-only: the label stays the accessible name, nothing is written.
    if !icon_only {
        button = button.append([el(El::Span).text(label)]);
    }
    button = button.append([trailing]);
    match phase {
        SendPhase::Idle => button.hover([St::BgAccentHover]),
        SendPhase::Disabled => button.bool_attr(At::Disabled),
        _ => button,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The card lights up while the field is focused, unless the app asks for a
    /// quiet field: the app's prompt block shows its own focus.
    #[test]
    fn the_focus_ring_is_opt_out() {
        let lit = Composer::new().build().to_static_html();
        assert!(
            lit.contains(":focus-within") || lit.contains("h14"),
            "{lit}"
        );
        let quiet = Composer::new().focus_ring(false).build().to_static_html();
        assert!(!quiet.contains("h14"), "{quiet}");
    }
    /// Compact is one box, not a field beside a labelled button: the field and
    /// the send button live in the same rounded shell, and the button is
    /// icon-only — its label survives as the accessible name and the tooltip.
    #[test]
    fn a_compact_composer_is_one_box_with_an_inline_icon_button() {
        let html = Composer::new()
            .placeholder("Message…")
            .send_label("Envoyer")
            .compact(true)
            .build()
            .to_static_html();
        // Same form: the textarea comes before the button, no actions row.
        let field = html.find("<textarea").expect("champ");
        let button = html.find("<button").expect("bouton");
        assert!(field < button, "{html}");
        assert!(!html.contains("hint"), "no actions row: {html}");
        // The label is not written, only named.
        assert!(html.contains("aria-label=\"Envoyer\""), "{html}");
        assert!(html.contains("title=\"Envoyer\""), "{html}");
        assert!(
            !html.contains(">Envoyer<"),
            "label taken off the button: {html}"
        );
        // The glyph is there (default ArrowUp).
        assert!(html.contains("M12 19V5M5 12l7-7 7 7"), "{html}");
        // The pill, by contrast, writes its label.
        let pill = Composer::new()
            .send_label("Envoyer")
            .build()
            .to_static_html();
        assert!(pill.contains(">Envoyer<"), "{pill}");
    }

    /// The glyph is the app's to choose, and the label can be restored in the
    /// compact box when the surface has room for it.
    #[test]
    fn the_send_glyph_and_its_label_are_the_app_s() {
        let html = Composer::new()
            .send_icon(Icon::ArrowRight)
            .compact(true)
            .build()
            .to_static_html();
        assert!(html.contains("M5 12h14M12 5l7 7-7 7"), "{html}");
        assert!(!html.contains("M12 19V5M5 12l7-7 7 7"), "{html}");
        let labelled = Composer::new()
            .send_label("Envoyer")
            .send_icon_only(false)
            .compact(true)
            .build()
            .to_static_html();
        assert!(labelled.contains(">Envoyer<"), "{labelled}");
    }

    /// The compact box lights up as a whole while the field is focused — unless
    /// the app asks for silence there too.
    #[test]
    fn the_compact_box_can_light_up_or_stay_quiet() {
        let lit = Composer::new().compact(true).build().to_static_html();
        assert!(
            lit.contains(":focus-within") || lit.contains("h14"),
            "{lit}"
        );
        let quiet = Composer::new()
            .compact(true)
            .focus_ring(false)
            .build()
            .to_static_html();
        assert!(!quiet.contains("h14"), "{quiet}");
    }

    /// An empty field dims the send button and takes it out of the click path,
    /// in pure CSS (`:placeholder-shown`); a disabled composer has no guard (its
    /// button is already `disabled` and explains itself).
    #[test]
    fn an_empty_field_guards_the_send_button() {
        for compact in [false, true] {
            let html = Composer::new()
                .placeholder("Message…")
                .compact(compact)
                .build()
                .to_static_html();
            assert!(html.contains("data-send-guard"), "{html}");
            assert!(
                html.contains(":has(textarea:placeholder-shown) button[type=submit]"),
                "{html}"
            );
            assert!(html.contains("pointer-events:none"), "{html}");
        }
        let disabled = Composer::new()
            .placeholder("Message…")
            .disabled("lecture seule")
            .build()
            .to_static_html();
        // The style text mentions the attribute; the *form* must not carry it.
        assert!(!disabled.contains("data-send-guard="), "{disabled}");
    }

    use rwire::MemoryState;

    #[derive(Default)]
    struct S;
    impl MemoryState for S {}
    fn send(_: &mut S) {}

    #[test]
    fn a_disabled_composer_says_why_and_cannot_send() {
        for compact in [false, true] {
            let reason = "Read-only preview: you cannot reply here";
            let composer = Composer::new()
                .placeholder("Message…")
                .hint("⏎ send")
                .on_submit(HandlerSpec::memory::<S>(send))
                .on_draft(HandlerSpec::memory::<S>(send))
                .compact(compact)
                .disabled(reason)
                .build();
            assert!(composer.events().is_empty(), "no submit bound");
            let html = composer.to_static_html();
            let field = &html[html.find("<textarea").unwrap()..];
            let field = &field[..field.find('>').unwrap()];
            // The reason is in the empty field (read before any click) and its tooltip.
            assert!(
                field.contains(&format!("placeholder=\"{reason}\"")),
                "{html}"
            );
            assert!(field.contains(&format!("title=\"{reason}\"")), "{html}");
            assert!(field.contains("disabled"), "{html}");
            assert!(
                !html.contains("Message…"),
                "the reason replaces the placeholder"
            );
            assert!(!html.contains("⏎ send"), "no keyboard hint");
            let button = &html[html.find("<button").unwrap()..];
            let button = &button[..button.find('>').unwrap()];
            assert!(button.contains("disabled"), "{html}");
            assert!(button.contains(&format!("title=\"{reason}\"")), "{html}");
        }
    }

    #[test]
    fn an_enabled_composer_binds_its_submit() {
        let composer = Composer::new()
            .on_submit(HandlerSpec::memory::<S>(send))
            .build();
        assert_eq!(composer.events().len(), 1);
        let html = composer.to_static_html();
        assert!(!html.contains(" disabled"), "{html}");
    }
}
