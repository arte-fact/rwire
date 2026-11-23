//! DecisionCard component.
//!
//! A validation request in a conversation (screens 2 and 4 of the front): a
//! warning-tinted card (amber at 12 %, radius 12, padding 14 × 16) with a
//! header (title + the tool as a mono tag), the command in a preformatted
//! block, an optional collapsible dialogue, and one two-line `<button>` per
//! choice. Once handled, the actions are replaced by a `role=status` line
//! (`resolved`); a hub refusal shows a `role=alert` line (`error`).
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{DecisionAction, DecisionCard};
//!
//! DecisionCard::new("Validation requested")
//!     .tag("bash")
//!     .body(el(El::Pre).text("rm -rf /tmp/build"))
//!     .action(DecisionAction::new("Allow").hint("run once").on_click(handler))
//!     .action(DecisionAction::new("Refuse").intent(ButtonIntent::Destructive).on_click(handler2))
//!     .build()
//! ```

use std::borrow::Cow;

use rwire::style_tokens::St;
use rwire::{el, At, Av, El, ElementBuilder, Ev, HandlerSpec, Tone};

use crate::ButtonIntent;

/// One choice of a [`DecisionCard`]: a two-line `<button>` (label + hint), the
/// hint repeated as the tooltip.
pub struct DecisionAction {
    label: Cow<'static, str>,
    hint: Option<Cow<'static, str>>,
    intent: ButtonIntent,
    handler: Option<HandlerSpec>,
    disabled: bool,
}

impl DecisionAction {
    pub fn new(label: impl Into<Cow<'static, str>>) -> Self {
        Self {
            label: label.into(),
            hint: None,
            intent: ButtonIntent::Secondary,
            handler: None,
            disabled: false,
        }
    }

    /// The small second line (the terminal's option), also the tooltip.
    pub fn hint(mut self, hint: impl Into<Cow<'static, str>>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// The button's intent (Primary, Secondary, Destructive, …).
    pub fn intent(mut self, intent: ButtonIntent) -> Self {
        self.intent = intent;
        self
    }

    /// The click handler.
    pub fn on_click(mut self, handler: HandlerSpec) -> Self {
        self.handler = Some(handler);
        self
    }

    /// Read-only preview: the button renders disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn intent_tokens(intent: ButtonIntent) -> Vec<St> {
        match intent {
            ButtonIntent::Primary => vec![St::BgPrimary, St::TextOnPrimary],
            ButtonIntent::Secondary => {
                vec![St::BgSecondary, St::TextOnSecondary, St::BorderDefault]
            }
            ButtonIntent::Ghost => vec![St::BgTransparent, St::TextHigh],
            ButtonIntent::Outline => {
                vec![St::BgTransparent, St::TextMuted, St::BorderDefault]
            }
            ButtonIntent::Destructive => vec![St::BgDestructive, St::TextOnDestructive],
            ButtonIntent::DangerGhost => {
                vec![St::BgTransparent, St::TextError, St::BorderDefault]
            }
        }
    }

    fn build(self) -> ElementBuilder {
        let mut children = vec![el(El::Span)
            .st([St::TextSm, St::FontW650])
            .text(self.label.as_ref())];
        if let Some(hint) = &self.hint {
            children.push(
                el(El::Span)
                    .st([St::TextXs, St::TextMuted])
                    .text(hint.as_ref()),
            );
        }
        let mut btn = el(El::Button)
            .st([
                St::DisplayFlex,
                St::FlexCol,
                St::ItemsStart,
                St::GapXs,
                St::WFull,
                St::TextLeft,
                St::PxMd,
                St::PySm,
                St::RoundedMd,
                St::CursorPointer,
            ])
            .st(Self::intent_tokens(self.intent))
            .data("testid", "decision-action")
            .append(children);
        if let Some(hint) = &self.hint {
            btn = btn.at_str(At::Title, hint.as_ref());
        }
        if let Some(handler) = self.handler {
            btn = btn.on(Ev::Click, handler);
        }
        if self.disabled {
            btn = btn
                .bool_attr(At::Disabled)
                .st([St::Opacity50, St::CursorNotAllowed]);
        }
        btn
    }
}

/// DecisionCard builder.
pub struct DecisionCard {
    title: Cow<'static, str>,
    tone: Tone,
    tag: Option<Cow<'static, str>>,
    body: Option<ElementBuilder>,
    reason: Option<Cow<'static, str>>,
    details: Option<(Cow<'static, str>, ElementBuilder)>,
    actions: Vec<DecisionAction>,
    resolved: Option<Cow<'static, str>>,
    error: Option<Cow<'static, str>>,
}

impl DecisionCard {
    /// A warning-tinted card (amber at 12 %) titled `title`.
    pub fn new(title: impl Into<Cow<'static, str>>) -> Self {
        Self {
            title: title.into(),
            tone: Tone::Warning,
            tag: None,
            body: None,
            reason: None,
            details: None,
            actions: Vec::new(),
            resolved: None,
            error: None,
        }
    }

    /// Another tone than the default `Warning` (its tinted ground is used then).
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// The tool as a mono tag in the header.
    pub fn tag(mut self, tag: impl Into<Cow<'static, str>>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// The command or input, in a preformatted block.
    pub fn body(mut self, body: ElementBuilder) -> Self {
        self.body = Some(body);
        self
    }

    /// An explanation line (or the read-only note).
    pub fn reason(mut self, reason: impl Into<Cow<'static, str>>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// A collapsible block under the body (e.g. the dialogue shown in the terminal).
    pub fn details(mut self, summary: impl Into<Cow<'static, str>>, body: ElementBuilder) -> Self {
        self.details = Some((summary.into(), body));
        self
    }

    /// Add a choice.
    pub fn action(mut self, action: DecisionAction) -> Self {
        self.actions.push(action);
        self
    }

    /// Resolved: `Some(text)` replaces the actions with a `role=status` line.
    pub fn resolved(mut self, text: Option<impl Into<Cow<'static, str>>>) -> Self {
        self.resolved = text.map(Into::into);
        self
    }

    /// A hub refusal: `Some(text)` shows a `role=alert` line.
    pub fn error(mut self, text: Option<impl Into<Cow<'static, str>>>) -> Self {
        self.error = text.map(Into::into);
        self
    }

    /// Build the card.
    pub fn build(self) -> ElementBuilder {
        let ground = match self.tone {
            Tone::Warning => St::BgWarning12,
            other => other.ground(),
        };
        let mut children: Vec<ElementBuilder> = Vec::new();

        // Header: title (650) + the tool tag (mono) on the right.
        let mut header = vec![el(El::B)
            .st([St::Text13, St::FontW650])
            .data("testid", "decision-title")
            .text(self.title.as_ref())];
        if let Some(tag) = &self.tag {
            header.push(el(El::Span).st([St::Flex1]));
            header.push(
                el(El::Code)
                    .st([
                        St::FontMono,
                        St::TextXs,
                        St::BgCode,
                        St::RoundedSm,
                        St::PxXs,
                        St::PyXs,
                        St::TextMuted,
                    ])
                    .data("testid", "decision-tag")
                    .text(tag.as_ref()),
            );
        }
        children.push(
            el(El::Div)
                .st([St::DisplayFlex, St::ItemsBaseline, St::GapSm])
                .append(header),
        );

        if let Some(body) = self.body {
            children.push(
                el(El::Pre)
                    .st([
                        St::FontMono,
                        St::Text12_5,
                        St::WhitespacePre,
                        St::OverflowXAuto,
                        St::BgCode,
                        St::RoundedMd,
                        St::PMd,
                        St::BorderSubtle,
                        St::M0,
                    ])
                    .data("testid", "decision-body")
                    .append([body]),
            );
        }
        if let Some(reason) = &self.reason {
            children.push(
                el(El::P)
                    .st([St::TextSm, St::TextMuted, St::M0])
                    .data("testid", "decision-reason")
                    .text(reason.as_ref()),
            );
        }
        if let Some((summary, body)) = self.details {
            children.push(
                el(El::Details)
                    .st([St::TextSm])
                    .data("testid", "decision-details")
                    .append([
                        el(El::Summary)
                            .st([St::TextXs, St::TextMuted, St::CursorPointer])
                            .text(summary.as_ref()),
                        body,
                    ]),
            );
        }
        if let Some(error) = &self.error {
            children.push(
                el(El::Div)
                    .st([St::TextSm, St::TextError])
                    .at(At::Role, Av::RoleAlert)
                    .data("testid", "decision-error")
                    .text(error.as_ref()),
            );
        }
        if let Some(resolved) = &self.resolved {
            children.push(
                el(El::Div)
                    .st([St::TextSm, St::TextSuccess])
                    .at(At::Role, Av::RoleStatus)
                    .data("testid", "decision-resolved")
                    .text(resolved.as_ref()),
            );
        } else if !self.actions.is_empty() {
            children.push(
                el(El::Div)
                    .st([St::DisplayFlex, St::FlexCol, St::GapSm])
                    .data("testid", "decision-actions")
                    .append(
                        self.actions
                            .into_iter()
                            .map(|a| a.build())
                            .collect::<Vec<_>>(),
                    ),
            );
        }

        el(El::Div)
            .st([
                ground,
                St::Rounded12,
                St::PadDecision,
                St::BorderSubtle,
                St::DisplayFlex,
                St::FlexCol,
                St::GapSm,
            ])
            .data("testid", "decision-card")
            .append(children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rwire::{ChangeSet, HandlerSpec};

    #[derive(rwire::State, Default)]
    #[storage(memory)]
    struct S;

    fn handler() -> HandlerSpec {
        fn noop(_s: &mut S) {}
        HandlerSpec::from_fn_with_changes::<S>(noop, ChangeSet::all())
            .with_handler_id(rwire::stable_handler_id("decision_card_tests", "noop"))
    }

    #[test]
    fn pending_card_is_warning_tinted_with_two_line_actions() {
        let card = DecisionCard::new("Validation requested")
            .tag("bash")
            .body(el(El::Span).text("rm -rf /tmp/build"))
            .reason("The agent wants to run this")
            .action(
                DecisionAction::new("Allow")
                    .hint("run once")
                    .intent(ButtonIntent::Primary)
                    .on_click(handler()),
            )
            .action(
                DecisionAction::new("Refuse")
                    .hint("deny this time")
                    .intent(ButtonIntent::Destructive)
                    .on_click(handler()),
            )
            .build();
        let html = card.to_static_html();
        assert!(html.contains("data-testid=\"decision-card\""), "{html}");
        assert!(
            html.contains("data-testid=\"decision-tag\">bash<"),
            "{html}"
        );
        assert!(html.contains("data-testid=\"decision-body\""), "{html}");
        assert!(html.contains("data-testid=\"decision-reason\""), "{html}");
        assert!(html.contains("data-testid=\"decision-actions\""), "{html}");
        assert_eq!(
            html.matches("data-testid=\"decision-action\"").count(),
            2,
            "{html}"
        );
        // Two lines per action: the hint, also as the tooltip.
        assert!(html.contains("run once"), "{html}");
        assert!(html.contains("title=\"run once\""), "{html}");
        assert!(!html.contains("decision-resolved"), "{html}");
        // Warning ground at 12 %, radius 12, padding 14x16, no inline style.
        let utils = card.get_style_utils();
        for st in [St::BgWarning12, St::Rounded12, St::PadDecision] {
            assert!(utils.contains(&st.as_u16()), "{st:?}");
        }
        assert!(!html.contains("style="), "{html}");
        assert_eq!(
            St::BgWarning12.css(),
            "background:color-mix(in srgb,var(--p) 12%,transparent)"
        );
        assert_eq!(St::PadDecision.css(), "padding:14px 16px");
    }

    #[test]
    fn resolved_replaces_the_actions_with_a_status_line() {
        let card = DecisionCard::new("Validation handled")
            .action(DecisionAction::new("Allow").on_click(handler()))
            .resolved(Some("Allowed once"))
            .build();
        let html = card.to_static_html();
        assert!(html.contains("role=\"status\""), "{html}");
        assert!(html.contains("Allowed once"), "{html}");
        assert!(!html.contains("decision-actions"), "{html}");
        assert!(html.contains("data-testid=\"decision-resolved\""), "{html}");
    }

    #[test]
    fn error_is_an_alert_and_details_render_collapsed() {
        let card = DecisionCard::new("Validation requested")
            .error(Some("Response refused: not while a session runs"))
            .details(
                "Dialogue shown in the terminal",
                el(El::Pre).text("Allow? [y/n]"),
            )
            .action(
                DecisionAction::new("Allow")
                    .disabled(true)
                    .on_click(handler()),
            )
            .build();
        let html = card.to_static_html();
        assert!(html.contains("role=\"alert\""), "{html}");
        assert!(html.contains("Response refused"), "{html}");
        assert!(html.contains("data-testid=\"decision-details\""), "{html}");
        assert!(
            !html.contains("<details open"),
            "details closed by default: {html}"
        );
        assert!(
            html.contains("disabled"),
            "read-only preview disables the action: {html}"
        );
    }
}
