//! Empire — terminal front-end for `empire-lib`.
//!
//! The whole game lives in one [`EmpireGame`] value owned by `main`; every
//! page receives it by reference.

mod assets;
mod pages;
mod ui;

use empire_lib::brain::Memory;
use empire_lib::demography::{apply_feed, Council};
use empire_lib::economy::{apply_economy, economy_report};
use empire_lib::harvests::{apply_grain_harvest, apply_rat_loss_rate, apply_seed_grain};
use empire_lib::trade::apply_trade;
use empire_lib::{EmpireGame, Kingdoms, KINGDOMS};

use pages::demographic_report::display_demographic_report;
use pages::economic_management::{investments_management, taxes_management};
use pages::game_setup::game_setup;
use pages::ia_turn::ia_turn;
use pages::resources_management::{
    prompt_grain_for_peasants, prompt_grain_for_soldiers, prompt_trade_option,
};
use pages::resume::resume_page;
use pages::war_management::war_management;
use pages::weather::weather_page;
use pages::welcome::welcome;
use ui::alternate_screen_buffer;

fn main() {
    let mut game = EmpireGame::default();
    // What each computer's brain remembers of its years.
    let mut memories: [Memory; 6] = Default::default();

    alternate_screen_buffer();
    welcome();
    game_setup(&mut game);

    loop {
        game_loop(&mut game, &mut memories);
    }
}

fn game_loop(game: &mut EmpireGame, memories: &mut [Memory; 6]) {
    game.random_weather();
    game.open_market();
    weather_page(game);
    for id in KINGDOMS {
        kingdom_turn(game, id, &mut memories[id.index()]);
    }
    resume_page(game);
    game.increment_year();
}

fn kingdom_turn(game: &mut EmpireGame, id: Kingdoms, memory: &mut Memory) {
    let weather = game.kingdom(id).weather;
    let year = game.year;

    {
        let kingdom = game.kingdom_mut(id);
        apply_seed_grain(kingdom);
        apply_rat_loss_rate(kingdom);
        apply_grain_harvest(kingdom, weather);
    }

    if !game.kingdom(id).is_player {
        ia_turn(game, id, memory);
        return;
    }

    let trade = prompt_trade_option(game, id);
    apply_trade(game, id, trade);

    let grain_for_soldiers = prompt_grain_for_soldiers(game, id);
    let grain_for_peasants = prompt_grain_for_peasants(game, id);

    let kingdom = game.kingdom_mut(id);
    let council = Council::from_grain(
        kingdom,
        grain_for_peasants,
        grain_for_soldiers,
        kingdom.taxes(),
    );
    let demographic_report = apply_feed(kingdom, council);
    display_demographic_report(kingdom, year, &demographic_report);

    let economic_report = economy_report(kingdom, weather, demographic_report.immigrants);
    apply_economy(kingdom, &economic_report);
    taxes_management(kingdom, year, &economic_report);
    investments_management(kingdom, year, &economic_report);

    war_management(game, id);
}
