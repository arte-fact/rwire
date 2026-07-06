//! ChatEntry component.
//!
//! One authored transcript entry: an accent rail, an optional avatar slot, a
//! header row (author · tag · time), the body, and an optional native
//! `<details>` disclosure. The layout mirrors the pattern proven in
//! claw-rwire's thread widgets (`MinW0`/`BreakWords` so bodies wrap at the
//! column edge; rail via a hairline flex strip).
//!
//! Use it à la carte, or let [`ChatTranscript`](crate::ChatTranscript) build
//! entries from a [`ChatItem`](crate::ChatItem) implementation.

use std::borrow::Cow;

use rwire::style::Style;
use rwire::style_tokens::St;
use rwire::{el, At, El, ElementBuilder};

use crate::chat_item::{ChatAuthor, ChatDetail, ChatIntent, ChatItem, ChatItemCtx, ChatTag};
use crate::TypingIndicator;

/// ChatEntry builder.
pub struct ChatEntry {
    key: Option<Cow<'static, str>>,
    author: ChatAuthor,
    time: Option<Cow<'static, str>>,
    tag: Option<ChatTag>,
    body: ElementBuilder,
    detail: Option<ChatDetail>,
    grouped: bool,
    streaming: bool,
}

impl ChatEntry {
    /// Start an entry from its author and body.
    pub fn new(author: ChatAuthor, body: ElementBuilder) -> Self {
        Self {
            key: None,
            author,
            time: None,
            tag: None,
            body,
            detail: None,
            grouped: false,
            streaming: false,
        }
    }

    /// Build an entry (or a chrome-free row) from a [`ChatItem`].
    pub fn from_item<T: ChatItem>(item: &T, ctx: &ChatItemCtx<T>, grouped: bool) -> ElementBuilder {
        if let Some(row) = item.row(ctx) {
            return row;
        }
        let mut entry = ChatEntry::new(item.author(), item.body(ctx));
        entry.key = Some(Cow::Owned(item.key().into_owned()));
        entry.time = item.time().map(|t| Cow::Owned(t.into_owned()));
        entry.tag = item.tag();
        entry.detail = item.detail(ctx);
        entry.grouped = grouped;
        entry.streaming = item.streaming();
        entry.build()
    }

    /// Stable identity → DOM id `ce-{key}` (keyed morph reuse).
    pub fn key(mut self, key: impl Into<Cow<'static, str>>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Preformatted timestamp for the header.
    pub fn time(mut self, time: impl Into<Cow<'static, str>>) -> Self {
        self.time = Some(time.into());
        self
    }

    /// Status/phase tag beside the author.
    pub fn tag(mut self, tag: ChatTag) -> Self {
        self.tag = Some(tag);
        self
    }

    /// Collapsible detail under the body.
    pub fn detail(mut self, detail: ChatDetail) -> Self {
        self.detail = Some(detail);
        self
    }

    /// Suppress the header (consecutive entries from the same author).
    pub fn grouped(mut self, grouped: bool) -> Self {
        self.grouped = grouped;
        self
    }

    /// Append a subtle writing cue (this entry is still being produced).
    pub fn streaming(mut self, streaming: bool) -> Self {
        self.streaming = streaming;
        self
    }

    fn intent_text(intent: ChatIntent) -> St {
        match intent {
            ChatIntent::Default => St::TextHigh,
            ChatIntent::Accent => St::TextAccent,
            ChatIntent::Muted => St::TextMuted,
            ChatIntent::Danger => St::TextError,
        }
    }

    fn intent_rail(intent: ChatIntent) -> St {
        match intent {
            ChatIntent::Accent => St::BgAccent,
            ChatIntent::Danger => St::BgError,
            _ => St::BgMuted,
        }
    }

    /// The hairline accent rail (claw's proven 2px flex strip).
    fn rail(intent: ChatIntent) -> ElementBuilder {
        el(El::Span)
            .style(Style::new().width("2px").set("flex", "0 0 2px"))
            .st([St::SelfStretch, Self::intent_rail(intent)])
    }

    /// Build into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let intent = self.author.intent;

        // Header: author · tag · time (suppressed when grouped).
        let mut column: Vec<ElementBuilder> = Vec::new();
        if !self.grouped {
            let mut header = vec![el(El::Span)
                .st([St::FontBold, St::TextSm, Self::intent_text(intent)])
                .text(self.author.name.as_ref())];
            if let Some(tag) = self.tag {
                header.push(
                    el(El::Span)
                        .st([St::TextXs, Self::intent_text(tag.intent)])
                        .text(tag.label.as_ref()),
                );
            }
            if let Some(time) = &self.time {
                header.push(
                    el(El::Span)
                        .st([St::TextLow, St::TextXs])
                        .text(time.as_ref()),
                );
            }
            column.push(
                el(El::Div)
                    .st([St::DisplayFlex, St::ItemsCenter, St::GapSm])
                    .append(header),
            );
        }

        // Body (+ optional writing cue, + optional disclosure).
        let mut body_col = vec![self.body];
        if self.streaming {
            body_col.push(TypingIndicator::new().build());
        }
        if let Some(detail) = self.detail {
            let mut details = el(El::Details).st([St::TextSm]).append([
                el(El::Summary)
                    .st([St::TextXs, St::TextMuted, St::CursorPointer])
                    .text(detail.summary.as_ref()),
                detail.body,
            ]);
            if detail.open {
                details = details.bool_attr(At::Open);
            }
            body_col.push(details);
        }
        column.push(
            el(El::Div)
                .st([St::DisplayFlex, St::FlexCol, St::GapXs, St::MinW0])
                .append(body_col),
        );

        // MinW0 + BreakWords: wrap at the column edge instead of overflowing.
        let block = el(El::Div)
            .st([
                St::Flex1,
                St::MinW0,
                St::BreakWords,
                St::DisplayFlex,
                St::FlexCol,
                St::GapXs,
            ])
            .append(column);

        let mut row: Vec<ElementBuilder> = vec![Self::rail(intent)];
        if let Some(avatar) = self.author.avatar {
            row.push(
                el(El::Span)
                    .style(Style::new().width("1.25rem").set("flex", "0 0 1.25rem"))
                    .st([St::DisplayFlex, St::JustifyCenter])
                    .append([avatar]),
            );
        }
        row.push(block);

        let mut root = el(El::Div).st([St::DisplayFlex, St::GapMd, St::MinW0]);
        if let Some(key) = self.key {
            root = root.id(format!("ce-{key}").as_str());
        }
        root.append(row)
    }
}
