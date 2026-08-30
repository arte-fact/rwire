use empire_lib::{EmpireGame, KINGDOMS};

use super::number_of_players::prompt_number_of_players;
use super::prompt_player_name::prompt_player_name;

pub fn game_setup(game: &mut EmpireGame) {
    let number_of_players = prompt_number_of_players();
    for id in KINGDOMS.iter().take(number_of_players) {
        let name = prompt_player_name(id.name(), id.default_king_name());
        let kingdom = game.kingdom_mut(*id);
        kingdom.player_name = name;
        kingdom.is_player = true;
    }
}
