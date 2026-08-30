//! Random events, per original Empire.bas:
//! - Plague: 2% chance per turn, kills a portion of the population
//! - Ruler death: 1% chance per turn, eliminates the kingdom

use serde::{Deserialize, Serialize};

use crate::kingdom::Kingdom;
use crate::random::random;

#[derive(Debug, Clone, Default)]
pub struct PlagueEvent {
    pub occurred: bool,
    pub serfs_killed: i32,
    pub merchants_killed: i32,
    pub soldiers_killed: i32,
    pub nobles_killed: i32,
}

#[derive(Debug, Clone, Default)]
pub struct RulerDeathEvent {
    pub occurred: bool,
    pub cause: RulerDeathCause,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RulerDeathCause {
    #[default]
    None,
    Assassination,
    HuntingAccident,
    FoodPoisoning,
    NaturalCauses,
    /// Assassination by a starving mob (only if starvation occurred).
    StarvationAssassination,
}

impl RulerDeathCause {
    pub fn description(&self) -> &'static str {
        match self {
            RulerDeathCause::None => "",
            RulerDeathCause::Assassination => "assassinated by an ambitious noble",
            RulerDeathCause::HuntingAccident => "killed in a fox hunting accident",
            RulerDeathCause::FoodPoisoning => "died of food poisoning (the cook was executed)",
            RulerDeathCause::NaturalCauses => "died of natural causes (weak heart)",
            RulerDeathCause::StarvationAssassination => "assassinated by a crazed starving mother",
        }
    }
}

/// 2% chance of plague; applies the casualties when it strikes.
pub fn check_plague(kingdom: &mut Kingdom) -> PlagueEvent {
    if random(1, 100) > 2 {
        return PlagueEvent::default();
    }
    strike_plague(kingdom)
}

fn strike_plague(kingdom: &mut Kingdom) -> PlagueEvent {
    let serfs_killed = if kingdom.peasants > 0 {
        random(1, kingdom.peasants / 2 + 1)
    } else {
        0
    };
    let merchants_killed = if kingdom.merchants > 0 {
        random(1, kingdom.merchants / 3 + 1)
    } else {
        0
    };
    let soldiers_killed = if kingdom.soldiers > 0 {
        random(1, kingdom.soldiers / 3 + 1)
    } else {
        0
    };
    let nobles_killed = if kingdom.nobles > 1 {
        random(1, kingdom.nobles / 3 + 1)
    } else {
        0
    };

    kingdom.peasants = (kingdom.peasants - serfs_killed).max(0);
    kingdom.merchants = (kingdom.merchants - merchants_killed).max(0);
    kingdom.soldiers = (kingdom.soldiers - soldiers_killed).max(0);
    // Keep at least the ruler.
    kingdom.nobles = (kingdom.nobles - nobles_killed).max(1);

    PlagueEvent {
        occurred: true,
        serfs_killed,
        merchants_killed,
        soldiers_killed,
        nobles_killed,
    }
}

/// 1% chance of ruler death; eliminates the kingdom when it happens.
pub fn check_ruler_death(kingdom: &mut Kingdom, starvation_occurred: bool) -> RulerDeathEvent {
    if random(1, 100) > 1 {
        return RulerDeathEvent::default();
    }
    kill_ruler(kingdom, starvation_occurred)
}

fn kill_ruler(kingdom: &mut Kingdom, starvation_occurred: bool) -> RulerDeathEvent {
    let cause = if starvation_occurred {
        RulerDeathCause::StarvationAssassination
    } else {
        match random(1, 5) {
            1 => RulerDeathCause::Assassination,
            2 => RulerDeathCause::HuntingAccident,
            3 => RulerDeathCause::FoodPoisoning,
            _ => RulerDeathCause::NaturalCauses,
        }
    };
    kingdom.is_dead = true;
    RulerDeathEvent {
        occurred: true,
        cause,
    }
}

/// Check all random events for a kingdom.
pub fn check_random_events(
    kingdom: &mut Kingdom,
    starvation_occurred: bool,
) -> (PlagueEvent, RulerDeathEvent) {
    let plague = check_plague(kingdom);
    let death = if kingdom.is_dead {
        RulerDeathEvent::default()
    } else {
        check_ruler_death(kingdom, starvation_occurred)
    };
    (plague, death)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kingdom::Kingdoms;

    #[test]
    fn plague_keeps_population_non_negative_and_the_ruler_alive() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.nobles = 1;
        let e = strike_plague(&mut k);
        assert!(e.occurred);
        assert_eq!(e.nobles_killed, 0);
        assert_eq!(k.nobles, 1);
        assert!(k.peasants >= 0 && k.merchants >= 0 && k.soldiers >= 0);
    }

    #[test]
    fn ruler_death_eliminates_kingdom_with_a_cause() {
        let mut k = Kingdom::new(Kingdoms::France);
        let e = kill_ruler(&mut k, true);
        assert!(k.is_dead);
        assert_eq!(e.cause, RulerDeathCause::StarvationAssassination);

        let mut k = Kingdom::new(Kingdoms::France);
        let e = kill_ruler(&mut k, false);
        assert_ne!(e.cause, RulerDeathCause::None);
        assert_ne!(e.cause, RulerDeathCause::StarvationAssassination);
        assert!(!e.cause.description().is_empty());
    }
}
