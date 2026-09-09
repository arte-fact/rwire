//! A table of six brains played silently by the rules a seigneur plays by:
//! the year opens, every Intendance sits, every census is read, every
//! Extérieur orders, the armies march together, the year ends. What each
//! seat made of it comes back as an [`Outcome`], scored for evolution.

use crate::brain::{
    bound_council, decode_intendance, decode_orders, sight, Brain, Intendance, Letters, Memory,
    Stage, A_OUT,
};
use crate::campaign::{apply_battle, march, Expedition};
use crate::demography::apply_feed;
use crate::economy::{apply_economy, apply_taxes, economy_report};
use crate::events::{check_plague, check_ruler_death, RulerDeathCause};
use crate::game::EmpireGame;
use crate::harvests::{apply_grain_harvest, apply_rat_loss_rate, apply_seed_grain};
use crate::intel::{heard, Rumour, Writing, AGENT_PRICE, SCOUT_PRICE};
use crate::investments::apply_investment;
use crate::kingdom::{Kingdom, Kingdoms, PlayerTitle, Requirement, KINGDOMS};
use crate::random::shuffled;
use crate::trade::{apply_trade, Trade};

/// How one seat's game went.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Outcome {
    /// The year the imperial crown was won.
    pub crowned: Option<i32>,
    /// The year the realm fell.
    pub fell: Option<i32>,
    /// Years sat at the table.
    pub years: i32,
    /// The road to the crown (see [`progress`]), averaged over the years
    /// sat: how far along it the realm kept itself — early and lasting,
    /// not a last-year rush.
    pub progress: f32,
    /// The people's ledger over the years sat: serfs born, foreigners
    /// settled, nobles come to court; serfs dead of hunger (malnutrition
    /// and famine alike), nobles gone from it.
    pub born: i32,
    pub settled: i32,
    pub nobles_come: i32,
    pub starved: i32,
    pub nobles_gone: i32,
    /// The first year the realm stood as Prince, and as King.
    pub prince: Option<i32>,
    pub king: Option<i32>,
    /// Reports read: the éclaireurs' on their return, the agents' letters.
    pub readings: i32,
}

impl Outcome {
    /// The score evolution climbs: a crown is worth everything, sooner is
    /// better; short of it, staying alive, then the road covered. The
    /// titles on the way are stairs up to it, each paid once, the sooner
    /// the more. The people count every year, crowned or not: each birth,
    /// each settler and each noble come to court pays, each serf dead of
    /// hunger and each noble gone costs — the crown's figures are reached
    /// by a court that draws people, and the fall a famine may bring is
    /// too rare a lesson.
    pub fn fitness(&self, longest: i32) -> f32 {
        let road = match self.crowned {
            Some(year) => 1000.0 - year as f32,
            None => 100.0 * self.years as f32 / longest as f32 + ROAD_WORTH * self.progress,
        };
        let prince = self.prince.map_or(0.0, |y| PRINCE_WORTH - y as f32 / 2.0);
        let king = self.king.map_or(0.0, |y| KING_WORTH - y as f32);
        let people = BIRTH_WORTH * self.born as f32 + SETTLER_WORTH * self.settled as f32
            - HUNGER_COST * self.starved as f32;
        let court = NOBLE_WORTH * (self.nobles_come - self.nobles_gone) as f32;
        road + prince + king + people + court
    }

    /// How many titles the realm rose through.
    fn titles(&self) -> f32 {
        f32::from(u8::from(self.prince.is_some())) + f32::from(u8::from(self.king.is_some()))
    }

    /// The order seats finish in: the crowned first and the sooner the
    /// better, then the living by title then by the road covered, then
    /// the fallen by how long they stood.
    fn standing(&self) -> (u8, f32) {
        match (self.crowned, self.fell) {
            (Some(year), _) => (0, year as f32),
            (None, None) => (1, -(self.titles() + self.progress)),
            (None, Some(year)) => (2, -year as f32),
        }
    }
}

/// What the road fully covered ([`progress`] at one) is worth against a
/// hundred for a whole game survived.
const ROAD_WORTH: f32 = 200.0;

/// What standing as Prince is worth the year it is first reached, less
/// half a point a year; as King, less a point a year. Stairs up to the
/// crown, each of the order of a whole road.
const PRINCE_WORTH: f32 = 150.0;
const KING_WORTH: f32 = 300.0;

/// What the people are worth, a head at a time — a dense signal beside the
/// road, never a rival to it: a whole game's ledger (thirty thousand
/// births, twenty-five thousand settlers over 150 years) comes to a road's
/// worth, well under a crown. A serf dead of hunger costs five settlers,
/// so that a people drawn by a lavish ration and starved the next year is
/// a loss, not a harvest; a noble come to court or gone from it weighs
/// thirty settlers, so the last nobles to the crown pull.
const BIRTH_WORTH: f32 = 0.002;
const SETTLER_WORTH: f32 = 0.01;
const HUNGER_COST: f32 = 0.05;
const NOBLE_WORTH: f32 = 0.3;

/// The most [`progress`] can reach.
pub const WHOLE_ROAD: f32 = 1.0;

/// The road covered towards the crown: the mean, over what the imperial
/// crown asks — the serfs, the arpents per serf, the nobles, the mills,
/// the marketplaces, the foundry, the palaces — of the share reached of
/// each. Every figure weighs the same, since any one missing withholds
/// the crown; the treasury counts for nothing, a palace saved for is no
/// palace. Before the barbarians are taught the land ratio is out of
/// reach and left out, so that starving the serfs does not read as
/// progress.
pub fn progress(k: &Kingdom, stage: Stage) -> f32 {
    let asked: Vec<f32> = k
        .progress(PlayerTitle::Emperor)
        .into_iter()
        .filter(|c| c.what != Requirement::LandRatio || stage.barbarians())
        .map(|c| (c.have as f32 / c.need as f32).min(1.0))
        .collect();
    asked.iter().sum::<f32>() / asked.len() as f32
}

/// The game's end: every seat crowned or fallen. Each seat is scored on
/// its own road to the crown, so a crown ends nothing for the others —
/// the crowned realm plays on at peace, its armies home. (A seigneur's
/// game ends at the first Emperor; a brain that reaches its own crown
/// soonest while keeping its realm is the one that gets there first.)
fn over(outcomes: &[Outcome; 6]) -> bool {
    outcomes
        .iter()
        .all(|o| o.crowned.is_some() || o.fell.is_some())
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
    /// What every seat that finished ahead costs a seat, once war is
    /// taught. Nothing by default: each seat is scored on its own road,
    /// so that war pays only what it brings to one's own crown.
    pub rank_cost: f32,
    /// What intelligence costs nothing: a school of letters. A brain that
    /// never bought a report cannot learn to read one, and a report it
    /// cannot read only muddles it, so it never buys one — the letters
    /// are free while the reading is learnt, then paid.
    pub letters: Letters,
}

impl Table {
    pub fn at(stage: Stage, longest: i32) -> Table {
        Table {
            stage,
            seats: [stage; 6],
            longest,
            rank_cost: 0.0,
            letters: Letters::None,
        }
    }

    /// What each seat scores: its own fitness — and, once war is taught
    /// and a `rank_cost` set, that much less for every seat that finished
    /// ahead of it.
    pub fn scores(&self, outcomes: &[Outcome; 6]) -> [f32; 6] {
        std::array::from_fn(|i| {
            let own = outcomes[i].fitness(self.longest);
            if !self.stage.war() || self.rank_cost == 0.0 {
                return own;
            }
            let mine = outcomes[i].standing();
            let ahead = outcomes
                .iter()
                .filter(|o| {
                    let s = o.standing();
                    s.0 < mine.0 || (s.0 == mine.0 && s.1 < mine.1)
                })
                .count();
            own - self.rank_cost * ahead as f32
        })
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
        ..
    } = *table;
    let mut game = EmpireGame::default();
    let mut memories: [Memory; 6] = Default::default();
    let mut outcomes = [Outcome::default(); 6];
    let mut scouts: Vec<(Kingdoms, Kingdoms)> = Vec::new();
    // The agents each seat wants in place, by seat then realm.
    let mut agents = [[false; 6]; 6];
    while game.year <= longest && !over(&outcomes) {
        let year = game.year;
        // Bushels bought this year, by buyer then seller: what an agent reads.
        let mut bought = [[0; 6]; 6];
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
        // The Intendances sit one after the other at the same stalls, in an
        // order drawn every year: the first served buys the cheapest grain.
        let mut answers = [[0.0f32; A_OUT]; 6];
        for id in shuffled(KINGDOMS) {
            if game.kingdom(id).is_dead {
                continue;
            }
            let i = id.index();
            let s = sight(&game, id, &memories[i]);
            let a = brains[i].intendance.forward(&s);
            answers[i].copy_from_slice(&a);
            intendance(
                &mut game,
                id,
                &a,
                seats[i],
                &mut memories[i],
                &mut bought[i],
            );
        }
        // The éclaireurs sent last year report as the Extérieur opens; then
        // the agents: kept where wanted and paid for the year, bought on a
        // fresh report where wanted and absent, dismissed elsewhere.
        for (id, on) in scouts.drain(..) {
            memories[id.index()].dossiers[on.index()].scout(game.kingdom(on), year);
            outcomes[id.index()].readings += 1;
        }
        for id in KINGDOMS {
            let i = id.index();
            if game.kingdom(id).is_dead {
                continue;
            }
            for on in KINGDOMS {
                if on == id {
                    continue;
                }
                let d = &mut memories[i].dossiers[on.index()];
                if !agents[i][on.index()] {
                    d.agent = false;
                    continue;
                }
                let standing = d.agent;
                let free = table.letters.agents();
                if !standing {
                    let fresh = d.report.is_some_and(|r| r.year == year);
                    let k = game.kingdom_mut(id);
                    if !fresh || (!free && k.treasury < AGENT_PRICE) {
                        continue;
                    }
                    if !free {
                        k.treasury -= AGENT_PRICE;
                    }
                }
                // A free agent is paid from a purse that is not the realm's.
                let mut treasury = if free {
                    i32::MAX / 2
                } else {
                    game.kingdom(id).treasury
                };
                let writing = d.agent_year(
                    game.kingdom(on),
                    year,
                    bought[on.index()],
                    &mut treasury,
                    standing,
                );
                if !free {
                    game.kingdom_mut(id).treasury = treasury;
                }
                if matches!(writing, Writing::Letter { .. }) {
                    outcomes[i].readings += 1;
                }
            }
        }
        let mut expeditions = Vec::new();
        for id in KINGDOMS {
            let k = game.kingdom(id);
            if k.is_dead {
                continue;
            }
            let i = id.index();
            let mut x = sight(&game, id, &memories[i]);
            x.extend(answers[i]);
            let o = brains[i].exterieur.forward(&x);
            let orders = decode_orders(&o, k, &game, seats[i], table.letters);
            memories[i].last_orders.copy_from_slice(&o);
            if outcomes[i].crowned.is_some() {
                continue;
            }
            expeditions.extend(orders.expeditions.into_iter().map(|(target, soldiers)| {
                Expedition {
                    attacker: id,
                    target,
                    soldiers,
                }
            }));
            if let Some(on) = orders.scout {
                if !table.letters.scouts() {
                    game.kingdom_mut(id).treasury -= SCOUT_PRICE;
                }
                scouts.push((id, on));
            }
            agents[i] = [false; 6];
            for on in orders.agents {
                agents[i][on.index()] = true;
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
            let starvation = m.demo.as_ref().map_or(0, |d| d.starvation_victims);
            // Plague is weathered; the 1 % death is spared, luck teaches nothing.
            m.plague = check_plague(k).is_some();
            let death = check_ruler_death(k, starvation, false);
            outcomes[i].years = year;
            if let Some(d) = &m.demo {
                let o = &mut outcomes[i];
                o.born += d.births;
                o.settled += d.immigrants;
                o.nobles_come += d.nobles_immigrants;
                o.starved += d.malnutrition_victims + d.starvation_victims;
                o.nobles_gone += d.nobles_departed;
            }
            let p = progress(k, stage);
            outcomes[i].progress += (p - outcomes[i].progress) / year as f32;
            deaths[i] = death;
            let title = k.title();
            if death.is_some() {
                outcomes[i].fell = Some(year);
            } else if title == PlayerTitle::Emperor {
                outcomes[i].crowned = Some(year);
            }
            if title >= PlayerTitle::Prince {
                outcomes[i].prince.get_or_insert(year);
            }
            if title >= PlayerTitle::King {
                outcomes[i].king.get_or_insert(year);
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
fn intendance(
    game: &mut EmpireGame,
    id: Kingdoms,
    answer: &[f32],
    stage: Stage,
    m: &mut Memory,
    bought: &mut [i32; 6],
) {
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
        bought[seller.index()] += amount;
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
        for stage in [
            Stage::Survive,
            Stage::Emperor,
            Stage::Market,
            Stage::Guard,
            Stage::War,
        ] {
            let outcomes = play(table, &Table::at(stage, 40));
            for o in outcomes {
                assert!(o.years >= 1 && o.years <= 40, "{o:?}");
                assert!((0.0..=WHOLE_ROAD).contains(&o.progress) && o.fitness(40).is_finite());
            }
        }
    }

    #[test]
    fn at_a_school_of_letters_the_reports_come_and_cost_nothing() {
        // Brains that mean to send: "no one" pushed down, the agents up.
        let bs: Vec<Brain> = (0..6)
            .map(|i| {
                let mut g = vec![0.02 * (i as f32 + 1.0) / 6.0; Brain::GENOME];
                g[Brain::exterieur_output(11).end - 1] = -3.0;
                for o in 12..17 {
                    g[Brain::exterieur_output(o).end - 1] = 3.0;
                }
                Brain::from_genome(&g)
            })
            .collect();
        let table: [&Brain; 6] = std::array::from_fn(|i| &bs[i]);
        let at = |letters| Table {
            letters,
            ..Table::at(Stage::War, 40)
        };
        // Free, the éclaireur goes every year (the last one reports after
        // the game, or never to a court that fell) and the agents write
        // on top of him.
        for o in play(table, &at(Letters::Scouts)) {
            assert!(o.readings >= o.years - 2, "{o:?}");
        }
        for o in play(table, &at(Letters::All)) {
            assert!(o.readings > o.years - 2, "{o:?}");
        }
    }

    #[test]
    fn a_crown_outscores_any_road_and_an_early_one_a_late_one() {
        let road = Outcome {
            crowned: None,
            fell: None,
            years: 150,
            progress: WHOLE_ROAD,
            ..Outcome::default()
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
    fn the_starved_cost_a_crown_its_years() {
        let fed = Outcome {
            crowned: Some(30),
            fell: None,
            years: 30,
            progress: WHOLE_ROAD,
            ..Outcome::default()
        };
        let famine = Outcome {
            starved: 400,
            ..fed
        };
        let later = Outcome {
            crowned: Some(40),
            years: 40,
            ..fed
        };
        assert!(famine.fitness(150) < fed.fitness(150));
        assert!(famine.fitness(150) < later.fitness(150));
    }

    #[test]
    fn the_people_pay_a_head_at_a_time_and_the_court_most_of_all() {
        let road = Outcome {
            crowned: None,
            fell: None,
            years: 60,
            progress: 0.6,
            ..Outcome::default()
        };
        let peopled = Outcome {
            born: 30_000,
            settled: 20_000,
            ..road
        };
        let courted = Outcome {
            nobles_come: 300,
            ..road
        };
        let deserted = Outcome {
            nobles_come: 300,
            nobles_gone: 300,
            ..road
        };
        // A long game's births and settlers are worth a road and a third;
        // three hundred nobles come to court about a third as much, and
        // gone again leave nothing.
        assert!((peopled.fitness(150) - road.fitness(150) - 260.0).abs() < 1e-3);
        assert!((courted.fitness(150) - road.fitness(150) - 90.0).abs() < 1e-2);
        assert_eq!(deserted.fitness(150), road.fitness(150));
        // A famine's dead cost five settlers apiece, malnutrition counted
        // with them: a thousand settlers starved lose four times what they
        // paid.
        let starved = Outcome {
            settled: 1000,
            starved: 1000,
            ..road
        };
        assert!((road.fitness(150) - starved.fitness(150) - 40.0).abs() < 1e-2);
    }

    #[test]
    fn at_war_the_table_ranks_its_seats() {
        let alive = Outcome {
            crowned: None,
            fell: None,
            years: 90,
            progress: 0.8,
            ..Outcome::default()
        };
        let outcomes = [
            Outcome {
                crowned: Some(90),
                ..alive
            },
            alive,
            Outcome {
                progress: 0.5,
                ..alive
            },
            Outcome {
                fell: Some(70),
                years: 70,
                ..alive
            },
            Outcome {
                fell: Some(30),
                years: 30,
                ..alive
            },
            Outcome {
                fell: Some(30),
                years: 30,
                ..alive
            },
        ];
        let ranked = Table {
            rank_cost: 50.0,
            ..Table::at(Stage::War, 150)
        };
        let war = ranked.scores(&outcomes);
        for i in 0..5 {
            assert!(war[i] > war[i + 1] || i == 4, "{war:?}");
        }
        // The two that fell the same year share their rank.
        assert_eq!(war[4], war[5]);
        assert_eq!(war[1], alive.fitness(150) - 50.0);
        // Before war, and at war unless a rank cost is set, every seat is
        // scored on its own.
        for alone in [
            Table::at(Stage::Market, 150).scores(&outcomes),
            Table::at(Stage::War, 150).scores(&outcomes),
        ] {
            assert!(
                (alone[1] - alone[2] - 0.3 * ROAD_WORTH).abs() < 1e-3,
                "{alone:?}"
            );
            assert_eq!(alone[4], alone[5]);
        }
    }

    #[test]
    fn the_land_ratio_counts_only_once_the_barbarians_are_taught() {
        let k = Kingdom::new(Kingdoms::France);
        assert!(progress(&k, Stage::Emperor) > progress(&k, Stage::Survive));
    }

    #[test]
    fn gold_saved_is_no_building_and_every_figure_weighs_the_same() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.treasury = 0;
        let bare = progress(&k, Stage::Emperor);
        k.treasury = 1_000_000;
        assert_eq!(progress(&k, Stage::Emperor), bare);
        k.palaces += 1;
        let palace = progress(&k, Stage::Emperor) - bare;
        k.palaces -= 1;
        k.marketplaces += 1;
        let marketplace = progress(&k, Stage::Emperor) - bare;
        // A palace is one of ten asked, a marketplace one of fourteen.
        assert!((palace - 1.0 / 10.0 / 7.0).abs() < 1e-6, "{palace}");
        assert!(
            (marketplace - 1.0 / 14.0 / 7.0).abs() < 1e-6,
            "{marketplace}"
        );
        k.peasants = 10_000;
        k.nobles = 100;
        k.marketplaces = 20;
        k.grain_mills = 10;
        k.foundries = 2;
        k.palaces = 12;
        k.surface = 1_000_000;
        assert!((progress(&k, Stage::Emperor) - WHOLE_ROAD).abs() < 1e-6);
    }

    #[test]
    fn a_title_pays_once_and_the_sooner_the_more() {
        let duke = Outcome {
            crowned: None,
            fell: None,
            years: 150,
            progress: 0.5,
            ..Outcome::default()
        };
        let prince = Outcome {
            prince: Some(40),
            ..duke
        };
        let early_prince = Outcome {
            prince: Some(20),
            ..duke
        };
        let king = Outcome {
            king: Some(80),
            ..prince
        };
        assert!(prince.fitness(150) > duke.fitness(150));
        assert!(early_prince.fitness(150) > prince.fitness(150));
        assert!(king.fitness(150) > prince.fitness(150));
        // A whole road short of the crown stays under a King.
        let road = Outcome {
            progress: WHOLE_ROAD,
            ..duke
        };
        assert!(king.fitness(150) > road.fitness(150));
        // At the table, a King outranks a Duke further down the road.
        let ranked = Table {
            rank_cost: 50.0,
            ..Table::at(Stage::War, 150)
        };
        let scores = ranked.scores(&[king, road, duke, duke, duke, duke]);
        assert_eq!(scores[0], king.fitness(150));
        assert_eq!(scores[1], road.fitness(150) - 50.0);
    }

    #[test]
    fn the_titles_reached_are_dated_at_the_table() {
        let brain = Brain::from_genome(&vec![0.0; Brain::GENOME]);
        let outcomes = play([&brain; 6], &Table::at(Stage::War, 30));
        for o in outcomes {
            assert!(o.prince.is_none_or(|y| (1..=30).contains(&y)));
            assert!(o.king.is_none_or(|y| o.prince.is_some_and(|p| p <= y)));
        }
    }
}
