use std::thread::sleep;
use std::time::Duration;

use empire_lib::arena;
use empire_lib::brain::{Brain, Letters, Memory, Stage};
use empire_lib::intel::SCOUT_PRICE;
use empire_lib::war::{
    apply_barbarian_battle_result, apply_kingdom_battle_result, simulate_barbarian_battle,
    simulate_kingdom_battle,
};
use empire_lib::{EmpireGame, Kingdoms};

use crate::ui::{clear_screen, print_at};

/// A computer's turn: its brain's Intendance sealed, then its orders —
/// every expedition fought on the spot, and the éclaireur, paid, reading the
/// realm as it stands for next year's council.
pub fn ia_turn(game: &mut EmpireGame, id: Kingdoms, memory: &mut Memory) {
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

    let brain = Brain::schooled();
    let mut bought = [0; 6];
    arena::intendance(brain, game, id, Stage::War, memory, &mut bought);
    let orders = brain.orders(game, id, memory, Stage::War, Letters::None);
    for (target, soldiers) in orders.expeditions {
        match target {
            Some(on) => {
                let result = simulate_kingdom_battle(game, id, on, soldiers, |_| {});
                apply_kingdom_battle_result(game, id, on, soldiers, &result);
            }
            None => {
                let result = simulate_barbarian_battle(game, id, soldiers, |_| {});
                apply_barbarian_battle_result(game, id, soldiers, &result);
            }
        }
    }
    if let Some(on) = orders.scout {
        game.kingdom_mut(id).treasury -= SCOUT_PRICE;
        memory.dossiers[on.index()].scout(game.kingdom(on), game.year);
    }
}
