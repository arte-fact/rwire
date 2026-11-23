//! ChatCalls component.
//!
//! A run of tool calls collapsed into **one** transcript block: a summary line
//! ("4 tool calls · Bash, Read, Edit") and, behind a native `<details>`,
//! each call with its argument summary, its state and its own disclosure for the
//! full input and output. Native `<details>` means the toggle costs no server
//! round-trip and keeps keyboard and screen-reader semantics.
//!
//! It fits where a chat surface would otherwise interleave dozens of one-line
//! tool rows with the actual conversation: the prose stays readable, and the
//! machinery is one click away. The run stays **closed** by default (the native
//! `<summary>` triangle is the only affordance, no "details" label), and the
//! expanded calls are indented under their summary.
//!
//! The run carries no chrome of its own — no background, no rounding, no
//! padding: it reads as one line of the transcript, not as a message bubble.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{ChatCall, ChatCalls, ChatCallState};
//!
//! ChatCalls::new()
//!     .call(
//!         ChatCall::new("Bash", "cargo test --all")
//!             .state(ChatCallState::Ok)
//!             .detail(el(El::Pre).text("41 passed")),
//!     )
//!     .call(
//!         ChatCall::new("Read", "src/lib.rs")
//!             .state(ChatCallState::Error)
//!             .detail(el(El::Pre).text("no such file")),
//!     )
//!     .build()
//! ```

use std::borrow::Cow;

use rwire::style_tokens::St;
use rwire::{el, At, El, ElementBuilder};

/// How a tool call ended.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChatCallState {
    /// Sent, no result yet.
    #[default]
    Running,
    /// Returned normally.
    Ok,
    /// Returned an error (or the hub refused it).
    Error,
}

impl ChatCallState {
    /// The state's word, from the run's text table (English by default).
    fn label(self, text: &ChatCallsText) -> &'static str {
        match self {
            Self::Running => text.running,
            Self::Ok => text.ok,
            Self::Error => text.error,
        }
    }

    fn ink(self) -> St {
        match self {
            Self::Running => St::TextAccent,
            Self::Ok => St::TextMuted,
            Self::Error => St::TextError,
        }
    }
}

/// The words a run of tool calls shows. English by default: a library speaks its
/// own language, the app brings its strings in with [`ChatCalls::text`].
///
/// `{n}` (the count) and `{tools}` (up to three tool names) are substituted in the
/// summary templates.
#[derive(Clone, Debug)]
pub struct ChatCallsText {
    /// No call in the run.
    pub none: &'static str,
    /// Exactly one call; `{tools}` is its tool name.
    pub one: &'static str,
    /// Several calls; `{n}` is the count and `{tools}` the names.
    pub many: &'static str,
    /// Exactly one failed call.
    pub failure_one: &'static str,
    /// Several failed calls; `{n}` is the count.
    pub failure_many: &'static str,
    /// A call still in flight.
    pub running: &'static str,
    /// A call that returned.
    pub ok: &'static str,
    /// A call that failed.
    pub error: &'static str,
}

impl Default for ChatCallsText {
    fn default() -> Self {
        Self {
            none: "No tool call",
            one: "1 tool call · {tools}",
            many: "{n} tool calls · {tools}",
            failure_one: "1 failure",
            failure_many: "{n} failures",
            running: "running",
            ok: "ok",
            error: "error",
        }
    }
}

/// One call in a [`ChatCalls`] run.
#[derive(Clone)]
pub struct ChatCall {
    name: Cow<'static, str>,
    summary: Cow<'static, str>,
    state: ChatCallState,
    detail: Option<ElementBuilder>,
}

impl ChatCall {
    /// A call to `name` (the tool) with a one-line `summary` of what it did.
    pub fn new(name: impl Into<Cow<'static, str>>, summary: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            summary: summary.into(),
            state: ChatCallState::default(),
            detail: None,
        }
    }

    /// How the call ended (default: still running).
    pub fn state(mut self, state: ChatCallState) -> Self {
        self.state = state;
        self
    }

    /// The full input and output, behind a disclosure under the call's line.
    pub fn detail(mut self, detail: ElementBuilder) -> Self {
        self.detail = Some(detail);
        self
    }
}

/// A run of tool calls, collapsed into one block.
#[derive(Default)]
pub struct ChatCalls {
    calls: Vec<ChatCall>,
    open: Option<bool>,
    text: ChatCallsText,
    extra_class: Option<Cow<'static, str>>,
}

impl ChatCalls {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a call, in run order.
    pub fn call(mut self, call: ChatCall) -> Self {
        self.calls.push(call);
        self
    }

    /// Force the block open or closed. Closed by default: the run is a single
    /// line of the transcript, expanded on demand by the reader.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Replace the words the run shows (see [`ChatCallsText`]): a French app
    /// passes French, the library stays English.
    pub fn text(mut self, text: ChatCallsText) -> Self {
        self.text = text;
        self
    }

    /// Add a custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Whether the block starts expanded: never, unless the app asked.
    pub fn starts_open(&self) -> bool {
        self.open.unwrap_or(false)
    }

    /// The summary line: how many calls, and which tools (three names at most).
    pub fn summary_text(&self) -> String {
        let n = self.calls.len();
        let mut names: Vec<&str> = Vec::new();
        for call in &self.calls {
            let name = call.name.as_ref();
            if !names.contains(&name) {
                names.push(name);
            }
        }
        let shown = names.iter().take(3).copied().collect::<Vec<_>>().join(", ");
        let more = names.len().saturating_sub(3);
        let tools = if more > 0 {
            format!("{shown} +{more}")
        } else {
            shown
        };
        let template = match n {
            0 => return self.text.none.to_string(),
            1 => self.text.one,
            _ => self.text.many,
        };
        template
            .replace("{n}", &n.to_string())
            .replace("{tools}", &tools)
    }

    /// Build the block into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let open = self.starts_open();
        let errors = self
            .calls
            .iter()
            .filter(|c| c.state == ChatCallState::Error)
            .count();

        let text = self.summary_text();
        let mut summary = vec![el(El::Span).st([St::TextSm, St::TextMuted]).text(&text)];
        if errors > 0 {
            summary.push(
                el(El::Span)
                    .st([St::TextXs, St::TextError, St::WhitespaceNowrap])
                    .text(&if errors > 1 {
                        self.text.failure_many.replace("{n}", &errors.to_string())
                    } else {
                        self.text.failure_one.to_string()
                    }),
            );
        }

        let mut calls = Vec::with_capacity(self.calls.len());
        for call in self.calls {
            let line = vec![
                el(El::Span)
                    .st([St::TextSm, St::FontSemibold, St::TextHigh])
                    .text(call.name.as_ref()),
                el(El::Code)
                    .st([
                        St::FontMono,
                        St::TextXs,
                        St::TextMuted,
                        St::Truncate,
                        St::MinW0,
                        St::Flex1,
                    ])
                    .text(call.summary.as_ref()),
                el(El::Span)
                    .st([St::TextXs, St::WhitespaceNowrap, call.state.ink()])
                    .text(call.state.label(&self.text)),
            ];
            let mut rows: Vec<ElementBuilder> = Vec::new();
            match call.detail {
                Some(detail) => {
                    let mut inner = el(El::Details).st([St::TextSm]).append([
                        el(El::Summary)
                            .st([
                                St::DisplayFlex,
                                St::ItemsCenter,
                                St::GapSm,
                                St::MinW0,
                                St::CursorPointer,
                            ])
                            .append(line),
                        detail,
                    ]);
                    if call.state != ChatCallState::Ok {
                        inner = inner.bool_attr(At::Open);
                    }
                    rows.push(inner);
                }
                None => {
                    rows.push(
                        el(El::Div)
                            .st([
                                St::DisplayFlex,
                                St::ItemsCenter,
                                St::GapSm,
                                St::MinW0,
                                St::PyXs,
                            ])
                            .append(line),
                    );
                }
            }
            calls.push(
                el(El::Div)
                    .st([St::DisplayFlex, St::FlexCol, St::GapXs, St::MinW0])
                    .append(rows),
            );
        }

        // No chrome of its own: the run sits on the transcript like a line of
        // text, never in a filled rounded block that would read as a message
        // bubble of its own. The `<summary>` stays a list item (a flex one
        // would lose the native triangle), and the expanded calls are indented
        // under it.
        let mut block = el(El::Details).st([St::MinW0, St::TextSm]).append([
            el(El::Summary)
                .st([St::CursorPointer, St::MinW0])
                .append([el(El::Span)
                    .st([
                        St::DisplayInlineFlex,
                        St::ItemsCenter,
                        St::GapSm,
                        St::FlexWrap,
                        St::MinW0,
                    ])
                    .append(summary)]),
            el(El::Div)
                .st([St::DisplayFlex, St::FlexCol, St::GapSm, St::MinW0, St::PlMd])
                .append(calls),
        ]);
        if open {
            block = block.bool_attr(At::Open);
        }
        if let Some(extra) = self.extra_class {
            block = block.class(extra.as_ref());
        }
        block
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> ChatCalls {
        ChatCalls::new()
            .call(ChatCall::new("Bash", "cargo test").state(ChatCallState::Ok))
            .call(ChatCall::new("Bash", "cargo clippy").state(ChatCallState::Ok))
            .call(ChatCall::new("Read", "src/lib.rs").state(ChatCallState::Ok))
    }

    #[test]
    fn the_summary_counts_the_calls_and_names_the_tools_once() {
        let html = run().build().to_static_html();
        assert!(html.contains("3 tool calls"), "{html}");
        assert!(html.contains("Bash, Read"), "{html}");
    }

    #[test]
    fn a_run_of_successes_stays_collapsed() {
        let calls = run();
        assert!(!calls.starts_open());
        assert!(!calls.build().to_static_html().contains("open"));
    }

    #[test]
    fn the_run_carries_no_bubble_chrome() {
        // A filled rounded block would pass for a message bubble: the run is a
        // plain line of the transcript, so its own box paints nothing.
        let html = run().build().to_static_html();
        let head = html.split("<summary").next().unwrap_or(&html).to_string();
        assert!(!head.contains("background"), "{head}");
        assert!(!head.contains("border-radius"), "{head}");
        assert!(!head.contains("padding"), "{head}");
    }

    #[test]
    fn the_block_stays_closed_even_while_running_or_after_a_failure() {
        let running = ChatCalls::new().call(ChatCall::new("Bash", "cargo test"));
        assert!(
            !running.starts_open(),
            "closed by default, even while running"
        );
        let failed = run().call(
            ChatCall::new("Edit", "src/lib.rs")
                .state(ChatCallState::Error)
                .detail(el(El::Pre).text("permission denied")),
        );
        assert!(
            !failed.starts_open(),
            "closed by default, even after a failure"
        );
        let html = failed.build().to_static_html();
        // The failure reads in the summary without opening the block: the
        // failing call opens, not the run.
        assert!(html.contains("1 failure"), "{html}");
        let outer = html.split('>').next().unwrap_or_default();
        assert!(!outer.contains("open"), "run open: {outer}");
    }

    /// The summary carries the native triangle (no "details" label) and the
    /// expanded calls are indented under it.
    #[test]
    fn the_summary_keeps_its_triangle_and_the_calls_are_indented() {
        let html = run().build().to_static_html();
        assert!(html.contains("<summary"), "{html}");
        // No "details" word in the summary: the native triangle is the whole
        // affordance, the count and the tool names are the label.
        let line = html
            .split("<summary")
            .nth(1)
            .and_then(|r| r.split("</summary>").next())
            .unwrap_or_default();
        assert!(!line.contains("details"), "{line}");
        assert!(line.contains("3 tool calls"), "{line}");
        // A flex `<summary>` would lose its marker: the line is a span.
        let summary = line.split('>').next().unwrap_or_default();
        assert!(!summary.contains("display:flex"), "summary flex: {summary}");
        // The calls are indented under the summary (`PlMd`).
        assert!(
            html.contains(&format!("u{}", rwire::style_tokens::St::PlMd.as_u16())),
            "{html}"
        );
    }

    #[test]
    fn a_single_call_reads_in_the_singular() {
        let one = ChatCalls::new().call(ChatCall::new("Bash", "ls"));
        assert_eq!(one.summary_text(), "1 tool call · Bash");
    }

    #[test]
    fn more_than_three_tools_are_counted() {
        let many = ChatCalls::new()
            .call(ChatCall::new("Bash", "a"))
            .call(ChatCall::new("Read", "b"))
            .call(ChatCall::new("Edit", "c"))
            .call(ChatCall::new("Grep", "d"));
        assert_eq!(many.summary_text(), "4 tool calls · Bash, Read, Edit +1");
    }

    #[test]
    fn the_app_brings_its_own_words() {
        let text = ChatCallsText {
            none: "Nothing ran",
            one: "1 call ({tools})",
            many: "{n} calls ({tools})",
            failure_one: "1 failed",
            failure_many: "{n} failed",
            running: "in flight",
            ..ChatCallsText::default()
        };
        assert_eq!(
            ChatCalls::new().text(text.clone()).summary_text(),
            "Nothing ran"
        );
        assert_eq!(
            ChatCalls::new()
                .call(ChatCall::new("Bash", "ls"))
                .text(text.clone())
                .summary_text(),
            "1 call (Bash)"
        );
        let html = run()
            .text(text)
            .call(ChatCall::new("Read", "src/lib.rs").state(ChatCallState::Running))
            .call(ChatCall::new("Edit", "src/a.rs").state(ChatCallState::Error))
            .call(ChatCall::new("Write", "src/b.rs").state(ChatCallState::Error))
            .build()
            .to_static_html();
        assert!(html.contains("6 calls (Bash, Read, Edit +1)"), "{html}");
        assert!(html.contains("2 failed"), "{html}");
        assert!(html.contains("in flight"), "{html}");
    }

    #[test]
    fn the_block_can_be_forced_open_or_closed() {
        assert!(run().open(true).build().to_static_html().contains("open"));
        let mut failed = run();
        failed = failed.open(false);
        assert!(!failed.starts_open());
    }
}
