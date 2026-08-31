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
//! Run with: `cargo run -p empire-web` — open http://127.0.0.1:7782, create a
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
    /// Bottom tab: 0 = Partie, 1 = Royaumes, 2 = Journal.
    tab: u8,
    /// Room code from the URL (`/r/<code>`); `None` = home.
    room: Option<String>,
}

impl Default for Me {
    fn default() -> Self {
        Me {
            token: rand::random::<u64>() | 1,
            tab: 0,
            room: None,
        }
    }
}

#[handler]
fn set_tab(me: &mut Me, ctx: &EventContext) {
    me.tab = ctx.param_bytes().first().copied().unwrap_or(0).min(2);
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
    me.tab = 0;
}

#[handler]
fn go_home(me: &mut Me, ctx: &EventContext) {
    me.room = None;
    me.tab = 0;
    ctx.navigate("/");
}

#[renderer]
fn root(me: &Me) -> ElementBuilder {
    let (token, tab, room) = (me.token, me.tab, me.room.clone());
    ElementBuilder::synced_with_storage::<Rooms, _>(
        move |rooms| ui::page(rooms, token, tab, room.as_deref()),
        RendererDeps::always(),
    )
}

#[theme]
fn app_theme() -> Theme {
    Theme::dark().palette(palettes::gruvbox())
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut server = Server::bind("0.0.0.0:7782")?
        .root(root)
        .on_route(on_route())
        .capsule_config(CapsuleConfig::new().pwa(
            Pwa::new("Empire")
                .short_name("Empire")
                .description("Six royaumes, un seul empereur — jouez entre amis.")
                .display(PwaDisplay::Standalone)
                .icon(192, &include_bytes!("../assets/icon-192.png")[..])
                .icon(512, &include_bytes!("../assets/icon-512.png")[..])
                .maskable_icon(512, &include_bytes!("../assets/icon-512-maskable.png")[..]),
        ))
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
