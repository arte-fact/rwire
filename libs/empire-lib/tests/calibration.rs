//! The computers' intelligence, calibrated at the table: games of six
//! computers played silently on the web's campaign model, so the
//! temperaments can be read in the numbers — how long a game lasts, who
//! wins, how often each council marches.
//!
//! `cargo test --release -p empire-lib --test calibration -- --ignored --nocapture`
//! prints the thousand-game reading. The original's AI (a random realm
//! three years in four, a third to all of the men) ends a game of six
//! computers in 69 years (median); the temperaments, with an ost of at
//! least three quarters of the men, in about 90.

use empire_lib::campaign::{apply_battle, march, Expedition};
use empire_lib::harvests::{apply_grain_harvest, apply_rat_loss_rate, apply_seed_grain};
use empire_lib::ia::plan_ai_turn;
use empire_lib::mind::{Mind, Seen, Temper, SCOUT_CAUGHT};
use empire_lib::random::random;
use empire_lib::{EmpireGame, Kingdoms, PlayerTitle, KINGDOMS};

/// A game stops here whatever happens (none should get near).
const LONGEST: i32 = 400;

#[derive(Default, Debug)]
struct Tally {
    games: usize,
    years: Vec<i32>,
    /// Games won (an Emperor crowned or the last realm standing) per temper.
    wins: [usize; 3],
    /// Council-years and attacks ordered, per temper (year three on).
    years_sat: [usize; 3],
    attacks: [usize; 3],
    /// Games ended by a crowning rather than by the last realm standing.
    crowned: usize,
}

fn slot(t: Temper) -> usize {
    match t {
        Temper::Bold => 0,
        Temper::Measured => 1,
        Temper::Cautious => 2,
    }
}

/// The game's end: the first Emperor, or the last realm alive.
fn winner(game: &EmpireGame) -> Option<(Kingdoms, bool)> {
    if let Some(k) = game
        .kingdoms
        .iter()
        .find(|k| !k.is_dead && k.title() == PlayerTitle::Emperor)
    {
        return Some((k.id, true));
    }
    let alive = game.alive_kingdoms();
    (alive.len() <= 1).then(|| (alive[0], false))
}

/// One year at the table, as the web plays it: the councils sit, the
/// armies march together, the éclaireurs read the realms as they stand.
fn play_year(game: &mut EmpireGame, minds: &mut [Mind; 6], tally: &mut Tally) {
    let weather = game.random_weather();
    game.open_market();
    let mut orders = Vec::new();
    let mut scouts = Vec::new();
    for id in KINGDOMS {
        let k = game.kingdom_mut(id);
        if k.is_dead {
            continue;
        }
        apply_seed_grain(k);
        apply_rat_loss_rate(k);
        apply_grain_harvest(k, weather);
        let mind = &mut minds[id.index()];
        let d = plan_ai_turn(game, id, mind);
        if game.year >= 3 {
            let s = slot(mind.temper);
            tally.years_sat[s] += 1;
            tally.attacks[s] += d.kingdom_attacks.len();
        }
        orders.extend(d.barbarian_attacks.into_iter().map(|soldiers| Expedition {
            attacker: id,
            target: None,
            soldiers,
        }));
        orders.extend(
            d.kingdom_attacks
                .into_iter()
                .map(|(target, soldiers)| Expedition {
                    attacker: id,
                    target: Some(target),
                    soldiers,
                }),
        );
        if let Some(on) = d.scout {
            scouts.push((id, on));
        }
    }
    for f in march(game, orders) {
        apply_battle(game, &f);
    }
    for (id, on) in scouts {
        if !game.kingdom(on).is_dead && random(0, SCOUT_CAUGHT) != 0 {
            minds[id.index()].seen = Some(Seen::read(game.kingdom(on)));
        }
    }
    game.increment_year();
}

fn play(tally: &mut Tally) {
    let mut game = EmpireGame::default();
    let mut minds: [Mind; 6] = std::array::from_fn(|_| Mind::default());
    tally.games += 1;
    while game.year <= LONGEST {
        if let Some((id, crowned)) = winner(&game) {
            tally.wins[slot(minds[id.index()].temper)] += 1;
            tally.crowned += usize::from(crowned);
            break;
        }
        play_year(&mut game, &mut minds, tally);
    }
    tally.years.push(game.year);
}

fn reading(t: &Tally) -> String {
    let mut years = t.years.clone();
    years.sort_unstable();
    let pct = |n: usize, of: usize| {
        if of == 0 {
            0.0
        } else {
            100.0 * n as f64 / of as f64
        }
    };
    let mut s = format!(
        "{} games · years: min {} · median {} · p90 {} · max {} · crowned {:.0}%\n",
        t.games,
        years[0],
        years[years.len() / 2],
        years[years.len() * 9 / 10],
        years[years.len() - 1],
        pct(t.crowned, t.games)
    );
    let sat: usize = t.years_sat.iter().sum();
    for (i, name) in ["bold", "measured", "cautious"].into_iter().enumerate() {
        s.push_str(&format!(
            "  {name:9} seats {:5.1}% · wins {:5.1}% · attacks per council-year {:.2}\n",
            pct(t.years_sat[i], sat),
            pct(t.wins[i], t.games),
            if t.years_sat[i] == 0 {
                0.0
            } else {
                t.attacks[i] as f64 / t.years_sat[i] as f64
            }
        ));
    }
    s
}

#[test]
fn games_end_and_every_temper_can_win() {
    let mut tally = Tally::default();
    for _ in 0..60 {
        play(&mut tally);
    }
    let text = reading(&tally);
    assert!(
        tally.years.iter().all(|&y| y <= LONGEST),
        "a game never ended\n{text}"
    );
    assert!(tally.attacks[2] > 0, "the cautious never marched\n{text}");
    assert!(
        tally.wins.iter().filter(|&&w| w > 0).count() >= 2,
        "one temper wins everything\n{text}"
    );
}

#[test]
#[ignore = "a thousand games; prints the reading"]
fn a_thousand_games() {
    let mut tally = Tally::default();
    for _ in 0..1000 {
        play(&mut tally);
    }
    println!("{}", reading(&tally));
}
