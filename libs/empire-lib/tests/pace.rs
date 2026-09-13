//! Where a training generation spends its time — run by hand:
//! `cargo test --release -p empire-lib --test pace -- --ignored --nocapture`

use std::time::Instant;

use empire_lib::arena::{play, Table};
use empire_lib::brain::{sight, Brain, Memory, Reads, Stage, A_IN, A_OUT, B_IN, B_OUT};
use empire_lib::front::{simulate_front, Host};
use empire_lib::game::EmpireGame;
use empire_lib::kingdom::Kingdoms;

fn secs(t: Instant) -> f64 {
    t.elapsed().as_secs_f64()
}

#[test]
#[ignore = "a stopwatch, not a check"]
fn where_the_time_goes() {
    let brain = &Brain::schools()[0];
    let table = Table {
        rank_cost: 40.0,
        ..Table::at(Stage::War, 150)
    };

    // Whole games, as evaluate() plays them.
    let games = 60;
    let t = Instant::now();
    let mut years = 0i64;
    for _ in 0..games {
        let outcomes = play([brain; 6], &table);
        years += outcomes.iter().map(|o| o.years as i64).sum::<i64>();
    }
    let full = secs(t);
    println!(
        "play: {:.1} ms/game · {years} kingdom-years · {:.1} µs/kingdom-year",
        full / games as f64 * 1e3,
        full / years as f64 * 1e6
    );

    // The brain alone: sight building and the two nets, as one kingdom-year
    // costs them (1 intendance + 2 exterieur readings).
    let game = EmpireGame::default();
    let m = Memory::default();
    let n = 100_000;
    let t = Instant::now();
    let mut sink = 0.0f32;
    for _ in 0..n {
        sink += sight(&game, Kingdoms::France, &m, Reads::ALL)[0];
    }
    let sight_s = secs(t);
    let view = sight(&game, Kingdoms::France, &m, Reads::ALL);
    let mut wide = view.clone();
    wide.extend([0.0; A_OUT]);
    let t = Instant::now();
    for _ in 0..n {
        sink += brain.intendance.forward(&view)[0];
    }
    let a_s = secs(t);
    let t = Instant::now();
    for _ in 0..n {
        sink += brain.exterieur.forward(&wide)[0];
    }
    let b_s = secs(t);
    println!(
        "sight {:.2} µs · A({A_IN}->{A_OUT}) {:.2} µs · B({B_IN}->{B_OUT}) {:.2} µs \
         · per kingdom-year (3 sights + A + 2B): {:.2} µs (sink {sink:.3})",
        sight_s / n as f64 * 1e6,
        a_s / n as f64 * 1e6,
        b_s / n as f64 * 1e6,
        (3.0 * sight_s + a_s + 2.0 * b_s) / n as f64 * 1e6,
    );

    // One siege, as a campaign year fights it.
    let host = Host {
        attacker: Kingdoms::France,
        men: 200,
        rams: 0,
    };
    let m2 = 2_000;
    let t = Instant::now();
    let mut rounds = 0usize;
    for _ in 0..m2 {
        let r = simulate_front(&game, Kingdoms::Spain, &[host]);
        rounds += r.rounds.len();
    }
    println!(
        "front(200 men): {:.1} µs · {:.0} rounds each",
        secs(t) / m2 as f64 * 1e6,
        rounds as f64 / m2 as f64
    );
}
