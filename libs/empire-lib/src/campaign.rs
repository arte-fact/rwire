//! The year's campaign: every expedition of every realm, declared at once and
//! fought at once. The men leave their garrisons first, so each front meets
//! the defender as it then stands. All the armies marching on one realm fight
//! it together, as one front (see [`crate::front`]); each expedition against
//! the barbarians is a front of its own, fought by the old rule, on what
//! land the barbarians have left. Sending men costs gold and grain, per man, when they leave.

use crate::front::{
    apply_front, deciles, forecast_front, simulate_front, Army, Forecast, FrontResult, Line,
    People, Round, Stand,
};
use crate::game::EmpireGame;
use crate::kingdom::{Kingdom, Kingdoms};
use crate::war::simulate_barbarian_battle;
use rand::seq::SliceRandom;

/// What a campaign costs per man sent, paid when the men leave — the road's
/// pay and the wagons' provisions, over the recruit's eight francs and his
/// eight bushels a year at home. A hundred men cost half a year's taxes and a
/// season's surplus of grain: a garrison is cheap, a campaign is not.
pub const EXPEDITION_GOLD_PER_MAN: i32 = 10;
/// The first year a realm may march on another: before it, only the
/// barbarians can be attacked (the original's truce).
pub const FIRST_WAR_YEAR: i32 = 3;
pub const EXPEDITION_GRAIN_PER_MAN: i32 = 20;

/// What sending `men` costs: `(gold, grain)`.
pub fn expedition_cost(men: i32) -> (i32, i32) {
    (
        men * EXPEDITION_GOLD_PER_MAN,
        men * EXPEDITION_GRAIN_PER_MAN,
    )
}

/// The most men a realm can send with what its coffers and granaries hold
/// (and has under arms).
pub fn affordable(k: &Kingdom) -> i32 {
    (k.treasury / EXPEDITION_GOLD_PER_MAN)
        .min(k.grain_stocks / EXPEDITION_GRAIN_PER_MAN)
        .min(k.soldiers)
        .max(0)
}

/// One expedition of the campaign.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Expedition {
    pub attacker: Kingdoms,
    /// `None` = the barbarians.
    pub target: Option<Kingdoms>,
    pub soldiers: i32,
}

/// A front fought, to be applied with [`apply_battle`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fought {
    /// `None` = the barbarians.
    pub target: Option<Kingdoms>,
    pub result: FrontResult,
}

impl Fought {
    pub fn armies(&self) -> &[Army] {
        &self.result.armies
    }

    pub fn expeditions(&self) -> impl Iterator<Item = Expedition> + '_ {
        self.result.armies.iter().map(|a| Expedition {
            attacker: a.attacker,
            target: self.target,
            soldiers: a.sent,
        })
    }

    /// The serfs took up arms from the first blow, for want of an army.
    pub fn levy(&self) -> bool {
        self.target.is_some() && self.result.garrison_start == 0
    }

    /// The realm falls at the end of this front, to this attacker.
    pub fn annexed_by(&self) -> Option<Kingdoms> {
        self.result
            .annexed_by
            .map(|i| self.result.armies[i].attacker)
    }
}

/// The armies march: the men leave their garrisons (no more than the realm
/// has, or can pay for — the cost is taken as they go), then every front is
/// fought against the realms as they now stand, in the order the targets were
/// first named. The realms are drawn in a fresh order every march — no seat
/// is first at the barbarians' land, or first on a garrison, by right — each
/// keeping the order of its own expeditions. Expeditions that make no sense
/// (a fallen realm on either side, a realm marching on itself before
/// [`FIRST_WAR_YEAR`] or at all, no barbarian land left, no men) are
/// dropped, costing nothing.
pub fn march(
    game: &mut EmpireGame,
    expeditions: impl IntoIterator<Item = Expedition>,
) -> Vec<Fought> {
    let mut by_realm: Vec<Vec<Expedition>> = Vec::new();
    for e in expeditions {
        match by_realm.iter_mut().find(|v| v[0].attacker == e.attacker) {
            Some(v) => v.push(e),
            None => by_realm.push(vec![e]),
        }
    }
    by_realm.shuffle(&mut rand::thread_rng());
    let mut fronts: Vec<(Option<Kingdoms>, Vec<Expedition>)> = Vec::new();
    for mut e in by_realm.into_iter().flatten() {
        let a = game.kingdom(e.attacker);
        let sensible = !a.is_dead
            && match e.target {
                Some(t) => {
                    game.year >= FIRST_WAR_YEAR && t != e.attacker && !game.kingdom(t).is_dead
                }
                None => game.barbarians_surface > 0,
            };
        e.soldiers = e.soldiers.min(affordable(a));
        if !sensible || e.soldiers < 1 {
            continue;
        }
        let (gold, grain) = expedition_cost(e.soldiers);
        let a = game.kingdom_mut(e.attacker);
        a.soldiers -= e.soldiers;
        a.treasury -= gold;
        a.grain_stocks -= grain;
        match fronts.iter_mut().find(|(t, _)| *t == e.target) {
            Some((_, armies)) => armies.push(e),
            None => fronts.push((e.target, vec![e])),
        }
    }
    let mut field = game.clone();
    let mut fought = Vec::new();
    for (target, armies) in fronts {
        match target {
            Some(t) => {
                let armies: Vec<(Kingdoms, i32)> =
                    armies.iter().map(|e| (e.attacker, e.soldiers)).collect();
                fought.push(Fought {
                    target,
                    result: simulate_front(&field, t, &armies),
                });
            }
            // Every barbarian expedition is its own front, taking at most
            // what the ones before left.
            None => {
                for e in armies {
                    let result = raid(&field, e);
                    field.barbarians_surface -= result.spoils().arpents;
                    fought.push(Fought { target, result });
                }
            }
        }
    }
    fought
}

/// An expedition against the barbarians, told as a front with one army on a
/// bare line of what land the barbarians have left; taken whole, the
/// barbarians flee (an expedition arriving after them finds nothing).
fn raid(field: &EmpireGame, e: Expedition) -> FrontResult {
    let land = field.barbarians_surface;
    let mut frames = Vec::new();
    let r = simulate_barbarian_battle(field, e.attacker, e.soldiers, |p| frames.push(p.clone()));
    let band = frames
        .iter()
        .map(|f| f.defender_soldiers)
        .max()
        .unwrap_or(0);
    let advance = if r.attacker_won {
        r.surface_conquered
    } else {
        0
    };
    let last = frames.len().saturating_sub(1).max(1);
    let mut rounds = vec![Round {
        garrison: band,
        armies: vec![Stand {
            men: e.soldiers,
            ..Stand::default()
        }],
    }];
    rounds.extend(frames.iter().enumerate().map(|(i, f)| Round {
        garrison: f.defender_soldiers.max(0),
        armies: vec![Stand {
            men: f.attacker_soldiers.max(0),
            advance: (advance as i64 * i as i64 / last as i64) as i32,
            ..Stand::default()
        }],
    }));
    let band_left = frames.last().map_or(band, |f| f.defender_soldiers.max(0));
    FrontResult {
        armies: vec![Army {
            attacker: e.attacker,
            sent: e.soldiers,
            men: r.attacker_remaining_soldiers,
            victory: r.attacker_won,
            advance,
            rallied: People::default(),
            killed: People::default(),
            line: Line {
                arpents: land,
                ..Line::default()
            },
        }],
        rounds,
        garrison_start: band,
        garrison_left: band_left,
        annexed_by: (r.all_barbarians_conquered && land > 0).then_some(0),
    }
}

/// What `sent` men of `attacker` may bring back from `target` (`None` = the
/// barbarians), fought once on every game given.
pub fn forecast<'a>(
    games: impl IntoIterator<Item = &'a EmpireGame>,
    target: Option<Kingdoms>,
    attacker: Kingdoms,
    sent: i32,
) -> Forecast {
    match target {
        Some(t) => forecast_front(games, t, attacker, sent),
        None => {
            let mut arpents = Vec::new();
            let mut lost = Vec::new();
            let mut victories = 0;
            for game in games {
                let r = simulate_barbarian_battle(game, attacker, sent, |_| {});
                arpents.push(if r.attacker_won {
                    r.surface_conquered
                } else {
                    0
                });
                lost.push(sent - r.attacker_remaining_soldiers.max(0));
                victories += i32::from(r.attacker_won);
            }
            Forecast {
                arpents: deciles(&mut arpents),
                lost: deciles(&mut lost),
                victories,
                annexations: 0,
            }
        }
    }
}

/// Apply one fought front: the defender's losses, the ground and goods
/// changing hands, the survivors coming home.
pub fn apply_battle(game: &mut EmpireGame, b: &Fought) {
    match b.target {
        Some(t) => apply_front(game, t, &b.result),
        None => {
            let a = &b.result.armies[0];
            let arpents = a.spoils().arpents.min(game.barbarians_surface);
            game.barbarians_surface -= arpents;
            // The men have left already: only the survivors come home.
            let k = game.kingdom_mut(a.attacker);
            if !k.is_dead {
                k.surface += arpents;
                k.soldiers += a.men;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kingdom::Fate;

    fn against(target: Option<Kingdoms>, attacker: Kingdoms, soldiers: i32) -> Expedition {
        Expedition {
            attacker,
            target,
            soldiers,
        }
    }

    /// A game whose realms can pay for any army they have.
    fn rich_game() -> EmpireGame {
        let mut game = EmpireGame {
            year: FIRST_WAR_YEAR,
            ..EmpireGame::default()
        };
        for k in &mut game.kingdoms {
            k.treasury = 100_000;
            k.grain_stocks = 200_000;
        }
        game
    }

    #[test]
    fn an_expedition_costs_gold_and_grain_per_man_and_no_more_than_the_realm_can_pay() {
        let mut game = EmpireGame {
            year: FIRST_WAR_YEAR,
            ..EmpireGame::default()
        };
        let f = game.kingdom_mut(Kingdoms::France);
        f.soldiers = 500;
        f.treasury = 1500;
        f.grain_stocks = 10_000;
        // 150 by the coffers, 500 by the granaries: the coffers decide.
        assert_eq!(affordable(f), 150);
        assert_eq!(expedition_cost(150), (1500, 3000));
        let fought = march(
            &mut game,
            [
                against(None, Kingdoms::France, 100),
                against(Some(Kingdoms::Spain), Kingdoms::France, 100),
            ],
        );
        // The second expedition gets what the first left: fifty men.
        let sent: Vec<i32> = fought.iter().map(|f| f.armies()[0].sent).collect();
        assert_eq!(sent, vec![100, 50]);
        let f = game.kingdom(Kingdoms::France);
        assert_eq!((f.soldiers, f.treasury, f.grain_stocks), (350, 0, 7000));
        // Broke: nobody leaves, nothing is paid.
        let fought = march(
            &mut game,
            [against(Some(Kingdoms::Spain), Kingdoms::France, 100)],
        );
        assert!(fought.is_empty());
        assert_eq!(game.kingdom(Kingdoms::France).soldiers, 350);
    }

    #[test]
    fn the_men_leave_their_garrisons_before_any_battle() {
        let mut game = rich_game();
        game.kingdom_mut(Kingdoms::France).soldiers = 100;
        game.kingdom_mut(Kingdoms::Spain).soldiers = 100;
        let fought = march(
            &mut game,
            [
                against(Some(Kingdoms::Spain), Kingdoms::France, 60),
                against(Some(Kingdoms::France), Kingdoms::Spain, 100),
            ],
        );
        assert_eq!(fought.len(), 2);
        let on = |t: Kingdoms| fought.iter().find(|f| f.target == Some(t)).unwrap();
        // Spain sent everyone: France's expedition met its serfs.
        assert!(on(Kingdoms::Spain).levy());
        // France kept 40 at home: Spain's met 40 men of arms.
        assert!(!on(Kingdoms::France).levy());
        assert_eq!(on(Kingdoms::France).result.garrison_start, 40);
    }

    #[test]
    fn senseless_expeditions_are_dropped_and_men_capped() {
        let mut game = EmpireGame {
            year: FIRST_WAR_YEAR,
            ..EmpireGame::default()
        };
        game.kingdom_mut(Kingdoms::France).soldiers = 30;
        game.kingdom_mut(Kingdoms::Spain).is_dead = true;
        let fought = march(
            &mut game,
            [
                against(Some(Kingdoms::Spain), Kingdoms::France, 10),
                against(Some(Kingdoms::France), Kingdoms::France, 10),
                against(None, Kingdoms::France, 50),
                against(None, Kingdoms::France, 50),
            ],
        );
        assert_eq!(fought.len(), 1);
        assert_eq!(fought[0].armies()[0].sent, 30);
        assert_eq!(game.kingdom(Kingdoms::France).soldiers, 0);
    }

    #[test]
    fn armies_on_one_realm_fight_one_front() {
        let mut game = EmpireGame {
            year: FIRST_WAR_YEAR,
            ..EmpireGame::default()
        };
        for id in [Kingdoms::France, Kingdoms::Germany, Kingdoms::Britanny] {
            game.kingdom_mut(id).soldiers = 100;
        }
        let fought = march(
            &mut game,
            [
                against(Some(Kingdoms::Spain), Kingdoms::France, 30),
                against(None, Kingdoms::France, 10),
                against(Some(Kingdoms::Spain), Kingdoms::Germany, 30),
                against(Some(Kingdoms::France), Kingdoms::Britanny, 30),
            ],
        );
        let mut targets: Vec<Option<Kingdoms>> = fought.iter().map(|f| f.target).collect();
        targets.sort_by_key(|t| t.map(|k| k.index()));
        assert_eq!(
            targets,
            vec![None, Some(Kingdoms::France), Some(Kingdoms::Spain)]
        );
        let on = |t: Kingdoms| fought.iter().find(|f| f.target == Some(t)).unwrap();
        let mut spain: Vec<Kingdoms> = on(Kingdoms::Spain)
            .armies()
            .iter()
            .map(|a| a.attacker)
            .collect();
        spain.sort_by_key(|k| k.index());
        assert_eq!(spain, vec![Kingdoms::France, Kingdoms::Germany]);
        assert_eq!(on(Kingdoms::Spain).armies()[0].line.arpents, 5000);
        // Britanny met France's garrison as it stood after the men left.
        assert_eq!(on(Kingdoms::France).result.garrison_start, 60);
    }

    #[test]
    fn a_defeat_takes_nothing() {
        let mut game = EmpireGame {
            year: FIRST_WAR_YEAR,
            ..EmpireGame::default()
        };
        game.kingdom_mut(Kingdoms::France).soldiers = 5;
        game.kingdom_mut(Kingdoms::Spain).soldiers = 5000;
        let fought = march(
            &mut game,
            [against(Some(Kingdoms::Spain), Kingdoms::France, 5)],
        );
        assert!(!fought[0].armies()[0].victory);
        assert_eq!(fought[0].result.spoils(), crate::front::Spoils::default());
        apply_battle(&mut game, &fought[0]);
        assert_eq!(game.kingdom(Kingdoms::Spain).surface, 10_000);
        assert_eq!(game.kingdom(Kingdoms::France).soldiers, 0);
    }

    #[test]
    fn no_realm_can_be_marched_on_before_the_third_year() {
        let mut game = rich_game();
        game.year = FIRST_WAR_YEAR - 1;
        game.kingdom_mut(Kingdoms::France).soldiers = 500;
        let before = game.kingdom(Kingdoms::France).clone();
        let fought = march(
            &mut game,
            [
                against(Some(Kingdoms::Germany), Kingdoms::France, 200),
                against(None, Kingdoms::France, 100),
            ],
        );
        // Only the raid marched; the men meant for Germany stayed home
        // and paid nothing.
        assert_eq!(fought.len(), 1);
        assert_eq!(fought[0].target, None);
        let after = game.kingdom(Kingdoms::France);
        assert_eq!(after.soldiers, 400);
        let (gold, grain) = expedition_cost(100);
        assert_eq!(after.treasury, before.treasury - gold);
        assert_eq!(after.grain_stocks, before.grain_stocks - grain);
    }

    #[test]
    fn barbarian_raids_take_at_most_the_land_there_is() {
        let mut game = rich_game();
        game.barbarians_surface = 40;
        game.kingdom_mut(Kingdoms::France).soldiers = 5000;
        game.kingdom_mut(Kingdoms::Germany).soldiers = 5000;
        let fought = march(
            &mut game,
            [
                against(None, Kingdoms::France, 5000),
                against(None, Kingdoms::Germany, 5000),
            ],
        );
        assert_eq!(fought.len(), 2);
        let taken: i32 = fought.iter().map(|f| f.result.spoils().arpents).sum();
        assert!(taken <= 40);
        let conquered = fought.iter().filter(|f| f.annexed_by().is_some()).count();
        assert!(conquered <= 1);
        for f in &fought {
            let a = &f.result.armies[0];
            assert!(a.advance <= a.line.arpents);
            assert_eq!(f.result.rounds[0].armies[0].men, 5000);
            assert!(f
                .result
                .rounds
                .windows(2)
                .all(|w| w[0].armies[0].advance <= w[1].armies[0].advance));
            apply_battle(&mut game, f);
        }
        assert_eq!(game.barbarians_surface, 40 - taken);
        let raid = fought
            .iter()
            .find(|f| f.armies()[0].attacker == Kingdoms::France)
            .unwrap();
        let f = game.kingdom(Kingdoms::France);
        assert_eq!(f.surface, 10_000 + raid.result.spoils().arpents);
        assert_eq!(f.soldiers, raid.armies()[0].men);
    }

    #[test]
    fn no_raid_marches_on_barbarians_who_fled() {
        let mut game = rich_game();
        game.barbarians_surface = 0;
        game.kingdom_mut(Kingdoms::France).soldiers = 100;
        let fought = march(&mut game, [against(None, Kingdoms::France, 100)]);
        assert!(fought.is_empty());
        assert_eq!(game.kingdom(Kingdoms::France).soldiers, 100);
    }

    #[test]
    fn a_realm_falls_when_its_last_defender_does() {
        let mut game = rich_game();
        game.kingdom_mut(Kingdoms::France).soldiers = 5000;
        let s = game.kingdom_mut(Kingdoms::Spain);
        s.soldiers = 1;
        s.peasants = 3;
        let fought = march(
            &mut game,
            [against(Some(Kingdoms::Spain), Kingdoms::France, 5000)],
        );
        assert_eq!(fought[0].annexed_by(), Some(Kingdoms::France));
        apply_battle(&mut game, &fought[0]);
        let d = game.kingdom(Kingdoms::Spain);
        assert!(d.is_dead);
        assert_eq!(d.fate, Some(Fate::Annexed(Kingdoms::France)));
        assert_eq!(game.kingdom(Kingdoms::France).surface, 20_000);
        assert_eq!(game.alive_kingdoms().len(), 5);
    }

    #[test]
    fn defender_losses_add_up_across_fronts() {
        let mut game = rich_game();
        game.kingdom_mut(Kingdoms::Spain).soldiers = 2000;
        game.kingdom_mut(Kingdoms::France).soldiers = 300;
        game.kingdom_mut(Kingdoms::Germany).soldiers = 300;
        let fought = march(
            &mut game,
            [
                against(Some(Kingdoms::Spain), Kingdoms::France, 300),
                against(Some(Kingdoms::Spain), Kingdoms::Germany, 300),
            ],
        );
        assert_eq!(fought.len(), 1);
        let r = &fought[0].result;
        apply_battle(&mut game, &fought[0]);
        assert_eq!(game.kingdom(Kingdoms::Spain).soldiers, r.garrison_left);
        assert_eq!(
            game.kingdom(Kingdoms::Spain).surface,
            10_000 - r.spoils().arpents
        );
    }
}
