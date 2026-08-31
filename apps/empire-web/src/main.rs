//! Empire — multiplayer web edition.
//!
//! One process-global [`Room`] (`#[storage(shared)]`) holds the whole game; every
//! connection renders it and rwire broadcasts each change to the others. Turns
//! are sequential like the original: one kingdom acts at a time (humans through
//! the forms, the computer through a ticker), and every battle is animated live
//! for everyone by the same ticker.
//!
//! A connection is identified by [`Me::token`], minted in per-connection memory
//! state, threaded into the shared view through a closure region and attached
//! to every handler call as param bytes.
//!
//! Run with: `cargo run -p empire-web` — open http://127.0.0.1:7782 on several
//! phones/tabs.

mod room;
mod ui;

use std::error::Error;
use std::time::Duration;

use rwire::{
    handler, renderer, theme, CapsuleConfig, ChangeSet, ElementBuilder, RendererDeps, Server,
    State, Theme,
};
use rwire_themes::palettes;

use room::Room;

/// Per-connection identity and UI preference.
#[derive(State)]
#[storage(memory)]
struct Me {
    token: u64,
    /// Bottom tab: 0 = Partie, 1 = Royaumes, 2 = Journal.
    tab: u8,
}

impl Default for Me {
    fn default() -> Self {
        Me {
            token: rand::random::<u64>() | 1,
            tab: 0,
        }
    }
}

#[handler]
fn set_tab(me: &mut Me, ctx: &EventContext) {
    me.tab = ctx.param_bytes().first().copied().unwrap_or(0).min(2);
}

#[renderer]
fn root(me: &Me) -> ElementBuilder {
    let (token, tab) = (me.token, me.tab);
    ElementBuilder::synced_with_storage::<Room, _>(
        move |room| ui::page(room, token, tab),
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
        .capsule_config(CapsuleConfig::new())
        .theme(app_theme());

    // The clock of the table: animates battles and plays the computer's turns.
    let shared = server.shared_state();
    async_std::task::spawn(async move {
        loop {
            async_std::task::sleep(Duration::from_millis(room::TICK_MS)).await;
            shared.update_shared_if::<Room>(|room| room.tick().then(ChangeSet::all));
        }
    });

    server.run().await
}
