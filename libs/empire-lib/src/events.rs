//! Random events, per the original Empire.bas:
//! - Plague (lines 196-202): 2 % a year, kills a share of every class.
//! - Ruler death (lines 187-194), two independent draws after the year's
//!   demography: a starving mother's revenge, as likely as the year's
//!   starvation deaths are many; then 1 % of a death among four causes.
//!
//! Empire.bas spared its computers plague and famine (lines 206-207); ours
//! face them like everyone else.

use serde::{Deserialize, Serialize};

use crate::demography::spared;
use crate::kingdom::{Fate, Kingdom};
use crate::random::random;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlagueEvent {
    pub serfs_killed: i32,
    pub merchants_killed: i32,
    pub soldiers_killed: i32,
    pub nobles_killed: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RulerDeathCause {
    Assassination,
    HuntingAccident,
    FoodPoisoning,
    NaturalCauses,
    /// Assassinated by a mother whose child starved to death.
    StarvationAssassination,
}

/// The four causes of the 1 % death, equally likely (line 189).
const CHANCE_DEATHS: [RulerDeathCause; 4] = [
    RulerDeathCause::Assassination,
    RulerDeathCause::HuntingAccident,
    RulerDeathCause::FoodPoisoning,
    RulerDeathCause::NaturalCauses,
];

/// Line 187: `INT(RND*DS) > INT(RND*110)` — a starving mother's revenge,
/// `deaths` being the year's starvation victims.
const STARVATION_ODDS_SCALE: i32 = 110;

/// 2 % chance of plague; applies the casualties when it strikes.
pub fn check_plague(kingdom: &mut Kingdom) -> Option<PlagueEvent> {
    (random(0, 100) < 2).then(|| strike_plague(kingdom))
}

/// Lines 199-202: `INT(RND*serfs/2)`, a third of the others. The ruler is
/// one of the nobles and survives.
fn strike_plague(kingdom: &mut Kingdom) -> PlagueEvent {
    let h = kingdom.hospices;
    let serfs_killed = spared(random(0, kingdom.peasants / 2), h);
    let merchants_killed = spared(random(0, kingdom.merchants / 3), h);
    let soldiers_killed = spared(random(0, kingdom.soldiers / 3), h);
    let nobles_killed = spared(random(0, kingdom.nobles / 3), h)
        .min(kingdom.nobles - 1)
        .max(0);
    kingdom.peasants -= serfs_killed;
    kingdom.merchants -= merchants_killed;
    kingdom.soldiers -= soldiers_killed;
    kingdom.nobles -= nobles_killed;
    PlagueEvent {
        serfs_killed,
        merchants_killed,
        soldiers_killed,
        nobles_killed,
    }
}

/// Whether the year's `starvation_deaths` cost the ruler their life.
fn starving_mother_strikes(starvation_deaths: i32) -> bool {
    random(0, starvation_deaths) > random(0, STARVATION_ODDS_SCALE)
}

/// The 1 % death (line 188), by one of four causes.
fn chance_death() -> Option<RulerDeathCause> {
    (random(0, 100) < 1).then(|| CHANCE_DEATHS[random(0, 4) as usize])
}

/// The ruler's death draws for the year; the kingdom falls when one hits.
/// The 1 % `chance` can be spared — the arena does, having nothing to
/// teach from luck.
pub fn check_ruler_death(
    kingdom: &mut Kingdom,
    starvation_deaths: i32,
    chance: bool,
) -> Option<RulerDeathCause> {
    let cause = if starving_mother_strikes(starvation_deaths) {
        Some(RulerDeathCause::StarvationAssassination)
    } else if chance {
        chance_death()
    } else {
        None
    }?;
    kingdom.fall(Fate::RulerDied(cause));
    Some(cause)
}

/// The year's random events for a living kingdom: plague, then the ruler's
/// death.
pub fn check_random_events(
    kingdom: &mut Kingdom,
    starvation_deaths: i32,
) -> (Option<PlagueEvent>, Option<RulerDeathCause>) {
    let plague = check_plague(kingdom);
    let death = check_ruler_death(kingdom, starvation_deaths, true);
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
        assert_eq!(e.nobles_killed, 0);
        assert_eq!(k.nobles, 1);
        assert!(k.peasants >= 0 && k.merchants >= 0 && k.soldiers >= 0);
        let mut k = Kingdom::new(Kingdoms::France);
        k.nobles = 30;
        for _ in 0..200 {
            let e = strike_plague(&mut k);
            assert!(e.nobles_killed <= 10);
            k.nobles = 30;
        }
    }

    #[test]
    fn a_starving_mother_needs_starvation_deaths() {
        for _ in 0..1000 {
            assert!(!starving_mother_strikes(0));
        }
        // Original odds: about `deaths / 220`, and past 110 deaths a coin toss.
        let strikes = (0..20_000).filter(|_| starving_mother_strikes(110)).count();
        assert!((8_000..12_000).contains(&strikes), "{strikes}");
        let strikes = (0..20_000).filter(|_| starving_mother_strikes(22)).count();
        assert!((1_000..3_000).contains(&strikes), "{strikes}");
    }

    #[test]
    fn the_chance_death_has_four_causes_about_one_year_in_a_hundred() {
        let mut seen = std::collections::HashSet::new();
        let deaths = (0..50_000)
            .filter_map(|_| chance_death())
            .inspect(|c| {
                seen.insert(*c);
            })
            .count();
        assert!((300..700).contains(&deaths), "{deaths}");
        assert_eq!(seen.len(), 4);
        assert!(!seen.contains(&RulerDeathCause::StarvationAssassination));
    }

    #[test]
    fn ruler_death_eliminates_kingdom_with_a_cause() {
        let mut k = Kingdom::new(Kingdoms::France);
        let cause = check_ruler_death(&mut k, 100_000, true).unwrap();
        assert!(k.is_dead);
        assert_eq!(cause, RulerDeathCause::StarvationAssassination);
        assert_eq!(k.fate, Some(Fate::RulerDied(cause)));
    }
}
