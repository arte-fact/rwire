//! The table as the GPU holds it: every figure a 32-bit word, every
//! `Option` a flag or a `-1`, laid out exactly as the WGSL structs of
//! `wgsl/state.wgsl` (all-scalar structs pack the same way on both
//! sides). Built from the CPU game to start a table or to re-sync a
//! conformance test, compared field by field on the way back.

use bytemuck::{Pod, Zeroable};
use empire_lib::arena;
use empire_lib::brain::{Letters, Memory, Stage, A_OUT, B_OUT};
use empire_lib::game::EmpireGame;
use empire_lib::intel::{Dossier, Heard};
use empire_lib::kingdom;
use empire_lib::random::Rng;

/// A realm, dead or alive (`dead` is 1 once fallen).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct Kingdom {
    pub dead: i32,
    pub surface: i32,
    pub peasants: i32,
    pub nobles: i32,
    pub merchants: i32,
    pub soldiers: i32,
    pub efficiency: i32,
    pub ration: i32,
    pub treasury: i32,
    pub stocks: i32,
    pub harvest: i32,
    pub weather: i32,
    pub price: i32,
    pub to_sell: i32,
    /// The lot listed for next year's market, none when 0 bushels.
    pub listed: i32,
    pub listed_price: i32,
    pub rats: i32,
    pub marketplaces: i32,
    pub mills: i32,
    pub foundries: i32,
    pub shipyards: i32,
    pub palaces: i32,
    pub forts: i32,
    pub hospices: i32,
    pub rams: i32,
    pub scouts: i32,
    pub customs: i32,
    pub sales: i32,
    pub income: i32,
}

impl Kingdom {
    pub fn from_game(k: &kingdom::Kingdom) -> Kingdom {
        let (listed, listed_price) = k.listing.unwrap_or((0, 0));
        Kingdom {
            dead: i32::from(k.is_dead),
            surface: k.surface,
            peasants: k.peasants,
            nobles: k.nobles,
            merchants: k.merchants,
            soldiers: k.soldiers,
            efficiency: k.soldiers_efficiency,
            ration: k.soldiers_ration,
            treasury: k.treasury,
            stocks: k.grain_stocks,
            harvest: k.grain_harvest,
            weather: k.weather.value(),
            price: k.grain_price,
            to_sell: k.grain_to_sell,
            listed,
            listed_price,
            rats: k.rats_loss_rate,
            marketplaces: k.marketplaces,
            mills: k.grain_mills,
            foundries: k.foundries,
            shipyards: k.shipyards,
            palaces: k.palaces,
            forts: k.fortifications,
            hospices: k.hospices,
            rams: k.rams,
            scouts: k.scouts,
            customs: k.immigration_taxes,
            sales: k.commercial_taxes,
            income: k.income_taxes,
        }
    }
}

/// The year's demography as a seat remembers it (`has` is 0 before the
/// first Intendance).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct Demo {
    pub has: i32,
    pub births: i32,
    pub immigrants: i32,
    pub nobles_immigrants: i32,
    pub merchants_immigrants: i32,
    pub merchants_settled: i32,
    pub nobles_departed: i32,
    pub merchants_departed: i32,
    pub disease: i32,
    pub malnutrition: i32,
    pub starvation: i32,
    pub efficiency: i32,
    pub soldiers_starvation: i32,
    pub soldiers_desertion: i32,
}

/// What a seat holds on a realm: the éclaireur's report, the agent's
/// ledger (flags at 0 when none was ever read; `aim` is -1 when the
/// realm has no title left to reach).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct GpuDossier {
    pub report: i32,
    pub report_year: i32,
    pub surface: i32,
    pub garrison: i32,
    pub forts: i32,
    pub efficiency: i32,
    pub ledger: i32,
    pub ledger_year: i32,
    pub treasury: i32,
    pub stocks: i32,
    pub aim: i32,
    pub met: i32,
    pub all: i32,
}

impl GpuDossier {
    fn from_memory(d: &Dossier) -> GpuDossier {
        let mut out = GpuDossier::default();
        if let Some(r) = &d.report {
            out.report = 1;
            out.report_year = r.year;
            out.surface = r.surface;
            out.garrison = r.garrison;
            out.forts = r.fortifications;
            out.efficiency = r.efficiency;
        }
        if let Some(l) = &d.ledger {
            out.ledger = 1;
            out.ledger_year = l.year;
            out.treasury = l.treasury;
            out.stocks = l.grain_stocks;
            match l.aim {
                Some((title, met, all)) => {
                    out.aim = title as i32;
                    out.met = met;
                    out.all = all;
                }
                None => out.aim = -1,
            }
        }
        out
    }
}

/// A seat's memory: what the Intendance left, the last answers, the
/// dossiers. (What was heard of the campaign is the same for every
/// seat, so it is kept once per table.)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct GpuMemory {
    pub demo: Demo,
    pub eco: i32,
    pub net: i32,
    pub maintenance: i32,
    pub plague: i32,
    pub answer: [f32; A_OUT],
    pub orders: [f32; B_OUT],
    pub dossiers: [GpuDossier; 6],
}

impl Default for GpuMemory {
    fn default() -> Self {
        Self::zeroed()
    }
}

impl GpuMemory {
    pub fn from_memory(m: &Memory) -> GpuMemory {
        let demo = m.demo.as_ref().map_or(Demo::default(), |d| Demo {
            has: 1,
            births: d.births,
            immigrants: d.immigrants,
            nobles_immigrants: d.nobles_immigrants,
            merchants_immigrants: d.merchants_immigrants,
            merchants_settled: d.merchants_settled,
            nobles_departed: d.nobles_departed,
            merchants_departed: d.merchants_departed,
            disease: d.disease_victims,
            malnutrition: d.malnutrition_victims,
            starvation: d.starvation_victims,
            efficiency: d.soldiers_efficiency,
            soldiers_starvation: d.soldiers_starvation_victims,
            soldiers_desertion: d.soldiers_desertion_victims,
        });
        let (eco, net, maintenance) = m
            .eco
            .as_ref()
            .map_or((0, 0, 0), |e| (1, e.net(), e.soldiers_maintenance));
        GpuMemory {
            demo,
            eco,
            net,
            maintenance,
            plague: i32::from(m.plague),
            answer: m.answer,
            orders: m.last_orders,
            dossiers: std::array::from_fn(|i| GpuDossier::from_memory(&m.dossiers[i])),
        }
    }
}

/// What everyone heard of the last campaign about one realm.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct GpuHeard {
    pub marched_on: [i32; 6],
    pub marched_by: [i32; 6],
    pub beaten_by: [i32; 6],
    pub lost_to: [i32; 6],
}

impl GpuHeard {
    pub fn from_heard(h: &Heard) -> GpuHeard {
        GpuHeard {
            marched_on: h.marched_on.map(i32::from),
            marched_by: h.marched_by.map(i32::from),
            beaten_by: h.beaten_by.map(i32::from),
            lost_to: h.lost_to,
        }
    }
}

/// A seat's outcome, years at -1 while not reached.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct GpuOutcome {
    pub crowned: i32,
    pub fell: i32,
    pub starved_out: i32,
    pub years: i32,
    pub progress: f32,
    pub born: i32,
    pub settled: i32,
    pub nobles_come: i32,
    pub starved: i32,
    pub nobles_gone: i32,
    pub prince: i32,
    pub king: i32,
    pub readings: i32,
    pub walls: f32,
}

impl Default for GpuOutcome {
    fn default() -> Self {
        GpuOutcome::from_outcome(&arena::Outcome::default())
    }
}

fn year_flag(y: Option<i32>) -> i32 {
    y.unwrap_or(-1)
}

fn flagged_year(y: i32) -> Option<i32> {
    (y >= 0).then_some(y)
}

impl GpuOutcome {
    pub fn from_outcome(o: &arena::Outcome) -> GpuOutcome {
        GpuOutcome {
            crowned: year_flag(o.crowned),
            fell: year_flag(o.fell),
            starved_out: i32::from(o.starved_out),
            years: o.years,
            progress: o.progress,
            born: o.born,
            settled: o.settled,
            nobles_come: o.nobles_come,
            starved: o.starved,
            nobles_gone: o.nobles_gone,
            prince: year_flag(o.prince),
            king: year_flag(o.king),
            readings: o.readings,
            walls: o.walls,
        }
    }

    pub fn to_outcome(self) -> arena::Outcome {
        arena::Outcome {
            crowned: flagged_year(self.crowned),
            fell: flagged_year(self.fell),
            starved_out: self.starved_out != 0,
            years: self.years,
            progress: self.progress,
            born: self.born,
            settled: self.settled,
            nobles_come: self.nobles_come,
            starved: self.starved,
            nobles_gone: self.nobles_gone,
            prince: flagged_year(self.prince),
            king: flagged_year(self.king),
            readings: self.readings,
            walls: self.walls,
        }
    }
}

/// Who sits at a table and how it is played: the genome index of each
/// seat in the batch's genomes, whether each brain is told the year's
/// figures, the rung each brain is read at, the table's own rung, its
/// length and its letters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seating {
    pub genomes: [u32; 6],
    pub told: [bool; 6],
    pub seats: [Stage; 6],
    pub stage: Stage,
    pub longest: i32,
    pub letters: Letters,
}

/// A whole table, as one storage record.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Table {
    pub year: i32,
    pub barbarians: i32,
    /// 1 once the game is over or the last year played.
    pub done: i32,
    pub rng: [u32; 4],
    pub genome: [u32; 6],
    pub told: [u32; 6],
    pub seat: [u32; 6],
    pub stage: u32,
    pub longest: i32,
    pub letters: u32,
    pub kingdoms: [Kingdom; 6],
    pub memories: [GpuMemory; 6],
    pub heard: [GpuHeard; 6],
    pub outcomes: [GpuOutcome; 6],
}

impl Table {
    /// The table as it stands on the CPU: the game and the memories after
    /// a year (or fresh), the dice as they are, the outcomes so far.
    pub fn from_game(
        seating: &Seating,
        game: &EmpireGame,
        memories: &[Memory; 6],
        outcomes: &[arena::Outcome; 6],
        rng: Rng,
    ) -> Table {
        Table {
            year: game.year,
            barbarians: game.barbarians_surface,
            done: i32::from(
                game.year > seating.longest
                    || outcomes
                        .iter()
                        .all(|o| o.crowned.is_some() || o.fell.is_some()),
            ),
            rng: rng.words(),
            genome: seating.genomes,
            told: seating.told.map(u32::from),
            seat: seating.seats.map(|s| s as u32),
            stage: seating.stage as u32,
            longest: seating.longest,
            letters: seating.letters as u32,
            kingdoms: std::array::from_fn(|i| Kingdom::from_game(&game.kingdoms[i])),
            memories: std::array::from_fn(|i| GpuMemory::from_memory(&memories[i])),
            heard: std::array::from_fn(|i| GpuHeard::from_heard(&memories[0].heard[i])),
            outcomes: std::array::from_fn(|i| GpuOutcome::from_outcome(&outcomes[i])),
        }
    }

    /// A fresh table on a fresh game, seeded like the CPU's.
    pub fn fresh(seating: &Seating, game: &EmpireGame, rng: Rng) -> Table {
        Table::from_game(
            seating,
            game,
            &Default::default(),
            &[arena::Outcome::default(); 6],
            rng,
        )
    }

    pub fn outcomes(&self) -> [arena::Outcome; 6] {
        self.outcomes.map(GpuOutcome::to_outcome)
    }

    /// Every field that differs between two tables, named — the answers
    /// and orders compared within `tolerance`.
    pub fn differences(&self, other: &Table, tolerance: f32) -> Vec<String> {
        fn scalar(out: &mut Vec<String>, name: &str, a: i32, b: i32) {
            if a != b {
                out.push(format!("{name}: {a} vs {b}"));
            }
        }
        let mut out = Vec::new();
        scalar(&mut out, "year", self.year, other.year);
        scalar(&mut out, "barbarians", self.barbarians, other.barbarians);
        scalar(&mut out, "done", self.done, other.done);
        for (i, (a, b)) in self.rng.iter().zip(&other.rng).enumerate() {
            if a != b {
                out.push(format!("rng[{i}]: {a} vs {b}"));
            }
        }
        for i in 0..6 {
            let (a, b) = (&self.kingdoms[i], &other.kingdoms[i]);
            if a != b {
                out.push(format!("kingdom {i}: {a:?}\n           vs {b:?}"));
            }
            let (a, b) = (&self.memories[i], &other.memories[i]);
            if a.demo != b.demo {
                out.push(format!("demo {i}: {:?}\n        vs {:?}", a.demo, b.demo));
            }
            scalar(&mut out, &format!("eco {i}"), a.eco, b.eco);
            scalar(&mut out, &format!("net {i}"), a.net, b.net);
            scalar(
                &mut out,
                &format!("maintenance {i}"),
                a.maintenance,
                b.maintenance,
            );
            scalar(&mut out, &format!("plague {i}"), a.plague, b.plague);
            for (j, (x, y)) in a.dossiers.iter().zip(&b.dossiers).enumerate() {
                if x != y {
                    out.push(format!("dossier {i} on {j}: {x:?}\n              vs {y:?}"));
                }
            }
            let far = |p: &[f32], q: &[f32]| {
                p.iter()
                    .zip(q)
                    .map(|(x, y)| (x - y).abs())
                    .fold(0.0f32, f32::max)
            };
            let d = far(&a.answer, &b.answer);
            if d > tolerance {
                out.push(format!("answer {i}: off by {d}"));
            }
            let d = far(&a.orders, &b.orders);
            if d > tolerance {
                out.push(format!("orders {i}: off by {d}"));
            }
            let (a, b) = (&self.heard[i], &other.heard[i]);
            if a != b {
                out.push(format!("heard {i}: {a:?}\n         vs {b:?}"));
            }
            let (a, b) = (&self.outcomes[i], &other.outcomes[i]);
            let close = |x: f32, y: f32| (x - y).abs() <= tolerance;
            if a.crowned != b.crowned
                || a.fell != b.fell
                || a.starved_out != b.starved_out
                || a.years != b.years
                || a.born != b.born
                || a.settled != b.settled
                || a.nobles_come != b.nobles_come
                || a.starved != b.starved
                || a.nobles_gone != b.nobles_gone
                || a.prince != b.prince
                || a.king != b.king
                || a.readings != b.readings
                || !close(a.progress, b.progress)
                || !close(a.walls, b.walls)
            {
                out.push(format!("outcome {i}: {a:?}\n           vs {b:?}"));
            }
        }
        out
    }
}
