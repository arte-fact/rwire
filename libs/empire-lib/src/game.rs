use rwire::State;
use serde::{Deserialize, Serialize};

use crate::events::RulerDeathCause;
use crate::kingdom::{Fate, Kingdom, Kingdoms, KINGDOMS};
use crate::weather::Weather;

/// Complete state of one Empire game: the calendar, the barbarian lands and
/// the six kingdoms, each under its own sky.
///
/// Derives `rwire::State` (memory storage) so an rwire app can take it as
/// handler/renderer state directly; a synchronous front-end simply owns one.
#[derive(State, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[storage(memory)]
pub struct EmpireGame {
    pub year: i32,
    /// Arpents the barbarians still hold: what the raids share, until the
    /// last is taken and the barbarians flee.
    pub barbarians_surface: i32,
    pub kingdoms: [Kingdom; 6],
}

/// The barbarian lands as the game opens, per the original.
pub const BARBARIAN_LANDS: i32 = 6000;

impl Default for EmpireGame {
    fn default() -> Self {
        EmpireGame {
            year: 1,
            barbarians_surface: BARBARIAN_LANDS,
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

    /// The ruler dead, the realm breaks up: nobody holds its land, which
    /// goes back to the barbarians — a conqueror's spoils are its
    /// neighbour's, a widow's realm is everyone's — and the seat is out of
    /// the game.
    pub fn break_up(&mut self, id: Kingdoms, cause: RulerDeathCause) {
        let k = self.kingdom_mut(id);
        let surface = std::mem::take(&mut k.surface);
        k.fall(Fate::RulerDied(cause));
        self.barbarians_surface += surface;
    }

    /// Roll the year's weather, a sky for every realm.
    pub fn random_weather(&mut self) {
        for k in &mut self.kingdoms {
            k.weather = Weather::random();
        }
    }

    /// The year's skies told as one line — "Temps superbe en Bretagne et en
    /// Perse ; sécheresse en Castille…" — worst last, so a famine somewhere
    /// is the news.
    pub fn weather_news(&self) -> String {
        let mut parts = Vec::new();
        for w in Weather::ALL {
            let names: Vec<String> = self
                .kingdoms
                .iter()
                .filter(|k| !k.is_dead && k.weather == w)
                .map(|k| format!("en {}", k.name()))
                .collect();
            if names.is_empty() {
                continue;
            }
            let places = match names.len() {
                1 => names[0].clone(),
                n => format!("{} et {}", names[..n - 1].join(", "), names[n - 1]),
            };
            parts.push(format!("{} {places}", w.short()));
        }
        let news = parts.join(" ; ");
        let mut chars = news.chars();
        let first: String = chars
            .next()
            .into_iter()
            .flat_map(char::to_uppercase)
            .collect();
        format!("{first}{}.", chars.as_str())
    }

    pub fn increment_year(&mut self) {
        self.year += 1;
    }

    /// Put last year's listings on the stalls; called once at the start of a
    /// year, before anyone trades.
    pub fn open_market(&mut self) {
        for k in &mut self.kingdoms {
            k.open_market();
        }
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
        game.random_weather();
        assert!(game
            .kingdoms
            .iter()
            .all(|k| Weather::ALL.contains(&k.weather)));
    }

    #[test]
    fn the_weather_news_groups_the_realms_by_sky() {
        let mut game = EmpireGame::default();
        for k in &mut game.kingdoms {
            k.weather = Weather::VeryGood;
        }
        game.kingdoms[0].weather = Weather::Disastrous;
        game.kingdoms[5].weather = Weather::Disastrous;
        game.kingdoms[2].weather = Weather::Bad;
        game.kingdoms[3].is_dead = true;
        assert_eq!(
            game.weather_news(),
            "Été long en Bretagne et en Moscovie ; inondations en Germanie ; \
             sécheresse en France et en Perse."
        );
    }

    #[test]
    fn serde_round_trip() {
        let game = EmpireGame::default();
        let json = serde_json::to_string(&game).unwrap();
        let back: EmpireGame = serde_json::from_str(&json).unwrap();
        assert_eq!(game, back);
    }
}
