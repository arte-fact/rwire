//! What a computer knows and how it takes it.
//!
//! The original's AI is a string of dice rolls; this keeps its spirit and
//! puts intelligence between two rolls. Each computer draws a temperament
//! as the game opens and keeps one eye: at most one realm watched, and the
//! éclaireur's report on it. Every year from the third, in this order: the
//! coup de sang (a blind attack, one chance in `a`), the reading of the
//! report (favourable → one chance in `b` of an ost cut to size), then the
//! éclaireur (a new one if the eye has no fresh report, aimed by land).

use crate::front::MILITIA_EFFICIENCY;
use crate::intel::SCOUT_PRICE;
use crate::kingdom::{Kingdom, Kingdoms};
use crate::random::random;

/// The ost cut to a report: this many tenths of the adverse force…
const OST_TENTHS: i32 = 15;
/// …and never less than this many quarters of the men at hand. The force
/// alone beats the garrison but takes little ground: without the floor the
/// games last three times longer than the original's (see the calibration
/// test); at three quarters half again as long, at every man a fifth.
const OST_FLOOR_QUARTERS: i32 = 3;
/// One serf in this many takes up arms against an army that meets no
/// garrison: what an empty realm weighs in a computer's reckoning.
const MILITIA_PER_SERFS: i32 = 100;

/// How a computer's council leans: three numbers, drawn once per game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Temper {
    /// Attacks as soon as it thinks itself a little stronger, and one year
    /// in three without even looking.
    Bold,
    /// The reference: watches, strikes weakness, strays sometimes.
    Measured,
    /// Almost only strikes on a clear superiority; its spies come often,
    /// its armies rarely — until the day one has stripped the garrison.
    Cautious,
}

impl Temper {
    /// Measured two times in three, bold or cautious one in six each.
    pub fn draw() -> Temper {
        match random(0, 6) {
            0 => Temper::Bold,
            1 => Temper::Cautious,
            _ => Temper::Measured,
        }
    }

    /// The coup de sang: one chance in this of a blind attack.
    fn rage(self) -> i32 {
        match self {
            Temper::Bold => 3,
            Temper::Measured => 5,
            Temper::Cautious => 8,
        }
    }

    /// The chance of striking on a favourable report, as `(numerator,
    /// denominator)`.
    fn strike(self) -> (i32, i32) {
        match self {
            Temper::Bold => (1, 1),
            Temper::Measured => (3, 4),
            Temper::Cautious => (2, 3),
        }
    }

    /// How much stronger than the watched realm it must be to find the
    /// report favourable, in tenths. The garrison is read as the Extérieur
    /// opens, at its yearly peak, before the realm's own armies leave it:
    /// the bold march on a realm a little stronger than theirs.
    fn margin_tenths(self) -> i32 {
        match self {
            Temper::Bold => 8,
            Temper::Measured => 10,
            Temper::Cautious => 12,
        }
    }

    /// What an agent writes of the council.
    pub fn told(self) -> &'static str {
        match self {
            Temper::Bold => "belliqueux",
            Temper::Measured => "mesuré",
            Temper::Cautious => "prudent",
        }
    }
}

/// What the éclaireur saw of the watched realm: the figures a war is
/// fought with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seen {
    pub garrison: i32,
    pub efficiency: i32,
    pub serfs: i32,
}

impl Seen {
    pub fn read(k: &Kingdom) -> Seen {
        Seen {
            garrison: k.soldiers,
            efficiency: k.soldiers_efficiency,
            serfs: k.peasants,
        }
    }

    /// The force an army would meet: the garrison at its efficiency, or —
    /// with none — the people who take up arms, at the militia's.
    fn strength(&self) -> i32 {
        if self.garrison > 0 {
            self.garrison * self.efficiency
        } else {
            self.serfs / MILITIA_PER_SERFS * MILITIA_EFFICIENCY
        }
    }
}

/// A computer's mind: its temperament, the realm it watches and what its
/// éclaireur reported on it this year.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mind {
    pub temper: Temper,
    /// The realm watched, if any.
    pub eye: Option<Kingdoms>,
    /// The report of the year on the watched realm, once the éclaireur is
    /// back; read and forgotten as the year's orders are given.
    pub seen: Option<Seen>,
}

impl Default for Mind {
    fn default() -> Self {
        Mind {
            temper: Temper::draw(),
            eye: None,
            seen: None,
        }
    }
}

/// What a computer orders at the Extérieur.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Orders {
    /// The blind attack of the coup de sang: `(target, soldiers)`.
    pub blind: Option<(Kingdoms, i32)>,
    /// The attack cut to the report: `(target, soldiers)`.
    pub aimed: Option<(Kingdoms, i32)>,
    /// The realm the éclaireur rides to; the caller pays [`SCOUT_PRICE`].
    pub scout: Option<Kingdoms>,
}

impl Mind {
    /// The year's war, for `k` facing `enemies` (the living realms but its
    /// own): the coup de sang, the reading of the report, the éclaireur.
    pub fn campaign(&mut self, k: &Kingdom, enemies: &[&Kingdom]) -> Orders {
        let mut orders = Orders::default();
        let seen = self.seen.take();
        if enemies.is_empty() {
            self.eye = None;
            return orders;
        }
        let soldiers = k.soldiers;
        if soldiers > 0 && random(0, self.temper.rage()) == 0 {
            let target = enemies[random(0, enemies.len() as i32) as usize];
            orders.blind = Some((target.id, random(soldiers / 3, soldiers).max(1)));
        }
        // A favourable report keeps the eye on its realm, struck or not; an
        // unfavourable one, a lost eye or a dead realm closes it.
        let mut keep_eye = false;
        if let (Some(eye), Some(seen)) = (self.eye, seen) {
            if enemies.iter().any(|e| e.id == eye) {
                let theirs = seen.strength();
                let mine = soldiers * k.soldiers_efficiency;
                if soldiers > 0 && mine * 10 >= theirs * self.temper.margin_tenths() {
                    keep_eye = true;
                    let (num, den) = self.temper.strike();
                    if random(0, den) < num {
                        let eff = k.soldiers_efficiency.max(1);
                        let men = (theirs * OST_TENTHS / 10 + eff - 1) / eff;
                        let men = men.max(soldiers * OST_FLOOR_QUARTERS / 4);
                        orders.aimed = Some((eye, men.clamp(1, soldiers)));
                    }
                }
            }
        }
        if !keep_eye {
            self.eye = Some(draw_by_land(enemies));
        }
        if k.treasury >= SCOUT_PRICE {
            orders.scout = self.eye;
        }
        orders
    }
}

/// A realm drawn at random, weighted by its land: the big neighbours draw
/// the eye.
fn draw_by_land(enemies: &[&Kingdom]) -> Kingdoms {
    let total: i32 = enemies.iter().map(|e| e.surface.max(1)).sum();
    let mut roll = random(0, total);
    for e in enemies {
        roll -= e.surface.max(1);
        if roll < 0 {
            return e.id;
        }
    }
    enemies[enemies.len() - 1].id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::EmpireGame;

    fn mind(temper: Temper, eye: Kingdoms, seen: Seen) -> Mind {
        Mind {
            temper,
            eye: Some(eye),
            seen: Some(seen),
        }
    }

    fn table() -> EmpireGame {
        let mut game = EmpireGame::default();
        for k in &mut game.kingdoms {
            k.soldiers = 60;
            k.soldiers_efficiency = 100;
            k.treasury = 1000;
        }
        game
    }

    fn enemies(game: &EmpireGame, id: Kingdoms) -> Vec<&Kingdom> {
        game.kingdoms.iter().filter(|k| k.id != id).collect()
    }

    #[test]
    fn the_draw_is_measured_two_times_in_three() {
        let draws: Vec<Temper> = (0..3000).map(|_| Temper::draw()).collect();
        let count = |t| draws.iter().filter(|&&d| d == t).count();
        assert!((1800..2200).contains(&count(Temper::Measured)));
        assert!((350..650).contains(&count(Temper::Bold)));
        assert!((350..650).contains(&count(Temper::Cautious)));
    }

    #[test]
    fn an_empty_garrison_weighs_its_militia() {
        let seen = Seen {
            garrison: 0,
            efficiency: 100,
            serfs: 3000,
        };
        assert_eq!(seen.strength(), 30 * MILITIA_EFFICIENCY);
        let held = Seen {
            garrison: 20,
            ..seen
        };
        assert_eq!(held.strength(), 2000);
    }

    #[test]
    fn a_bold_council_strikes_every_favourable_report_with_an_ost_cut_to_size() {
        let game = table();
        let id = Kingdoms::France;
        let seen = Seen {
            garrison: 20,
            efficiency: 100,
            serfs: 1000,
        };
        for _ in 0..50 {
            let mut m = mind(Temper::Bold, Kingdoms::Spain, seen);
            let o = m.campaign(game.kingdom(id), &enemies(&game, id));
            // 1.5 × 2000 / 100 = 30 men: the floor, 45 of 60, marches.
            assert_eq!(o.aimed, Some((Kingdoms::Spain, 45)));
            assert_eq!(m.eye, Some(Kingdoms::Spain));
            assert_eq!(o.scout, Some(Kingdoms::Spain));
            assert_eq!(m.seen, None);
        }
    }

    #[test]
    fn the_ost_is_bounded_by_the_men_at_hand() {
        let game = table();
        let id = Kingdoms::France;
        // 55 × 100 vs 60 × 100 × 1.2: favourable for a bold council.
        let seen = Seen {
            garrison: 50,
            efficiency: 100,
            serfs: 1000,
        };
        let mut m = mind(Temper::Bold, Kingdoms::Spain, seen);
        let o = m.campaign(game.kingdom(id), &enemies(&game, id));
        assert_eq!(o.aimed, Some((Kingdoms::Spain, 60)));
    }

    #[test]
    fn an_unfavourable_report_closes_the_eye_and_opens_another() {
        let game = table();
        let id = Kingdoms::France;
        let seen = Seen {
            garrison: 60,
            efficiency: 100,
            serfs: 1000,
        };
        for _ in 0..50 {
            let mut m = mind(Temper::Cautious, Kingdoms::Spain, seen);
            let o = m.campaign(game.kingdom(id), &enemies(&game, id));
            assert_eq!(o.aimed, None);
            assert!(m.eye.is_some_and(|e| e != id));
            assert_eq!(o.scout, m.eye);
        }
    }

    #[test]
    fn a_favourable_report_kept_unstruck_keeps_the_eye() {
        let game = table();
        let id = Kingdoms::France;
        let seen = Seen {
            garrison: 10,
            efficiency: 100,
            serfs: 1000,
        };
        let mut kept = 0;
        for _ in 0..200 {
            let mut m = mind(Temper::Cautious, Kingdoms::Spain, seen);
            let o = m.campaign(game.kingdom(id), &enemies(&game, id));
            assert_eq!(m.eye, Some(Kingdoms::Spain));
            if o.aimed.is_none() {
                kept += 1;
            }
        }
        // Two in three strike: the eye is kept without a blow about a third
        // of the years.
        assert!((30..110).contains(&kept), "{kept}");
    }

    #[test]
    fn no_money_no_eyes() {
        let mut game = table();
        let id = Kingdoms::France;
        game.kingdom_mut(id).treasury = SCOUT_PRICE - 1;
        let mut m = Mind {
            temper: Temper::Measured,
            eye: None,
            seen: None,
        };
        let o = m.campaign(game.kingdom(id), &enemies(&game, id));
        assert_eq!(o.scout, None);
        assert!(m.eye.is_some());
    }

    #[test]
    fn the_coup_de_sang_falls_one_year_in_three_for_a_bold_council() {
        let game = table();
        let id = Kingdoms::France;
        let mut blind = 0;
        for _ in 0..600 {
            let mut m = Mind {
                temper: Temper::Bold,
                eye: None,
                seen: None,
            };
            let o = m.campaign(game.kingdom(id), &enemies(&game, id));
            if let Some((t, men)) = o.blind {
                assert_ne!(t, id);
                assert!((20..=60).contains(&men));
                blind += 1;
            }
        }
        assert!((150..250).contains(&blind), "{blind}");
    }

    #[test]
    fn the_eye_follows_the_land() {
        let mut game = table();
        let id = Kingdoms::France;
        for k in &mut game.kingdoms {
            k.surface = 1;
        }
        game.kingdom_mut(Kingdoms::Spain).surface = 100_000;
        let draws = (0..100)
            .filter(|_| draw_by_land(&enemies(&game, id)) == Kingdoms::Spain)
            .count();
        assert!(draws > 95, "{draws}");
    }

    #[test]
    fn a_dead_eye_is_closed_and_the_last_realm_alone_has_none() {
        let mut game = table();
        let id = Kingdoms::France;
        let seen = Seen {
            garrison: 1,
            efficiency: 100,
            serfs: 1000,
        };
        game.kingdom_mut(Kingdoms::Spain).is_dead = true;
        let living: Vec<&Kingdom> = game
            .kingdoms
            .iter()
            .filter(|k| k.id != id && !k.is_dead)
            .collect();
        let mut m = mind(Temper::Bold, Kingdoms::Spain, seen);
        let o = m.campaign(game.kingdom(id), &living);
        assert_eq!(o.aimed, None);
        assert!(m.eye.is_some_and(|e| e != Kingdoms::Spain));

        let mut m = mind(Temper::Bold, Kingdoms::Spain, seen);
        let o = m.campaign(game.kingdom(id), &[]);
        assert_eq!(o, Orders::default());
        assert_eq!(m.eye, None);
    }
}
