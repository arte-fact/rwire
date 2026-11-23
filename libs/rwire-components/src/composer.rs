//! Composer component.
//!
//! The message-entry bar of a chat surface: an auto-growing textarea where **Enter submits
//! and Shift+Enter inserts a newline** (the runtime's `data-enter-submit` behavior), plus a
//! send button. Pairs with [`ChatScroll`](crate::ChatScroll) as the other half of a chat UI.
//!
//! Two form factors:
//! - the default **pill** — a bordered, shadowed card with the field above an actions row
//!   (an optional hint or tools on the left, Send on the right);
//! - **compact** — a single row (field beside the send button) for inline composers like a
//!   thread's steer bar.
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
use rwire::style_tokens::{Bp, St};
use rwire::{el, icons, reflect_action, At, Av, El, ElementBuilder, Ev, HandlerSpec, Pc};
use std::borrow::Cow;

/// Composer builder.
#[derive(Debug, Default)]
pub struct Composer {
    id: Option<Cow<'static, str>>,
    name: Option<Cow<'static, str>>,
    placeholder: Option<Cow<'static, str>>,
    hint: Option<Cow<'static, str>>,
    hint_below: Option<Bp>,
    send_label: Option<Cow<'static, str>>,
    send_icon: Option<icons::Icon>,
    min_rows: u32,
    reset_on_ack: bool,
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

    /// Hide the hint below `bp` (visually; the hint text stays in the DOM). E.g.
    /// `hint_hidden_below(Bp::Md)` keeps it from the `md` breakpoint up.
    pub fn hint_hidden_below(mut self, bp: Bp) -> Self {
        self.hint_below = Some(bp);
        self
    }

    /// The send button's label (default `Send`).
    pub fn send_label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.send_label = Some(label.into());
        self
    }

    /// Icon-only send button: a 34 px square (44 px under a finger, via the
    /// theme's touch rule), radius 9, the icon centred, the label in
    /// `aria-label`. Default: a labelled button with the arrow icon.
    pub fn send_icon(mut self, icon: icons::Icon) -> Self {
        self.send_icon = Some(icon);
        self
    }

    /// The field's starting row count (default 1); it auto-grows past it.
    pub fn min_rows(mut self, rows: u32) -> Self {
        self.min_rows = rows;
        self
    }

    /// Clear the field when the submit's `EVENT_ACK` comes back — a successful
    /// send. A send whose action errored keeps its text (the runtime checks the
    /// `data-action-error` marker). Off by default: key the composer with
    /// `.id(...)` instead if you prefer a fresh element per send.
    pub fn reset_on_ack(mut self, on: bool) -> Self {
        self.reset_on_ack = on;
        self
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
        let rows = if self.min_rows == 0 { 1 } else { self.min_rows };
        let mut field = el(El::Textarea)
            .at_str(At::Name, self.name.as_deref().unwrap_or("message"))
            .at_str(At::Rows, &rows.to_string())
            .at(At::Autocomplete, Av::Off)
            .data("enter-submit", "1")
            .st([
                St::WFull,
                St::TextDefault,
                // The conversation's field runs at 14.5 px (the /app value; the
                // last call wins over TextDefault's size).
                St::Text14_5,
                // Transparent by default so the field shows its themed parent, never the UA white
                // default. The full mode nests it in a `BgSurface` box; the compact mode borders it
                // directly on the page — without this the compact `<textarea>` renders white.
                St::BgTransparent,
                St::BorderNone,
                St::OutlineNone,
                St::ResizeNone,
                St::OverflowYAuto,
                St::FieldSizingContent,
            ]);
        // The auto-grow cap: a token by default (40vh), an inline style only
        // when the caller overrides it with a value the tokens don't cover.
        field = match &self.max_height {
            Some(mh) => field.style(rwire::style::Style::new().set("max-height", mh)),
            None => field.st([St::MaxH40vh]),
        };
        if self.reset_on_ack {
            field = field.data("reset-on-ack", "1");
        }
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
        let icon = self.send_icon;
        if let Some(reason) = &self.disabled {
            return send_button_el(&label, SendPhase::Disabled, icon)
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
                    move || send_button_el(&l_idle, SendPhase::Idle, icon),
                    move || send_button_el(&l_load, SendPhase::Loading, icon),
                    move |message| {
                        Tooltip::new(message.to_string())
                            .child(send_button_el(&l_err, SendPhase::Errored, icon))
                            .build()
                    },
                )
            }
            _ => send_button_el(&label, SendPhase::Idle, icon),
        }
    }

    /// Build the composer into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut form = el(El::Form);
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
            return form
                .st([St::DisplayFlex, St::ItemsCenter, St::GapSm])
                .append([
                    self.field()
                        .st([St::Flex1, St::PSm, St::BorderDefault, St::RoundedMd])
                        .focus([St::BorderAccent]),
                    self.send_button(),
                ]);
        }
        let mut hint = el(El::Div).st([
            St::DisplayFlex,
            St::ItemsCenter,
            St::GapXs,
            St::TextXs,
            St::TextLow,
        ]);
        if let Some(bp) = self.hint_below {
            // Hidden below `bp` (the base rule), shown from `bp` up.
            hint = hint
                .st([St::DisplayNone])
                .breakpoint(bp, [St::DisplayFlex])
                .data("testid", "composer-hint");
        }
        let hint = hint.append(
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
        let field = self.field().st([St::PadField]);
        form.st([
            St::DisplayFlex,
            St::FlexCol,
            St::GapSm,
            St::BgSurface,
            St::BorderDefault,
            St::RoundedLg,
            St::PMd,
            St::ShadowLg,
        ])
        .pseudo(Pc::FocusWithin, [St::BorderAccent])
        .append([field, actions])
    }
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
/// so it retries) — matching `Button`'s reflection. With `icon`, the button is an icon-only
/// 34 px square (radius 9, the label in `aria-label`); without, a labelled button with the arrow.
fn send_button_el(label: &str, phase: SendPhase, icon: Option<icons::Icon>) -> ElementBuilder {
    let icon_only = icon.is_some();
    let mut tokens = vec![
        St::BorderNone,
        St::DisplayFlex,
        St::ItemsCenter,
        St::JustifyCenter,
    ];
    if icon_only {
        tokens.extend([St::Sq34, St::Rounded9]);
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
        _ => icons::icon_sized(icon.unwrap_or(icons::Icon::ArrowUp), 16),
    };
    let button = el(El::Button)
        .at(At::Type, Av::Submit)
        .at_str(At::AriaLabel, label)
        .data("testid", "composer-send")
        .st(tokens);
    let button = if icon_only {
        button.append([trailing])
    } else {
        button.append([el(El::Span).text(label), trailing])
    };
    match phase {
        SendPhase::Idle => button.hover([St::BgAccentHover]),
        SendPhase::Disabled => button.bool_attr(At::Disabled),
        _ => button,
    }
}

/// A composer docked at the bottom of the page: full-width sticky bar on the
/// canvas, with the composer as a centred card at the reading measure (760 px),
/// radius 14, a soft shadow, and the bottom safe-area inset.
///
/// ```ignore
/// ComposerDock::new(Composer::new().on_submit(send).reset_on_ack(true)).build()
/// ```
pub struct ComposerDock {
    composer: Composer,
}

impl ComposerDock {
    pub fn new(composer: Composer) -> Self {
        Self { composer }
    }

    /// Build the dock: a sticky bottom bar (canvas ground, gutters, safe-area)
    /// holding the composer card centred at the reading measure.
    pub fn build(self) -> ElementBuilder {
        el(El::Div)
            .st([
                St::PositionSticky,
                St::Bottom0,
                St::WFull,
                St::BgCanvas,
                St::PtSm,
                St::PbSafe,
                St::PxGutterPhone,
            ])
            .sm([St::PxGutter])
            .data("testid", "composer-dock")
            .append([el(El::Div).st([St::MeasureReading]).append([self
                .composer
                .build()
                // The dock's card: radius 14, a soft shadow (the last call wins).
                .st([St::RoundedXl, St::ShadowMd])
                .data("testid", "composer-card")])])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn find_by_testid<'a>(e: &'a ElementBuilder, testid: &str) -> Option<&'a ElementBuilder> {
        if e.attributes()
            .iter()
            .any(|(k, v)| k == "data-testid" && v == testid)
        {
            return Some(e);
        }
        e.children().iter().find_map(|c| find_by_testid(c, testid))
    }

    #[test]
    fn send_icon_makes_a_34px_square_icon_button_with_an_aria_label() {
        // No on_submit: the reflected send button renders lazily, so inspect
        // the idle button directly.
        let composer = Composer::new()
            .send_label("Send")
            .send_icon(icons::Icon::ArrowUp)
            .build();
        let html = composer.to_static_html();
        assert!(html.contains("aria-label=\"Send\""), "{html}");
        let send = find_by_testid(&composer, "composer-send").expect("send button");
        for st in [St::Sq34, St::Rounded9, St::BgAccent, St::JustifyCenter] {
            assert!(send.get_style_utils().contains(&st.as_u16()), "{st:?}");
        }
        // Icon-only: no visible text label, just the svg.
        let btn_html = send.to_static_html();
        assert!(!btn_html.contains(">Send<"), "{btn_html}");
        assert!(btn_html.contains("<svg"), "{btn_html}");
        // Without send_icon, the labelled button keeps its text.
        let labelled = Composer::new().send_label("Send").build();
        let btn = find_by_testid(&labelled, "composer-send").unwrap();
        assert!(!btn.get_style_utils().contains(&St::Sq34.as_u16()));
        assert!(btn.to_static_html().contains(">Send<"));
    }

    #[test]
    fn min_rows_and_reset_on_ack_land_on_the_field() {
        let composer = Composer::new()
            .min_rows(2)
            .reset_on_ack(true)
            .on_submit(HandlerSpec::memory::<S>(send))
            .build();
        let html = composer.to_static_html();
        assert!(html.contains("rows=\"2\""), "{html}");
        assert!(html.contains("data-reset-on-ack=\"1\""), "{html}");
        assert!(html.contains("data-enter-submit=\"1\""), "{html}");
        // Default: rows=1, no reset-on-ack.
        let plain = Composer::new().build().to_static_html();
        assert!(plain.contains("rows=\"1\""), "{plain}");
        assert!(!plain.contains("reset-on-ack"), "{plain}");
    }

    #[test]
    fn hint_hidden_below_hides_the_hint_below_the_breakpoint() {
        let composer = Composer::new()
            .hint("⏎ send · ⇧⏎ newline")
            .hint_hidden_below(Bp::Md)
            .on_submit(HandlerSpec::memory::<S>(send))
            .build();
        let hint = find_by_testid(&composer, "composer-hint").expect("hint");
        // Base: hidden; from Bp::Md up: flex again.
        assert!(hint.get_style_utils().contains(&St::DisplayNone.as_u16()));
        let groups = hint.get_breakpoint_groups();
        let md: Vec<u16> = groups
            .iter()
            .filter(|(bp, _)| *bp == Bp::Md.as_u8())
            .flat_map(|(_, c)| c.iter().copied())
            .collect();
        assert!(md.contains(&St::DisplayFlex.as_u16()), "{groups:?}");
        // Without the call, the hint stays visible and unmarked.
        let plain = Composer::new().hint("⏎ send").build();
        assert!(find_by_testid(&plain, "composer-hint").is_none());
    }

    #[test]
    fn the_dock_is_a_sticky_full_width_bar_with_a_centred_760_card() {
        let dock = ComposerDock::new(
            Composer::new()
                .on_submit(HandlerSpec::memory::<S>(send))
                .reset_on_ack(true),
        )
        .build();
        let utils = dock.get_style_utils();
        for st in [
            St::PositionSticky,
            St::Bottom0,
            St::WFull,
            St::BgCanvas,
            St::PbSafe,
            St::PxGutterPhone,
        ] {
            assert!(utils.contains(&st.as_u16()), "{st:?}");
        }
        let sm: Vec<u16> = dock
            .get_breakpoint_groups()
            .iter()
            .flat_map(|(_, c)| c.iter().copied())
            .collect();
        assert!(sm.contains(&St::PxGutter.as_u16()));
        let html = dock.to_static_html();
        assert!(html.contains("data-testid=\"composer-dock\""), "{html}");
        assert!(html.contains("data-testid=\"composer-card\""), "{html}");
        let card = &dock.children()[0];
        assert!(card
            .get_style_utils()
            .contains(&St::MeasureReading.as_u16()));
        let form = &card.children()[0];
        for st in [St::RoundedXl, St::ShadowMd] {
            assert!(form.get_style_utils().contains(&st.as_u16()), "{st:?}");
        }
        assert!(!html.contains("style="), "no inline style: {html}");
    }
}
