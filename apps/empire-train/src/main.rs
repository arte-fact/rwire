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

use std::borrow::Cow;
use std::fs;
use std::time::{Duration, Instant};

use empire_gpu::state::{self, Seating};
use empire_gpu::{Arena, Gpu, Pool};
use empire_lib::arena::{play, watch, Outcome, Table, YearEnd};
use empire_lib::brain::{Brain, Letters, Reads, Shape, Stage, HIDDEN, MAX_HIDDEN, ORDERS, RECALL};
use empire_lib::campaign::Fought;
use empire_lib::game::EmpireGame;
use empire_lib::kingdom::{Kingdoms, KINGDOMS};
use empire_lib::random;
use empire_lib::RULES;
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
    /// The hidden layer; the schools from before it was a choice have 32.
    #[serde(default = "Widths::narrow")]
    hidden: usize,
}

impl Widths {
    const BEFORE_THE_WALLS: Widths = Widths {
        own: 38,
        rival: 19,
        a_out: 15,
        b_out: 18,
        hidden: HIDDEN,
    };

    fn now() -> Widths {
        Widths::from(Shape::NOW)
    }

    fn narrow() -> usize {
        HIDDEN
    }
}

impl From<Shape> for Widths {
    fn from(s: Shape) -> Widths {
        Widths {
            own: s.own,
            rival: s.rival,
            a_out: s.a_out,
            b_out: s.b_out,
            hidden: s.hidden,
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
            hidden: w.hidden,
        }
    }
}

/// What is carried from one run to the next.
#[derive(Serialize, Deserialize)]
struct School {
    /// The rules it was schooled under ([`RULES`]).
    #[serde(default = "School::first_rules")]
    rules: u32,
    stage: String,
    generation: usize,
    #[serde(default = "Widths::now")]
    widths: Widths,
    /// What its brains read of their sight ([`Reads`]): the Chronique
    /// and last year's orders, the recall. The schools from before the
    /// choice had a recall and were not told.
    #[serde(default)]
    told: bool,
    #[serde(default = "School::had_recall")]
    recall: bool,
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
    /// The parents of the next generation when the school breeds
    /// (`--breed`): the last elite, best first. Only in the school's own
    /// file, not in the copies kept along the run.
    #[serde(default)]
    parents: Vec<Vec<f32>>,
}

impl School {
    fn first_rules() -> u32 {
        1
    }

    fn had_recall() -> bool {
        true
    }

    fn reads(&self) -> Reads {
        Reads {
            told: self.told,
            recall: self.recall,
        }
    }
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

/// Where a generation kept along the run goes (`--keep`):
/// `<out>.g<generation>.json` beside `<out>.json`.
fn kept_out(out: &str, generation: usize) -> String {
    match out.strip_suffix(".json") {
        Some(stem) => format!("{stem}.g{generation}.json"),
        None => format!("{out}.g{generation}"),
    }
}

struct Args {
    stage: Stage,
    generations: usize,
    population: usize,
    /// The first generation is drawn wider: a lottery for a start that lives.
    first: usize,
    elite: usize,
    /// How many tables each genome sits a generation, a multiple of
    /// [`GROUP`]: they come by groups of a company at as many seeds.
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
    /// Play `--measure` tables of the best genome of `--from` against
    /// five of `--against` on the CPU and write, one row a year, what its
    /// seat saw and the 32 recall entries its Extérieur wrote down, to
    /// this CSV; then stop.
    trace: Option<String>,
    /// Read the best genome of `--from` with its recall cut (zero on its
    /// sight) for `--measure`, `--show` and `--trace`: the ablation.
    no_recall: bool,
    /// Play the generations' and `--measure`'s tables on the GPU
    /// (`--show` stays on the CPU, whose years it watches).
    gpu: bool,
    /// How many schools to raise at once, each on its own draw, all their
    /// tables played together; `--out` then names them with `{n}`, and
    /// `--from` may too, each going on from its own.
    trials: usize,
    /// The hidden layer of the population's brains; a narrower school
    /// taken up is widened, silent neurons added. The rivals keep their
    /// own width (the GPU reads every brain at the widest, rounded up to
    /// 32, the narrower ones widened on their way to it).
    hidden: usize,
    /// Every `keep` generations, the school is also written beside
    /// `--out` as `<out>.g<generation>.json`, for the curve of a run.
    keep: usize,
    /// Breed instead of redrawing: the elite are parents, each of which
    /// begets its share of the population by mutation, and the next elite
    /// is chosen among parents and children — as many lineages as parents,
    /// where the redraw fuses them into one. The mutation is drawn at
    /// `breed` times the starting scale of each weight; 0 keeps the
    /// redraw.
    breed: f32,
    /// What a school raised from nothing reads of its sight ([`Reads`]):
    /// `--told` the Chronique, last year's orders and its old reports;
    /// `--recall` what its Extérieur wrote down. None by default: the
    /// year's figures only.
    reads: Reads,
}

impl Args {
    /// The shape of the population's genomes.
    fn shape(&self) -> Shape {
        Shape::wide(self.hidden)
    }
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
        tables: 32,
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
        trace: None,
        no_recall: false,
        gpu: false,
        trials: 1,
        hidden: HIDDEN,
        keep: 0,
        breed: 0.0,
        reads: Reads {
            told: false,
            recall: false,
        },
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
        if flag == "--told" {
            a.reads.told = true;
            continue;
        }
        if flag == "--recall" {
            a.reads.recall = true;
            continue;
        }
        if flag == "--no-recall" {
            a.no_recall = true;
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
            "--trace" => a.trace = Some(value),
            "--trials" => a.trials = value.parse().unwrap(),
            "--hidden" => a.hidden = value.parse().unwrap(),
            "--keep" => a.keep = value.parse().unwrap(),
            "--breed" => a.breed = value.parse().unwrap(),
            other => panic!("unknown flag {other}"),
        }
    }
    assert!(
        a.trials == 1 || a.out.contains("{n}"),
        "--out needs a {{n}} to name the trials"
    );
    assert!(
        a.tables > 0 && a.tables.is_multiple_of(GROUP),
        "--tables is a multiple of {GROUP}: a company sits that many tables"
    );
    assert!(
        a.hidden > 0 && a.hidden <= MAX_HIDDEN,
        "--hidden must be between 1 and {MAX_HIDDEN}"
    );
    a
}

/// A genome drawn around `mean`, `sigma` wide, on the game's own dice
/// (a Box–Muller pair per two weights).
fn draw(mean: &[f32], sigma: &[f32]) -> Vec<f32> {
    let mut rng = random::Rng::seeded(rand::thread_rng().gen());
    let mut out = Vec::with_capacity(mean.len());
    for (m, s) in mean.chunks(2).zip(sigma.chunks(2)) {
        let u = (rng.word() as f32 + 1.0) * (1.0 / 4_294_967_296.0);
        let v = rng.word() as f32 * (std::f32::consts::TAU / 4_294_967_296.0);
        let r = (-2.0 * u.ln()).sqrt();
        let (sin, cos) = v.sin_cos();
        out.push(m[0] + s[0] * r * cos);
        if let (Some(m), Some(s)) = (m.get(1), s.get(1)) {
            out.push(m + s * r * sin);
        }
    }
    out
}

/// A brain the population sits with — a rival school's best, a laureate
/// of the Hall: its genome, what it was told, the rung it is read at.
struct Other {
    genome: Vec<f32>,
    reads: Reads,
    stage: Stage,
}

impl Other {
    fn brain(&self) -> Brain {
        Brain::from_genome(&self.genome, self.reads)
    }
}

/// A genome of the generation's pool, what it is told, the rung it is
/// read at, and whose score its seats are — a trial's genome, by trial
/// and rank — or nobody's (a rival, a laureate).
struct Seat<'a> {
    genome: &'a [f32],
    reads: Reads,
    stage: Stage,
    scored: Option<(usize, usize)>,
}

/// A trial's tables of the generation: its own genomes in the pool, and
/// the others it sits with.
struct Round {
    ours: std::ops::Range<usize>,
    others: Vec<usize>,
}

/// How many bytes of drawn genomes a generation holds at once: past it,
/// the trials are played in turns.
const MEMORY_BUDGET: usize = 12 << 30;

/// A table's company: the pool's genome on each chair.
type Company = [usize; 6];

/// How many tables a company sits, at as many seeds and chair orders: the
/// tables of one GPU workgroup, so that its lanes read the same six
/// genomes — from the cache, not the memory — and every genome is scored
/// on that many games of each company it is drawn into.
const GROUP: usize = empire_gpu::TABLES_PER_WORKGROUP;

/// The companies of a round among others: one to five of the trial's
/// genomes at random chairs and the `others` on the rest, so a genome
/// can never count on a given number of kindred stalls. The company
/// varies from table to table — all of the trial's, a single other at
/// every free chair, or a different other on each — so no fixed set of
/// rivals can be farmed. `groups` companies per genome.
fn companies_among(round: &Round, groups: usize) -> Vec<Company> {
    let mut rng = rand::thread_rng();
    let others = &round.others;
    let mut companies = Vec::new();
    for _ in 0..groups {
        let mut order: Vec<usize> = round.ours.clone().collect();
        order.shuffle(&mut rng);
        let mut rest = order.as_slice();
        while !rest.is_empty() {
            let company = if others.is_empty() {
                0
            } else {
                rng.gen_range(0..3)
            };
            let m = match company {
                0 if rest.len() >= 6 => 6,
                0 if others.is_empty() => break,
                _ => rng.gen_range(1..=5).min(rest.len()),
            };
            let one = others.get(rng.gen_range(0..others.len().max(1)));
            let mut chairs: Vec<usize> = (0..6).collect();
            chairs.shuffle(&mut rng);
            let mut seats: Company = std::array::from_fn(|_| {
                match company {
                    1 => one,
                    _ => others.get(rng.gen_range(0..others.len().max(1))),
                }
                .copied()
                .unwrap_or(round.ours.start)
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

/// `companies` cut into runs whose distinct genomes fit a pool of
/// `capacity`: each run is one pool, one dispatch.
fn batches(companies: &[Company], capacity: usize) -> Vec<std::ops::Range<usize>> {
    let mut batches = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut from = 0;
    for (i, company) in companies.iter().enumerate() {
        let fresh: Vec<usize> = company
            .iter()
            .copied()
            .filter(|g| !seen.contains(g))
            .collect();
        if seen.len() + fresh.len() > capacity && i > from {
            batches.push(from..i);
            from = i;
            seen.clear();
        }
        seen.extend(company.iter().copied());
    }
    batches.push(from..companies.len());
    batches
}

/// `genomes` as the arena reads them: those narrower than its hidden
/// layer widened to it, silent neurons added, the others as they are.
fn widened<'a>(genomes: &[&'a [f32]], arena: &Arena) -> Vec<Cow<'a, [f32]>> {
    let now = Shape::wide(arena.hidden());
    genomes
        .par_iter()
        .map(|&g| {
            let was = Shape::of(g.len());
            if was == now {
                Cow::Borrowed(g)
            } else {
                Cow::Owned(Brain::grown(g, was, now))
            }
        })
        .collect()
}

/// The widest hidden layer the GPU kernel must read, rounded up to the
/// 32 neurons it takes at a time: the population's, or a rival's.
fn kernel_width(a: &Args, rivals: &[Other]) -> usize {
    rivals
        .iter()
        .map(|o| Shape::of(o.genome.len()).hidden)
        .fold(a.hidden, usize::max)
        .next_multiple_of(32)
}

/// The companies of `rounds` on the GPU: the seats they sit, and only
/// them, pooled and every table played to its end in one go.
/// The pool and the fresh tables of `companies`, ready for `arena`.
fn lay_out(
    arena: &Arena,
    seats: &[Seat],
    companies: &[Company],
    table_of: impl Fn(&Company) -> Table,
) -> (Pool, Vec<state::Table>) {
    let mut local = vec![u32::MAX; seats.len()];
    let mut pooled: Vec<&[f32]> = Vec::new();
    for &g in companies.iter().flatten() {
        if local[g] == u32::MAX {
            local[g] = pooled.len() as u32;
            pooled.push(seats[g].genome);
        }
    }
    let widened = widened(&pooled, arena);
    let pool = arena.pool(&widened.iter().map(Cow::as_ref).collect::<Vec<_>>());
    let game = EmpireGame::default();
    let mut rng = rand::thread_rng();
    let tables: Vec<state::Table> = companies
        .iter()
        .map(|company| {
            let table = table_of(company);
            let seating = Seating {
                genomes: company.map(|g| local[g]),
                reads: company.map(|g| seats[g].reads),
                seats: table.seats,
                stage: table.stage,
                longest: table.longest,
                letters: table.letters,
            };
            state::Table::fresh(&seating, &game, random::Rng::seeded(rng.gen()))
        })
        .collect();
    (pool, tables)
}

/// `companies` played on `arena`, in runs whose genomes fit its pool —
/// every run laid out and on the card before the first is played, and
/// `staged` told then: the cores are free from that point on.
fn play_on(
    arena: &Arena,
    seats: &[Seat],
    companies: &[Company],
    a: &Args,
    table_of: impl Fn(&Company) -> Table,
    staged: impl FnOnce(),
) -> Vec<[Outcome; 6]> {
    let runs: Vec<(Pool, Vec<state::Table>)> = batches(companies, arena.capacity())
        .into_iter()
        .map(|batch| lay_out(arena, seats, &companies[batch], &table_of))
        .collect();
    staged();
    runs.iter()
        .flat_map(|(pool, tables)| {
            let out: Vec<[Outcome; 6]> = arena
                .play(pool, tables, a.longest as u32)
                .iter()
                .map(state::Table::outcomes)
                .collect();
            out
        })
        .collect()
}

/// `companies` played on the CPU, every core on its own tables.
fn play_all(
    seats: &[Seat],
    companies: &[Company],
    table_of: impl Fn(&Company) -> Table + Sync,
) -> Vec<[Outcome; 6]> {
    let mut used = vec![false; seats.len()];
    for &g in companies.iter().flatten() {
        used[g] = true;
    }
    let brains: Vec<Option<Brain>> = seats
        .par_iter()
        .zip(&used)
        .map(|(s, &used)| used.then(|| Brain::from_genome(s.genome, s.reads)))
        .collect();
    companies
        .par_iter()
        .map(|company| {
            play(
                std::array::from_fn(|chair| brains[company[chair]].as_ref().unwrap()),
                &table_of(company),
            )
        })
        .collect()
}

/// How the tables are shared when the GPU plays: the CPU's part, moved
/// after every round onto the pace each showed — both are done at once.
struct Pace {
    cpu: f32,
}

impl Pace {
    fn new() -> Pace {
        Pace { cpu: 0.3 }
    }

    /// Where the card's tables end: whole groups.
    fn cut(&self, tables: usize) -> usize {
        (tables - (tables as f32 * self.cpu) as usize).div_ceil(GROUP) * GROUP
    }

    fn learn(&mut self, cpu: (usize, Duration), gpu: (usize, Duration)) {
        if cpu.0 == 0 || gpu.0 == 0 {
            return;
        }
        let rate = |(n, took): (usize, Duration)| n as f32 / took.as_secs_f32().max(1e-3);
        let (c, g) = (rate(cpu), rate(gpu));
        self.cpu = (c / (c + g)).clamp(0.0, 0.9);
    }
}

/// Play the generation, every trial's tables at once: alone at a table
/// of its own clones before the market is taught, among others after
/// (see [`companies_among`]). Each genome sits `tables` tables, [`GROUP`]
/// per company it is drawn into; its fitness is the mean of its scores.
/// One fitness and outcomes per trial.
fn evaluate(
    seats: &[Seat],
    rounds: &[Round],
    a: &Args,
    arena: Option<&Arena>,
    pace: &mut Pace,
) -> Vec<(Vec<f32>, Vec<Outcome>)> {
    let groups = a.tables / GROUP;
    let companies: Vec<Company> = rounds
        .iter()
        .flat_map(|round| {
            if a.stage.market() {
                companies_among(round, groups)
            } else {
                round
                    .ours
                    .clone()
                    .flat_map(|g| std::iter::repeat_n([g; 6], groups))
                    .collect()
            }
        })
        .collect();
    let table_of = |company: &Company| a.table_of(|chair| Some(seats[company[chair]].stage));
    // Every company at GROUP tables in a row (one workgroup), the chairs
    // dealt anew at each.
    let mut rng = rand::thread_rng();
    let mut all: Vec<Company> = Vec::with_capacity(companies.len() * GROUP);
    for company in &companies {
        for _ in 0..GROUP {
            let mut c = *company;
            c.shuffle(&mut rng);
            all.push(c);
        }
    }
    let outcomes: Vec<[Outcome; 6]> = match arena {
        // The card takes the first tables, the cores the rest, at once —
        // the cores waiting until the card's first run is staged, so its
        // pool is laid out at full speed.
        Some(arena) => {
            let (on_gpu, on_cpu) = all.split_at(pace.cut(all.len()));
            let (tx, rx) = std::sync::mpsc::channel();
            let (mut from_gpu, from_cpu) = std::thread::scope(|scope| {
                let gpu = scope.spawn(|| {
                    let clock = Instant::now();
                    let out = play_on(arena, seats, on_gpu, a, table_of, || {
                        let _ = tx.send(());
                    });
                    (out, clock.elapsed())
                });
                let _ = rx.recv();
                let clock = Instant::now();
                let out = play_all(seats, on_cpu, table_of);
                let cpu_took = clock.elapsed();
                let (from_gpu, gpu_took) = gpu.join().unwrap();
                pace.learn((on_cpu.len(), cpu_took), (on_gpu.len(), gpu_took));
                (from_gpu, out)
            });
            from_gpu.extend(from_cpu);
            from_gpu
        }
        None => play_all(seats, &all, table_of),
    };
    // Every seat a genome sat: its score at that table, and the outcome.
    let mut per: Vec<Vec<Vec<(f32, Outcome)>>> = rounds
        .iter()
        .map(|round| vec![Vec::new(); round.ours.len()])
        .collect();
    for (company, outcomes) in all.iter().zip(&outcomes) {
        let scores = table_of(company).scores(outcomes);
        for chair in 0..6 {
            if let Some((trial, i)) = seats[company[chair]].scored {
                per[trial][i].push((scores[chair], outcomes[chair]));
            }
        }
    }
    per.into_iter()
        .map(|per| {
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
        })
        .collect()
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
fn show(best: &[f32], reads: Reads, a: &Args) {
    let b = Brain::from_genome(best, reads);
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

/// `--measure` tables of `best` on seat 0 against five of `--against`,
/// every year of seat 0 written to `path`: the game, the year, how the
/// realm stood, its wars of the year, its intelligence, then the 32
/// recall entries the Extérieur wrote down that year (read again the
/// next). For reading what a brain keeps in its memory.
fn trace(best: &[f32], reads: Reads, a: &Args, path: &str) {
    let b = Brain::from_genome(best, reads);
    let rivals = rivals(a);
    assert!(!rivals.is_empty(), "--trace needs --against");
    let brains: Vec<Brain> = rivals.iter().map(Other::brain).collect();
    let mut csv = String::from(
        "game,year,title,dead,crowned,treasury,soldiers,efficiency,surface,peasants,nobles,merchants,stocks,forts,rams,marched,attacked,scouted,scout_target,expeditions,mills,markets,foundries,shipyards,palaces,hospices,price,to_sell,land_ratio",
    );
    for i in 0..RECALL {
        csv.push_str(&format!(",r{i}"));
    }
    csv.push('\n');
    let rows: Vec<String> = (0..a.measure.max(1))
        .into_par_iter()
        .flat_map(|game| {
            let seats = seat_rivals(&rivals);
            let table = a.table_of(|i| (i > 0).then(|| rivals[seats[i]].stage));
            let mut rows = Vec::new();
            let me = KINGDOMS[0];
            watch(
                [
                    &b,
                    &brains[seats[1]],
                    &brains[seats[2]],
                    &brains[seats[3]],
                    &brains[seats[4]],
                    &brains[seats[5]],
                ],
                &table,
                |y: YearEnd| {
                    let k = &y.game.kingdoms[0];
                    let m = &y.memories[0];
                    let marched: i32 = y
                        .fought
                        .iter()
                        .flat_map(Fought::expeditions)
                        .filter(|e| e.attacker == me)
                        .map(|e| e.soldiers)
                        .sum();
                    let attacked: i32 = y
                        .fought
                        .iter()
                        .filter(|f| f.target == Some(me))
                        .flat_map(Fought::expeditions)
                        .map(|e| e.soldiers)
                        .sum();
                    let expeditions = y
                        .fought
                        .iter()
                        .flat_map(Fought::expeditions)
                        .filter(|e| e.attacker == me)
                        .count();
                    let mut row = format!(
                        "{game},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{marched},{attacked},{},{},{expeditions},{},{},{},{},{},{},{},{},{}",
                        y.game.year,
                        k.title() as i32,
                        u8::from(k.is_dead),
                        u8::from(y.outcomes[0].crowned.is_some()),
                        k.treasury,
                        k.soldiers,
                        k.soldiers_efficiency,
                        k.surface,
                        k.peasants,
                        k.nobles,
                        k.merchants,
                        k.grain_stocks,
                        k.fortifications,
                        k.rams,
                        u8::from(y.sent[0].scout.is_some()),
                        y.sent[0].scout.map_or(-1, |t| t.index() as i32),
                        k.grain_mills,
                        k.marketplaces,
                        k.foundries,
                        k.shipyards,
                        k.palaces,
                        k.hospices,
                        k.grain_price,
                        k.grain_to_sell,
                        k.land_ratio(),
                    );
                    for r in &m.last_orders[ORDERS..] {
                        row.push_str(&format!(",{r:.4}"));
                    }
                    row.push('\n');
                    rows.push(row);
                },
            );
            rows
        })
        .collect();
    csv.extend(rows);
    eprintln!(
        "{path}: {} rows of {} games",
        csv.lines().count() - 1,
        a.measure.max(1)
    );
    fs::write(path, csv).unwrap();
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

/// The `--against` brains: a school's best at its rung, or a delivered
/// `.f32` (the game's computers: told + recall, read at `--stage`).
fn rivals(a: &Args) -> Vec<Other> {
    a.against
        .iter()
        .map(|path| match path.strip_suffix(".f32") {
            Some(_) => {
                let bytes = fs::read(path).unwrap();
                let genome: Vec<f32> = bytes
                    .chunks_exact(4)
                    .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                    .collect();
                Other {
                    genome,
                    reads: Reads::ALL,
                    stage: a.stage,
                }
            }
            None => {
                let school = load(path, None);
                Other {
                    reads: school.reads(),
                    stage: stage(&school.stage),
                    genome: school.best,
                }
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
fn measure(best: &[f32], reads: Reads, a: &Args, arena: Option<&Arena>) {
    let b = Brain::from_genome(best, reads);
    let rivals = rivals(a);
    if rivals.is_empty() {
        let outcomes: Vec<Outcome> = (0..a.measure)
            .into_par_iter()
            .flat_map(|_| play([&b; 6], &a.table()))
            .collect();
        tally(&format!("{:?}", a.stage), &outcomes, a);
        return;
    }
    let seatings: Vec<[usize; 6]> = (0..a.measure).map(|_| seat_rivals(&rivals)).collect();
    let table_of = |seats: &[usize; 6]| a.table_of(|i| (i > 0).then(|| rivals[seats[i]].stage));
    let tables: Vec<[Outcome; 6]> = match arena {
        Some(arena) => {
            let mut genomes: Vec<&[f32]> = vec![best];
            genomes.extend(rivals.iter().map(|o| o.genome.as_slice()));
            let widened = widened(&genomes, arena);
            let pool = arena.pool(&widened.iter().map(Cow::as_ref).collect::<Vec<_>>());
            let game = EmpireGame::default();
            let mut rng = rand::thread_rng();
            let tables: Vec<state::Table> = seatings
                .iter()
                .map(|seats| {
                    let table = table_of(seats);
                    let seating = Seating {
                        genomes: std::array::from_fn(
                            |i| if i == 0 { 0 } else { seats[i] as u32 + 1 },
                        ),
                        reads: std::array::from_fn(|i| {
                            if i == 0 {
                                reads
                            } else {
                                rivals[seats[i]].reads
                            }
                        }),
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
            let brains: Vec<Brain> = rivals.iter().map(Other::brain).collect();
            seatings
                .par_iter()
                .map(|seats| {
                    play(
                        std::array::from_fn(|i| if i == 0 { &b } else { &brains[seats[i]] }),
                        &table_of(seats),
                    )
                })
                .collect()
        }
    };
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
fn wake(mean: &mut [f32], sigma: &mut [f32], best: &mut [f32], letters: Letters, shape: Shape) {
    let scales = shape.scales();
    let rows = if letters.agents() { 6..17 } else { 6..12 };
    for o in rows {
        for k in shape.exterieur_output(o) {
            sigma[k] = scales[k];
        }
    }
    let mut seeds = vec![(11, -3.0)];
    if letters.agents() {
        seeds.extend((12..17).map(|o| (o, 3.0)));
    }
    for (o, bias) in seeds {
        let k = shape.exterieur_output(o).end - 1;
        mean[k] = bias;
        best[k] = bias;
    }
}

/// A school saved by an earlier run — grown to today's widths at the
/// `hidden` layer asked (its own when none is) if it was schooled
/// narrower: the new entries and neurons start blind, at their starting
/// spread, the new answers blank. A school from before the recall was
/// told everything, and stays so.
fn load(path: &str, hidden: Option<usize>) -> School {
    let mut s: School = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(
        s.rules, RULES,
        "{path} was schooled under rules {}, these are {RULES}: raise it again",
        s.rules
    );
    if s.mean.len() != Shape::from(s.widths).genome() {
        // Written before the widths were: one shape for all of them.
        s.widths = Widths::BEFORE_THE_WALLS;
    }
    let was = Shape::from(s.widths);
    let now = Shape::wide(hidden.unwrap_or(was.hidden));
    assert_eq!(
        s.mean.len(),
        was.genome(),
        "{path} is not a genome of the shape it claims"
    );
    if was == now {
        return s;
    }
    if was.b_out == ORDERS {
        // From before the recall: told, nothing to remember with.
        s.told = true;
        s.recall = false;
    }
    let seen = Brain::grown(&vec![1.0; s.mean.len()], was, now);
    let scales = now.scales();
    s.mean = Brain::grown(&s.mean, was, now);
    s.best = Brain::grown(&s.best, was, now);
    s.sigma = Brain::grown(&s.sigma, was, now)
        .into_iter()
        .zip(seen)
        .zip(scales)
        .map(|((sigma, seen), scale)| if seen > 0.0 { sigma } else { scale })
        .collect();
    for l in &mut s.hall {
        l.genome = Brain::grown(&l.genome, was, now);
    }
    for p in &mut s.parents {
        *p = Brain::grown(p, was, now);
    }
    s.widths = Widths::from(now);
    eprintln!("{path}: grown from {was:?} to {now:?}");
    s
}

/// The best genome as the game reads it: floats, little-endian.
fn deliver(best: &[f32], path: &str) {
    let bytes: Vec<u8> = best.iter().flat_map(|w| w.to_le_bytes()).collect();
    fs::write(path, &bytes).unwrap();
    eprintln!("{path}: {} weights delivered", best.len());
}

/// A school in the making: its distribution, its best, its Hall, what
/// its brains are told, the file it is written to.
#[derive(Clone)]
struct Trial {
    reads: Reads,
    out: String,
    mean: Vec<f32>,
    sigma: Vec<f32>,
    best: Vec<f32>,
    best_fitness: f32,
    hall: Vec<Laureate>,
    /// The elite's mean score of the best generation so far, kept apart
    /// in case the population drifts away from it later.
    best_elite: f32,
    /// The last elite, best first, when the school breeds.
    parents: Vec<Vec<f32>>,
}

impl Trial {
    /// A trial from nothing: blind, at the starting spread.
    fn fresh(shape: Shape, reads: Reads) -> Trial {
        Trial {
            reads,
            out: String::new(),
            mean: vec![0.0; shape.genome()],
            sigma: shape.scales(),
            best: vec![0.0; shape.genome()],
            best_fitness: f32::MIN,
            hall: Vec::new(),
            best_elite: f32::MIN,
            parents: Vec::new(),
        }
    }

    /// The trial going on from a school.
    fn from_school(s: &School) -> Trial {
        Trial {
            reads: s.reads(),
            out: String::new(),
            mean: s.mean.clone(),
            sigma: s.sigma.clone(),
            best: s.best.clone(),
            best_fitness: f32::MIN,
            hall: s.hall.clone(),
            best_elite: f32::MIN,
            parents: s.parents.clone(),
        }
    }

    /// The generation's population: drawn around the mean, the best so
    /// far sitting again — a lucky draw must prove itself. A breeding
    /// school with parents sits them again and fills the rest with their
    /// children, each parent begetting its share in turn.
    fn draw(&self, size: usize, a: &Args) -> Vec<Vec<f32>> {
        if a.breed > 0.0 && !self.parents.is_empty() {
            let spread: Vec<f32> = a.shape().scales().iter().map(|s| s * a.breed).collect();
            let parents = &self.parents;
            let mut genomes = parents.clone();
            genomes.extend(
                (0..size.saturating_sub(parents.len()))
                    .into_par_iter()
                    .map(|i| draw(&parents[i % parents.len()], &spread))
                    .collect::<Vec<_>>(),
            );
            return genomes;
        }
        let mut genomes: Vec<Vec<f32>> = (0..size)
            .into_par_iter()
            .map(|_| draw(&self.mean, &self.sigma))
            .collect();
        if self.best_fitness > f32::MIN {
            genomes[0].clone_from(&self.best);
        }
        genomes
    }

    /// The mean and spread moved onto the elite of `genomes`, the best
    /// kept, the Hall told; the school written out (and beside it when
    /// its elite is the best seen). The generation's reading.
    fn learn(
        &mut self,
        generation: usize,
        genomes: &[Vec<f32>],
        fitness: &[f32],
        outcomes: &[Outcome],
        a: &Args,
        clock: Instant,
    ) -> String {
        let size = genomes.len();
        let mut order: Vec<usize> = (0..size).collect();
        order.sort_by(|&i, &j| fitness[j].total_cmp(&fitness[i]));
        let elite = &order[..a.elite.min(size)];
        self.best_fitness = fitness[order[0]];
        self.best.clone_from(&genomes[order[0]]);
        if a.hall > 0 {
            induct(&mut self.hall, generation, &self.best, a.hall);
        }
        let n = elite.len() as f32;
        let elite_mean = elite.iter().map(|&i| fitness[i]).sum::<f32>() / n;
        let floor = a.shape().scales();
        for w in 0..genomes[0].len() {
            let m = elite.iter().map(|&i| genomes[i][w]).sum::<f32>() / n;
            let v = elite
                .iter()
                .map(|&i| (genomes[i][w] - m).powi(2))
                .sum::<f32>()
                / n;
            self.mean[w] = 0.3 * self.mean[w] + 0.7 * m;
            self.sigma[w] = (0.3 * self.sigma[w] + 0.7 * v.sqrt()).max(floor[w] * 0.05);
        }
        if a.breed > 0.0 {
            self.parents = elite.iter().map(|&i| genomes[i].clone()).collect();
        }
        let mut school = School {
            rules: RULES,
            stage: format!("{:?}", a.stage).to_lowercase(),
            generation,
            widths: Widths::from(a.shape()),
            told: self.reads.told,
            recall: self.reads.recall,
            mean: self.mean.clone(),
            sigma: self.sigma.clone(),
            best: self.best.clone(),
            best_fitness: self.best_fitness,
            hall: self.hall.clone(),
            parents: Vec::new(),
        };
        let light = serde_json::to_string(&school).unwrap();
        if elite_mean > self.best_elite {
            self.best_elite = elite_mean;
            fs::write(best_out(&self.out), &light).unwrap();
        }
        if a.keep > 0 && generation.is_multiple_of(a.keep) {
            fs::write(kept_out(&self.out, generation), &light).unwrap();
        }
        school.parents.clone_from(&self.parents);
        let json = if school.parents.is_empty() {
            light
        } else {
            serde_json::to_string(&school).unwrap()
        };
        fs::write(&self.out, &json).unwrap();
        reading(generation, fitness, outcomes, clock.elapsed().as_secs_f32())
    }
}

/// A trial going on from the school at `path` (its `{n}` filled in, so
/// that every trial may go on from its own) — the school's next
/// generation, or a fresh one from nothing; its letters woken, its Hall
/// cut to size.
fn trial_from(path: Option<&str>, n: usize, a: &Args) -> (Trial, usize) {
    let (mut trial, start) = match path {
        Some(path) => {
            let s = load(&path.replace("{n}", &n.to_string()), Some(a.hidden));
            (Trial::from_school(&s), s.generation + 1)
        }
        None => (Trial::fresh(a.shape(), a.reads), 0),
    };
    if a.letters.scouts() {
        wake(
            &mut trial.mean,
            &mut trial.sigma,
            &mut trial.best,
            a.letters,
            a.shape(),
        );
    }
    trial.hall.truncate(a.hall);
    trial.out = a.out.replace("{n}", &n.to_string());
    (trial, start)
}

fn main() {
    let a = args();
    let (first, start) = trial_from(a.from.as_deref(), 1, &a);
    let mut reads = first.reads;
    if a.no_recall {
        reads.recall = false;
    }
    if let Some(path) = &a.deliver {
        deliver(&first.best, path);
        return;
    }
    if a.show {
        show(&first.best, reads, &a);
        return;
    }
    if let Some(path) = &a.trace {
        trace(&first.best, reads, &a, path);
        return;
    }
    let rivals = rivals(&a);
    let gpu = a.gpu.then(|| Gpu::open().expect("a GPU to play on"));
    let arena = gpu
        .as_ref()
        .map(|gpu| Arena::new(gpu, kernel_width(&a, &rivals)));
    if a.measure > 0 {
        measure(&first.best, reads, &a, arena.as_ref());
        return;
    }
    let mut trials: Vec<Trial> = std::iter::once(first)
        .chain((2..=a.trials).map(|n| trial_from(a.from.as_deref(), n, &a).0))
        .collect();
    eprintln!(
        "{:?} · reads {:?} · hidden {} · genome {} · population {} (first {}) · elite {}{} · tables {} · hall {} · rank {} · letters {:?} · longest {} years · {} trials · {}",
        a.stage,
        reads,
        a.hidden,
        a.shape().genome(),
        a.population,
        a.first,
        a.elite,
        if a.breed > 0.0 {
            format!(" breeding at {}", a.breed)
        } else {
            String::new()
        },
        a.tables,
        a.hall,
        a.rank,
        a.letters,
        a.longest,
        a.trials,
        gpu.as_ref().map_or("CPU", |g| g.name.as_str()),
    );
    let mut pace = Pace::new();
    for generation in start..start + a.generations {
        let clock = Instant::now();
        let size = if generation == 0 {
            a.first
        } else {
            a.population
        };
        // As many trials at once as their populations fit in memory.
        let group = (MEMORY_BUDGET / (size * a.shape().genome() * 4)).clamp(1, a.trials);
        for first in (0..a.trials).step_by(group) {
            let group = &mut trials[first..(first + group).min(a.trials)];
            let populations: Vec<Vec<Vec<f32>>> = group.iter().map(|t| t.draw(size, &a)).collect();
            // The pool: every trial's population, then the rivals, then
            // every trial's Hall.
            let mut seats: Vec<Seat> = Vec::new();
            let mut rounds: Vec<Round> = Vec::new();
            for (t, genomes) in populations.iter().enumerate() {
                let from = seats.len();
                seats.extend(genomes.iter().enumerate().map(|(i, g)| Seat {
                    genome: g,
                    reads,
                    stage: a.stage,
                    scored: Some((t, i)),
                }));
                rounds.push(Round {
                    ours: from..seats.len(),
                    others: Vec::new(),
                });
            }
            let shared: Vec<usize> = (seats.len()..seats.len() + rivals.len()).collect();
            seats.extend(rivals.iter().map(|o| Seat {
                genome: &o.genome,
                reads: o.reads,
                stage: o.stage,
                scored: None,
            }));
            for (round, trial) in rounds.iter_mut().zip(group.iter()) {
                round.others.clone_from(&shared);
                round
                    .others
                    .extend(seats.len()..seats.len() + trial.hall.len());
                seats.extend(trial.hall.iter().map(|l| Seat {
                    genome: &l.genome,
                    reads,
                    stage: a.stage,
                    scored: None,
                }));
            }
            let played = evaluate(&seats, &rounds, &a, arena.as_ref(), &mut pace);
            // Every trial learns at once: the school files are big.
            let readings: Vec<String> = group
                .par_iter_mut()
                .zip(&populations)
                .zip(&played)
                .map(|((trial, genomes), (fitness, outcomes))| {
                    trial.learn(generation, genomes, fitness, outcomes, &a, clock)
                })
                .collect();
            for (n, reading) in readings.into_iter().enumerate() {
                if a.trials > 1 {
                    println!("try {:2} · {reading}", first + n + 1);
                } else {
                    println!("{reading}");
                }
            }
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
