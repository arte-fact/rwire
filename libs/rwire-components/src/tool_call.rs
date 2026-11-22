//! ToolCall component.
//!
//! A tool invocation in a conversation: a native `<details>` **closed by
//! default**, radius 10, 13 px. The summary holds on one line — the tool's name
//! in bold, the command in mono with an ellipsis, the status on the right
//! (`Running` | `Ok` | `Err`) — and the opened body is the output in a `pre`
//! capped at 320 px, scrolling inside. Use it as a [`ChatItem::row()`] or a
//! [`ChatEntry::detail`](crate::ChatDetail).
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{ToolCall, ToolStatus};
//!
//! ToolCall::new("bash", "cargo test -p rwire")
//!     .status(ToolStatus::Ok)
//!     .output("test result: ok. 320 passed")
//!     .build()
//! ```

use std::borrow::Cow;

use rwire::style_tokens::St;
use rwire::{el, At, El, ElementBuilder};

/// The outcome of a tool call, shown on the right of the summary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolStatus {
    /// Still running: muted label.
    #[default]
    Running,
    /// Succeeded: green `ok`.
    Ok,
    /// Failed: red `err`.
    Err,
}

/// ToolCall builder.
pub struct ToolCall {
    name: Cow<'static, str>,
    command: Cow<'static, str>,
    status: ToolStatus,
    output: Option<Cow<'static, str>>,
    open: bool,
}

impl ToolCall {
    /// A collapsed tool card: `name` (bold) and `command` (mono, ellipsed).
    pub fn new(name: impl Into<Cow<'static, str>>, command: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            status: ToolStatus::Running,
            output: None,
            open: false,
        }
    }

    /// The status on the right of the summary.
    pub fn status(mut self, status: ToolStatus) -> Self {
        self.status = status;
        self
    }

    /// The output, rendered in a bounded `pre` (320 px, scrolling inside).
    pub fn output(mut self, output: impl Into<Cow<'static, str>>) -> Self {
        self.output = Some(output.into());
        self
    }

    /// Start opened (closed by default).
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    fn status_el(status: ToolStatus) -> ElementBuilder {
        let (label, tokens) = match status {
            ToolStatus::Running => ("Running", vec![St::TextMuted]),
            ToolStatus::Ok => ("ok", vec![St::TextSuccess, St::FontW650]),
            ToolStatus::Err => ("err", vec![St::TextError, St::FontW650]),
        };
        el(El::Span)
            .st([St::TextXs, St::WhitespaceNowrap])
            .st(tokens)
            .data("testid", "tool-status")
            .text(label)
    }

    /// Build the `<details>` card.
    pub fn build(self) -> ElementBuilder {
        let summary = el(El::Summary)
            .st([
                St::DisplayFlex,
                St::ItemsBaseline,
                St::GapSm,
                St::MinW0,
                St::CursorPointer,
                St::PadToolSummary,
            ])
            .data("testid", "tool-summary")
            .append([
                el(El::B).st([St::FontW650]).text(self.name.as_ref()),
                el(El::Code)
                    .st([St::FontMono, St::TextXs, St::Flex1, St::MinW0, St::Truncate])
                    .text(self.command.as_ref()),
                Self::status_el(self.status),
            ]);
        let mut details = el(El::Details)
            .st([
                St::Text13,
                St::BgSurfaceRaised,
                St::BorderSubtle,
                St::RoundedLg,
                St::OverflowHidden,
            ])
            .data("testid", "tool-call")
            .append([summary]);
        if let Some(output) = self.output {
            details = details.append([el(El::Pre)
                .st([
                    St::FontMono,
                    St::Text12_5,
                    St::WhitespacePre,
                    St::OverflowAuto,
                    St::MaxH320,
                    St::BgCode,
                    St::M0,
                    St::PMd,
                    St::BorderTSubtle,
                ])
                .data("testid", "tool-output")
                .text(output.as_ref())]);
        }
        if self.open {
            details = details.bool_attr(At::Open);
        }
        details
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_by_default_one_line_summary_and_bounded_output() {
        let card = ToolCall::new("bash", "cargo test -p rwire --workspace")
            .status(ToolStatus::Ok)
            .output("test result: ok. 320 passed; 0 failed")
            .build();
        let html = card.to_static_html();
        assert!(html.contains("<details"), "{html}");
        assert!(!html.contains("open"), "closed by default: {html}");
        assert!(html.contains("data-testid=\"tool-call\""), "{html}");
        assert!(html.contains("data-testid=\"tool-summary\""), "{html}");
        assert!(html.contains("data-testid=\"tool-output\""), "{html}");
        assert!(html.contains("data-testid=\"tool-status\">ok<"), "{html}");
        let utils = card.get_style_utils();
        assert!(
            utils.contains(&St::RoundedLg.as_u16()),
            "radius 10 via --R3"
        );
        assert!(utils.contains(&St::Text13.as_u16()), "13 px");
        // The summary ellipsizes the command on one line.
        let summary = &card.children()[0];
        assert!(summary
            .get_style_utils()
            .contains(&St::PadToolSummary.as_u16()));
        let code = &summary.children()[1];
        assert!(code.get_style_utils().contains(&St::Truncate.as_u16()));
        assert!(code.get_style_utils().contains(&St::FontMono.as_u16()));
        // The output is a pre capped at 320 px, scrolling inside.
        let output = &card.children()[1];
        let out_utils = output.get_style_utils();
        assert!(out_utils.contains(&St::MaxH320.as_u16()));
        assert!(out_utils.contains(&St::OverflowAuto.as_u16()));
        assert_eq!(St::MaxH320.css(), "max-height:320px");
    }

    #[test]
    fn status_labels_and_tones() {
        for (status, label) in [
            (ToolStatus::Running, "Running"),
            (ToolStatus::Ok, "ok"),
            (ToolStatus::Err, "err"),
        ] {
            let html = ToolCall::new("t", "c")
                .status(status)
                .build()
                .to_static_html();
            assert!(html.contains(&format!(">{label}<")), "{html}");
        }
        let err = ToolCall::new("t", "c").status(ToolStatus::Err).build();
        let status = &err.children()[0].children()[2];
        assert!(status.get_style_utils().contains(&St::TextError.as_u16()));
    }

    #[test]
    fn open_renders_the_open_attribute_and_no_output_means_no_pre() {
        let open = ToolCall::new("t", "c").open(true).build().to_static_html();
        assert!(open.contains("open"), "{open}");
        let bare = ToolCall::new("t", "c").build();
        assert!(!bare.to_static_html().contains("tool-output"));
    }
}
