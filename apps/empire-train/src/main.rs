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

use empire_lib::arena::{play, watch, Outcome, Table, YearEnd};
use empire_lib::brain::{Brain, Stage};
use rand::seq::SliceRandom;
use rand::Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// What is carried from one run to the next.
#[derive(Serialize, Deserialize)]
struct School {
    stage: String,
    generation: usize,
    mean: Vec<f32>,
    sigma: Vec<f32>,
    /// The best genome seen, with its fitness.
    best: Vec<f32>,
    best_fitness: f32,
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
    /// A school whose best sits at every table, read at that school's
    /// stage: while schooling, three of it face three of the population;
    /// with `--measure`, five of it face one genome of `--from`.
    against: Option<String>,
}

impl Args {
    /// The table as set by the flags, every seat read at `--stage`.
    fn table(&self) -> Table {
        Table::at(self.stage, self.longest)
    }

    /// The table with the rival school's brains, read at their stage, on
    /// the seats where `theirs` is true.
    fn table_with(&self, stage: Stage, theirs: impl Fn(usize) -> bool) -> Table {
        Table {
            seats: std::array::from_fn(|i| if theirs(i) { stage } else { self.stage }),
            ..self.table()
        }
    }
}

fn stage(name: &str) -> Stage {
    match name {
        "survive" => Stage::Survive,
        "emperor" => Stage::Emperor,
        "market" => Stage::Market,
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
        against: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        if flag == "--show" {
            a.show = true;
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
            "--against" => a.against = Some(value),
            "--out" => a.out = value,
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

/// The population's genomes at a table and the chairs they sit on.
type Seating = Vec<(usize, usize)>;

/// Play the population: alone at a table of its own clones before the
/// market is taught, among others of the population (and the rival's best)
/// after. Each genome sits `tables` tables; its fitness is the mean of its
/// outcomes.
fn evaluate(
    genomes: &[Vec<f32>],
    rival: Option<&(Brain, Stage)>,
    a: &Args,
) -> (Vec<f32>, Vec<Outcome>) {
    let brains: Vec<Brain> = genomes.par_iter().map(|g| Brain::from_genome(g)).collect();
    let n = brains.len();
    let outcomes: Vec<Vec<Outcome>> = if a.stage.market() {
        // Seatings: every table round is a shuffle of the population, six
        // to a table — or, with a rival, one to five of them at random
        // seats and the rival's best on the others, so a genome can never
        // count on a given number of kindred stalls.
        let mut rng = rand::thread_rng();
        // A seating: (genome, chair) pairs.
        let mut seatings: Vec<Seating> = Vec::new();
        for _ in 0..a.tables {
            let mut order: Vec<usize> = (0..n).collect();
            order.shuffle(&mut rng);
            let mut rest = order.as_slice();
            while !rest.is_empty() {
                let m = match rival {
                    Some(_) => rng.gen_range(1..=5).min(rest.len()),
                    None if rest.len() >= 6 => 6,
                    None => break,
                };
                let mut chairs: Vec<usize> = (0..6).collect();
                chairs.shuffle(&mut rng);
                seatings.push(rest[..m].iter().copied().zip(chairs).collect());
                rest = &rest[m..];
            }
        }
        let played: Vec<(&Seating, [Outcome; 6])> = seatings
            .par_iter()
            .map(|seats| {
                let ours = |chair: usize| seats.iter().find(|(_, c)| *c == chair);
                let table: [&Brain; 6] = std::array::from_fn(|chair| match (ours(chair), rival) {
                    (Some((g, _)), _) => &brains[*g],
                    (None, Some((r, _))) => r,
                    (None, None) => unreachable!("six of the population fill a table"),
                });
                let table_spec = match rival {
                    Some((_, theirs)) => a.table_with(*theirs, |chair| ours(chair).is_none()),
                    None => a.table(),
                };
                (seats, play(table, &table_spec))
            })
            .collect();
        let mut per: Vec<Vec<Outcome>> = vec![Vec::new(); n];
        for (seats, outcomes) in played {
            for (g, chair) in seats {
                per[*g].push(outcomes[*chair]);
            }
        }
        per
    } else {
        brains
            .par_iter()
            .map(|b| {
                (0..a.tables)
                    .flat_map(|_| play([b; 6], &a.table()))
                    .collect()
            })
            .collect()
    };
    let fitness = outcomes
        .iter()
        .map(|os| {
            if os.is_empty() {
                f32::MIN
            } else {
                os.iter().map(|o| o.fitness(a.longest)).sum::<f32>() / os.len() as f32
            }
        })
        .collect();
    (fitness, outcomes.into_iter().flatten().collect())
}

fn reading(generation: usize, fitness: &[f32], outcomes: &[Outcome], elapsed: f32) -> String {
    let mut sorted = fitness.to_vec();
    sorted.sort_by(|a, b| b.total_cmp(a));
    let crowned: Vec<i32> = outcomes.iter().filter_map(|o| o.crowned).collect();
    let fell = outcomes.iter().filter(|o| o.fell.is_some()).count();
    let years: f32 = outcomes.iter().map(|o| o.years as f32).sum::<f32>() / outcomes.len() as f32;
    let progress: f32 = outcomes.iter().map(|o| o.progress).sum::<f32>() / outcomes.len() as f32;
    let mut crown_years = crowned.clone();
    crown_years.sort_unstable();
    format!(
        "gen {generation:4} · best {:8.1} · elite {:8.1} · median {:8.1} · years {:5.1} · road {:5.2} · fell {:4.1}% · crowned {:4.1}%{} · {:.1}s",
        sorted[0],
        sorted[..sorted.len().min(100)].iter().sum::<f32>() / sorted.len().min(100) as f32,
        sorted[sorted.len() / 2],
        years,
        progress,
        100.0 * fell as f32 / outcomes.len() as f32,
        100.0 * crowned.len() as f32 / outcomes.len() as f32,
        if crown_years.is_empty() {
            String::new()
        } else {
            format!(" (median year {})", crown_years[crown_years.len() / 2])
        },
        elapsed
    )
}

/// One table of the best genome, year by year, as France lived it.
fn show(best: &[f32], a: &Args) {
    let b = Brain::from_genome(best);
    let (table, other) = match &a.against {
        Some(path) => {
            let school = load(path);
            (
                a.table_with(stage(&school.stage), |i| i > 0),
                Some(Brain::from_genome(&school.best)),
            )
        }
        None => (a.table(), None),
    };
    let o = other.as_ref().unwrap_or(&b);
    println!(
        "year wthr  surface peasants nobles merch soldiers eff treasury   stocks  harvest rat  peas sold taxes     starved title  listed@px bought fair mill fndr ship pal"
    );
    let outcomes = watch([&b, o, o, o, o, o], &table, |y: YearEnd| {
        let k = &y.game.kingdoms[0];
        let m = &y.memories[0];
        let d = m.demo.as_ref().unwrap();
        let i = m.intendance.as_ref().unwrap();
        let c = &i.council;
        println!(
            "{:4} {:4} {:8} {:8} {:6} {:5} {:8} {:3} {:8} {:8} {:8} {:3}  {:4} {:4} {:2}/{:2}/{:2} {:7} {:6} {:6}@{:<3} {:6} {:4} {:4} {:4} {:4} {:3}{}",
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
            y.deaths[0].map(|c| format!("  † {c:?}")).unwrap_or_default()
        );
    });
    println!("{:?}", outcomes[0]);
}

/// The crown years and the falls of a set of seats.
fn tally(label: &str, outcomes: &[Outcome], a: &Args) {
    let mut crowned: Vec<i32> = outcomes.iter().filter_map(|o| o.crowned).collect();
    crowned.sort_unstable();
    let fell = outcomes.iter().filter(|o| o.fell.is_some()).count();
    let pct = |n: usize| 100.0 * n as f32 / outcomes.len() as f32;
    let at = |q: usize| crowned.get(crowned.len() * q / 100).copied().unwrap_or(0);
    println!(
        "{label} · {} seats · crowned {:.1}% (years: p10 {} · median {} · p90 {}) · fell {:.1}% · fitness {:.1}",
        outcomes.len(),
        pct(crowned.len()),
        at(10),
        at(50),
        at(90),
        pct(fell),
        outcomes.iter().map(|o| o.fitness(a.longest)).sum::<f32>() / outcomes.len() as f32
    );
}

/// Many tables of one genome — clones, or one seat against five of
/// another school — at `--stage`.
fn measure(best: &[f32], a: &Args) {
    let b = Brain::from_genome(best);
    let Some(path) = &a.against else {
        let outcomes: Vec<Outcome> = (0..a.measure)
            .into_par_iter()
            .flat_map(|_| play([&b; 6], &a.table()))
            .collect();
        tally(&format!("{:?}", a.stage), &outcomes, a);
        return;
    };
    let school = load(path);
    let other = Brain::from_genome(&school.best);
    let table = a.table_with(stage(&school.stage), |i| i > 0);
    let tables: Vec<[Outcome; 6]> = (0..a.measure)
        .into_par_iter()
        .map(|_| play([&b, &other, &other, &other, &other, &other], &table))
        .collect();
    let first: Vec<Outcome> = tables.iter().map(|t| t[0]).collect();
    let rest: Vec<Outcome> = tables.iter().flat_map(|t| t[1..].to_vec()).collect();
    tally(&format!("{:?} · one of --from", a.stage), &first, a);
    tally(&format!("{:?} · five of --against", a.stage), &rest, a);
}

/// A school saved by an earlier run.
fn load(path: &str) -> School {
    let s: School = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(
        s.mean.len(),
        Brain::GENOME,
        "{path} is not a genome of this shape"
    );
    s
}

fn main() {
    let a = args();
    let (mut mean, mut sigma, mut best, mut best_fitness, start) = match &a.from {
        Some(path) => {
            let s = load(path);
            (s.mean, s.sigma, s.best, f32::MIN, s.generation + 1)
        }
        None => (
            vec![0.0; Brain::GENOME],
            Brain::scales(),
            vec![0.0; Brain::GENOME],
            f32::MIN,
            0,
        ),
    };
    if a.show {
        show(&best, &a);
        return;
    }
    if a.measure > 0 {
        measure(&best, &a);
        return;
    }
    let rival = a.against.as_deref().map(|path| {
        let school = load(path);
        (Brain::from_genome(&school.best), stage(&school.stage))
    });
    let floor: Vec<f32> = Brain::scales().iter().map(|s| s * 0.05).collect();
    eprintln!(
        "{:?} · genome {} · population {} (first {}) · elite {} · tables {} · longest {} years",
        a.stage,
        Brain::GENOME,
        a.population,
        a.first,
        a.elite,
        a.tables,
        a.longest,
    );
    for generation in start..start + a.generations {
        let clock = Instant::now();
        let size = if generation == 0 {
            a.first
        } else {
            a.population
        };
        let mut genomes: Vec<Vec<f32>> = (0..size).map(|_| draw(&mean, &sigma)).collect();
        // The best so far sits again: a lucky draw must prove itself.
        if best_fitness > f32::MIN {
            genomes[0].clone_from(&best);
        }
        let (fitness, outcomes) = evaluate(&genomes, rival.as_ref(), &a);
        let mut order: Vec<usize> = (0..size).collect();
        order.sort_by(|&i, &j| fitness[j].total_cmp(&fitness[i]));
        let elite = &order[..a.elite.min(size)];
        best_fitness = fitness[order[0]];
        best.clone_from(&genomes[order[0]]);
        let n = elite.len() as f32;
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
            mean: mean.clone(),
            sigma: sigma.clone(),
            best: best.clone(),
            best_fitness,
        };
        fs::write(&a.out, serde_json::to_string(&school).unwrap()).unwrap();
    }
}
