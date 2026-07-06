//! ChatTranscript component.
//!
//! The windowed entry list of a chat surface: renders [`ChatItem`]s through
//! [`ChatEntry`], arms the F1 scroll sentinel at the top for **seamless
//! history** (older turns load as you scroll up — no button), shows an empty
//! state, and appends the writing-state row. Pair with
//! [`ChatScroll`](crate::ChatScroll)/[`Chat`](crate::Chat) for the
//! bottom-pinned scroller.
//!
//! History loading is scroll-anchor-stable by construction: the scroller is
//! `column-reverse` (scroll origin at the bottom), so prepending older entries
//! above doesn't move what the reader is looking at.

use std::borrow::Cow;

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder, HandlerSpec, ItemRef};

use crate::chat_item::{ChatItem, ChatItemCtx};
use crate::{ChatEntry, Spinner, SpinnerSize, TypingIndicator};

/// ChatTranscript builder.
#[derive(Default)]
pub struct ChatTranscript {
    rows: Vec<ElementBuilder>,
    older: Option<(HandlerSpec, u32)>,
    writing: Option<Option<Cow<'static, str>>>,
    empty: Option<ElementBuilder>,
}

impl ChatTranscript {
    pub fn new() -> Self {
        Self::default()
    }

    /// Render items bound with `iter_with_ref()` — bodies get the `ItemRef`
    /// for per-item handlers. Items oldest first.
    pub fn items<'a, T, I>(mut self, items: I) -> Self
    where
        T: ChatItem + 'a,
        I: IntoIterator<Item = (ItemRef<T>, &'a T)>,
    {
        let mut prev: Option<&'a T> = None;
        for (item_ref, item) in items {
            let ctx = ChatItemCtx {
                item_ref: Some(item_ref),
            };
            let grouped = prev.is_some_and(|p| item.groups_with(p));
            self.rows.push(ChatEntry::from_item(item, &ctx, grouped));
            prev = Some(item);
        }
        self
    }

    /// Render items without refs (read-only bodies). Items oldest first.
    pub fn items_plain<'a, T, I>(mut self, items: I) -> Self
    where
        T: ChatItem + 'a,
        I: IntoIterator<Item = &'a T>,
    {
        let mut prev: Option<&'a T> = None;
        for item in items {
            let ctx = ChatItemCtx { item_ref: None };
            let grouped = prev.is_some_and(|p| item.groups_with(p));
            self.rows.push(ChatEntry::from_item(item, &ctx, grouped));
            prev = Some(item);
        }
        self
    }

    /// Seamless history: when `has_older`, a sentinel row at the top fires
    /// `handler` with `next` (read via `ctx.item_index()`; pass the current
    /// window start so stale fires are detectable) as the reader scrolls up.
    pub fn on_load_older(mut self, handler: HandlerSpec, next: u32, has_older: bool) -> Self {
        self.older = if has_older {
            Some((handler, next))
        } else {
            None
        };
        self
    }

    /// Show the writing-state row (someone/something is producing a turn),
    /// with an optional label ("claw is typing…").
    pub fn writing(mut self, label: Option<Cow<'static, str>>) -> Self {
        self.writing = Some(label);
        self
    }

    /// Rendered when there are no entries (and no history above).
    pub fn empty_state(mut self, empty: ElementBuilder) -> Self {
        self.empty = Some(empty);
        self
    }

    /// Build the inner column (wrap in `ChatScroll` — or use [`Chat`](crate::Chat)).
    pub fn build(self) -> ElementBuilder {
        let mut column: Vec<ElementBuilder> = Vec::new();
        if let Some((handler, next)) = self.older {
            column.push(
                el(El::Div)
                    .id("ct-older")
                    .st([St::DisplayFlex, St::JustifyCenter, St::PSm])
                    .on_visible(handler, next)
                    .append([Spinner::new().size(SpinnerSize::Sm).build()]),
            );
        } else if self.rows.is_empty() {
            if let Some(empty) = self.empty {
                column.push(empty);
            }
        }
        column.extend(self.rows);
        if let Some(label) = self.writing {
            let mut t = TypingIndicator::new();
            if let Some(label) = label {
                t = t.label(label);
            }
            column.push(t.build());
        }
        el(El::Div)
            .st([St::DisplayFlex, St::FlexCol, St::GapMd, St::MinW0])
            .append(column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_item::{ChatAuthor, ChatDetail, ChatTag};
    use rwire::{ChangeSet, HandlerSpec};
    use std::borrow::Cow;

    enum Item {
        Msg {
            id: u32,
            who: &'static str,
            text: &'static str,
        },
        Tool {
            id: u32,
            name: &'static str,
            ok: bool,
            result: &'static str,
        },
        Fold,
    }

    impl ChatItem for Item {
        fn key(&self) -> Cow<'_, str> {
            match self {
                Item::Msg { id, .. } => Cow::Owned(format!("m{id}")),
                Item::Tool { id, .. } => Cow::Owned(format!("t{id}")),
                Item::Fold => Cow::Borrowed("fold"),
            }
        }
        fn author(&self) -> ChatAuthor {
            match self {
                Item::Msg { who: "you", .. } => ChatAuthor::user("you"),
                Item::Msg { .. } => ChatAuthor::agent("claw"),
                _ => ChatAuthor::agent("claw").muted(),
            }
        }
        fn body(&self, _ctx: &ChatItemCtx<Self>) -> ElementBuilder {
            match self {
                Item::Msg { text, .. } => el(El::P).text(text),
                Item::Tool { name, .. } => el(El::Code).text(name),
                Item::Fold => el(El::Span),
            }
        }
        fn tag(&self) -> Option<ChatTag> {
            match self {
                Item::Tool { name, ok: true, .. } => Some(ChatTag::muted(name.to_string())),
                Item::Tool {
                    name, ok: false, ..
                } => Some(ChatTag::danger(name.to_string())),
                _ => None,
            }
        }
        fn detail(&self, _ctx: &ChatItemCtx<Self>) -> Option<ChatDetail> {
            match self {
                Item::Tool { result, .. } => {
                    Some(ChatDetail::closed("result", el(El::Pre).text(result)))
                }
                _ => None,
            }
        }
        fn row(&self, _ctx: &ChatItemCtx<Self>) -> Option<ElementBuilder> {
            match self {
                Item::Fold => Some(
                    el(El::Div)
                        .id("ce-fold")
                        .st([St::TextCenter, St::TextMuted])
                        .text("· older messages condensed ·"),
                ),
                _ => None,
            }
        }
        fn groups_with(&self, prev: &Self) -> bool {
            matches!((self, prev), (Item::Msg { who: a, .. }, Item::Msg { who: b, .. }) if a == b)
        }
    }

    #[derive(rwire::State, Default)]
    #[storage(memory)]
    struct S;

    fn handler() -> HandlerSpec {
        fn noop(_s: &mut S) {}
        HandlerSpec::from_fn_with_changes::<S>(noop, ChangeSet::all())
            .with_handler_id(rwire::stable_handler_id("chat_transcript_tests", "noop"))
    }

    fn items() -> Vec<Item> {
        vec![
            Item::Fold,
            Item::Msg {
                id: 1,
                who: "you",
                text: "hi",
            },
            Item::Msg {
                id: 2,
                who: "you",
                text: "again",
            },
            Item::Tool {
                id: 3,
                name: "bash",
                ok: true,
                result: "ok",
            },
            Item::Msg {
                id: 4,
                who: "claw",
                text: "done",
            },
        ]
    }

    #[test]
    fn renders_rows_row_override_and_grouping() {
        let built = ChatTranscript::new().items_plain(items().iter()).build();
        let rows = built.children();
        assert_eq!(rows.len(), 5);
        // row() override: chrome-free fold line kept as-is.
        assert_eq!(rows[0].text_content(), Some("· older messages condensed ·"));
        // grouped second consecutive "you" message: header suppressed means its
        // block has one child (body col) instead of two (header + body col).
        let first_cols = rows[1].children().last().unwrap().children().len();
        let grouped_cols = rows[2].children().last().unwrap().children().len();
        assert_eq!(first_cols, 2, "ungrouped entry has header + body");
        assert_eq!(grouped_cols, 1, "grouped entry suppresses the header");
    }

    #[test]
    fn sentinel_and_writing_and_empty_states() {
        let with_history = ChatTranscript::new()
            .items_plain(items().iter())
            .on_load_older(handler(), 7, true)
            .writing(Some("claw is typing…".into()))
            .build();
        let rows = with_history.children();
        assert_eq!(rows.len(), 7, "sentinel + 5 items + typing row");

        let empty = ChatTranscript::new()
            .empty_state(el(El::P).text("say hi"))
            .build();
        assert_eq!(empty.children().len(), 1);
        assert_eq!(empty.children()[0].text_content(), Some("say hi"));

        let complete = ChatTranscript::new()
            .items_plain(items().iter())
            .on_load_older(handler(), 0, false)
            .build();
        assert_eq!(
            complete.children().len(),
            5,
            "no sentinel when history exhausted"
        );
    }
}
