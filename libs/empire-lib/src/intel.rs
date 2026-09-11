//! Renseignement: what a seat learns of the other realms — the rumours
//! everyone hears after a campaign, what an éclaireur brings back from a
//! ride, what an agent reads in a court's registers. Both missions are sent
//! at the Extérieur and answer at once; either may be taken, one time in
//! six, and the court it was sent to then knows it is watched.

use crate::kingdom::{Kingdom, Kingdoms, PlayerTitle};
use crate::random::random;

/// What sending an éclaireur costs: about twenty men of arms — over the
/// éclaireur himself, bought at the Intendance.
pub const SCOUT_PRICE: i32 = 150;
/// What an agent costs: about fifty men of arms.
pub const AGENT_PRICE: i32 = 400;
/// One chance in this of a mission being taken.
pub const CAUGHT: i32 = 6;

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

/// What an éclaireur sees of a realm from its roads and drill grounds —
/// the figures a war is fought with — dated with the year he rode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Report {
    pub year: i32,
    pub surface: i32,
    pub garrison: i32,
    /// Tenths of fortification.
    pub fortifications: i32,
    pub efficiency: i32,
}

impl Report {
    pub fn read(k: &Kingdom, year: i32) -> Self {
        Report {
            year,
            surface: k.surface,
            garrison: k.soldiers,
            fortifications: k.fortifications,
            efficiency: k.soldiers_efficiency,
        }
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

    /// The realm as the report tells it, every figure `jitter` per cent
    /// off; the people it does not count are guessed at the starting
    /// density, a serf to five arpents.
    pub fn kingdom(&self, id: Kingdoms, jitter: i32) -> Kingdom {
        let off = |n: i32| (i64::from(n) * i64::from(100 + jitter) / 100) as i32;
        let mut k = Kingdom::new(id);
        k.soldiers = off(self.garrison);
        k.soldiers_efficiency = off(self.efficiency).max(1);
        k.fortifications = self.fortifications;
        k.surface = self.surface;
        k.peasants = off(self.surface / 5);
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
    pub nobles: i32,
    pub merchants: i32,
    pub serfs: i32,
    /// Palace completion in tenths.
    pub palaces: i32,
    pub marketplaces: i32,
    pub grain_mills: i32,
    pub foundries: i32,
    pub shipyards: i32,
    /// Hospice in tenths.
    pub hospices: i32,
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
            nobles: k.nobles,
            merchants: k.merchants,
            serfs: k.peasants,
            palaces: k.palaces,
            marketplaces: k.marketplaces,
            grain_mills: k.grain_mills,
            foundries: k.foundries,
            shipyards: k.shipyards,
            hospices: k.hospices,
            bought,
            aim,
        }
    }

    pub fn subjects(&self) -> i32 {
        self.nobles + self.merchants + self.serfs
    }
}

/// How an éclaireur's ride ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scouting {
    /// Home with his report.
    Back(Report),
    /// Taken: the éclaireur and the francs are lost, the court knows.
    Caught,
}

/// How an agent's errand ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Writing {
    /// The registers, read.
    Read(Ledger),
    /// Unmasked: the francs are lost, the court knows.
    Caught,
}

/// What a seat knows of a foreign realm.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dossier {
    /// The last report an éclaireur brought back.
    pub report: Option<Report>,
    /// The registers as an agent last read them.
    pub ledger: Option<Ledger>,
    /// Year an éclaireur was last sent there: one a year.
    pub scouted: Option<i32>,
    /// Year an agent was last sent there: one a year.
    pub read: Option<i32>,
    /// Year a spy was last taken there: that seigneur knows they are
    /// watched.
    pub caught: Option<i32>,
}

impl Dossier {
    /// No éclaireur has ridden there this `year`.
    pub fn can_scout(&self, year: i32) -> bool {
        self.scouted != Some(year)
    }

    /// No agent has been sent there this `year`.
    pub fn can_read(&self, year: i32) -> bool {
        self.read != Some(year)
    }

    /// The éclaireur sent to `on` rides and answers at once: he reads the
    /// realm as it stands, or is taken (one in six). The caller pays the
    /// mission and the man.
    pub fn scout(&mut self, on: &Kingdom, year: i32) -> Scouting {
        self.scouted = Some(year);
        if random(0, CAUGHT) == 0 {
            self.caught = Some(year);
            Scouting::Caught
        } else {
            let r = Report::read(on, year);
            self.report = Some(r);
            Scouting::Back(r)
        }
    }

    /// The agent sent to `on` reads its registers at once, or is unmasked
    /// (one in six). The caller pays.
    pub fn agent(&mut self, on: &Kingdom, year: i32, bought: [i32; 6]) -> Writing {
        self.read = Some(year);
        if random(0, CAUGHT) == 0 {
            self.caught = Some(year);
            Writing::Caught
        } else {
            let l = Ledger::read(on, year, bought);
            self.ledger = Some(l);
            Writing::Read(l)
        }
    }

    /// The realm as the dossier tells it, the report's figures `jitter` per
    /// cent off and the registers, if read, laid over its guesses; `None`
    /// without a report.
    pub fn kingdom(&self, id: Kingdoms, jitter: i32) -> Option<Kingdom> {
        let r = self.report?;
        let mut k = r.kingdom(id, jitter);
        if let Some(l) = self.ledger {
            k.nobles = l.nobles;
            k.merchants = l.merchants;
            k.peasants = l.serfs;
            k.treasury = l.treasury;
            k.grain_stocks = l.grain_stocks;
            k.palaces = l.palaces;
            k.marketplaces = l.marketplaces;
            k.grain_mills = l.grain_mills;
            k.foundries = l.foundries;
            k.shipyards = l.shipyards;
            k.hospices = l.hospices;
        }
        Some(k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_spreads_with_age_and_tells_a_realm() {
        let mut k = Kingdom::new(Kingdoms::Persia);
        k.soldiers = 1000;
        k.fortifications = 4;
        let r = Report::read(&k, 10);
        assert_eq!(r.spread(10), Some(0));
        assert_eq!(r.spread(11), Some(20));
        assert_eq!(r.spread(12), Some(40));
        assert_eq!(r.spread(13), None);
        let told = r.kingdom(Kingdoms::Persia, 20);
        assert_eq!(told.soldiers, 1200);
        assert_eq!(told.fortifications, 4);
        assert_eq!(told.surface, k.surface);
        assert_eq!(told.peasants, 2400);
        // The registers, once read, replace the guesses.
        let d = Dossier {
            report: Some(r),
            ledger: Some(Ledger::read(&k, 10, [0; 6])),
            ..Dossier::default()
        };
        let told = d.kingdom(Kingdoms::Persia, 20).unwrap();
        assert_eq!(told.peasants, 2000);
        assert_eq!(told.treasury, 1000);
        assert!(Dossier::default().kingdom(Kingdoms::Persia, 0).is_none());
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
    fn a_scout_reads_the_realm_or_is_taken() {
        let on = Kingdom::new(Kingdoms::Persia);
        let mut d = Dossier::default();
        assert!(d.can_scout(3));
        let mut back = 0;
        for _ in 0..200 {
            match d.scout(&on, 3) {
                Scouting::Back(r) => {
                    assert_eq!(r.garrison, on.soldiers);
                    assert_eq!(d.report, Some(r));
                    back += 1;
                }
                Scouting::Caught => assert_eq!(d.caught, Some(3)),
            }
            assert!(!d.can_scout(3) && d.can_scout(4));
        }
        assert!(back > 100 && back < 200, "{back}");
    }

    #[test]
    fn an_agent_reads_the_registers_or_is_unmasked() {
        let mut on = Kingdom::new(Kingdoms::Persia);
        on.palaces = 3;
        let mut d = Dossier::default();
        let mut read = 0;
        for year in 2..202 {
            assert!(d.can_read(year));
            match d.agent(&on, year, [0, 5, 0, 0, 0, 0]) {
                Writing::Read(l) => {
                    assert_eq!(l.year, year);
                    assert_eq!(l.palaces, 3);
                    assert_eq!(l.bought[1], 5);
                    assert_eq!(l.subjects(), on.population());
                    assert_eq!(d.ledger, Some(l));
                    read += 1;
                }
                Writing::Caught => assert_eq!(d.caught, Some(year)),
            }
            assert!(!d.can_read(year));
        }
        assert!(read > 100 && read < 200, "{read}");
    }
}
