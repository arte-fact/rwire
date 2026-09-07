use std::thread::sleep;
use std::time::Duration;

use empire_lib::ia::execute_ai_turn;
use empire_lib::mind::Mind;
use empire_lib::{EmpireGame, Kingdoms};

use crate::ui::{clear_screen, print_at};

pub fn ia_turn(game: &mut EmpireGame, id: Kingdoms, mind: &mut Mind) {
    if game.kingdom(id).is_dead {
        return;
    }
    clear_screen();
    print_at(
        10,
        24,
        &format!("Un moment, {} joue...", game.kingdom(id).full_title()),
    );
    sleep(Duration::from_millis(2500));

    execute_ai_turn(game, id, mind);
}
