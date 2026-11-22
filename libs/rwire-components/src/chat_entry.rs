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
use rwire::{el, At, Av, El, ElementBuilder, ProseFace, Tone};

use crate::chat_item::{
    ChatAuthor, ChatDelivery, ChatDetail, ChatIntent, ChatItem, ChatItemCtx, ChatRole, ChatTag,
    Delivery,
};
use crate::TypingIndicator;

/// How a transcript lays out its entries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChatLayout {
    /// A hairline rail, an optional small avatar, the header and the body.
    #[default]
    Rail,
    /// A grid: a square avatar (30 px, 26 px on a phone, radius 8) beside the
    /// header (name and time) and the body (a bubble for the user, the
    /// reading face for an agent).
    Avatar,
}

/// ChatEntry builder.
pub struct ChatEntry {
    layout: ChatLayout,
    bubble: Option<bool>,
    prose: Option<ProseFace>,
    key: Option<Cow<'static, str>>,
    author: ChatAuthor,
    time: Option<Cow<'static, str>>,
    tag: Option<ChatTag>,
    trailing: Option<ElementBuilder>,
    body: ElementBuilder,
    detail: Option<ChatDetail>,
    delivery: Option<ChatDelivery>,
    grouped: bool,
    streaming: bool,
}

impl ChatEntry {
    /// Start an entry from its author and body.
    pub fn new(author: ChatAuthor, body: ElementBuilder) -> Self {
        Self {
            layout: ChatLayout::Rail,
            bubble: None,
            prose: None,
            key: None,
            author,
            time: None,
            tag: None,
            trailing: None,
            body,
            detail: None,
            delivery: None,
            grouped: false,
            streaming: false,
        }
    }

    /// Build an entry (or a chrome-free row) from a [`ChatItem`].
    pub fn from_item<T: ChatItem>(item: &T, ctx: &ChatItemCtx<T>, grouped: bool) -> ElementBuilder {
        Self::from_item_in(item, ctx, grouped, ChatLayout::Rail)
    }

    /// [`ChatEntry::from_item`] in a given layout.
    pub fn from_item_in<T: ChatItem>(
        item: &T,
        ctx: &ChatItemCtx<T>,
        grouped: bool,
        layout: ChatLayout,
    ) -> ElementBuilder {
        if let Some(row) = item.row(ctx) {
            return row;
        }
        let mut entry = ChatEntry::new(item.author(), item.body(ctx));
        entry.layout = layout;
        entry.bubble = item.bubble();
        entry.prose = item.prose();
        entry.key = Some(Cow::Owned(item.key().into_owned()));
        entry.time = item.time().map(|t| Cow::Owned(t.into_owned()));
        entry.tag = item.tag();
        entry.trailing = item.trailing(ctx);
        entry.detail = item.detail(ctx);
        entry.delivery = item.delivery(ctx);
        entry.grouped = grouped;
        entry.streaming = item.streaming();
        entry.build()
    }

    /// The layout (`Rail` by default; a transcript passes its own).
    pub fn layout(mut self, layout: ChatLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Set the body in a bubble (radius 12, the raised ground, 14.5 px). By
    /// default the user's entries are bubbles.
    pub fn bubble(mut self, bubble: bool) -> Self {
        self.bubble = Some(bubble);
        self
    }

    /// The body's face: `Read` (by default for an agent) or `Ui`.
    pub fn prose(mut self, face: ProseFace) -> Self {
        self.prose = Some(face);
        self
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

    /// The outbox state of an outgoing entry: a status line under the body
    /// (`role=status`, `role=alert` when failed) plus an actions row (Resend,
    /// Withdraw). `Sending` sets the entry at reduced opacity; `Unconfirmed`
    /// and `Failed` give the body a dashed outline.
    pub fn delivery(
        mut self,
        state: Delivery,
        status: impl Into<Cow<'static, str>>,
        actions: impl IntoIterator<Item = ElementBuilder>,
    ) -> Self {
        self.delivery = Some(ChatDelivery {
            state,
            status: status.into(),
            actions: actions.into_iter().collect(),
        });
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

    /// The avatar square of the avatar layout: the author's element, else
    /// the initials on the author's tone (the raised ground for the user).
    fn avatar_square(author: &mut ChatAuthor) -> ElementBuilder {
        if let Some(avatar) = author.avatar.take() {
            return el(El::Span)
                .st([St::SqChatAvatarPhone, St::Rounded8, St::OverflowHidden])
                .sm([St::SqChatAvatar])
                .data("testid", "chat-avatar")
                .append([avatar]);
        }
        let initials = author.initials.clone().unwrap_or_else(|| {
            Cow::Owned(
                author
                    .name
                    .chars()
                    .next()
                    .map(|c| c.to_uppercase().collect())
                    .unwrap_or_default(),
            )
        });
        let colors: Vec<St> = match (author.tone, author.role) {
            (Some(tone), _) => vec![tone.solid(), tone.on_solid_mark()],
            (None, ChatRole::User) => {
                vec![St::BgSurfaceRaised, St::TextMuted, St::BorderDefault]
            }
            (None, _) => vec![Tone::Accent.solid(), Tone::Accent.on_solid()],
        };
        el(El::Span)
            .st([
                St::SqChatAvatarPhone,
                St::Rounded8,
                St::DisplayFlex,
                St::ItemsCenter,
                St::JustifyCenter,
                St::TextChatAvatar,
            ])
            .st(colors)
            .sm([St::SqChatAvatar])
            .attr("aria-hidden", "true")
            .data("testid", "chat-avatar")
            .text(initials.as_ref())
    }

    /// The chrome a delivery state adds to the body (a dashed outline when
    /// unconfirmed or failed).
    fn delivery_body_tokens(state: Delivery) -> Vec<St> {
        match state {
            Delivery::Unconfirmed | Delivery::Failed => vec![St::BorderDashed],
            _ => Vec::new(),
        }
    }

    /// The chrome a delivery state adds to the whole entry (reduced opacity
    /// while sending).
    fn delivery_root_tokens(state: Delivery) -> Vec<St> {
        match state {
            Delivery::Sending => vec![St::Opacity60],
            _ => Vec::new(),
        }
    }

    /// The status line (`role=status`, `role=alert` when failed) and the
    /// actions row of a delivery state, rendered under the body.
    fn delivery_rows(delivery: &ChatDelivery) -> Vec<ElementBuilder> {
        let mut status = el(El::Div)
            .st([St::TextXs, St::TextFaint])
            .data("testid", "chat-delivery");
        status = match delivery.state {
            Delivery::Failed => status.at(At::Role, Av::RoleAlert).st([St::TextError]),
            _ => status.at(At::Role, Av::RoleStatus),
        };
        let mut rows = vec![status.text(delivery.status.as_ref())];
        if !delivery.actions.is_empty() {
            rows.push(
                el(El::Div)
                    .st([St::DisplayFlex, St::GapSm, St::ItemsCenter])
                    .data("testid", "chat-delivery-actions")
                    .append(delivery.actions.clone()),
            );
        }
        rows
    }

    /// The avatar layout (see [`ChatLayout::Avatar`]).
    fn build_avatar(mut self) -> ElementBuilder {
        let user = self.author.role == ChatRole::User;
        let lead = if self.grouped {
            el(El::Span)
        } else {
            Self::avatar_square(&mut self.author)
        };
        let mut column: Vec<ElementBuilder> = Vec::new();
        if !self.grouped {
            let mut header = vec![el(El::B)
                .st([St::FontW650, St::TextDefault])
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
                    el(El::Time)
                        .st([St::Text11_5, St::TextFaint, St::TabularNums])
                        .data("testid", "chat-time")
                        .text(time.as_ref()),
                );
            }
            if let Some(trailing) = self.trailing {
                header.push(el(El::Span).st([St::Flex1]));
                header.push(trailing);
            }
            column.push(
                el(El::Div)
                    .st([St::DisplayFlex, St::ItemsBaseline, St::GapSm, St::Text13])
                    .append(header),
            );
        }
        let delivery_body = self
            .delivery
            .as_ref()
            .map(|d| Self::delivery_body_tokens(d.state))
            .unwrap_or_default();
        let body = if self.bubble.unwrap_or(user) {
            el(El::Div)
                .st([
                    St::Text14_5,
                    St::BgSurfaceRaised,
                    St::Rounded12,
                    St::PadBubble,
                    St::WFitMax,
                    St::TextDefault,
                    St::BreakWords,
                ])
                .st(delivery_body.clone())
                .data("testid", "chat-bubble")
                .append([self.body])
        } else {
            let face = self
                .prose
                .unwrap_or(if self.author.role == ChatRole::Agent {
                    ProseFace::Read
                } else {
                    ProseFace::Ui
                });
            face.apply(
                el(El::Div)
                    .st([St::MinW0, St::BreakWords, St::TextDefault])
                    .st(delivery_body.clone())
                    .data("testid", "chat-prose"),
            )
            .append([self.body])
        };
        let mut body_col = vec![body];
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
        if let Some(delivery) = &self.delivery {
            body_col.extend(Self::delivery_rows(delivery));
        }
        column.extend(body_col);
        let root_tokens = self
            .delivery
            .as_ref()
            .map(|d| Self::delivery_root_tokens(d.state))
            .unwrap_or_default();
        let mut root = el(El::Div)
            .st([St::GridChatAvatarPhone, St::MinW0])
            .st(root_tokens)
            .sm([St::GridChatAvatar])
            .data("testid", "chat-entry")
            .append([
                lead,
                el(El::Div)
                    .st([St::DisplayFlex, St::FlexCol, St::GapXs, St::MinW0])
                    .append(column),
            ]);
        if let Some(key) = self.key {
            root = root.id(format!("ce-{key}").as_str());
        }
        root
    }

    /// Build into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        if self.layout == ChatLayout::Avatar {
            return self.build_avatar();
        }
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
        if let Some(delivery) = &self.delivery {
            body_col.extend(Self::delivery_rows(delivery));
        }
        column.push(
            el(El::Div)
                .st([St::DisplayFlex, St::FlexCol, St::GapXs, St::MinW0])
                .append(body_col),
        );

        // MinW0 + BreakWords: wrap at the column edge instead of overflowing.
        let delivery_body = self
            .delivery
            .as_ref()
            .map(|d| Self::delivery_body_tokens(d.state))
            .unwrap_or_default();
        let block = el(El::Div)
            .st([
                St::Flex1,
                St::MinW0,
                St::BreakWords,
                St::DisplayFlex,
                St::FlexCol,
                St::GapXs,
            ])
            .st(delivery_body)
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

        let root_tokens = self
            .delivery
            .as_ref()
            .map(|d| Self::delivery_root_tokens(d.state))
            .unwrap_or_default();
        let mut root = el(El::Div)
            .st([St::DisplayFlex, St::GapMd, St::MinW0])
            .st(root_tokens);
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
    fn the_avatar_layout_sets_a_square_a_header_and_a_bubble_for_the_user() {
        let entry = ChatEntry::new(ChatAuthor::user("You"), el(El::P).text("hi"))
            .time("12:00")
            .layout(ChatLayout::Avatar)
            .build();
        let html = entry.to_static_html();
        assert!(html.contains("data-testid=\"chat-avatar\""), "{html}");
        assert!(html.contains(">Y<"), "initial: {html}");
        assert!(html.contains("<time "), "{html}");
        assert!(html.contains("data-testid=\"chat-bubble\""), "{html}");
        let grid = entry.get_style_utils();
        assert!(grid.contains(&St::GridChatAvatarPhone.as_u16()));
        let sm: Vec<u16> = entry
            .get_breakpoint_groups()
            .iter()
            .flat_map(|(_, c)| c.iter().copied())
            .collect();
        assert!(sm.contains(&St::GridChatAvatar.as_u16()));
        let avatar = entry.children()[0].get_style_utils();
        for st in [St::SqChatAvatarPhone, St::Rounded8, St::BgSurfaceRaised] {
            assert!(avatar.contains(&st.as_u16()), "{st:?}");
        }
    }

    #[test]
    fn an_agent_reads_in_the_reading_face_on_its_tone() {
        let entry = ChatEntry::new(
            ChatAuthor::agent("claude").tone(Tone::Cat(1)).initials("C"),
            el(El::P).text("answer"),
        )
        .layout(ChatLayout::Avatar)
        .build();
        let html = entry.to_static_html();
        assert!(html.contains("data-testid=\"chat-prose\""), "{html}");
        assert!(!html.contains("chat-bubble"), "{html}");
        let avatar = entry.children()[0].get_style_utils();
        assert!(avatar.contains(&St::BgCat1.as_u16()));
        assert!(avatar.contains(&St::TextCat1Mark.as_u16()));
        let prose = &entry.children()[1].children()[1];
        assert!(prose.get_style_utils().contains(&St::FontRead.as_u16()));
        // The defaults can be overridden.
        let html = ChatEntry::new(ChatAuthor::agent("a"), el(El::P))
            .layout(ChatLayout::Avatar)
            .bubble(true)
            .build()
            .to_static_html();
        assert!(html.contains("chat-bubble"), "{html}");
    }

    #[test]
    fn a_grouped_entry_keeps_the_column_but_drops_the_avatar_and_header() {
        let html = ChatEntry::new(ChatAuthor::agent("claude"), el(El::P).text("more"))
            .layout(ChatLayout::Avatar)
            .grouped(true)
            .build()
            .to_static_html();
        assert!(!html.contains("chat-avatar"), "{html}");
        assert!(!html.contains("<b "), "{html}");
    }

    #[test]
    fn no_trailing_by_default() {
        let entry = ChatEntry::new(ChatAuthor::agent("claw"), el(El::P).text("hi")).build();
        assert_eq!(header_texts(&entry), vec!["claw"]);
    }

    #[test]
    fn sending_sets_reduced_opacity_and_a_status_line() {
        let entry = ChatEntry::new(ChatAuthor::user("You"), el(El::P).text("hi"))
            .layout(ChatLayout::Avatar)
            .delivery(Delivery::Sending, "Sending…", [])
            .build();
        let html = entry.to_static_html();
        assert!(html.contains("data-testid=\"chat-delivery\""), "{html}");
        assert!(html.contains("role=\"status\""), "{html}");
        assert!(html.contains("Sending…"), "{html}");
        assert!(!html.contains("chat-delivery-actions"), "{html}");
        let root = entry.get_style_utils();
        assert!(root.contains(&St::Opacity60.as_u16()));
        let bubble = entry
            .children()
            .last()
            .unwrap()
            .children()
            .iter()
            .find(|c| c.get_style_utils().contains(&St::BgSurfaceRaised.as_u16()))
            .unwrap();
        assert!(
            !bubble
                .get_style_utils()
                .contains(&St::BorderDashed.as_u16()),
            "sending has no dashed outline"
        );
    }

    #[test]
    fn unconfirmed_and_failed_get_a_dashed_outline_and_actions() {
        for (state, role, text) in [
            (Delivery::Unconfirmed, "status", "Not confirmed"),
            (Delivery::Failed, "alert", "Send failed: boom"),
        ] {
            let entry = ChatEntry::new(ChatAuthor::user("You"), el(El::P).text("hi"))
                .layout(ChatLayout::Avatar)
                .delivery(
                    state,
                    text,
                    [
                        el(El::Button).text("Resend"),
                        el(El::Button).text("Withdraw"),
                    ],
                )
                .build();
            let html = entry.to_static_html();
            assert!(html.contains("data-testid=\"chat-delivery\""), "{html}");
            assert!(html.contains(&format!("role=\"{role}\"")), "{html}");
            assert!(
                html.contains("data-testid=\"chat-delivery-actions\""),
                "{html}"
            );
            assert!(
                html.contains("Resend") && html.contains("Withdraw"),
                "{html}"
            );
            let bubble = entry
                .children()
                .last()
                .unwrap()
                .children()
                .iter()
                .find(|c| c.get_style_utils().contains(&St::BgSurfaceRaised.as_u16()))
                .unwrap();
            assert!(
                bubble
                    .get_style_utils()
                    .contains(&St::BorderDashed.as_u16()),
                "{state:?} dashed outline"
            );
            assert!(!entry.get_style_utils().contains(&St::Opacity60.as_u16()));
        }
        // Failed colors the status line as an error.
        let failed = ChatEntry::new(ChatAuthor::user("You"), el(El::P).text("hi"))
            .layout(ChatLayout::Avatar)
            .delivery(Delivery::Failed, "Send failed", [])
            .build();
        let body_col = &failed.children().last().unwrap();
        let status = body_col
            .children()
            .iter()
            .find(|c| {
                c.get_style_utils().contains(&St::TextFaint.as_u16())
                    || c.get_style_utils().contains(&St::TextError.as_u16())
            })
            .unwrap();
        assert!(status.get_style_utils().contains(&St::TextError.as_u16()));
    }
}
