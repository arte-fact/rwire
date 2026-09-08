//! A table of six brains played silently by the rules a seigneur plays by:
//! the year opens, every Intendance sits, every census is read, every
//! Extérieur orders, the armies march together, the year ends. What each
//! seat made of it comes back as an [`Outcome`], scored for evolution.

use crate::brain::{
    bound_council, decode_intendance, decode_orders, sight, Brain, Intendance, Memory, Stage, A_OUT,
};
use crate::campaign::{apply_battle, march, Expedition};
use crate::demography::apply_feed;
use crate::economy::{apply_economy, apply_taxes, economy_report};
use crate::events::{check_plague, check_ruler_death, RulerDeathCause};
use crate::game::EmpireGame;
use crate::harvests::{apply_grain_harvest, apply_rat_loss_rate, apply_seed_grain};
use crate::intel::{heard, Rumour, SCOUT_PRICE};
use crate::investments::apply_investment;
use crate::kingdom::{Kingdom, Kingdoms, PlayerTitle, Requirement, KINGDOMS};
use crate::trade::{apply_trade, Trade};

/// How one seat's game went.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outcome {
    /// The year the imperial crown was won.
    pub crowned: Option<i32>,
    /// The year the realm fell.
    pub fell: Option<i32>,
    /// Years sat at the table.
    pub years: i32,
    /// Requirements of the three ranks, each as a share of what is asked,
    /// summed, averaged over the years sat: how far along the road to the
    /// crown the realm kept itself — early and lasting, not a last-year rush.
    pub progress: f32,
}

impl Outcome {
    /// The score evolution climbs: a crown is worth everything, sooner is
    /// better; short of it, staying alive, then the road covered.
    pub fn fitness(&self, longest: i32) -> f32 {
        match self.crowned {
            Some(year) => 1000.0 - year as f32,
            None => 100.0 * self.years as f32 / longest as f32 + 10.0 * self.progress,
        }
    }
}

/// The road covered towards the crown: every requirement of every rank as a
/// share of what is asked, capped at one. Before the barbarians are taught
/// the land ratio is out of reach and left out, so that starving the serfs
/// does not read as progress.
pub fn progress(k: &Kingdom, stage: Stage) -> f32 {
    [PlayerTitle::Prince, PlayerTitle::King, PlayerTitle::Emperor]
        .into_iter()
        .flat_map(|t| k.progress(t))
        .filter(|c| stage.barbarians() || c.what != Requirement::LandRatio)
        .map(|c| (c.have as f32 / c.need as f32).min(1.0))
        .sum()
}

/// The game's end. Once war is taught, the real one: the first Emperor, or
/// the last realm alive. Before, the seats are clones of one brain and
/// each is scored on its own: the table plays until every seat is crowned
/// or fallen, a crown ending nothing for the others.
fn over(game: &EmpireGame, stage: Stage, outcomes: &[Outcome; 6]) -> bool {
    if stage.war() {
        let alive = game.alive_kingdoms().len();
        game.kingdoms
            .iter()
            .any(|k| !k.is_dead && k.title() == PlayerTitle::Emperor)
            || alive <= 1
    } else {
        outcomes
            .iter()
            .all(|o| o.crowned.is_some() || o.fell.is_some())
    }
}

/// What a year left behind, shown to whoever watches a table.
pub struct YearEnd<'a> {
    pub game: &'a EmpireGame,
    pub memories: &'a [Memory; 6],
    pub deaths: [Option<RulerDeathCause>; 6],
}

/// How a game is set.
#[derive(Debug, Clone, Copy)]
pub struct Table {
    /// The rung the table is played at: what ends the game, what counts as
    /// progress.
    pub stage: Stage,
    /// The rung each seat's brain is read at (a brain schooled before the
    /// market lists nothing at a market table).
    pub seats: [Stage; 6],
    /// The game is stopped after this many years.
    pub longest: i32,
}

impl Table {
    pub fn at(stage: Stage, longest: i32) -> Table {
        Table {
            stage,
            seats: [stage; 6],
            longest,
        }
    }
}

/// Play one game of `brains` at `table`.
pub fn play(brains: [&Brain; 6], table: &Table) -> [Outcome; 6] {
    watch(brains, table, |_| {})
}

/// [`play`], with `watch` called at the end of every year.
pub fn watch(brains: [&Brain; 6], table: &Table, mut watch: impl FnMut(YearEnd)) -> [Outcome; 6] {
    let Table {
        stage,
        seats,
        longest,
    } = *table;
    let mut game = EmpireGame::default();
    let mut memories: [Memory; 6] = Default::default();
    let mut outcomes = [Outcome {
        crowned: None,
        fell: None,
        years: 0,
        progress: 0.0,
    }; 6];
    let mut scouts: Vec<(Kingdoms, Kingdoms)> = Vec::new();
    while game.year <= longest && !over(&game, stage, &outcomes) {
        let year = game.year;
        game.random_weather();
        game.open_market();
        for k in &mut game.kingdoms {
            if k.is_dead {
                continue;
            }
            apply_seed_grain(k);
            apply_rat_loss_rate(k);
            apply_grain_harvest(k, k.weather);
        }
        // The Intendances sit, one after the other at the same stalls.
        let mut answers = [[0.0f32; A_OUT]; 6];
        for id in KINGDOMS {
            if game.kingdom(id).is_dead {
                continue;
            }
            let i = id.index();
            let s = sight(&game.kingdoms, year, id, &memories[i]);
            let a = brains[i].intendance.forward(&s);
            answers[i].copy_from_slice(&a);
            intendance(&mut game, id, &a, seats[i], &mut memories[i]);
        }
        // The éclaireurs sent last year report as the Extérieur opens.
        for (id, on) in scouts.drain(..) {
            memories[id.index()].dossiers[on.index()].scout(game.kingdom(on), year);
        }
        let mut expeditions = Vec::new();
        for id in KINGDOMS {
            let k = game.kingdom(id);
            if k.is_dead {
                continue;
            }
            let i = id.index();
            let mut x = sight(&game.kingdoms, year, id, &memories[i]);
            x.extend(answers[i]);
            let o = brains[i].exterieur.forward(&x);
            let orders = decode_orders(&o, k, &game.kingdoms, seats[i]);
            memories[i].last_orders.copy_from_slice(&o);
            expeditions.extend(orders.expeditions.into_iter().map(|(target, soldiers)| {
                Expedition {
                    attacker: id,
                    target,
                    soldiers,
                }
            }));
            if let Some(on) = orders.scout {
                game.kingdom_mut(id).treasury -= SCOUT_PRICE;
                scouts.push((id, on));
            }
        }
        let fought = march(&mut game, expeditions);
        let mut rumours = Vec::new();
        for f in &fought {
            apply_battle(&mut game, f);
            let annexed = f.target.is_some_and(|t| game.kingdom(t).is_dead);
            rumours.extend(f.armies().iter().map(|a| Rumour {
                year,
                attacker: a.attacker,
                target: f.target,
                victory: a.victory,
                arpents: a.spoils().arpents,
                annexed: annexed && a.victory,
            }));
        }
        let heard = heard(&rumours);
        // The year ends: plague, the ruler's death, the ranks judged.
        let mut deaths = [None; 6];
        for id in KINGDOMS {
            let i = id.index();
            let m = &mut memories[i];
            m.heard = heard;
            let k = game.kingdom_mut(id);
            if outcomes[i].crowned.is_some() {
                // Crowned already: the seat plays on unscored.
                continue;
            }
            if k.is_dead {
                if outcomes[i].fell.is_none() {
                    // Annexed this campaign.
                    outcomes[i].fell = Some(year);
                }
                continue;
            }
            let starved = m.demo.as_ref().map_or(0, |d| d.starvation_victims);
            // Plague is weathered; the 1 % death is spared, luck teaches nothing.
            m.plague = check_plague(k).is_some();
            let death = check_ruler_death(k, starved, false);
            outcomes[i].years = year;
            let p = progress(k, stage);
            outcomes[i].progress += (p - outcomes[i].progress) / year as f32;
            deaths[i] = death;
            if death.is_some() {
                outcomes[i].fell = Some(year);
            } else if k.title() == PlayerTitle::Emperor {
                outcomes[i].crowned = Some(year);
            }
        }
        watch(YearEnd {
            game: &game,
            memories: &memories,
            deaths,
        });
        game.increment_year();
    }
    outcomes
}

/// One seat's Intendance: the answer applied as a seigneur's roll is sealed
/// — market, purchases, then the council promulgated and its census read.
fn intendance(game: &mut EmpireGame, id: Kingdoms, answer: &[f32], stage: Stage, m: &mut Memory) {
    let stalls: Vec<(Kingdoms, i32, i32)> = game
        .kingdoms
        .iter()
        .filter(|s| s.id != id && !s.is_dead)
        .map(|s| (s.id, s.for_sale(), s.grain_price))
        .collect();
    let d = decode_intendance(answer, game.kingdom(id), &stalls, stage);
    if let Some((amount, price)) = d.listed {
        apply_trade(game, id, Trade::Sell { amount, price });
    }
    if let Some((seller, amount)) = d.bought {
        apply_trade(game, id, Trade::Buy { amount, seller });
    }
    if d.land_sold > 0 {
        apply_trade(
            game,
            id,
            Trade::SellLand {
                arpents: d.land_sold,
            },
        );
    }
    let k = game.kingdom_mut(id);
    for &(kind, n) in &d.purchases {
        apply_investment(k, kind, n);
    }
    let council = bound_council(d.council, k);
    apply_taxes(k, council.taxes);
    let demo = apply_feed(k, council);
    m.intendance = Some(Intendance { council, ..d });
    let eco = economy_report(k, k.weather, demo.immigrants);
    apply_economy(k, &eco);
    m.demo = Some(demo);
    m.eco = Some(eco);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brains(fill: f32) -> Vec<Brain> {
        (0..6)
            .map(|i| Brain::from_genome(&vec![fill * (i as f32 + 1.0) / 6.0; Brain::GENOME]))
            .collect()
    }

    #[test]
    fn a_table_of_random_brains_plays_to_the_end_at_every_stage() {
        let bs = brains(0.02);
        let table: [&Brain; 6] = std::array::from_fn(|i| &bs[i]);
        for stage in [Stage::Survive, Stage::Emperor, Stage::Market, Stage::War] {
            let outcomes = play(table, &Table::at(stage, 40));
            for o in outcomes {
                assert!(o.years >= 1 && o.years <= 40, "{o:?}");
                assert!(o.progress >= 0.0 && o.fitness(40).is_finite());
            }
        }
    }

    #[test]
    fn a_crown_outscores_any_road_and_an_early_one_a_late_one() {
        let road = Outcome {
            crowned: None,
            fell: None,
            years: 150,
            progress: 20.0,
        };
        let late = Outcome {
            crowned: Some(140),
            ..road
        };
        let early = Outcome {
            crowned: Some(60),
            ..road
        };
        assert!(late.fitness(150) > road.fitness(150));
        assert!(early.fitness(150) > late.fitness(150));
    }

    #[test]
    fn the_land_ratio_counts_only_once_the_barbarians_are_taught() {
        let k = Kingdom::new(Kingdoms::France);
        assert!(progress(&k, Stage::Emperor) > progress(&k, Stage::Survive));
    }
}
