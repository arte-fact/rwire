//! The year's campaign: every expedition of every realm, declared at once and
//! fought at once. The men leave their garrisons first, so each front meets
//! the defender as it then stands. All the armies marching on one realm fight
//! it together, as one front (see [`crate::front`]); each expedition against
//! the barbarians is a front of its own, fought by the old rule, on lands
//! without end.

use crate::front::{
    apply_front, deciles, forecast_front, simulate_front, Army, Forecast, FrontResult, Line,
    People, Round, Stand,
};
use crate::game::EmpireGame;
use crate::kingdom::Kingdoms;
use crate::war::simulate_barbarian_battle;

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
/// has), then every front is fought against the realms as they now stand, in
/// the order the targets were first named. Expeditions that make no sense (a
/// fallen realm on either side, a realm marching on itself, no men) are
/// dropped.
pub fn march(
    game: &mut EmpireGame,
    expeditions: impl IntoIterator<Item = Expedition>,
) -> Vec<Fought> {
    let mut fronts: Vec<(Option<Kingdoms>, Vec<Expedition>)> = Vec::new();
    for mut e in expeditions {
        let a = game.kingdom(e.attacker);
        let sensible = !a.is_dead
            && match e.target {
                Some(t) => t != e.attacker && !game.kingdom(t).is_dead,
                None => true,
            };
        e.soldiers = e.soldiers.min(a.soldiers);
        if !sensible || e.soldiers < 1 {
            continue;
        }
        game.kingdom_mut(e.attacker).soldiers -= e.soldiers;
        match fronts.iter_mut().find(|(t, _)| *t == e.target) {
            Some((_, armies)) => armies.push(e),
            None => fronts.push((e.target, vec![e])),
        }
    }
    let field = game.clone();
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
            // Every barbarian expedition is its own front.
            None => fought.extend(armies.into_iter().map(|e| Fought {
                target,
                result: raid(&field, e),
            })),
        }
    }
    fought
}

/// An expedition against the barbarians, told as a front with one army on a
/// bare line as long as the band was worth at best: the most land one blow
/// yields, times the blows it takes to fell the band.
fn raid(field: &EmpireGame, e: Expedition) -> FrontResult {
    let mut frames = Vec::new();
    let r = simulate_barbarian_battle(field, e.attacker, e.soldiers, |p| frames.push(p.clone()));
    let band = frames
        .iter()
        .map(|f| f.defender_soldiers)
        .max()
        .unwrap_or(0);
    let tu = e.soldiers / 15 + 1;
    let worth = (band + tu - 1) / tu * (tu * 26 - 2);
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
                arpents: worth.max(advance),
                ..Line::default()
            },
        }],
        rounds,
        garrison_start: band,
        garrison_left: band_left,
        annexed_by: None,
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
            // The men have left already: only the survivors come home.
            let k = game.kingdom_mut(a.attacker);
            if !k.is_dead {
                k.surface += a.spoils().arpents;
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

    #[test]
    fn the_men_leave_their_garrisons_before_any_battle() {
        let mut game = EmpireGame::default();
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
        // Spain sent everyone: France's expedition met its serfs.
        assert!(fought[0].levy());
        // France kept 40 at home: Spain's met 40 men of arms.
        assert!(!fought[1].levy());
        assert_eq!(fought[1].result.garrison_start, 40);
    }

    #[test]
    fn senseless_expeditions_are_dropped_and_men_capped() {
        let mut game = EmpireGame::default();
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
    fn armies_on_one_realm_fight_one_front_in_the_order_the_targets_were_named() {
        let mut game = EmpireGame::default();
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
        let targets: Vec<Option<Kingdoms>> = fought.iter().map(|f| f.target).collect();
        assert_eq!(
            targets,
            vec![Some(Kingdoms::Spain), None, Some(Kingdoms::France)]
        );
        let spain: Vec<Kingdoms> = fought[0].armies().iter().map(|a| a.attacker).collect();
        assert_eq!(spain, vec![Kingdoms::France, Kingdoms::Germany]);
        assert_eq!(fought[0].armies()[0].line.arpents, 5000);
        // Britanny met France's garrison as it stood after the men left.
        assert_eq!(fought[2].result.garrison_start, 60);
    }

    #[test]
    fn a_defeat_takes_nothing() {
        let mut game = EmpireGame::default();
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
    fn barbarian_raids_are_fronts_of_their_own_on_lands_without_end() {
        let mut game = EmpireGame::default();
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
        assert!(fought.iter().all(|f| f.annexed_by().is_none()));
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
        let f = game.kingdom(Kingdoms::France);
        assert_eq!(f.surface, 10_000 + fought[0].result.spoils().arpents);
        assert_eq!(f.soldiers, fought[0].armies()[0].men);
    }

    #[test]
    fn a_realm_falls_when_its_last_defender_does() {
        let mut game = EmpireGame::default();
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
        let mut game = EmpireGame::default();
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
