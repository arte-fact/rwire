use std::thread::sleep;
use std::time::Duration;

use empire_lib::arena;
use empire_lib::brain::{Brain, Letters, Memory, Stage};
use empire_lib::intel::AGENT_PRICE;
use empire_lib::war::{
    apply_barbarian_battle_result, apply_kingdom_battle_result, simulate_barbarian_battle,
    simulate_kingdom_battle,
};
use empire_lib::{EmpireGame, Kingdoms};

use crate::ui::{clear_screen, print_at};

/// A computer's turn: its brain's Intendance sealed, then its Extérieur in
/// two readings — the éclaireur and the agents, paid, reading the realms as
/// they stand, then every expedition fought on the spot.
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

    // Each seat keeps its school: the four sisters, then the first two again.
    let brain = &Brain::schools()[id.index() % 4];
    let mut bought = [0; 6];
    arena::intendance(brain, game, id, Stage::War, memory, &mut bought);
    let missions = brain.missions(game, id, memory, Stage::War, Letters::None);
    if let Some(on) = missions.scout {
        if arena::send_scout(game.kingdom_mut(id), false) {
            memory.dossiers[on.index()].scout(game.kingdom(on), game.year);
        }
    }
    for on in missions.agents {
        let k = game.kingdom_mut(id);
        if k.treasury >= AGENT_PRICE {
            k.treasury -= AGENT_PRICE;
            memory.dossiers[on.index()].agent(game.kingdom(on), game.year, [0; 6]);
        }
    }
    // The terminal's battles know no siege: the rams stay home.
    for e in brain.expeditions(game, id, memory, Stage::War) {
        match e.target {
            Some(on) => {
                let result = simulate_kingdom_battle(game, id, on, e.soldiers, |_| {});
                apply_kingdom_battle_result(game, id, on, e.soldiers, &result);
            }
            None => {
                let result = simulate_barbarian_battle(game, id, e.soldiers, |_| {});
                apply_barbarian_battle_result(game, id, e.soldiers, &result);
            }
        }
    }
}
