//! Renseignement: what a seat learns of the other realms — the rumours
//! everyone hears after a campaign, what an éclaireur brings back from a
//! ride, what an agent bought among a court's officials writes each year.

use crate::kingdom::{Kingdom, Kingdoms, PlayerTitle};
use crate::random::random;

/// What an éclaireur costs: about twenty men of arms.
pub const SCOUT_PRICE: i32 = 150;
/// One chance in this of the éclaireur being taken.
pub const SCOUT_CAUGHT: i32 = 6;
/// What an agent costs each year: about fifty men of arms.
pub const AGENT_PRICE: i32 = 400;
/// One chance in this, each year, of an agent being unmasked.
pub const AGENT_UNMASKED: i32 = 8;

/// A spy ordered at the Extérieur, resolved as the next year's Extérieur
/// opens: an éclaireur rides to a realm and comes back; an agent is bought
/// among its officials and stays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mission {
    Scout(Kingdoms),
    Agent(Kingdoms),
}

impl Mission {
    pub fn at(self) -> Kingdoms {
        match self {
            Mission::Scout(on) | Mission::Agent(on) => on,
        }
    }

    pub fn price(self) -> i32 {
        match self {
            Mission::Scout(_) => SCOUT_PRICE,
            Mission::Agent(_) => AGENT_PRICE,
        }
    }
}

/// What everyone hears of a front of the last campaign: who marched on whom
/// and how it went — never how many men.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rumour {
    pub year: i32,
    pub attacker: Kingdoms,
    /// `None` = the barbarians.
    pub target: Option<Kingdoms>,
    pub victory: bool,
    /// Arpents taken (the whole realm on an annexation).
    pub arpents: i32,
    pub annexed: bool,
}

/// The rumours about one realm, as a table sums them up: who it marched on,
/// who marched on it, who beat it, what ground it lost to whom.
#[derive(Debug, Clone, Copy, Default)]
pub struct Heard {
    pub marched_on: [bool; 6],
    pub marched_by: [bool; 6],
    pub beaten_by: [bool; 6],
    pub lost_to: [i32; 6],
}

/// What the rumours say of each realm, by realm index.
pub fn heard(rumours: &[Rumour]) -> [Heard; 6] {
    let mut h = [Heard::default(); 6];
    for r in rumours {
        let Some(t) = r.target else { continue };
        let (a, t) = (r.attacker.index(), t.index());
        h[a].marched_on[t] = true;
        h[t].marched_by[a] = true;
        h[t].beaten_by[a] |= r.victory;
        h[t].lost_to[a] += r.arpents;
    }
    h
}

/// What an éclaireur saw of a realm — the figures a war is fought with —
/// dated with the year it was read, as the Extérieur opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Report {
    pub year: i32,
    pub garrison: i32,
    pub efficiency: i32,
    pub nobles: i32,
    pub merchants: i32,
    pub serfs: i32,
    pub surface: i32,
}

impl Report {
    pub fn read(k: &Kingdom, year: i32) -> Self {
        Report {
            year,
            garrison: k.soldiers,
            efficiency: k.soldiers_efficiency,
            nobles: k.nobles,
            merchants: k.merchants,
            serfs: k.peasants,
            surface: k.surface,
        }
    }

    pub fn subjects(&self) -> i32 {
        self.nobles + self.merchants + self.serfs
    }

    /// How wide of the truth a forecast built on this report may be, in
    /// per cent, by its age in `year`; `None` once too old to compute on.
    pub fn spread(&self, year: i32) -> Option<i32> {
        match year - self.year {
            0 => Some(0),
            1 => Some(20),
            2 => Some(40),
            _ => None,
        }
    }

    /// The realm as the report tells it, every figure `jitter` per cent off.
    pub fn kingdom(&self, id: Kingdoms, jitter: i32) -> Kingdom {
        let off = |n: i32| (i64::from(n) * i64::from(100 + jitter) / 100) as i32;
        let mut k = Kingdom::new(id);
        k.soldiers = off(self.garrison);
        k.soldiers_efficiency = off(self.efficiency).max(1);
        k.nobles = off(self.nobles);
        k.merchants = off(self.merchants);
        k.peasants = off(self.serfs);
        k.surface = self.surface;
        k
    }
}

/// What an agent reads in a realm's registers — what a scout on the roads
/// never sees — dated like a report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ledger {
    pub year: i32,
    pub treasury: i32,
    pub grain_stocks: i32,
    /// Palace completion in tenths.
    pub palaces: i32,
    pub marketplaces: i32,
    pub grain_mills: i32,
    pub foundries: i32,
    pub shipyards: i32,
    /// Bushels bought this year from each realm, by kingdom index.
    pub bought: [i32; 6],
    /// The rank the court works towards, with the criteria met out of all;
    /// `None` for an emperor.
    pub aim: Option<(PlayerTitle, i32, i32)>,
}

impl Ledger {
    pub fn read(k: &Kingdom, year: i32, bought: [i32; 6]) -> Self {
        let aim = k.title().next().map(|t| {
            let all = k.progress(t);
            let met = all.iter().filter(|c| c.met()).count() as i32;
            (t, met, all.len() as i32)
        });
        Ledger {
            year,
            treasury: k.treasury,
            grain_stocks: k.grain_stocks,
            palaces: k.palaces,
            marketplaces: k.marketplaces,
            grain_mills: k.grain_mills,
            foundries: k.foundries,
            shipyards: k.shipyards,
            bought,
            aim,
        }
    }
}

/// How an éclaireur's year ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scouting {
    /// Home with his report (its garrison, for the Chronique).
    Back {
        garrison: i32,
    },
    Caught,
    /// The realm was annexed while he rode: no court left to look at.
    Gone,
}

/// How an agent's year ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Writing {
    /// His yearly letter: the registers, and what moved since the last
    /// report — men of arms levied, francs spent (`None` without a
    /// previous report or ledger to compare with).
    Letter {
        ledger: Ledger,
        levied: Option<i32>,
        spent: Option<i32>,
    },
    Unmasked,
    /// The treasury could not pay him: he kept quiet and left.
    Unpaid,
    /// The realm was annexed: no registers left to read.
    Gone,
}

/// What a seat knows of a foreign realm.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dossier {
    /// The last report brought back, by an éclaireur or an agent.
    pub report: Option<Report>,
    /// The registers as the agent last read them.
    pub ledger: Option<Ledger>,
    /// An agent is in place, paid each year as long as he is kept.
    pub agent: bool,
    /// Year a spy was last taken there: that seigneur knows they are
    /// watched.
    pub caught: Option<i32>,
}

impl Dossier {
    /// The éclaireur sent to `on` comes back as the Extérieur opens: he
    /// reads the realm as it stands, or is taken (one in six).
    pub fn scout(&mut self, on: &Kingdom, year: i32) -> Scouting {
        if on.is_dead {
            Scouting::Gone
        } else if random(0, SCOUT_CAUGHT) == 0 {
            self.caught = Some(year);
            Scouting::Caught
        } else {
            let r = Report::read(on, year);
            self.report = Some(r);
            Scouting::Back {
                garrison: r.garrison,
            }
        }
    }

    /// The agent's year in `on`: gone with the realm, unmasked (one in
    /// eight), unpaid, or — a `standing` agent paid his year out of
    /// `treasury`, a new one paid on purchase — writing his letter. He is
    /// off the books on anything but a letter.
    pub fn agent_year(
        &mut self,
        on: &Kingdom,
        year: i32,
        bought: [i32; 6],
        treasury: &mut i32,
        standing: bool,
    ) -> Writing {
        let outcome = if on.is_dead {
            Writing::Gone
        } else if random(0, AGENT_UNMASKED) == 0 {
            self.caught = Some(year);
            Writing::Unmasked
        } else if standing && *treasury < AGENT_PRICE {
            Writing::Unpaid
        } else {
            if standing {
                *treasury -= AGENT_PRICE;
            }
            let report = Report::read(on, year);
            let ledger = Ledger::read(on, year, bought);
            let levied = self.report.map(|p| report.garrison - p.garrison);
            let spent = self.ledger.map(|p| p.treasury - ledger.treasury);
            self.report = Some(report);
            self.ledger = Some(ledger);
            Writing::Letter {
                ledger,
                levied,
                spent,
            }
        };
        self.agent = matches!(outcome, Writing::Letter { .. });
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_spreads_with_age_and_tells_a_realm() {
        let mut k = Kingdom::new(Kingdoms::Persia);
        k.soldiers = 1000;
        let r = Report::read(&k, 10);
        assert_eq!(r.spread(10), Some(0));
        assert_eq!(r.spread(11), Some(20));
        assert_eq!(r.spread(12), Some(40));
        assert_eq!(r.spread(13), None);
        assert_eq!(r.subjects(), k.population());
        let told = r.kingdom(Kingdoms::Persia, 20);
        assert_eq!(told.soldiers, 1200);
        assert_eq!(told.surface, k.surface);
    }

    #[test]
    fn the_rumours_sum_up_per_realm() {
        let rumours = [
            Rumour {
                year: 4,
                attacker: Kingdoms::France,
                target: Some(Kingdoms::Persia),
                victory: true,
                arpents: 300,
                annexed: false,
            },
            Rumour {
                year: 4,
                attacker: Kingdoms::France,
                target: None,
                victory: false,
                arpents: 0,
                annexed: false,
            },
        ];
        let h = heard(&rumours);
        let (f, p) = (Kingdoms::France.index(), Kingdoms::Persia.index());
        assert!(h[f].marched_on[p]);
        assert!(h[p].marched_by[f] && h[p].beaten_by[f]);
        assert_eq!(h[p].lost_to[f], 300);
        assert!(!h[p].marched_on.iter().any(|&b| b));
    }

    #[test]
    fn a_scout_reads_a_living_realm_or_is_taken() {
        let on = Kingdom::new(Kingdoms::Persia);
        let mut d = Dossier::default();
        let mut back = 0;
        for _ in 0..200 {
            match d.scout(&on, 3) {
                Scouting::Back { garrison } => {
                    assert_eq!(garrison, on.soldiers);
                    assert_eq!(d.report.map(|r| r.year), Some(3));
                    back += 1;
                }
                Scouting::Caught => assert_eq!(d.caught, Some(3)),
                Scouting::Gone => unreachable!(),
            }
        }
        assert!(back > 100 && back < 200);
        let mut dead = Kingdom::new(Kingdoms::Persia);
        dead.is_dead = true;
        assert_eq!(Dossier::default().scout(&dead, 3), Scouting::Gone);
    }

    #[test]
    fn an_agent_writes_while_paid_and_leaves_otherwise() {
        let on = Kingdom::new(Kingdoms::Persia);
        let mut d = Dossier {
            agent: true,
            ..Default::default()
        };
        let mut treasury = AGENT_PRICE - 1;
        assert!(matches!(
            d.agent_year(&on, 2, [0; 6], &mut treasury, true),
            Writing::Unpaid | Writing::Unmasked
        ));
        assert!(!d.agent);
        assert_eq!(treasury, AGENT_PRICE - 1);
        let mut letters = 0;
        for year in 2..200 {
            d.agent = true;
            let mut treasury = 10_000;
            let before = treasury;
            match d.agent_year(&on, year, [0; 6], &mut treasury, true) {
                Writing::Letter {
                    ledger,
                    levied,
                    spent,
                } => {
                    assert_eq!(treasury, before - AGENT_PRICE);
                    assert_eq!(ledger.year, year);
                    assert!(levied.is_none_or(|l| l == 0));
                    assert!(spent.is_none_or(|s| s == 0));
                    assert!(d.agent);
                    letters += 1;
                }
                Writing::Unmasked => {
                    assert_eq!(treasury, before);
                    assert_eq!(d.caught, Some(year));
                    assert!(!d.agent);
                }
                other => unreachable!("{other:?}"),
            }
        }
        assert!(letters > 100);
    }
}
