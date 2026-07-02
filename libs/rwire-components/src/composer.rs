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

use rwire::style_tokens::St;
use rwire::{el, icons, At, Av, El, ElementBuilder, Ev, HandlerSpec, Pc};
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

    /// The form's submit handler — read the message with `ctx.field(name)`.
    pub fn on_submit(mut self, handler: HandlerSpec) -> Self {
        self.on_submit = Some(handler);
        self
    }

    /// Single-row form factor (field beside the send button, no card chrome).
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
        if let Some(ref placeholder) = self.placeholder {
            field = field.at_str(At::Placeholder, placeholder.as_ref());
        }
        field
    }

    fn send_button(&self) -> ElementBuilder {
        let label = self.send_label.as_deref().unwrap_or("Send").to_owned();
        el(El::Button)
            .at(At::Type, Av::Submit)
            .at_str(At::AriaLabel, &label)
            .st([
                St::BorderNone,
                St::DisplayFlex,
                St::ItemsCenter,
                St::GapSm,
                St::PxMd,
                St::PySm,
                St::RoundedMd,
                St::BgAccent,
                St::TextOnAccent,
                St::CursorPointer,
            ])
            .hover([St::BgAccentHover])
            .append([
                el(El::Span).text(&label),
                icons::icon_sized(icons::Icon::ArrowUp, 16),
            ])
    }

    /// Build the composer into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut form = el(El::Form);
        if let Some(handler) = self.on_submit.clone() {
            form = form.on(Ev::Submit, handler);
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
                    .map(|hint| el(El::Span).text(hint))
                    .into_iter()
                    .collect::<Vec<_>>(),
            );
        let actions = el(El::Div)
            .st([St::DisplayFlex, St::ItemsCenter, St::JustifyBetween])
            .append([hint, self.send_button()]);
        let field = self
            .field()
            .st([St::BgTransparent])
            .style(rwire::style::Style::new().set("padding", "0.25rem"));
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
