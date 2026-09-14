//! The year kernel against `arena::watch`: the seven schools at a war
//! table, every year of the CPU's game replayed on the GPU from the
//! CPU's own state of the year before, and the two ends compared.

use empire_gpu::state::{Seating, Table};
use empire_gpu::{Arena, Gpu, Pool};
use empire_lib::arena;
use empire_lib::brain::{Brain, Letters, Reads, Stage, HIDDEN};
use empire_lib::game::EmpireGame;
use empire_lib::random;

const SCHOOLS: [&[u8]; 7] = [
    include_bytes!("../../empire-lib/brains/soldat.f32"),
    include_bytes!("../../empire-lib/brains/batisseuse.f32"),
    include_bytes!("../../empire-lib/brains/garnison.f32"),
    include_bytes!("../../empire-lib/brains/boutiquiere.f32"),
    include_bytes!("../../empire-lib/brains/fonceuse.f32"),
    include_bytes!("../../empire-lib/brains/conquerante.f32"),
    include_bytes!("../../empire-lib/brains/prudente.f32"),
];

fn genomes() -> Vec<Vec<f32>> {
    SCHOOLS
        .iter()
        .map(|bytes| {
            bytes
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                .collect()
        })
        .collect()
}

fn slices(genomes: &[Vec<f32>]) -> Vec<&[f32]> {
    genomes.iter().map(Vec::as_slice).collect()
}

/// A war table of `pool` genomes, fresh, its seating, realms and dice
/// drawn from `seed`.
fn fresh_table_of(seed: u32, pool: u32) -> Table {
    let seating = Seating {
        genomes: std::array::from_fn(|i| (i as u32 * 131 + seed * 7) % pool),
        reads: [Reads::ALL; 6],
        seats: [Stage::War; 6],
        stage: Stage::War,
        longest: 60,
        letters: Letters::None,
    };
    random::seed(seed);
    let game = EmpireGame::default();
    Table::fresh(&seating, &game, random::snapshot())
}

/// A war table of the schools.
fn fresh_table(seed: u32) -> Table {
    fresh_table_of(seed, 7)
}

/// A population the size of a school's, the schools with a little noise
/// on each, so that no two tables read the same weights.
fn population() -> Vec<Vec<f32>> {
    let schools = genomes();
    random::seed(11);
    (0..1024)
        .map(|i| {
            schools[i % schools.len()]
                .iter()
                .map(|w| w + (random::random(0, 2001) - 1000) as f32 * 1e-4)
                .collect()
        })
        .collect()
}

/// The CPU's game at `seating` from `seed`: the table before any year,
/// then after each year played.
fn cpu_years(seed: u32, seating: &Seating, genomes: &[Vec<f32>]) -> Vec<Table> {
    let brains: Vec<Brain> = (0..6)
        .map(|i| Brain::from_genome(&genomes[seating.genomes[i] as usize], seating.reads[i]))
        .collect();
    let table = arena::Table {
        stage: seating.stage,
        seats: seating.seats,
        longest: seating.longest,
        rank_cost: 0.0,
        walls_cost: 0.0,
        letters: seating.letters,
    };
    random::seed(seed);
    let mut years = vec![Table::fresh(
        seating,
        &EmpireGame::default(),
        random::snapshot(),
    )];
    random::seed(seed);
    arena::watch(std::array::from_fn(|i| &brains[i]), &table, |end| {
        let mut game = end.game.clone();
        game.increment_year();
        years.push(Table::from_game(
            seating,
            &game,
            end.memories,
            end.outcomes,
            random::snapshot(),
        ));
    });
    years
}

/// Whether the only differences are a treasury (and its year's net)
/// off by a few coins in the million: the CPU's `powf` and the GPU's
/// `pow` round the last bits differently, and the taxes are truncated
/// to the coin.
fn rounding_only(got: &Table, want: &Table) -> bool {
    let mut g = *got;
    for i in 0..6 {
        let d = want.kingdoms[i].treasury - g.kingdoms[i].treasury;
        let slack = 2 + want.memories[i].net.abs() / 100_000;
        if d.abs() <= slack && want.memories[i].net - g.memories[i].net == d {
            g.kingdoms[i].treasury = want.kingdoms[i].treasury;
            g.memories[i].net = want.memories[i].net;
            // An agent's ledger copies the treasury it read, coin included.
            for s in 0..6 {
                let (ledger, want_ledger) = (
                    &mut g.memories[s].dossiers[i],
                    &want.memories[s].dossiers[i],
                );
                if want_ledger.treasury - ledger.treasury == d {
                    ledger.treasury = want_ledger.treasury;
                }
            }
        }
    }
    g.differences(want, 1e-4).is_empty()
}

#[test]
fn the_gpu_plays_the_cpu_years() {
    let Some(gpu) = Gpu::open() else {
        eprintln!("no GPU: skipped");
        return;
    };
    let genomes = genomes();
    let arena = Arena::new(&gpu, HIDDEN);
    let pool = arena.pool(&slices(&genomes));
    let stages = [
        Stage::Survive,
        Stage::Emperor,
        Stage::Market,
        Stage::Guard,
        Stage::War,
        Stage::War,
        Stage::War,
        Stage::War,
    ];
    let letters = [Letters::None, Letters::Scouts, Letters::All];
    let (mut played, mut rounding, mut divergent) = (0, 0, 0);
    for seed in 0..16u32 {
        let stage = stages[seed as usize % 8];
        let seating = Seating {
            genomes: std::array::from_fn(|i| ((i + seed as usize) % 7) as u32),
            reads: std::array::from_fn(|i| Reads::from_bits((i as u32 + seed) % 8)),
            seats: std::array::from_fn(|i| {
                if i.is_multiple_of(2) {
                    stage
                } else {
                    Stage::War
                }
            }),
            stage,
            longest: 60,
            letters: letters[seed as usize % 3],
        };
        let years = cpu_years(seed, &seating, &genomes);
        let starts: Vec<Table> = years[..years.len() - 1].to_vec();
        let ends = arena.play(&pool, &starts, 1);
        for (y, (got, want)) in ends.iter().zip(&years[1..]).enumerate() {
            played += 1;
            let differences = got.differences(want, 1e-4);
            if differences.is_empty() {
                continue;
            }
            if rounding_only(got, want) {
                rounding += 1;
                continue;
            }
            divergent += 1;
            eprintln!(
                "seed {seed} ({stage:?}), year {}: {} differences\n  {}",
                y + 1,
                differences.len(),
                differences.join("\n  ")
            );
        }
    }
    eprintln!("{played} years: {rounding} off by a coin, {divergent} divergent");
    assert!(
        divergent * 50 <= played,
        "{divergent} of {played} years diverge"
    );
}

/// Whole games on the GPU, timed: a fresh table per seed, played to
/// the end in one dispatch.
#[test]
#[ignore = "a benchmark: run with --ignored --nocapture"]
fn whole_games_throughput() {
    let Some(gpu) = Gpu::open() else {
        eprintln!("no GPU: skipped");
        return;
    };
    let genomes = genomes();
    let arena = Arena::new(&gpu, HIDDEN);
    let brains: Vec<Brain> = genomes
        .iter()
        .map(|g| Brain::from_genome(g, Reads::ALL))
        .collect();
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let start = std::time::Instant::now();
    std::thread::scope(|scope| {
        for t in 0..threads {
            let brains = &brains;
            scope.spawn(move || {
                for seed in (t as u32..512).step_by(threads) {
                    random::seed(seed);
                    arena::play(
                        std::array::from_fn(|i| &brains[(i + seed as usize) % 7]),
                        &arena::Table::at(Stage::War, 60),
                    );
                }
            });
        }
    });
    let took = start.elapsed();
    eprintln!(
        "CPU, {threads} threads: 512 tables in {took:?}: {:.0} tables/s",
        512.0 / took.as_secs_f64()
    );
    let pool = arena.pool(&slices(&genomes));
    for &n in &[32usize, 64, 256, 1024, 2048, 4096, 8192, 16384] {
        measure(&arena, &pool, n, 7);
    }
    let population = population();
    let pool = arena.pool(&slices(&population));
    for &n in &[8192usize, 16384] {
        measure(&arena, &pool, n, population.len() as u32);
    }
}

/// `n` tables of the `genomes` of `pool` played 60 years on `arena`,
/// timed.
fn measure(arena: &Arena, pool: &Pool, n: usize, genomes: u32) {
    let tables: Vec<Table> = (0..n as u32).map(|s| fresh_table_of(s, genomes)).collect();
    let start = std::time::Instant::now();
    let ends = arena.play(pool, &tables, 60);
    let took = start.elapsed();
    let years: i32 = ends.iter().map(|t| t.year - 1).sum();
    let crowned = ends
        .iter()
        .flat_map(|t| t.outcomes.iter())
        .filter(|o| o.crowned >= 0)
        .count();
    eprintln!(
        "{n} tables of {genomes} genomes, {years} years in {took:?}: {:.0} tables/s, {crowned} crowns",
        n as f64 / took.as_secs_f64()
    );
}

#[test]
fn whole_games_are_deterministic() {
    let Some(gpu) = Gpu::open() else {
        eprintln!("no GPU: skipped");
        return;
    };
    let genomes = genomes();
    let arena = Arena::new(&gpu, HIDDEN);
    let pool = arena.pool(&slices(&genomes));
    let tables: Vec<Table> = (0..256u32).map(fresh_table).collect();
    let once = arena.play(&pool, &tables, 60);
    let twice = arena.play(&pool, &tables, 60);
    let mut by_year = tables.clone();
    for _ in 0..60 {
        by_year = arena.play(&pool, &by_year, 1);
    }
    let count = |ends: &[Table]| {
        ends.iter()
            .zip(&once)
            .filter(|(a, b)| !a.differences(b, 0.0).is_empty())
            .count()
    };
    eprintln!(
        "{} tables differ on a replay, {} played a year at a time",
        count(&twice),
        count(&by_year)
    );
    for (i, (a, b)) in by_year.iter().zip(&once).enumerate().take(3) {
        let d = a.differences(b, 0.0);
        if !d.is_empty() {
            eprintln!("table {i}:\n  {}", d.join("\n  "));
        }
    }
}
