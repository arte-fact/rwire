//! ChatCalls component.
//!
//! A run of tool calls collapsed into **one** transcript block: a summary line
//! ("4 appels d'outils · Bash, Read, Edit") and, behind a native `<details>`,
//! each call with its argument summary, its state and its own disclosure for the
//! full input and output. Native `<details>` means the toggle costs no server
//! round-trip and keeps keyboard and screen-reader semantics.
//!
//! It fits where a chat surface would otherwise interleave dozens of one-line
//! tool rows with the actual conversation: the prose stays readable, and the
//! machinery is one click away. The block opens by itself while a call is still
//! running or when one failed, and stays closed once everything succeeded.
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
    fn label(self) -> &'static str {
        match self {
            Self::Running => "en cours",
            Self::Ok => "ok",
            Self::Error => "erreur",
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

    /// Force the block open or closed. By default it opens while a call runs or
    /// after a failure, and stays closed once every call succeeded.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Add a custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Whether the block starts expanded.
    pub fn starts_open(&self) -> bool {
        self.open
            .unwrap_or_else(|| self.calls.iter().any(|c| c.state != ChatCallState::Ok))
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
        match n {
            0 => "Aucun appel d'outil".to_string(),
            1 => format!("1 appel d'outil · {tools}"),
            n => format!("{n} appels d'outils · {tools}"),
        }
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
        let mut summary = vec![
            el(El::Span)
                .st([St::TextSm, St::FontSemibold, St::TextHigh])
                .text(&text),
            el(El::Span)
                .st([St::TextXs, St::TextMuted, St::WhitespaceNowrap])
                .text("détails"),
        ];
        if errors > 0 {
            summary.push(
                el(El::Span)
                    .st([St::TextXs, St::TextError, St::WhitespaceNowrap])
                    .text(&format!(
                        "{errors} échec{}",
                        if errors > 1 { "s" } else { "" }
                    )),
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
                    .text(call.state.label()),
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

        let mut block = el(El::Details)
            .st([
                St::DisplayFlex,
                St::FlexCol,
                St::GapXs,
                St::MinW0,
                St::PSm,
                St::BgMuted,
                St::RoundedMd,
                St::TextSm,
            ])
            .append([
                el(El::Summary)
                    .st([
                        St::DisplayFlex,
                        St::ItemsCenter,
                        St::GapSm,
                        St::FlexWrap,
                        St::CursorPointer,
                    ])
                    .append(summary),
                el(El::Div)
                    .st([St::DisplayFlex, St::FlexCol, St::GapSm, St::MinW0, St::MtXs])
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
        assert!(
            html.contains("3 appels d&#39;outils") || html.contains("3 appels d'outils"),
            "{html}"
        );
        assert!(html.contains("Bash, Read"), "{html}");
    }

    #[test]
    fn a_run_of_successes_stays_collapsed() {
        let calls = run();
        assert!(!calls.starts_open());
        assert!(!calls.build().to_static_html().contains("open"));
    }

    #[test]
    fn a_running_or_failed_call_opens_the_block() {
        let running = ChatCalls::new().call(ChatCall::new("Bash", "cargo test"));
        assert!(running.starts_open());
        let failed = run().call(
            ChatCall::new("Edit", "src/lib.rs")
                .state(ChatCallState::Error)
                .detail(el(El::Pre).text("permission denied")),
        );
        assert!(failed.starts_open());
        let html = failed.build().to_static_html();
        assert!(html.contains("1 échec"), "{html}");
        assert!(html.contains("permission denied"), "{html}");
    }

    #[test]
    fn a_single_call_reads_in_the_singular() {
        let one = ChatCalls::new().call(ChatCall::new("Bash", "ls"));
        assert_eq!(one.summary_text(), "1 appel d'outil · Bash");
    }

    #[test]
    fn more_than_three_tools_are_counted() {
        let many = ChatCalls::new()
            .call(ChatCall::new("Bash", "a"))
            .call(ChatCall::new("Read", "b"))
            .call(ChatCall::new("Edit", "c"))
            .call(ChatCall::new("Grep", "d"));
        assert_eq!(
            many.summary_text(),
            "4 appels d'outils · Bash, Read, Edit +1"
        );
    }

    #[test]
    fn the_block_can_be_forced_open_or_closed() {
        assert!(run().open(true).build().to_static_html().contains("open"));
        let mut failed = run();
        failed = failed.open(false);
        assert!(!failed.starts_open());
    }
}
