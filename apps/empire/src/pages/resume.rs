use empire_lib::EmpireGame;

use crate::assets::text;
use crate::ui::{black_on_gray, bottom_press_enter, clear_screen, print_at};

pub fn resume_page(game: &EmpireGame) {
    clear_screen();
    print_at(
        2,
        24,
        &black_on_gray(&format!(" {} {} ", text::ENDING_YEAR_RESUME, game.year)),
    );
    print_at(4, 10, text::NOBLES);
    print_at(4, 20, text::SOLDIERS);
    print_at(4, 30, text::MERCHANTS);
    print_at(4, 40, text::PEASANTS);
    print_at(4, 50, text::SURFACE);
    print_at(4, 60, text::PALACES);

    for (i, id) in game.alive_kingdoms().into_iter().enumerate() {
        let kingdom = game.kingdom(id);
        let row = 6 + i * 2;
        print_at(
            row,
            0,
            &black_on_gray(&format!("{:<80}", kingdom.full_title())),
        );
        print_at(row + 1, 10, &kingdom.nobles.to_string());
        print_at(row + 1, 20, &kingdom.soldiers.to_string());
        print_at(row + 1, 30, &kingdom.merchants.to_string());
        print_at(row + 1, 40, &kingdom.peasants.to_string());
        print_at(row + 1, 50, &kingdom.surface.to_string());
        print_at(row + 1, 60, &format!("{}%", kingdom.palaces * 10));
    }

    bottom_press_enter();
}
