use empire_lib::EmpireGame;

use crate::ui::{bottom_press_enter, clear_screen, print_at};

/// The year's skies, one line per realm, the player's first and in full.
pub fn weather_page(game: &EmpireGame) {
    clear_screen();
    print_at(5, 34, &format!("An {} ", game.year));
    let (players, others): (Vec<_>, Vec<_>) = game
        .kingdoms
        .iter()
        .filter(|k| !k.is_dead)
        .partition(|k| k.is_player);
    for (row, k) in players.into_iter().chain(others).enumerate() {
        let sky = if k.is_player {
            k.weather.sentence()
        } else {
            k.weather.short()
        };
        print_at(7 + row, 10, &format!("{:<10} {sky}", k.name()));
    }
    bottom_press_enter();
}
