use rwire::State;
use serde::{Deserialize, Serialize};

use crate::kingdom::{Kingdom, Kingdoms, KINGDOMS};
use crate::weather::Weather;

/// Complete state of one Empire game: the calendar, the barbarian lands, the
/// current weather and the six kingdoms.
///
/// Derives `rwire::State` (memory storage) so an rwire app can take it as
/// handler/renderer state directly; a synchronous front-end simply owns one.
#[derive(State, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[storage(memory)]
pub struct EmpireGame {
    pub year: i32,
    pub barbarians_surface: i32,
    pub weather: Weather,
    pub kingdoms: [Kingdom; 6],
}

impl Default for EmpireGame {
    fn default() -> Self {
        EmpireGame {
            year: 1,
            barbarians_surface: 6000,
            weather: Weather::default(),
            kingdoms: KINGDOMS.map(Kingdom::new),
        }
    }
}

impl EmpireGame {
    pub fn kingdom(&self, id: Kingdoms) -> &Kingdom {
        &self.kingdoms[id.index()]
    }

    pub fn kingdom_mut(&mut self, id: Kingdoms) -> &mut Kingdom {
        &mut self.kingdoms[id.index()]
    }

    /// Ids of every kingdom that has not been eliminated, in canonical order.
    pub fn alive_kingdoms(&self) -> Vec<Kingdoms> {
        self.kingdoms
            .iter()
            .filter(|k| !k.is_dead)
            .map(|k| k.id)
            .collect()
    }

    /// Roll and store a new weather for the year.
    pub fn random_weather(&mut self) -> Weather {
        self.weather = Weather::random();
        self.weather
    }

    pub fn increment_year(&mut self) {
        self.year += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_game_matches_original_setup() {
        let game = EmpireGame::default();
        assert_eq!(game.year, 1);
        assert_eq!(game.barbarians_surface, 6000);
        assert_eq!(game.alive_kingdoms(), KINGDOMS.to_vec());
        for id in KINGDOMS {
            assert_eq!(game.kingdom(id).id, id);
        }
    }

    #[test]
    fn kingdom_mut_targets_the_right_slot() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::Persia).is_dead = true;
        game.kingdom_mut(Kingdoms::Germany).soldiers = 99;
        assert!(game.kingdom(Kingdoms::Persia).is_dead);
        assert_eq!(game.kingdom(Kingdoms::Germany).soldiers, 99);
        assert_eq!(game.kingdom(Kingdoms::France).soldiers, 20);
        assert_eq!(game.alive_kingdoms().len(), 5);
        assert!(!game.alive_kingdoms().contains(&Kingdoms::Persia));
    }

    #[test]
    fn year_and_weather_roll() {
        let mut game = EmpireGame::default();
        game.increment_year();
        assert_eq!(game.year, 2);
        let w = game.random_weather();
        assert_eq!(game.weather, w);
    }

    #[test]
    fn serde_round_trip() {
        let game = EmpireGame::default();
        let json = serde_json::to_string(&game).unwrap();
        let back: EmpireGame = serde_json::from_str(&json).unwrap();
        assert_eq!(game, back);
    }
}
