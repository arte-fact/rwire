use serde::{Deserialize, Serialize};

use crate::random::random;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Weather {
    Great,
    VeryGood,
    #[default]
    Good,
    Bad,
    VeryBad,
    Disastrous,
}

impl Weather {
    /// Numeric weather factor used by harvest / economy formulas (1..=6).
    pub fn value(self) -> i32 {
        match self {
            Weather::Great => 6,
            Weather::VeryGood => 5,
            Weather::Good => 4,
            Weather::Bad => 3,
            Weather::VeryBad => 2,
            Weather::Disastrous => 1,
        }
    }

    pub fn sentence(self) -> &'static str {
        match self {
            Weather::Great => "Temps superbe! Grande année!",
            Weather::VeryGood => "Beau temps, été long.",
            Weather::Good => "Temps moyen. Bonne année",
            Weather::Bad => "Inondations. Trop de pluies.",
            Weather::VeryBad => "Gelées précoces. Aridité.",
            Weather::Disastrous => "Mauvais temps. Sécheresse. Sauterelles.",
        }
    }

    /// Uniform roll over all six outcomes.
    pub fn random() -> Weather {
        match random(1, 7) {
            6 => Weather::Great,
            5 => Weather::VeryGood,
            4 => Weather::Good,
            3 => Weather::Bad,
            2 => Weather::VeryBad,
            _ => Weather::Disastrous,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Weather;

    #[test]
    fn every_outcome_is_reachable() {
        let mut seen = [false; 6];
        for _ in 0..2000 {
            seen[(Weather::random().value() - 1) as usize] = true;
        }
        assert!(seen.iter().all(|&s| s), "unreached outcomes: {seen:?}");
    }
}
