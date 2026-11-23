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
    trailing: Option<ElementBuilder>,
    body: ElementBuilder,
    detail: Option<ChatDetail>,
    grouped: bool,
    streaming: bool,
    bubble: bool,
    rail: bool,
}

impl ChatEntry {
    /// Start an entry from its author and body.
    pub fn new(author: ChatAuthor, body: ElementBuilder) -> Self {
        Self {
            key: None,
            author,
            time: None,
            tag: None,
            trailing: None,
            body,
            detail: None,
            grouped: false,
            streaming: false,
            bubble: false,
            rail: true,
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
        entry.trailing = item.trailing(ctx);
        entry.detail = item.detail(ctx);
        entry.grouped = grouped;
        entry.streaming = item.streaming();
        entry.bubble = item.bubble();
        entry.rail = item.rail();
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

    /// A trailing header element pushed to the right edge — a status chip, a running dot, an
    /// action. A spacer separates it from the author·tag·time cluster.
    pub fn trailing(mut self, trailing: ElementBuilder) -> Self {
        self.trailing = Some(trailing);
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

    /// Render as a bubble: the local user's turn, right-aligned on a tinted
    /// rounded block instead of the accent rail of a full-width entry.
    pub fn bubble(mut self, bubble: bool) -> Self {
        self.bubble = bubble;
        self
    }

    /// The hairline tone rail down the left edge (default on). Turn it off for a
    /// surface that wants the body flush with the column: no lateral border, the
    /// text sitting on the page.
    pub fn rail(mut self, rail: bool) -> Self {
        self.rail = rail;
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
    fn rail_strip(intent: ChatIntent) -> ElementBuilder {
        el(El::Span)
            .style(Style::new().width("2px").set("flex", "0 0 2px"))
            .st([St::SelfStretch, Self::intent_rail(intent)])
    }

    /// Build into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let intent = self.author.intent;
        let bubble = self.bubble;

        // Header: author · tag · time (suppressed when grouped). In a bubble the
        // author is implied by the side the block sits on, so the header keeps
        // only the time and the name for screen readers.
        let mut column: Vec<ElementBuilder> = Vec::new();
        if bubble {
            if let Some(time) = &self.time {
                column.push(
                    el(El::Div)
                        .st([
                            St::DisplayFlex,
                            St::JustifyEnd,
                            St::ItemsCenter,
                            St::TextLow,
                        ])
                        .append([el(El::Span).st([St::TextXs]).text(time.as_ref())]),
                );
            }
        } else if !self.grouped {
            // The name is plain text unless the app supplied a node (a link to
            // the author's own session); the suffix then qualifies it.
            let name = match self.author.name_node {
                Some(node) => el(El::Span)
                    .st([St::FontBold, St::TextSm, Self::intent_text(intent)])
                    .append([node]),
                None => el(El::Span)
                    .st([St::FontBold, St::TextSm, Self::intent_text(intent)])
                    .text(self.author.name.as_ref()),
            };
            let mut header = vec![name];
            if let Some(suffix) = &self.author.suffix {
                header.push(
                    el(El::Span)
                        .st([St::TextXs, St::TextMuted])
                        .text(suffix.as_ref()),
                );
            }
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
            if let Some(trailing) = self.trailing {
                // A spacer pushes the trailing element (status chip / running dot) to the right.
                header.push(el(El::Span).st([St::Flex1]));
                header.push(trailing);
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

        if bubble {
            // The user's turn: tinted rounded block on the right, width follows
            // the content up to a share of the column — the model's own output
            // stays plain and full width.
            let content = el(El::Div)
                .st([
                    St::DisplayFlex,
                    St::FlexCol,
                    St::GapXs,
                    St::MinW0,
                    St::BgMuted,
                    St::RoundedLg,
                    St::PxMd,
                    St::PySm,
                    St::TextSm,
                ])
                .style(Style::new().set("max-width", "min(85%, 46rem)"))
                .append([el(El::Span)
                    .st([St::SrOnly])
                    .text(self.author.name.as_ref())])
                .append(column);
            let mut root = el(El::Div).st([St::DisplayFlex, St::JustifyEnd, St::MinW0]);
            if let Some(key) = self.key {
                root = root.id(format!("ce-{key}").as_str());
            }
            return root.append([content]);
        }

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

        let mut row: Vec<ElementBuilder> = Vec::new();
        if self.rail {
            row.push(Self::rail_strip(intent));
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The header row is the first child of the entry's content block (rail, [avatar], block).
    fn header_texts(entry: &ElementBuilder) -> Vec<&str> {
        let block = entry.children().last().unwrap();
        block.children()[0]
            .children()
            .iter()
            .filter_map(|child| child.text_content())
            .collect()
    }

    #[test]
    fn trailing_renders_at_the_header_right() {
        let entry = ChatEntry::new(ChatAuthor::agent("claw"), el(El::P).text("hi"))
            .time("12:00")
            .trailing(el(El::Span).text("✓ pass"))
            .build();
        let texts = header_texts(&entry);
        assert!(texts.contains(&"claw"));
        assert!(
            texts.contains(&"✓ pass"),
            "trailing shows in the header: {texts:?}"
        );
    }

    #[test]
    fn no_trailing_by_default() {
        let entry = ChatEntry::new(ChatAuthor::agent("claw"), el(El::P).text("hi")).build();
        assert_eq!(header_texts(&entry), vec!["claw"]);
    }

    #[test]
    fn the_name_can_be_a_node_with_a_muted_suffix() {
        // A multi-author thread links the name to the author's own session and
        // qualifies it (« terminal », « via MCP »).
        let link = el(El::A)
            .attr("href", "/s/office/flambeau")
            .text("flambeau@office");
        let entry = ChatEntry::new(
            ChatAuthor::agent("flambeau@office")
                .name_node(link)
                .suffix("via MCP"),
            el(El::P).text("fais le lint"),
        )
        .build();
        let html = entry.to_static_html();
        assert!(html.contains("href=\"/s/office/flambeau\""), "{html}");
        assert!(html.contains("flambeau@office"), "{html}");
        assert!(html.contains("via MCP"), "{html}");
        // The suffix follows the name, inside the same header row.
        assert_eq!(header_texts(&entry), vec!["via MCP"]);
    }

    #[test]
    fn without_a_node_the_name_stays_plain_text() {
        let entry = ChatEntry::new(
            ChatAuthor::user("Vous").suffix("terminal"),
            el(El::P).text("hi"),
        )
        .build();
        let html = entry.to_static_html();
        assert!(!html.contains("<a "), "{html}");
        assert_eq!(header_texts(&entry), vec!["Vous", "terminal"]);
    }
}
