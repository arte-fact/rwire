use empire_lib::EmpireGame;

use crate::ui::{bottom_press_enter, clear_screen, print_at};

pub fn weather_page(game: &EmpireGame) {
    clear_screen();
    print_at(7, 34, &format!("An {} ", game.year));
    print_at(9, 28, game.weather.sentence());
    bottom_press_enter();
}
