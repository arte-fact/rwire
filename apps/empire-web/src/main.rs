//! Empire — multiplayer web edition.
//!
//! One process-global [`Rooms`] (`#[storage(shared)]`) holds every table; each
//! connection renders the room named in its URL (`/r/<code>`, the shareable
//! link) and rwire broadcasts each change to the others. Turns are sequential
//! like the original: one kingdom acts at a time (humans through the forms, the
//! computer through a ticker), and every battle is animated live for everyone.
//!
//! A connection is identified by [`Me::token`], minted in per-connection memory
//! state, threaded into the shared view through a closure region and attached
//! to every handler call as param bytes.
//!
//! Run with: `cargo run -p empire-web` (`PORT=…` to move it) — open http://127.0.0.1:7782, create a
//! table and share its link.

mod room;
mod ui;

use std::error::Error;
use std::time::Duration;

use rwire::{
    handler, renderer, theme, CapsuleConfig, ChangeSet, ElementBuilder, Pwa, PwaDisplay,
    RendererDeps, Server, State, Theme,
};
use rwire_themes::palettes;

use room::Rooms;

/// Per-connection identity and UI state.
#[derive(State)]
#[storage(memory)]
struct Me {
    token: u64,
    /// The journal drawer and its read mark.
    journal: ui::Journal,
    /// Room code from the URL (`/r/<code>`); `None` = home.
    room: Option<String>,
    /// Open bottom sheet: `(action, step, year)` — shown only while the table
    /// is still at that step/year, so a stale sheet never reappears.
    sheet: Option<ui::Sheet>,
}

impl Default for Me {
    fn default() -> Self {
        Me {
            token: rand::random::<u64>() | 1,
            journal: ui::Journal::default(),
            room: None,
            sheet: None,
        }
    }
}

/// Open or close the journal drawer; params: `[open, len (u32 LE)]` where
/// `len` is the log length on screen — everything up to it counts as read.
#[handler]
fn set_journal(me: &mut Me, ctx: &EventContext) {
    let p = ctx.param_bytes();
    if p.len() >= 5 {
        me.journal = ui::Journal {
            open: p[0] != 0,
            seen: u32::from_le_bytes([p[1], p[2], p[3], p[4]]),
        };
    }
}

/// The URL is the source of truth for the current room: shared links, room
/// creation and code entry all navigate to `/r/<code>`.
#[handler]
fn on_route(me: &mut Me, ctx: &EventContext) {
    me.room = ctx
        .text()
        .and_then(|p| p.strip_prefix("/r/"))
        .map(room::normalize_code)
        .filter(|c| !c.is_empty());
    me.journal = ui::Journal::default();
}

/// Open the bottom sheet for an action; params:
/// `[action, step, year_lo, year_hi, gen_lo, gen_hi]`.
#[handler]
fn open_sheet(me: &mut Me, ctx: &EventContext) {
    let p = ctx.param_bytes();
    me.sheet = (p.len() >= 6).then(|| ui::Sheet {
        action: p[0],
        step: p[1],
        year: u16::from_le_bytes([p[2], p[3]]),
        gen: u16::from_le_bytes([p[4], p[5]]),
    });
}

#[handler]
fn close_sheet(me: &mut Me) {
    me.sheet = None;
}

#[handler]
fn go_home(me: &mut Me, ctx: &EventContext) {
    me.room = None;
    me.journal = ui::Journal::default();
    ctx.navigate("/");
}

#[renderer]
fn root(me: &Me) -> ElementBuilder {
    let (token, journal, room, sheet) = (me.token, me.journal, me.room.clone(), me.sheet);
    ElementBuilder::synced_with_storage::<Rooms, _>(
        move |rooms| ui::page(rooms, token, journal, room.as_deref(), sheet),
        RendererDeps::always(),
    )
}

#[theme]
fn app_theme() -> Theme {
    Theme::dark()
        .palette(palettes::gruvbox())
        .base_font_size(14, 16)
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "7782".to_string());
    let mut server = Server::bind(&format!("0.0.0.0:{port}"))?
        .root(root)
        .on_route(on_route())
        .capsule_config(
            CapsuleConfig::new().lang("fr").pwa(
                Pwa::new("Empire")
                    .short_name("Empire")
                    .description("Six royaumes, un seul empereur — jouez entre amis.")
                    .display(PwaDisplay::Standalone)
                    .icon(192, &include_bytes!("../assets/icon-192.png")[..])
                    .icon(512, &include_bytes!("../assets/icon-512.png")[..])
                    .maskable_icon(512, &include_bytes!("../assets/icon-512-maskable.png")[..]),
            ),
        )
        .theme(app_theme());

    // The clock of every table: animates battles, plays the computer's turns
    // and forgets abandoned rooms.
    let shared = server.shared_state();
    async_std::task::spawn(async move {
        loop {
            async_std::task::sleep(Duration::from_millis(room::TICK_MS)).await;
            shared.update_shared_if::<Rooms>(|rooms| rooms.tick().then(ChangeSet::all));
        }
    });

    server.run().await
}
