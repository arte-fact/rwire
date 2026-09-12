//! The computers' schooling: brains evolved by the cross-entropy method on
//! tables of [`empire_lib::arena`], one rung of [`Stage`] at a time.
//!
//! ```text
//! cargo run --release -p empire-train -- --stage survive --generations 200 --out survive.json
//! cargo run --release -p empire-train -- --stage emperor --from survive.json --out emperor.json
//! ```
//!
//! Every generation draws a population around the running mean, plays it,
//! keeps the elite and moves the mean and spread onto it. The file written
//! holds the distribution (to go on from) and the best genome seen.

use std::fs;
use std::time::Instant;

use empire_gpu::state::{self, Seating};
use empire_gpu::{Arena, Gpu};
use empire_lib::arena::{play, watch, Outcome, Table, YearEnd};
use empire_lib::brain::{Brain, Letters, Shape, Stage, ORDERS};
use empire_lib::game::EmpireGame;
use empire_lib::kingdom::{Kingdoms, KINGDOMS};
use empire_lib::random;
use rand::seq::SliceRandom;
use rand::Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// The widths a school's genomes were laid out for, as written in its
/// file; the schools written before the walls (s26 and older) carry none
/// and were all of one shape.
#[derive(Clone, Copy, Serialize, Deserialize)]
struct Widths {
    own: usize,
    rival: usize,
    a_out: usize,
    b_out: usize,
}

impl Widths {
    const BEFORE_THE_WALLS: Widths = Widths {
        own: 38,
        rival: 19,
        a_out: 15,
        b_out: 18,
    };

    fn now() -> Widths {
        Widths::from(Shape::NOW)
    }
}

impl From<Shape> for Widths {
    fn from(s: Shape) -> Widths {
        Widths {
            own: s.own,
            rival: s.rival,
            a_out: s.a_out,
            b_out: s.b_out,
        }
    }
}

impl From<Widths> for Shape {
    fn from(w: Widths) -> Shape {
        Shape {
            own: w.own,
            rival: w.rival,
            a_out: w.a_out,
            b_out: w.b_out,
        }
    }
}

/// What is carried from one run to the next.
#[derive(Serialize, Deserialize)]
struct School {
    stage: String,
    generation: usize,
    #[serde(default = "Widths::now")]
    widths: Widths,
    /// Whether its brains are told the Chronique, last year's orders and
    /// their old reports ([`Brain::told`]): the schools grown from before
    /// the recall are, one schooled with a recall is not.
    #[serde(default)]
    told: bool,
    mean: Vec<f32>,
    sigma: Vec<f32>,
    /// The best genome seen, with its fitness.
    best: Vec<f32>,
    best_fitness: f32,
    /// The Hall of Fame: bests of past generations spread over the whole
    /// run, oldest first, that the population keeps facing so it cannot
    /// drift away from what once beat it.
    #[serde(default)]
    hall: Vec<Laureate>,
}

/// A generation's best, kept in the Hall of Fame.
#[derive(Clone, Serialize, Deserialize)]
struct Laureate {
    generation: usize,
    genome: Vec<f32>,
}

/// `best` joins the Hall; past `keep`, the laureate whose leaving opens the
/// smallest gap between its neighbours goes, so that the Hall stays spread
/// over the run — the oldest and the newest always stay.
fn induct(hall: &mut Vec<Laureate>, generation: usize, best: &[f32], keep: usize) {
    hall.push(Laureate {
        generation,
        genome: best.to_vec(),
    });
    if hall.len() <= keep {
        return;
    }
    let gone = (1..hall.len() - 1)
        .min_by_key(|&i| hall[i + 1].generation - hall[i - 1].generation)
        .unwrap_or(0);
    hall.remove(gone);
}

/// Where the best generation of a run is kept: `<out>.best.json` beside
/// `<out>.json`.
fn best_out(out: &str) -> String {
    match out.strip_suffix(".json") {
        Some(stem) => format!("{stem}.best.json"),
        None => format!("{out}.best"),
    }
}

struct Args {
    stage: Stage,
    generations: usize,
    population: usize,
    /// The first generation is drawn wider: a lottery for a start that lives.
    first: usize,
    elite: usize,
    tables: usize,
    longest: i32,
    from: Option<String>,
    out: String,
    /// Play one table of the best genome of `--from` (against `--against`
    /// if given) and print seat 0's years.
    show: bool,
    /// Play this many tables of clones of the best genome of `--from` at
    /// `--stage`, and print the tally.
    measure: usize,
    /// Schools whose bests sit at the tables, each read at its school's
    /// stage (repeat the flag for a mix): while schooling, one to five of
    /// the population face them; with `--measure` and `--show`, one genome
    /// of `--from` faces five of them, drawn at random.
    against: Vec<String>,
    /// How many past bests, spread over the run, sit at the tables
    /// alongside the rivals'.
    hall: usize,
    /// What a seat finishing ahead costs at war (none unless `--rank` is set).
    rank: f32,
    /// What a tenth of wall kept costs at war (none unless `--walls` is set).
    walls: f32,
    /// What intelligence costs nothing (`--letters scouts|all`).
    letters: Letters,
    /// Write the best genome of `--from` as the game reads it (floats,
    /// little-endian) to this path, and stop.
    deliver: Option<String>,
    /// Play the generations' tables on the GPU (`--show` and `--measure`
    /// stay on the CPU, whose years they watch).
    gpu: bool,
}

impl Args {
    /// The table as set by the flags, every seat read at `--stage`.
    fn table(&self) -> Table {
        Table {
            rank_cost: self.rank,
            walls_cost: self.walls,
            letters: self.letters,
            ..Table::at(self.stage, self.longest)
        }
    }

    /// The table with each seat read at `seat(chair)`, `None` for `--stage`.
    fn table_of(&self, seat: impl Fn(usize) -> Option<Stage>) -> Table {
        Table {
            seats: std::array::from_fn(|i| seat(i).unwrap_or(self.stage)),
            ..self.table()
        }
    }
}

fn stage(name: &str) -> Stage {
    match name {
        "survive" => Stage::Survive,
        "emperor" => Stage::Emperor,
        "market" => Stage::Market,
        "guard" => Stage::Guard,
        "war" => Stage::War,
        other => panic!("unknown stage {other}"),
    }
}

fn args() -> Args {
    let mut a = Args {
        stage: Stage::Survive,
        generations: 100,
        population: 1000,
        first: 10_000,
        elite: 100,
        tables: 3,
        longest: 150,
        from: None,
        out: "school.json".to_string(),
        show: false,
        measure: 0,
        against: Vec::new(),
        hall: 0,
        rank: 0.0,
        walls: 0.0,
        letters: Letters::None,
        deliver: None,
        gpu: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        if flag == "--show" {
            a.show = true;
            continue;
        }
        if flag == "--gpu" {
            a.gpu = true;
            continue;
        }
        let value = it.next().unwrap_or_else(|| panic!("{flag} needs a value"));
        match flag.as_str() {
            "--stage" => a.stage = stage(&value),
            "--generations" => a.generations = value.parse().unwrap(),
            "--population" => a.population = value.parse().unwrap(),
            "--first" => a.first = value.parse().unwrap(),
            "--elite" => a.elite = value.parse().unwrap(),
            "--tables" => a.tables = value.parse().unwrap(),
            "--longest" => a.longest = value.parse().unwrap(),
            "--from" => a.from = Some(value),
            "--measure" => a.measure = value.parse().unwrap(),
            "--against" => a.against.push(value),
            "--hall" => a.hall = value.parse().unwrap(),
            "--rank" => a.rank = value.parse().unwrap(),
            "--walls" => a.walls = value.parse().unwrap(),
            "--letters" => {
                a.letters = match value.as_str() {
                    "none" => Letters::None,
                    "scouts" => Letters::Scouts,
                    "all" => Letters::All,
                    other => panic!("unknown letters {other}"),
                }
            }
            "--out" => a.out = value,
            "--deliver" => a.deliver = Some(value),
            other => panic!("unknown flag {other}"),
        }
    }
    a
}

/// A standard normal draw (Box–Muller).
fn gaussian(rng: &mut impl Rng) -> f32 {
    let u: f32 = rng.gen_range(f32::EPSILON..1.0);
    let v: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
    (-2.0 * u.ln()).sqrt() * v.cos()
}

fn draw(mean: &[f32], sigma: &[f32]) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    mean.iter()
        .zip(sigma)
        .map(|(m, s)| m + s * gaussian(&mut rng))
        .collect()
}

/// A brain the population sits with — a rival school's best, a laureate
/// of the Hall: its genome, what it was told, the rung it is read at.
struct Other {
    genome: Vec<f32>,
    told: bool,
    stage: Stage,
}

impl Other {
    fn brain(&self) -> Brain {
        Brain::from_genome(&self.genome, self.told)
    }
}

/// A table's company: on each chair a genome of the pool — the
/// population's first, the others' after them. The population's chairs
/// are the ones scored.
type Company = [usize; 6];

/// The companies of a generation among others: one to five of the
/// population at random chairs and the `others` on the rest, so a genome
/// can never count on a given number of kindred stalls. The company
/// varies from table to table — all of the population, a single other at
/// every free chair, or a different other on each — so no fixed set of
/// rivals can be farmed.
fn companies_among(n: usize, others: usize, tables: usize) -> Vec<Company> {
    let mut rng = rand::thread_rng();
    let mut companies = Vec::new();
    for _ in 0..tables {
        let mut order: Vec<usize> = (0..n).collect();
        order.shuffle(&mut rng);
        let mut rest = order.as_slice();
        while !rest.is_empty() {
            let company = if others == 0 { 0 } else { rng.gen_range(0..3) };
            let m = match company {
                0 if rest.len() >= 6 => 6,
                0 if others == 0 => break,
                _ => rng.gen_range(1..=5).min(rest.len()),
            };
            let one = rng.gen_range(0..others.max(1));
            let mut chairs: Vec<usize> = (0..6).collect();
            chairs.shuffle(&mut rng);
            let mut seats: Company = std::array::from_fn(|_| match company {
                1 => n + one,
                _ => n + rng.gen_range(0..others.max(1)),
            });
            for (g, chair) in rest[..m].iter().zip(chairs) {
                seats[chair] = *g;
            }
            companies.push(seats);
            rest = &rest[m..];
        }
    }
    companies
}

/// Play the population: alone at a table of its own clones before the
/// market is taught, among others of the population after (see
/// [`companies_among`]). Each genome sits `tables` tables; its fitness is
/// the mean of its scores.
fn evaluate(
    genomes: &[Vec<f32>],
    told: bool,
    others: &[Other],
    a: &Args,
    arena: Option<&Arena>,
) -> (Vec<f32>, Vec<Outcome>) {
    let n = genomes.len();
    let companies: Vec<Company> = if a.stage.market() {
        companies_among(n, others.len(), a.tables)
    } else {
        (0..n)
            .flat_map(|g| std::iter::repeat_n([g; 6], a.tables))
            .collect()
    };
    let table_of = |company: &Company| {
        a.table_of(|chair| {
            let g = company[chair];
            (g >= n).then(|| others[g - n].stage)
        })
    };
    let outcomes: Vec<[Outcome; 6]> = match arena {
        Some(arena) => {
            let all: Vec<&[f32]> = genomes
                .iter()
                .map(Vec::as_slice)
                .chain(others.iter().map(|o| o.genome.as_slice()))
                .collect();
            let pool = arena.pool(&all);
            let game = EmpireGame::default();
            let mut rng = rand::thread_rng();
            let tables: Vec<state::Table> = companies
                .iter()
                .map(|company| {
                    let table = table_of(company);
                    let seating = Seating {
                        genomes: company.map(|g| g as u32),
                        told: company.map(|g| if g < n { told } else { others[g - n].told }),
                        seats: table.seats,
                        stage: table.stage,
                        longest: table.longest,
                        letters: table.letters,
                    };
                    state::Table::fresh(&seating, &game, random::Rng::seeded(rng.gen()))
                })
                .collect();
            arena
                .play(&pool, &tables, a.longest as u32)
                .iter()
                .map(state::Table::outcomes)
                .collect()
        }
        None => {
            let brains: Vec<Brain> = genomes
                .par_iter()
                .map(|g| Brain::from_genome(g, told))
                .chain(others.par_iter().map(Other::brain))
                .collect();
            companies
                .par_iter()
                .map(|company| {
                    play(
                        std::array::from_fn(|chair| &brains[company[chair]]),
                        &table_of(company),
                    )
                })
                .collect()
        }
    };
    // Every seat a genome sat: its score at that table, and the outcome.
    let mut per: Vec<Vec<(f32, Outcome)>> = vec![Vec::new(); n];
    for (company, outcomes) in companies.iter().zip(&outcomes) {
        let scores = table_of(company).scores(outcomes);
        for chair in 0..6 {
            if let Some(seats) = per.get_mut(company[chair]) {
                seats.push((scores[chair], outcomes[chair]));
            }
        }
    }
    let fitness = per
        .iter()
        .map(|seats| {
            if seats.is_empty() {
                f32::MIN
            } else {
                seats.iter().map(|(s, _)| s).sum::<f32>() / seats.len() as f32
            }
        })
        .collect();
    (fitness, per.into_iter().flatten().map(|(_, o)| o).collect())
}

fn reading(generation: usize, fitness: &[f32], outcomes: &[Outcome], elapsed: f32) -> String {
    let mut sorted = fitness.to_vec();
    sorted.sort_by(|a, b| b.total_cmp(a));
    let crowned: Vec<i32> = outcomes.iter().filter_map(|o| o.crowned).collect();
    let fell = outcomes.iter().filter(|o| o.fell.is_some()).count();
    let starved_out = outcomes.iter().filter(|o| o.starved_out).count();
    let years: f32 = outcomes.iter().map(|o| o.years as f32).sum::<f32>() / outcomes.len() as f32;
    let progress: f32 = outcomes.iter().map(|o| o.progress).sum::<f32>() / outcomes.len() as f32;
    let seats = outcomes.len() as f32;
    let starved: f32 = outcomes.iter().map(|o| o.starved as f32).sum::<f32>() / seats;
    let nobles: f32 = outcomes
        .iter()
        .map(|o| (o.nobles_come - o.nobles_gone) as f32)
        .sum::<f32>()
        / seats;
    let readings: f32 = outcomes.iter().map(|o| o.readings as f32).sum::<f32>() / seats;
    let walls: f32 = outcomes.iter().map(|o| o.walls).sum::<f32>() / seats;
    let mut crown_years = crowned.clone();
    crown_years.sort_unstable();
    let pct = |n: usize| 100.0 * n as f32 / outcomes.len() as f32;
    format!(
        "gen {generation:4} · best {:8.1} · elite {:8.1} · median {:8.1} · years {:5.1} · road {:4.2} · starved {:5.0} · nobles {:+5.1} · read {:4.1} · walls {:4.1} · fell {:4.1}% (mother {:4.1}%) · prince {:4.1}% · king {:4.1}% · crowned {:4.1}%{} · {:.1}s",
        sorted[0],
        sorted[..sorted.len().min(100)].iter().sum::<f32>() / sorted.len().min(100) as f32,
        sorted[sorted.len() / 2],
        years,
        progress,
        starved,
        nobles,
        readings,
        walls,
        pct(fell),
        pct(starved_out),
        pct(outcomes.iter().filter(|o| o.prince.is_some()).count()),
        pct(outcomes.iter().filter(|o| o.king.is_some()).count()),
        pct(crowned.len()),
        if crown_years.is_empty() {
            String::new()
        } else {
            format!(" (median year {})", crown_years[crown_years.len() / 2])
        },
        elapsed
    )
}

/// One table of the best genome, year by year. Clones around the table
/// unless `--against` seats other schools: with exactly five it is a
/// match — one of each, in order, and every seat is told — otherwise
/// they are drawn at random and only France is.
fn show(best: &[f32], told: bool, a: &Args) {
    let b = Brain::from_genome(best, told);
    let rivals = rivals(a);
    let brains: Vec<Brain> = rivals.iter().map(Other::brain).collect();
    let a_match = rivals.len() == 5;
    let seats: Option<[usize; 6]> = match rivals.len() {
        0 => None,
        5 => Some(std::array::from_fn(|i| i.saturating_sub(1))),
        _ => Some(seat_rivals(&rivals)),
    };
    let table = a.table_of(|i| seats.filter(|_| i > 0).map(|s| rivals[s[i]].stage));
    let brain = |i: usize| match seats {
        Some(s) if i > 0 => &brains[s[i]],
        _ => &b,
    };
    let (o1, o2, o3, o4, o5) = (brain(1), brain(2), brain(3), brain(4), brain(5));
    let told = if a_match { 0..6 } else { 0..1 };
    if let Some(s) = seats {
        let legend: Vec<String> = KINGDOMS
            .iter()
            .enumerate()
            .map(|(i, k)| {
                let school = if i == 0 { "--from" } else { &a.against[s[i]] };
                format!("{k:?}: {school}")
            })
            .collect();
        println!("{}", legend.join(" · "));
    }
    println!(
        "seat     year wthr  surface peasants nobles merch soldiers eff treasury   stocks  harvest rat  peas sold taxes     starved title  listed@px bought fair mill fndr ship pal wall hosp rams"
    );
    let outcomes = watch([&b, o1, o2, o3, o4, o5], &table, |y: YearEnd| {
        for seat in told.clone() {
            row(&y, seat);
            tell(&y, seat);
        }
        if a_match {
            println!();
        }
    });
    for seat in told {
        println!("{:?}: {:?}", KINGDOMS[seat], outcomes[seat]);
    }
}

/// A seat's state at the end of the year, on one row.
fn row(y: &YearEnd, seat: usize) {
    let k = &y.game.kingdoms[seat];
    let m = &y.memories[seat];
    let d = m.demo.as_ref().unwrap();
    let i = m.intendance.as_ref().unwrap();
    let c = &i.council;
    println!(
        "{:<8} {:4} {:4} {:8} {:8} {:6} {:5} {:8} {:3} {:8} {:8} {:8} {:3}  {:4} {:4} {:2}/{:2}/{:2} {:7} {:6} {:6}@{:<3} {:6} {:4} {:4} {:4} {:4} {:3} {:4} {:4} {:4}{}",
        format!("{:?}", k.id),
        y.game.year,
        k.weather as u8,
        k.surface,
        k.peasants,
        k.nobles,
        k.merchants,
        k.soldiers,
        k.soldiers_efficiency,
        k.treasury,
        k.grain_stocks,
        k.grain_harvest,
        k.rats_loss_rate,
        c.peasants_ration,
        c.soldiers_ration,
        c.taxes.income,
        c.taxes.sales,
        c.taxes.customs,
        d.starvation_victims,
        format!("{:?}", k.title()),
        i.listed.map_or(0, |(n, _)| n),
        i.listed.map_or(0, |(_, p)| p),
        i.bought.map_or(0, |(_, n)| n),
        k.marketplaces,
        k.grain_mills,
        k.foundries,
        k.shipyards,
        k.palaces,
        k.fortifications,
        k.hospices,
        k.rams,
        y.deaths[seat].map(|c| format!("  † {c:?}")).unwrap_or_default()
    );
}

/// A seat's moves of the year, told under its row: purchases,
/// intelligence, marches fought and sieges suffered.
fn tell(y: &YearEnd, seat: usize) {
    let us = KINGDOMS[seat];
    let m = &y.memories[seat];
    if let Some(i) = &m.intendance {
        let buys: Vec<String> = i
            .purchases
            .iter()
            .map(|(what, n)| format!("{what:?} +{n}"))
            .collect();
        if !buys.is_empty() {
            println!("     ↳ buys: {}", buys.join(" · "));
        }
        if i.land_sold > 0 {
            println!("     ↳ sells {} arpents to the barbarians", i.land_sold);
        }
    }
    let sent = &y.sent[seat];
    if sent.scout.is_some() || !sent.agents.is_empty() {
        let scout = sent
            .scout
            .map_or(String::new(), |on| format!("scout→{on:?}"));
        let agents = if sent.agents.is_empty() {
            String::new()
        } else {
            format!("agents→{:?}", sent.agents)
        };
        println!("     ↳ intel: {scout} {agents}");
    }
    for f in y.fought {
        let against =
            |t: Option<Kingdoms>| t.map_or("the barbarians".to_string(), |t| format!("{t:?}"));
        for a in f.armies().iter().filter(|a| a.attacker == us) {
            let s = a.spoils();
            let rams = if a.rams > 0 {
                format!(" + {} rams ({} broken)", a.rams, a.rams_broken)
            } else {
                String::new()
            };
            println!(
                "     ↳ marches on {} with {}{rams}: {} · +{} arpents, +{} grain, +{} gold · {} men lost · killed {} rallied {}",
                against(f.target),
                a.sent,
                if a.victory { "victory" } else { "beaten" },
                s.arpents,
                s.grain,
                s.treasury,
                a.lost(),
                s.killed.total(),
                s.rallied.total(),
            );
        }
        if f.target == Some(us) {
            let r = &f.result;
            let lost: i32 = r.armies.iter().map(|a| a.spoils().arpents).sum();
            let by: Vec<String> = r
                .armies
                .iter()
                .map(|a| format!("{:?} ({})", a.attacker, a.sent))
                .collect();
            println!(
                "     ↳ besieged by {}: garrison {}→{} · walls {}→{} · {} arpents lost{}",
                by.join(" and "),
                r.garrison_start,
                r.garrison_left,
                r.walls_start,
                r.walls_left,
                lost,
                r.annexed_by.map(|_| " · ANNEXED").unwrap_or_default(),
            );
        }
    }
}

/// The crown years and the falls of a set of seats.
fn tally(label: &str, outcomes: &[Outcome], a: &Args) {
    let mut crowned: Vec<i32> = outcomes.iter().filter_map(|o| o.crowned).collect();
    crowned.sort_unstable();
    let fell = outcomes.iter().filter(|o| o.fell.is_some()).count();
    let starved_out = outcomes.iter().filter(|o| o.starved_out).count();
    let pct = |n: usize| 100.0 * n as f32 / outcomes.len() as f32;
    let at = |q: usize| crowned.get(crowned.len() * q / 100).copied().unwrap_or(0);
    println!(
        "{label} · {} seats · prince {:.1}% · king {:.1}% · crowned {:.1}% (years: p10 {} · median {} · p90 {}) · fell {:.1}% (mother {:.1}%) · starved {:.0}/seat · nobles {:+.1}/seat · read {:.1}/seat · fitness {:.1}",
        outcomes.len(),
        pct(outcomes.iter().filter(|o| o.prince.is_some()).count()),
        pct(outcomes.iter().filter(|o| o.king.is_some()).count()),
        pct(crowned.len()),
        at(10),
        at(50),
        at(90),
        pct(fell),
        pct(starved_out),
        outcomes.iter().map(|o| o.starved as f32).sum::<f32>() / outcomes.len() as f32,
        outcomes.iter().map(|o| (o.nobles_come - o.nobles_gone) as f32).sum::<f32>()
            / outcomes.len() as f32,
        outcomes.iter().map(|o| o.readings as f32).sum::<f32>() / outcomes.len() as f32,
        outcomes.iter().map(|o| o.fitness(a.longest)).sum::<f32>() / outcomes.len() as f32
    );
}

/// The bests of the `--against` schools, each at its stage.
fn rivals(a: &Args) -> Vec<Other> {
    a.against
        .iter()
        .map(|path| {
            let school = load(path);
            Other {
                genome: school.best,
                told: school.told,
                stage: stage(&school.stage),
            }
        })
        .collect()
}

/// Which rival sits at each chair: one at random per chair, the first
/// chair's (ours) drawn too and ignored.
fn seat_rivals(rivals: &[Other]) -> [usize; 6] {
    let mut rng = rand::thread_rng();
    std::array::from_fn(|_| rng.gen_range(0..rivals.len()))
}

/// Many tables of one genome — clones, or one seat against five of
/// the `--against` schools — at `--stage`.
fn measure(best: &[f32], told: bool, a: &Args) {
    let b = Brain::from_genome(best, told);
    let rivals = rivals(a);
    let brains: Vec<Brain> = rivals.iter().map(Other::brain).collect();
    if rivals.is_empty() {
        let outcomes: Vec<Outcome> = (0..a.measure)
            .into_par_iter()
            .flat_map(|_| play([&b; 6], &a.table()))
            .collect();
        tally(&format!("{:?}", a.stage), &outcomes, a);
        return;
    }
    let tables: Vec<[Outcome; 6]> = (0..a.measure)
        .into_par_iter()
        .map(|_| {
            let seats = seat_rivals(&rivals);
            let table = a.table_of(|i| (i > 0).then(|| rivals[seats[i]].stage));
            play(
                std::array::from_fn(|i| if i == 0 { &b } else { &brains[seats[i]] }),
                &table,
            )
        })
        .collect();
    let first: Vec<Outcome> = tables.iter().map(|t| t[0]).collect();
    let rest: Vec<Outcome> = tables.iter().flat_map(|t| t[1..].to_vec()).collect();
    tally(&format!("{:?} · one of --from", a.stage), &first, a);
    tally(&format!("{:?} · five of --against", a.stage), &rest, a);
}

/// The school of letters wakes the intelligence answers: their spread is
/// opened again, so the brain tries them — it never had a reason to, and
/// a school that read nothing has closed them — and the biases are seeded
/// so the reports come from the first day: "no one" pushed down for the
/// éclaireur, the agents pushed up when they are free too.
fn wake(mean: &mut [f32], sigma: &mut [f32], best: &mut [f32], letters: Letters) {
    let scales = Brain::scales();
    let rows = if letters.agents() { 6..17 } else { 6..12 };
    for o in rows {
        for k in Brain::exterieur_output(o) {
            sigma[k] = scales[k];
        }
    }
    let mut seeds = vec![(11, -3.0)];
    if letters.agents() {
        seeds.extend((12..17).map(|o| (o, 3.0)));
    }
    for (o, bias) in seeds {
        let k = Brain::exterieur_output(o).end - 1;
        mean[k] = bias;
        best[k] = bias;
    }
}

/// A school saved by an earlier run — grown to today's widths if it was
/// schooled on narrower ones (the new entries start blind, at their
/// starting spread; the new answers blank). A school from before the
/// recall was told everything, and stays so.
fn load(path: &str) -> School {
    let mut s: School = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    if s.mean.len() != Shape::from(s.widths).genome() {
        // Written before the widths were: one shape for all of them.
        s.widths = Widths::BEFORE_THE_WALLS;
    }
    let was = Shape::from(s.widths);
    assert_eq!(
        s.mean.len(),
        was.genome(),
        "{path} is not a genome of the shape it claims"
    );
    if was == Shape::NOW {
        return s;
    }
    s.told = was.b_out == ORDERS;
    let seen = Brain::grown(&vec![1.0; s.mean.len()], was);
    let scales = Brain::scales();
    s.mean = Brain::grown(&s.mean, was);
    s.best = Brain::grown(&s.best, was);
    s.sigma = Brain::grown(&s.sigma, was)
        .into_iter()
        .zip(seen)
        .zip(scales)
        .map(|((sigma, seen), scale)| if seen > 0.0 { sigma } else { scale })
        .collect();
    for l in &mut s.hall {
        l.genome = Brain::grown(&l.genome, was);
    }
    s.widths = Widths::now();
    eprintln!("{path}: grown from {was:?} to {:?}", Shape::NOW);
    s
}

/// The best genome as the game reads it: floats, little-endian.
fn deliver(best: &[f32], path: &str) {
    let bytes: Vec<u8> = best.iter().flat_map(|w| w.to_le_bytes()).collect();
    fs::write(path, &bytes).unwrap();
    eprintln!("{path}: {} weights delivered", best.len());
}

fn main() {
    let a = args();
    let (mut mean, mut sigma, mut best, mut best_fitness, mut hall, start, told) = match &a.from {
        Some(path) => {
            let s = load(path);
            (
                s.mean,
                s.sigma,
                s.best,
                f32::MIN,
                s.hall,
                s.generation + 1,
                s.told,
            )
        }
        None => (
            vec![0.0; Brain::GENOME],
            Brain::scales(),
            vec![0.0; Brain::GENOME],
            f32::MIN,
            Vec::new(),
            0,
            false,
        ),
    };
    if let Some(path) = &a.deliver {
        deliver(&best, path);
        return;
    }
    if a.show {
        show(&best, told, &a);
        return;
    }
    if a.measure > 0 {
        measure(&best, told, &a);
        return;
    }
    if a.letters.scouts() {
        wake(&mut mean, &mut sigma, &mut best, a.letters);
    }
    let rivals = rivals(&a);
    let gpu = a.gpu.then(|| Gpu::open().expect("a GPU to play on"));
    let arena = gpu.as_ref().map(Arena::new);
    let floor: Vec<f32> = Brain::scales().iter().map(|s| s * 0.05).collect();
    eprintln!(
        "{:?} · genome {} · population {} (first {}) · elite {} · tables {} · hall {} · rank {} · letters {:?} · longest {} years · {}",
        a.stage,
        Brain::GENOME,
        a.population,
        a.first,
        a.elite,
        a.tables,
        a.hall,
        a.rank,
        a.letters,
        a.longest,
        gpu.as_ref().map_or("CPU", |g| g.name.as_str()),
    );
    hall.truncate(a.hall);
    // The elite's mean score of the best generation so far, kept apart in
    // case the population drifts away from it later.
    let mut best_elite = f32::MIN;
    for generation in start..start + a.generations {
        let clock = Instant::now();
        let size = if generation == 0 {
            a.first
        } else {
            a.population
        };
        let mut genomes: Vec<Vec<f32>> = (0..size)
            .into_par_iter()
            .map(|_| draw(&mean, &sigma))
            .collect();
        // The best so far sits again: a lucky draw must prove itself.
        if best_fitness > f32::MIN {
            genomes[0].clone_from(&best);
        }
        let others: Vec<Other> = rivals
            .iter()
            .map(|o| Other {
                genome: o.genome.clone(),
                told: o.told,
                stage: o.stage,
            })
            .chain(hall.iter().map(|l| Other {
                genome: l.genome.clone(),
                told,
                stage: a.stage,
            }))
            .collect();
        let (fitness, outcomes) = evaluate(&genomes, told, &others, &a, arena.as_ref());
        let mut order: Vec<usize> = (0..size).collect();
        order.sort_by(|&i, &j| fitness[j].total_cmp(&fitness[i]));
        let elite = &order[..a.elite.min(size)];
        best_fitness = fitness[order[0]];
        best.clone_from(&genomes[order[0]]);
        if a.hall > 0 {
            induct(&mut hall, generation, &best, a.hall);
        }
        let n = elite.len() as f32;
        let elite_mean = elite.iter().map(|&i| fitness[i]).sum::<f32>() / n;
        for w in 0..Brain::GENOME {
            let m = elite.iter().map(|&i| genomes[i][w]).sum::<f32>() / n;
            let v = elite
                .iter()
                .map(|&i| (genomes[i][w] - m).powi(2))
                .sum::<f32>()
                / n;
            mean[w] = 0.3 * mean[w] + 0.7 * m;
            sigma[w] = (0.3 * sigma[w] + 0.7 * v.sqrt()).max(floor[w]);
        }
        println!(
            "{}",
            reading(
                generation,
                &fitness,
                &outcomes,
                clock.elapsed().as_secs_f32()
            )
        );
        let school = School {
            stage: format!("{:?}", a.stage).to_lowercase(),
            generation,
            widths: Widths::now(),
            told,
            mean: mean.clone(),
            sigma: sigma.clone(),
            best: best.clone(),
            best_fitness,
            hall: hall.clone(),
        };
        let json = serde_json::to_string(&school).unwrap();
        fs::write(&a.out, &json).unwrap();
        if elite_mean > best_elite {
            best_elite = elite_mean;
            fs::write(best_out(&a.out), &json).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hall_stays_spread_over_the_run() {
        let mut hall = Vec::new();
        for generation in 0..1000 {
            induct(&mut hall, generation, &[generation as f32], 5);
        }
        let kept: Vec<usize> = hall.iter().map(|l| l.generation).collect();
        assert_eq!(kept.len(), 5);
        assert_eq!(kept[0], 0);
        assert_eq!(kept[4], 999);
        // No gap is more than twice the even share of the run.
        let widest = kept.windows(2).map(|w| w[1] - w[0]).max().unwrap();
        assert!(widest <= 2 * 1000 / 4, "{kept:?}");
    }

    #[test]
    fn the_best_generation_is_kept_beside_the_run() {
        assert_eq!(best_out("l-war.json"), "l-war.best.json");
        assert_eq!(best_out("school"), "school.best");
    }
}
