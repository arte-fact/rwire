//! Chatroom example — an ephemeral multi-tab chat on shared state.
//!
//! Open http://127.0.0.1:9007 in several tabs: messages broadcast live to
//! every connection through `#[storage(shared)]` (the first public demo of
//! the cross-connection machinery), a writing indicator shows while anyone
//! is composing, and the transcript renders through the `ChatItem` trait +
//! `Chat` family. Non-persistent by design: restarting the server clears
//! the room, and the room caps at the last 100 messages.
//!
//! Identity is a nickname riding the composer as a hidden form field —
//! IRC-style trust, fitting an ephemeral demo room.

use std::borrow::Cow;
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use async_std::main;
use rwire::capsule_gen::CapsuleConfig;
use rwire::style_tokens::St;
use rwire::theme::Theme;
use rwire::{el, handler, renderer, theme, El, ElementBuilder, Ev, Server, State};
use rwire_components::{
    Button, ChatAuthor, ChatItem, ChatItemCtx, ChatScroll, ChatTranscript, Composer, Input, Text,
};
use rwire_themes::palettes;

const ROOM_CAP: usize = 100;
const TYPING_WINDOW_SECS: u64 = 4;

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn now_hm() -> String {
    let secs = now_secs();
    format!("{:02}:{:02}", (secs / 3600) % 24, (secs / 60) % 60)
}

// ============================================================================
// State
// ============================================================================

#[derive(Clone, Default)]
struct Msg {
    id: u64,
    name: String,
    text: String,
    time: String,
}

/// The room — one instance shared by every connection; mutations broadcast.
#[derive(State, Default)]
#[storage(shared)]
struct Room {
    messages: Vec<Msg>,
    next_id: u64,
    /// Epoch seconds of the latest draft keystroke (0 = idle). The writing row
    /// shows while a broadcast lands within the window.
    typing_at: u64,
}

/// Per-connection: who this tab is.
#[derive(State, Default)]
#[storage(memory)]
struct Me {
    name: String,
}

// ============================================================================
// Transcript items
// ============================================================================

impl ChatItem for Msg {
    fn key(&self) -> Cow<'_, str> {
        Cow::Owned(format!("m{}", self.id))
    }

    fn author(&self) -> ChatAuthor {
        ChatAuthor::user(self.name.clone())
    }

    fn body(&self, _ctx: &ChatItemCtx<Self>) -> ElementBuilder {
        el(El::P).st([St::TextSm]).text(&self.text)
    }

    fn time(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(&self.time))
    }

    fn groups_with(&self, prev: &Self) -> bool {
        self.name == prev.name
    }
}

// ============================================================================
// Handlers
// ============================================================================

#[handler]
fn join(me: &mut Me, ctx: &EventContext) {
    if let rwire::EventPayload::Form(fields) = ctx.payload() {
        if let Some(name) = fields.get("name") {
            let name = name.trim();
            if !name.is_empty() && name.len() <= 24 {
                me.name = name.to_string();
            }
        }
    }
}

#[handler]
fn send(room: &mut Room, ctx: &EventContext) {
    if let rwire::EventPayload::Form(fields) = ctx.payload() {
        let name = fields.get("name").map(|s| s.trim()).unwrap_or("");
        let text = fields.get("message").map(|s| s.trim()).unwrap_or("");
        if name.is_empty() || text.is_empty() || text.len() > 2000 {
            return;
        }
        room.next_id += 1;
        room.messages.push(Msg {
            id: room.next_id,
            name: name.to_string(),
            text: text.to_string(),
            time: now_hm(),
        });
        if room.messages.len() > ROOM_CAP {
            let drop = room.messages.len() - ROOM_CAP;
            room.messages.drain(..drop);
        }
        room.typing_at = 0;
    }
}

#[handler]
fn drafting(room: &mut Room) {
    room.typing_at = now_secs();
}

// ============================================================================
// UI
// ============================================================================

#[theme]
fn app_theme() -> Theme {
    Theme::dark().palette(palettes::nord())
}

#[main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("chatroom on http://127.0.0.1:9007 — open it in several tabs");
    Server::bind("127.0.0.1:9007")?
        .root(app)
        .capsule_config(CapsuleConfig::new())
        .theme(app_theme())
        .run()
        .await
}

fn app() -> ElementBuilder {
    el(El::Div)
        .st([St::DisplayFlex, St::FlexCol, St::HDvh, St::BgApp, St::MinH0])
        .append([
            header(),
            // The scroller owns the layout; the synced transcript region lives
            // inside it (a synced wrapper must not BE the flex scroller).
            ChatScroll::new(render_room()).build().st([St::PMd]),
            el(El::Div)
                .st([St::FlexShrink0, St::PMd, St::PtSm])
                .append([render_me()]),
        ])
}

fn header() -> ElementBuilder {
    el(El::Header)
        .st([
            St::DisplayFlex,
            St::ItemsCenter,
            St::GapSm,
            St::PMd,
            St::BorderB,
            St::FlexShrink0,
        ])
        .append([
            Text::heading3("rwire chatroom").build(),
            el(El::Span)
                .st([St::TextXs, St::TextMuted])
                .text("ephemeral · multi-tab · shared state"),
        ])
}

#[renderer]
fn render_room(room: &Room) -> ElementBuilder {
    let typing =
        room.typing_at > 0 && now_secs().saturating_sub(room.typing_at) < TYPING_WINDOW_SECS;
    ChatTranscript::new()
        .items_plain(room.messages.iter())
        .writing(typing.then_some(Cow::Borrowed("someone is typing…")))
        .empty_state(
            el(El::Div)
                .st([St::TextCenter, St::TextMuted, St::PLg])
                .text("No messages yet — say hi."),
        )
        .build()
}

#[renderer]
fn render_me(me: &Me) -> ElementBuilder {
    if me.name.is_empty() {
        // Join bar: pick a nickname.
        el(El::Form)
            .on(Ev::Submit, join())
            .st([St::DisplayFlex, St::GapSm, St::ItemsCenter])
            .append([
                Input::text()
                    .name("name")
                    .placeholder("Pick a nickname…")
                    .build()
                    .st([St::Flex1]),
                Button::primary("Join").build(),
            ])
    } else {
        Composer::new()
            .id(format!("composer-{}", me.name))
            .placeholder(format!("Message as {}…", me.name))
            .hint("⏎ send · ⇧⏎ newline")
            .hidden_field("name", me.name.clone())
            .on_submit(send())
            .on_draft(drafting())
            .build()
    }
}
