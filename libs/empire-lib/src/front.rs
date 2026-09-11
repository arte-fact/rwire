//! A front: one defending realm and every army marching on it this year. The
//! garrison defends the land: nothing is taken while it stands, and every army
//! hits it together. Once it falls the survivors have their victory and march
//! on, each on its own line of arpents, taking whatever lies on the ground it
//! crosses — the people met on the way rally one time in three and fight one
//! man of arms otherwise: serfs and merchants as a militia, the nobles with
//! the realm's ardour. Of the grain and gold lying there the army carries off
//! a third, a third burns or is scattered, and a third the fleeing people
//! save. The realm is laid out from the frontier (arpent 1) to the capital
//! (arpent N) and split equally between the lines. A man can hold no more
//! than [`ARPENTS_PER_MAN`]: an army marches while the men it has left can
//! hold more ground than it has crossed, then the survivors turn home with
//! the spoils — the take of a raid is bounded by its size, and a garrison the
//! size of the raid stops it at the gate.

use std::cmp::max;

use crate::game::EmpireGame;
use crate::kingdom::{Fate, Kingdom, Kingdoms};
use crate::random::random;

/// How serfs and merchants fight when the army meets them on its march: a
/// militia, whatever the realm's soldiers are worth (the peasants defending in
/// [`crate::war`] fight at the same). It sets the ground a march takes per man
/// lost: 1 / (people per arpent × ⅔ × P(a duel lost)), about 30 arpents at
/// the start of a game — what the original battle yields per man lost, once
/// the men fallen before the garrison are counted.
pub const MILITIA_EFFICIENCY: i32 = 50;

/// What a garrison fights with: its soldiers' efficiency and a half, for
/// the ground it knows, and a tenth more for every tenth of fortification
/// (`walls`) — twice the efficiency and a half behind full walls. The
/// militia and the nobles met on the march fight bare.
pub fn garrison_strength(soldiers_efficiency: i32, walls: i32) -> i32 {
    soldiers_efficiency * 3 * (TENTHS + walls.clamp(0, TENTHS)) / 20
}

/// Fortifications are counted in tenths, up to this.
pub const TENTHS: i32 = crate::investments::TENTHS;

/// Every this many exchanges of blows before the garrison, each ram still
/// standing knocks a tenth off the walls.
pub const RAM_PACE: i32 = 8;

/// Men of the army standing by each ram for it to be safe: below, every
/// missing man is a chance in ten of losing the ram at its blow.
pub const RAM_ESCORT: i32 = 10;

/// The ground one man of arms can hold: an army's advance never exceeds this
/// many arpents per man still standing. Annexing a realm of 10 000 arpents
/// takes 500 men alive at its capital.
pub const ARPENTS_PER_MAN: i32 = 20;

/// The buildings a realm has along its line, in the order of the spoils arrays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingKind {
    Mill,
    Foundry,
    Marketplace,
    Shipyard,
}

impl BuildingKind {
    pub const ALL: [BuildingKind; 4] = [
        BuildingKind::Mill,
        BuildingKind::Foundry,
        BuildingKind::Marketplace,
        BuildingKind::Shipyard,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    fn count(self, k: &Kingdom) -> i32 {
        match self {
            BuildingKind::Mill => k.grain_mills,
            BuildingKind::Foundry => k.foundries,
            BuildingKind::Marketplace => k.marketplaces,
            BuildingKind::Shipyard => k.shipyards,
        }
    }

    fn count_mut(self, k: &mut Kingdom) -> &mut i32 {
        match self {
            BuildingKind::Mill => &mut k.grain_mills,
            BuildingKind::Foundry => &mut k.foundries,
            BuildingKind::Marketplace => &mut k.marketplaces,
            BuildingKind::Shipyard => &mut k.shipyards,
        }
    }
}

/// A building standing at its arpent of a line; crossed, it is burned — or
/// taken, one time in three.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Building {
    pub kind: BuildingKind,
    /// 1 = the frontier end of the line.
    pub at: i32,
    pub burned: bool,
}

/// Those living on a line, or met on it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct People {
    pub peasants: i32,
    pub merchants: i32,
    pub nobles: i32,
}

impl People {
    pub fn add(&mut self, o: &People) {
        self.peasants += o.peasants;
        self.merchants += o.merchants;
        self.nobles += o.nobles;
    }

    pub fn total(&self) -> i32 {
        self.peasants + self.merchants + self.nobles
    }
}

/// One army's line of arpents: its equal share of the realm, with everything
/// that stands on it. Serfs, grain and merchants are spread evenly; the
/// treasury and the nobles thicken toward the capital.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Line {
    pub arpents: i32,
    pub peasants: i32,
    pub grain: i32,
    pub merchants: i32,
    pub treasury: i32,
    pub nobles: i32,
    /// Sorted by arpent.
    pub buildings: Vec<Building>,
}

/// What an army carries off — or leaves in ashes — of the ground it crossed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Spoils {
    pub arpents: i32,
    /// Carried off: a third of what lay on the ground.
    pub grain: i32,
    pub treasury: i32,
    /// Burned or scattered on the way: lost to both sides, another third.
    pub grain_lost: i32,
    pub treasury_lost: i32,
    /// Met on the way and rallied: they change sides.
    pub rallied: People,
    /// Met on the way, fought and put to the sword.
    pub killed: People,
    /// Per [`BuildingKind`].
    pub taken: [i32; 4],
    pub burned: [i32; 4],
}

impl Spoils {
    pub fn add(&mut self, o: &Spoils) {
        self.arpents += o.arpents;
        self.grain += o.grain;
        self.treasury += o.treasury;
        self.grain_lost += o.grain_lost;
        self.treasury_lost += o.treasury_lost;
        self.rallied.add(&o.rallied);
        self.killed.add(&o.killed);
        for (t, o) in self.taken.iter_mut().zip(&o.taken) {
            *t += o;
        }
        for (b, o) in self.burned.iter_mut().zip(&o.burned) {
            *b += o;
        }
    }

    pub fn buildings_taken(&self) -> i32 {
        self.taken.iter().sum()
    }

    pub fn buildings_burned(&self) -> i32 {
        self.burned.iter().sum()
    }
}

impl Line {
    fn even(&self, total: i32, x: i32) -> i32 {
        (total as i64 * x as i64 / self.arpents.max(1) as i64) as i32
    }

    fn deep(&self, total: i32, x: i32) -> i32 {
        let n = self.arpents.max(1) as i64;
        (total as i64 * x as i64 * x as i64 / (n * n)) as i32
    }

    /// The ground and goods on the first `advance` arpents of the line (the
    /// people met there are the army's affair): a third of the goods taken, a
    /// third lost, the last third saved by those who fled.
    pub fn ground(&self, advance: i32) -> Spoils {
        let x = advance.clamp(0, self.arpents);
        let grain = self.even(self.grain, x);
        let treasury = self.deep(self.treasury, x);
        let mut s = Spoils {
            arpents: x,
            grain: grain / 3,
            treasury: treasury / 3,
            grain_lost: grain / 3,
            treasury_lost: treasury / 3,
            ..Spoils::default()
        };
        for b in self.buildings.iter().take_while(|b| b.at <= x) {
            if b.burned {
                s.burned[b.kind.index()] += 1;
            } else {
                s.taken[b.kind.index()] += 1;
            }
        }
        s
    }

    /// Those living on the first `advance` arpents of the line.
    pub fn people(&self, advance: i32) -> People {
        let x = advance.clamp(0, self.arpents);
        People {
            peasants: self.even(self.peasants, x),
            merchants: self.even(self.merchants, x),
            nobles: self.deep(self.nobles, x),
        }
    }
}

/// `total` split into `n` equal shares, the remainder going to the first ones.
fn shares(total: i32, n: usize) -> impl Iterator<Item = i32> {
    let n = n.max(1) as i32;
    (0..n).map(move |i| total / n + i32::from(i < total % n))
}

/// The realm laid out on `n` lines.
fn lay_out(k: &Kingdom, n: usize) -> Vec<Line> {
    let mut lines: Vec<Line> = shares(k.surface, n)
        .zip(shares(k.peasants.max(0), n))
        .zip(shares(k.grain_stocks, n))
        .zip(shares(k.merchants, n))
        .zip(shares(k.treasury, n))
        .zip(shares(k.nobles, n))
        .map(
            |(((((arpents, peasants), grain), merchants), treasury), nobles)| Line {
                arpents,
                peasants,
                grain,
                merchants,
                treasury,
                nobles,
                buildings: Vec::new(),
            },
        )
        .collect();
    for kind in BuildingKind::ALL {
        for (line, count) in lines.iter_mut().zip(shares(kind.count(k), n)) {
            let len = line.arpents as i64;
            for j in 0..count as i64 {
                line.buildings.push(Building {
                    kind,
                    at: ((len * (2 * j + 1)) / (2 * count as i64)).max(1) as i32,
                    burned: random(0, 3) != 0,
                });
            }
        }
    }
    for line in &mut lines {
        line.buildings.sort_by_key(|b| b.at);
    }
    lines
}

/// An army as it sets out: who sends it, its men and the rams it brings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Host {
    pub attacker: Kingdoms,
    pub men: i32,
    pub rams: i32,
}

impl Host {
    pub fn new(attacker: Kingdoms, men: i32) -> Self {
        Host {
            attacker,
            men,
            rams: 0,
        }
    }

    pub fn with_rams(self, rams: i32) -> Self {
        Host { rams, ..self }
    }
}

/// One army of the front, as it fought.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Army {
    pub attacker: Kingdoms,
    pub sent: i32,
    /// Men still standing at the end.
    pub men: i32,
    /// Rams brought along.
    pub rams: i32,
    /// Rams broken at their blows.
    pub rams_broken: i32,
    /// Alive when the garrison fell: whatever happens next, the day is won.
    pub victory: bool,
    /// Arpents of its line crossed.
    pub advance: i32,
    pub rallied: People,
    pub killed: People,
    pub line: Line,
}

impl Army {
    pub fn spoils(&self) -> Spoils {
        Spoils {
            rallied: self.rallied,
            killed: self.killed,
            ..self.line.ground(self.advance)
        }
    }

    pub fn lost(&self) -> i32 {
        self.sent - self.men
    }

    /// Rams coming home: those not broken, if anyone is left to bring them.
    pub fn rams_home(&self) -> i32 {
        if self.men > 0 {
            self.rams - self.rams_broken
        } else {
            0
        }
    }

    /// Rams lost, broken or left with the dead.
    pub fn rams_lost(&self) -> i32 {
        self.rams - self.rams_home()
    }
}

/// One army as it stands after an exchange of blows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stand {
    pub men: i32,
    /// Arpents crossed.
    pub advance: i32,
    pub rallied: People,
    pub killed: People,
    /// Rams still standing.
    pub rams: i32,
    /// Rams broken so far.
    pub rams_broken: i32,
}

/// The state of the front after one exchange of blows (or one round of ram
/// blows, when [`RAM_PACE`] exchanges are up).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Round {
    pub garrison: i32,
    /// Tenths of fortification still standing.
    pub walls: i32,
    /// Exchanges of blows fought before the garrison so far.
    pub exchange: i32,
    pub armies: Vec<Stand>,
}

impl Round {
    /// Exchanges left before the rams strike again.
    pub fn blow_in(&self) -> i32 {
        RAM_PACE - self.exchange % RAM_PACE
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontResult {
    pub armies: Vec<Army>,
    /// The first round is the field before the first blow.
    pub rounds: Vec<Round>,
    pub garrison_start: i32,
    pub garrison_left: i32,
    /// Tenths of fortification before and after the rams.
    pub walls_start: i32,
    pub walls_left: i32,
    /// Every line crossed to its end: the army (by index) that takes what
    /// remains of the realm.
    pub annexed_by: Option<usize>,
}

impl FrontResult {
    pub fn garrison_fell(&self) -> bool {
        self.garrison_left == 0
    }

    pub fn garrison_fallen(&self) -> i32 {
        self.garrison_start - self.garrison_left
    }

    /// Tenths of fortification the rams knocked down, for good.
    pub fn walls_fallen(&self) -> i32 {
        self.walls_start - self.walls_left
    }

    /// Everything taken or burned, all armies together.
    pub fn spoils(&self) -> Spoils {
        let mut s = Spoils::default();
        for a in &self.armies {
            s.add(&a.spoils());
        }
        s
    }
}

/// Who an army meets crossing a stretch of its line: how many of each,
/// duelled frontier end first — peasants, then merchants, then nobles.
fn met(line: &Line, from: i32, to: i32) -> People {
    let a = line.people(from);
    let b = line.people(to);
    People {
        peasants: b.peasants - a.peasants,
        merchants: b.merchants - a.merchants,
        nobles: b.nobles - a.nobles,
    }
}

#[derive(Clone, Copy)]
enum Met {
    Peasant,
    Merchant,
    Noble,
}

impl Met {
    /// What this one fights with.
    fn ardour(self, d: &Kingdom) -> i32 {
        match self {
            Met::Peasant | Met::Merchant => MILITIA_EFFICIENCY,
            Met::Noble => d.soldiers_efficiency,
        }
    }

    fn count(self, p: &mut People) {
        match self {
            Met::Peasant => p.peasants += 1,
            Met::Merchant => p.merchants += 1,
            Met::Noble => p.nobles += 1,
        }
    }
}

/// Fight the front without touching the game; apply it with [`apply_front`].
/// The hosts fight in the order given within a round.
pub fn simulate_front(game: &EmpireGame, defender: Kingdoms, hosts: &[Host]) -> FrontResult {
    fight(game, defender, hosts, true)
}

/// The outcome of one army sent alone against `defender`, fought once per
/// draw: what it would take and lose, as bands from the first to the ninth
/// decile, and how often the garrison fell.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Forecast {
    pub arpents: (i32, i32),
    pub lost: (i32, i32),
    pub victories: i32,
    /// How often the whole realm fell to the army (never, for the bands).
    pub annexations: i32,
    /// Tenths of fortification left standing.
    pub walls: (i32, i32),
    /// Rams lost, broken or left with the dead.
    pub rams_lost: (i32, i32),
}

/// Deciles of `v` (sorted in place): the first and the ninth.
pub(crate) fn deciles(v: &mut [i32]) -> (i32, i32) {
    if v.is_empty() {
        return (0, 0);
    }
    v.sort_unstable();
    let last = v.len() - 1;
    (v[last / 10], v[last - last / 10])
}

/// One draw per game given: the realm may differ from one to the next when
/// it is only known through a report.
pub fn forecast_front<'a>(
    games: impl IntoIterator<Item = &'a EmpireGame>,
    defender: Kingdoms,
    host: Host,
) -> Forecast {
    let mut arpents = Vec::new();
    let mut lost = Vec::new();
    let mut walls = Vec::new();
    let mut rams_lost = Vec::new();
    let mut victories = 0;
    let mut annexations = 0;
    for game in games {
        let r = fight(game, defender, &[host], false);
        let a = &r.armies[0];
        arpents.push(a.advance);
        lost.push(a.lost());
        walls.push(r.walls_left);
        rams_lost.push(a.rams_lost());
        victories += i32::from(a.victory);
        annexations += i32::from(r.annexed_by.is_some());
    }
    Forecast {
        arpents: deciles(&mut arpents),
        lost: deciles(&mut lost),
        victories,
        annexations,
        walls: deciles(&mut walls),
        rams_lost: deciles(&mut rams_lost),
    }
}

/// The fight itself; `record` keeps every round for the replay.
pub(crate) fn fight(
    game: &EmpireGame,
    defender: Kingdoms,
    hosts: &[Host],
    record: bool,
) -> FrontResult {
    let d = game.kingdom(defender);
    let n = hosts.len();
    let lines = lay_out(d, n);
    let strength: Vec<i32> = hosts
        .iter()
        .map(|h| game.kingdom(h.attacker).soldiers_efficiency)
        .collect();
    // Original: I7=INT(I1/15)+1 — the men that fall to one blow.
    let units: Vec<i32> = hosts.iter().map(|h| h.men / 15 + 1).collect();
    let mut stands: Vec<Stand> = hosts
        .iter()
        .map(|h| Stand {
            men: h.men,
            rams: h.rams.max(0),
            ..Stand::default()
        })
        .collect();
    let mut garrison = d.soldiers.max(0);
    let walls_start = d.fortifications.clamp(0, TENTHS);
    let mut walls = walls_start;
    let mut exchange = 0;
    let mut victory: Vec<bool> = vec![garrison == 0; n];
    let mut rounds = Vec::new();
    let mut push = |garrison: i32, walls: i32, exchange: i32, stands: &[Stand]| {
        rounds.push(Round {
            garrison,
            walls,
            exchange,
            armies: stands.to_vec(),
        });
    };
    push(garrison, walls, exchange, &stands);

    // The garrison: every army hits it until it falls or none stands.
    while garrison > 0 && stands.iter().any(|s| s.men > 0) {
        for i in 0..n {
            if garrison == 0 || stands[i].men == 0 {
                continue;
            }
            // Original: IF INT(RND*I4+1)<INT(RND*I3+1) THEN 148 (defender wins
            // the round) — here the tie goes to the defender too, and the
            // garrison fights at a half more, and more behind its walls.
            if random(1, strength[i]) > random(1, garrison_strength(d.soldiers_efficiency, walls)) {
                garrison = max(0, garrison - units[i]);
                if garrison == 0 {
                    for (v, s) in victory.iter_mut().zip(&stands) {
                        *v |= s.men > 0;
                    }
                }
            } else {
                stands[i].men = max(0, stands[i].men - units[i]);
            }
            if record {
                push(garrison, walls, exchange, &stands);
            }
        }
        exchange += 1;
        // The rams: every RAM_PACE exchanges each one still standing knocks
        // a tenth off the walls, then takes its risk — a chance in ten per
        // man missing from its escort.
        if garrison > 0 && exchange % RAM_PACE == 0 && stands.iter().any(|s| s.rams > 0) {
            for s in stands.iter_mut().filter(|s| s.rams > 0) {
                let escort = s.men / s.rams;
                let missing = (RAM_ESCORT - escort).max(0);
                let mut broken = 0;
                for _ in 0..s.rams {
                    walls = max(0, walls - 1);
                    if random(1, RAM_ESCORT) <= missing {
                        broken += 1;
                    }
                }
                s.rams -= broken;
                s.rams_broken += broken;
            }
            if record {
                push(garrison, walls, exchange, &stands);
            }
        }
    }

    // The march: an army advances while its men can hold more ground than
    // they have crossed, meeting whoever lives on the ground it crosses.
    let reach = |i: usize, stands: &[Stand]| {
        lines[i]
            .arpents
            .min(stands[i].men.saturating_mul(ARPENTS_PER_MAN))
    };
    let marching = |i: usize, stands: &[Stand]| {
        garrison == 0 && stands[i].men > 0 && stands[i].advance < reach(i, stands)
    };
    while (0..n).any(|i| marching(i, &stands)) {
        for i in 0..n {
            if !marching(i, &stands) {
                continue;
            }
            let tu = units[i];
            // Original: I5=I5+INT(RND*I7*26+1)-INT(RND*(I7+5)+1)
            let gained = (random(1, tu * 26) - random(1, tu + 5)).max(0);
            let from = stands[i].advance;
            let to = (from + gained).min(reach(i, &stands));
            let people = met(&lines[i], from, to);
            let total = people.total();
            let mut reached = to;
            let mut k = 0;
            'duels: for (who, met) in [
                (Met::Peasant, people.peasants),
                (Met::Merchant, people.merchants),
                (Met::Noble, people.nobles),
            ] {
                for _ in 0..met.max(0) {
                    if random(0, 3) == 0 {
                        who.count(&mut stands[i].rallied);
                    } else if random(1, strength[i]) > random(1, who.ardour(d)) {
                        who.count(&mut stands[i].killed);
                    } else {
                        stands[i].men -= 1;
                        if stands[i].men == 0 {
                            // Fallen at this one's door: the rest of the stretch stands.
                            reached = from + ((to - from) as i64 * k as i64 / total as i64) as i32;
                            break 'duels;
                        }
                    }
                    k += 1;
                }
            }
            stands[i].advance = reached;
            if record {
                push(garrison, walls, exchange, &stands);
            }
        }
    }

    let fallen = garrison == 0
        && stands
            .iter()
            .zip(&lines)
            .all(|(s, l)| s.advance >= l.arpents);
    let annexed_by = fallen
        .then(|| {
            (0..n)
                .filter(|&i| victory[i])
                .max_by_key(|&i| (stands[i].advance, stands[i].men))
        })
        .flatten();
    let armies = hosts
        .iter()
        .zip(lines)
        .zip(&stands)
        .zip(victory)
        .map(|(((h, line), s), victory)| Army {
            attacker: h.attacker,
            sent: h.men,
            men: s.men,
            rams: h.rams.max(0),
            rams_broken: s.rams_broken,
            victory,
            advance: s.advance,
            rallied: s.rallied,
            killed: s.killed,
            line,
        })
        .collect();
    FrontResult {
        armies,
        rounds,
        garrison_start: d.soldiers.max(0),
        garrison_left: garrison,
        walls_start,
        walls_left: walls,
        annexed_by,
    }
}

/// Apply a fought front: the defender loses what lies on the ground crossed
/// (its fallen, the walls knocked down, and the goods taken or lost), every
/// living attacker brings home its men, rams and spoils, and a realm crossed
/// to its capital falls to the army that took the most.
pub fn apply_front(game: &mut EmpireGame, defender: Kingdoms, r: &FrontResult) {
    let total = r.spoils();
    let d = game.kingdom_mut(defender);
    // The fallen are taken off the garrison, not the garrison set to what
    // stood: a front applied earlier may have brought the defender's own
    // expedition home meanwhile.
    d.soldiers = (d.soldiers - r.garrison_fallen()).max(0);
    d.fortifications = (d.fortifications - r.walls_fallen()).max(0);
    d.peasants = (d.peasants - total.rallied.peasants - total.killed.peasants).max(0);
    d.merchants = (d.merchants - total.rallied.merchants - total.killed.merchants).max(0);
    d.nobles = (d.nobles - total.rallied.nobles - total.killed.nobles).max(0);
    d.surface = (d.surface - total.arpents).max(0);
    d.grain_stocks = (d.grain_stocks - total.grain - total.grain_lost).max(0);
    d.treasury = (d.treasury - total.treasury - total.treasury_lost).max(0);
    for kind in BuildingKind::ALL {
        let c = kind.count_mut(d);
        *c = (*c - total.taken[kind.index()] - total.burned[kind.index()]).max(0);
    }
    for a in &r.armies {
        let k = game.kingdom_mut(a.attacker);
        // A realm annexed meanwhile has no home to come to: its men are lost
        // and its spoils stay where they were taken.
        if k.is_dead {
            continue;
        }
        let s = a.spoils();
        k.soldiers += a.men;
        k.rams += a.rams_home();
        k.surface += s.arpents;
        k.peasants += s.rallied.peasants;
        k.merchants += s.rallied.merchants;
        k.nobles += s.rallied.nobles;
        k.grain_stocks += s.grain;
        k.treasury += s.treasury;
        for kind in BuildingKind::ALL {
            *kind.count_mut(k) += s.taken[kind.index()];
        }
    }
    if let Some(i) = r.annexed_by {
        let winner = r.armies[i].attacker;
        let d = game.kingdom_mut(defender);
        let rest = (
            d.surface,
            d.peasants,
            d.grain_stocks,
            d.merchants,
            d.treasury,
            d.nobles,
            BuildingKind::ALL.map(|kind| kind.count(d)),
        );
        d.surface = 0;
        d.peasants = 0;
        d.grain_stocks = 0;
        d.merchants = 0;
        d.treasury = 0;
        d.soldiers = 0;
        d.nobles = 0;
        d.fortifications = 0;
        d.hospices = 0;
        d.rams = 0;
        d.scouts = 0;
        for kind in BuildingKind::ALL {
            *kind.count_mut(d) = 0;
        }
        d.fall(Fate::Annexed(winner));
        let k = game.kingdom_mut(winner);
        if !k.is_dead {
            k.surface += rest.0;
            k.peasants += rest.1;
            k.grain_stocks += rest.2;
            k.merchants += rest.3;
            k.treasury += rest.4;
            k.nobles += rest.5;
            for kind in BuildingKind::ALL {
                *kind.count_mut(k) += rest.6[kind.index()];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rich(k: &mut Kingdom) {
        k.grain_mills = 12;
        k.foundries = 3;
        k.marketplaces = 6;
        k.shipyards = 2;
        k.nobles = 10;
        k.treasury = 5000;
        k.grain_stocks = 20_000;
        k.merchants = 40;
    }

    #[test]
    fn shares_are_equal_and_add_up() {
        assert_eq!(shares(10, 3).collect::<Vec<_>>(), vec![4, 3, 3]);
        assert_eq!(shares(7, 1).collect::<Vec<_>>(), vec![7]);
        assert_eq!(shares(2, 3).collect::<Vec<_>>(), vec![1, 1, 0]);
    }

    #[test]
    fn a_line_yields_what_lies_on_the_ground_crossed() {
        let mut k = Kingdom::new(Kingdoms::Spain);
        rich(&mut k);
        let line = lay_out(&k, 1).remove(0);
        assert_eq!(line.arpents, 10_000);
        assert_eq!(line.buildings.len(), 23);
        assert!(line.buildings.windows(2).all(|w| w[0].at <= w[1].at));
        assert!(line.buildings.iter().all(|b| (1..=10_000).contains(&b.at)));
        // Nothing crossed, nothing taken, no one met.
        assert_eq!(line.ground(0), Spoils::default());
        assert_eq!(line.people(0), People::default());
        // A tenth of the ground: a tenth of the serfs, a hundredth of the
        // gold — a third of the goods taken, a third lost.
        let s = line.ground(1000);
        assert_eq!(s.arpents, 1000);
        assert_eq!((s.grain, s.grain_lost), (666, 666));
        assert_eq!((s.treasury, s.treasury_lost), (16, 16));
        let p = line.people(1000);
        assert_eq!((p.peasants, p.merchants, p.nobles), (200, 4, 0));
        // The whole line: everything, every building taken or burned.
        let all = line.ground(10_000);
        assert_eq!(all.treasury + all.treasury_lost, 3332);
        assert_eq!(line.people(10_000).nobles, 10);
        assert_eq!(all.buildings_taken() + all.buildings_burned(), 23);
        assert_eq!(all.taken[0] + all.burned[0], 12);
        // Beyond the line is still the whole line.
        assert_eq!(line.ground(50_000), all);
    }

    #[test]
    fn two_lines_split_the_realm_equally() {
        let mut k = Kingdom::new(Kingdoms::Spain);
        rich(&mut k);
        k.surface = 10_001;
        let lines = lay_out(&k, 2);
        assert_eq!((lines[0].arpents, lines[1].arpents), (5001, 5000));
        assert_eq!(lines[0].treasury + lines[1].treasury, 5000);
        assert_eq!(lines[0].grain, lines[1].grain);
        assert_eq!(
            lines[0].buildings.len() + lines[1].buildings.len(),
            23,
            "every building is on one line"
        );
        assert_eq!(lines[0].buildings.len(), 12);
        assert!(lines
            .iter()
            .all(|l| l.buildings.iter().all(|b| b.at <= l.arpents)));
    }

    #[test]
    fn about_a_building_in_three_is_taken_the_rest_burn() {
        let mut k = Kingdom::new(Kingdoms::Spain);
        k.grain_mills = 3000;
        let line = lay_out(&k, 1).remove(0);
        let burned = line.buildings.iter().filter(|b| b.burned).count();
        assert!((1800..2200).contains(&burned), "{burned} burned");
    }

    #[test]
    fn those_met_on_the_way_come_frontier_end_first() {
        let mut k = Kingdom::new(Kingdoms::Spain);
        rich(&mut k);
        let line = lay_out(&k, 1).remove(0);
        // Serfs are spread evenly, nobles live by the capital.
        let near = met(&line, 0, 1000);
        assert_eq!(near.total(), 204);
        assert_eq!(near.nobles, 0);
        let far = met(&line, 9000, 10_000);
        assert_eq!(far.nobles, 10 - 8);
        assert_eq!(met(&line, 500, 500).total(), 0);
    }

    #[test]
    fn the_garrison_defends_the_land() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).soldiers = 3;
        game.kingdom_mut(Kingdoms::Spain).soldiers = 5000;
        for _ in 0..20 {
            let r = simulate_front(&game, Kingdoms::Spain, &[Host::new(Kingdoms::France, 3)]);
            assert!(!r.garrison_fell());
            assert!(!r.armies[0].victory);
            assert_eq!(r.armies[0].men, 0);
            assert_eq!(r.armies[0].advance, 0);
            assert_eq!(r.spoils(), Spoils::default());
            assert!(r.annexed_by.is_none());
            assert_eq!(r.rounds[0].armies[0].men, 3);
            assert!(r.rounds.len() > 1);
        }
    }

    #[test]
    fn the_march_goes_on_against_the_people_once_the_garrison_falls() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).soldiers = 300;
        game.kingdom_mut(Kingdoms::Spain).soldiers = 2;
        let r = simulate_front(&game, Kingdoms::Spain, &[Host::new(Kingdoms::France, 300)]);
        assert!(r.garrison_fell());
        let a = &r.armies[0];
        assert!(a.victory);
        assert!(a.advance > 0);
        // Whoever was met either rallied or fought; the fight costs men.
        let met = a.rallied.total() + a.killed.total();
        assert!(met > 0);
        assert!(a.rallied.peasants + a.killed.peasants <= a.line.people(a.advance).peasants);
        // Marched to the end: the army died on the ground, crossed its line,
        // or holds all the ground its men can.
        assert!(a.men == 0 || a.advance == a.line.arpents || a.advance >= a.men * ARPENTS_PER_MAN);
        assert!(a.advance <= a.line.arpents.min(a.sent * ARPENTS_PER_MAN));
        assert_eq!(r.annexed_by.is_some(), a.advance == a.line.arpents);
        // The rounds tell the same story, garrison first.
        let fell = r.rounds.iter().position(|x| x.garrison == 0).unwrap();
        assert!(r.rounds[..fell].iter().all(|x| x.armies[0].advance == 0));
        assert!(r
            .rounds
            .windows(2)
            .all(|w| w[0].armies[0].advance <= w[1].armies[0].advance));
        let last = &r.rounds.last().unwrap().armies[0];
        assert_eq!(last.advance, a.advance);
        assert_eq!(last.rallied, a.rallied);
        assert_eq!(last.killed, a.killed);
    }

    #[test]
    fn about_one_in_three_met_rallies() {
        let mut game = EmpireGame::default();
        let f = game.kingdom_mut(Kingdoms::France);
        f.soldiers = 5000;
        f.soldiers_efficiency = 300;
        let s = game.kingdom_mut(Kingdoms::Spain);
        s.soldiers = 0;
        s.peasants = 30_000;
        let r = simulate_front(&game, Kingdoms::Spain, &[Host::new(Kingdoms::France, 5000)]);
        let a = &r.armies[0];
        // Everyone met either rallied, was killed, or killed the man he fought.
        let met = a.rallied.peasants + a.killed.peasants + a.lost();
        assert!(met > 1000, "met {met}");
        let share = a.rallied.peasants as f64 / met as f64;
        assert!((0.28..0.38).contains(&share), "rallied {share}");
        // Each lost duel costs one man of arms; against a militia at 50, an
        // army at 300 wins eleven duels in twelve.
        assert!(
            a.lost() > 0 && a.lost() * 6 < a.killed.peasants,
            "lost {} killed {}",
            a.lost(),
            a.killed.peasants
        );
    }

    #[test]
    fn the_garrison_fights_at_a_half_more() {
        assert_eq!(garrison_strength(100, 0), 150);
        assert_eq!(garrison_strength(100, 10), 300);
        assert_eq!(garrison_strength(100, 3), 195);
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).soldiers = 1000;
        game.kingdom_mut(Kingdoms::France).soldiers_efficiency = 100;
        let s = game.kingdom_mut(Kingdoms::Spain);
        s.soldiers = 1000;
        s.soldiers_efficiency = 100;
        // Man for man at the same efficiency, the walls decide: a duel at
        // 100 against 150 is lost two times in three.
        let held = (0..200)
            .filter(|_| {
                simulate_front(&game, Kingdoms::Spain, &[Host::new(Kingdoms::France, 1000)])
                    .garrison_left
                    > 0
            })
            .count();
        assert!(held >= 180, "held {held}");
    }

    #[test]
    fn walls_hold_and_rams_bring_them_down() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).soldiers = 500;
        let s = game.kingdom_mut(Kingdoms::Spain);
        s.soldiers = 500;
        s.fortifications = 10;
        // Full walls: a duel at 150 against 300 is lost three times in four.
        let held = (0..20)
            .filter(|_| {
                simulate_front(&game, Kingdoms::Spain, &[Host::new(Kingdoms::France, 500)])
                    .garrison_left
                    > 0
            })
            .count();
        assert!(held >= 18, "held {held}");
        // An army at 900 wins five duels in six: the garrison falls in some
        // twenty exchanges, the escort never short — ten rams raze the
        // walls at their first blow and none is broken.
        game.kingdom_mut(Kingdoms::France).soldiers_efficiency = 900;
        let r = simulate_front(
            &game,
            Kingdoms::Spain,
            &[Host::new(Kingdoms::France, 500).with_rams(10)],
        );
        assert_eq!(r.walls_start, 10);
        let a = &r.armies[0];
        assert_eq!((a.rams, a.rams_broken), (10, 0));
        // The first blows land after RAM_PACE exchanges, all ten at once.
        let first = r.rounds.iter().position(|x| x.walls < 10).unwrap();
        assert_eq!(r.rounds[first].exchange, RAM_PACE);
        assert_eq!(r.rounds[first].walls, 0);
        assert_eq!(r.rounds[first - 1].blow_in(), 1);
        assert_eq!(r.rounds[0].blow_in(), RAM_PACE);
        assert!(r.rounds.iter().all(|x| x.armies[0].rams == 10));
        assert!(r.rounds.windows(2).all(|w| w[0].walls >= w[1].walls));
        assert_eq!(r.walls_left, 0);
        // Applied, the walls stay down and the rams come home.
        let mut g = game.clone();
        g.kingdom_mut(Kingdoms::France).soldiers = 0;
        apply_front(&mut g, Kingdoms::Spain, &r);
        assert_eq!(g.kingdom(Kingdoms::Spain).fortifications, 0);
        assert_eq!(g.kingdom(Kingdoms::France).rams, a.rams_home());
        assert_eq!(a.rams_home(), if a.men > 0 { 10 } else { 0 });
    }

    #[test]
    fn a_ram_without_escort_is_lost_at_its_blow() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).soldiers = 3;
        let s = game.kingdom_mut(Kingdoms::Spain);
        s.soldiers = 5000;
        s.fortifications = 0;
        let mut broken = 0;
        let mut blows = 0;
        for _ in 0..50 {
            let r = simulate_front(
                &game,
                Kingdoms::Spain,
                &[Host::new(Kingdoms::France, 3).with_rams(3)],
            );
            // No walls to break, the risk is taken all the same; wiped out,
            // the army leaves every ram behind.
            assert_eq!(r.walls_fallen(), 0);
            assert_eq!(r.armies[0].rams_home(), 0);
            assert_eq!(r.armies[0].rams_lost(), 3);
            if r.rounds.iter().any(|x| x.exchange >= RAM_PACE) {
                blows += 1;
                broken += r.armies[0].rams_broken;
            }
        }
        // Three men for three rams: one each, nine missing — nine chances in
        // ten of breaking at the first blow.
        if blows > 0 {
            assert!(broken * 10 >= blows * 3 * 7, "broken {broken} in {blows}");
        }
    }

    #[test]
    fn serfs_fight_as_a_militia_whatever_the_realms_soldiers_are_worth() {
        let mut game = EmpireGame::default();
        let f = game.kingdom_mut(Kingdoms::France);
        f.soldiers = 5000;
        f.soldiers_efficiency = 100;
        let s = game.kingdom_mut(Kingdoms::Spain);
        s.soldiers = 0;
        s.soldiers_efficiency = 300;
        s.peasants = 30_000;
        let r = simulate_front(&game, Kingdoms::Spain, &[Host::new(Kingdoms::France, 5000)]);
        let a = &r.armies[0];
        // Duels at 100 against 50: about three in four won, not one in six as
        // against the realm's own soldiers.
        let won = a.killed.peasants as f64 / (a.killed.peasants + a.lost()) as f64;
        assert!((0.68..0.82).contains(&won), "won {won}");
    }

    #[test]
    fn the_people_defend_a_realm_without_an_army_and_the_day_is_won_at_once() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).soldiers = 10;
        game.kingdom_mut(Kingdoms::Spain).soldiers = 0;
        let r = simulate_front(&game, Kingdoms::Spain, &[Host::new(Kingdoms::France, 10)]);
        assert!(r.armies[0].victory);
        assert_eq!(r.garrison_start, 0);
        assert!(r.armies[0].advance > 0);
    }

    #[test]
    fn a_small_army_takes_some_hundred_arpents_not_the_realm() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).soldiers = 10;
        game.kingdom_mut(Kingdoms::Spain).soldiers = 0;
        let mut total = 0;
        for _ in 0..50 {
            let r = simulate_front(&game, Kingdoms::Spain, &[Host::new(Kingdoms::France, 10)]);
            assert!(r.annexed_by.is_none());
            total += r.armies[0].advance;
        }
        // Bounded by the ground ten men can hold, and most of it taken: the
        // militia costs a man about every 45 arpents.
        let mean = total / 50;
        assert!((100..=200).contains(&mean), "mean advance {mean}");
    }

    #[test]
    fn the_take_of_a_raid_is_bounded_by_its_size() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).soldiers = 100;
        game.kingdom_mut(Kingdoms::Spain).soldiers = 0;
        for _ in 0..20 {
            let r = simulate_front(&game, Kingdoms::Spain, &[Host::new(Kingdoms::France, 100)]);
            let a = &r.armies[0];
            assert!(a.advance <= 100 * ARPENTS_PER_MAN);
            // The men who could hold no more ground come home.
            assert!(a.men > 0);
            assert!(a.advance >= a.men * ARPENTS_PER_MAN || a.advance == a.line.arpents);
            assert!(r.annexed_by.is_none());
        }
        // Enough men to hold the realm take it whole.
        game.kingdom_mut(Kingdoms::France).soldiers = 2000;
        let r = simulate_front(&game, Kingdoms::Spain, &[Host::new(Kingdoms::France, 2000)]);
        assert_eq!(r.annexed_by, Some(0));
    }

    #[test]
    fn two_armies_hit_one_garrison_and_each_marches_on_its_own_line() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).soldiers = 200;
        game.kingdom_mut(Kingdoms::Germany).soldiers = 100;
        let s = game.kingdom_mut(Kingdoms::Spain);
        s.soldiers = 40;
        rich(s);
        let r = simulate_front(
            &game,
            Kingdoms::Spain,
            &[
                Host::new(Kingdoms::France, 200),
                Host::new(Kingdoms::Germany, 100),
            ],
        );
        assert_eq!(r.armies.len(), 2);
        assert_eq!(r.armies[0].line.arpents + r.armies[1].line.arpents, 10_000);
        assert_eq!(r.armies[0].line.arpents, 5000);
        for x in &r.rounds {
            assert_eq!(x.armies.len(), 2);
            assert!(x.armies.iter().all(|s| s.advance <= 5000));
        }
        if r.garrison_fell() {
            assert!(r.armies.iter().any(|a| a.victory));
        }
        // Conservation: what the lines yield plus what the defender keeps is
        // the realm, less what burned. The men have left their garrisons.
        let mut g = game.clone();
        g.kingdom_mut(Kingdoms::France).soldiers = 0;
        g.kingdom_mut(Kingdoms::Germany).soldiers = 0;
        apply_front(&mut g, Kingdoms::Spain, &r);
        let d = g.kingdom(Kingdoms::Spain);
        let f = g.kingdom(Kingdoms::France);
        let ge = g.kingdom(Kingdoms::Germany);
        let land = d.surface + (f.surface - 10_000) + (ge.surface - 10_000);
        assert_eq!(land, 10_000);
        let grain = d.grain_stocks
            + (f.grain_stocks - game.kingdom(Kingdoms::France).grain_stocks)
            + (ge.grain_stocks - game.kingdom(Kingdoms::Germany).grain_stocks);
        let s = r.spoils();
        assert_eq!(grain, 20_000 - s.grain_lost);
        let serfs = d.peasants + (f.peasants - 2000) + (ge.peasants - 2000);
        assert_eq!(serfs, 2000 - s.killed.peasants);
        let mills = d.grain_mills + f.grain_mills + ge.grain_mills + s.burned[0];
        assert_eq!(mills, 12);
        assert_eq!(f.soldiers, r.armies[0].men);
        assert_eq!(ge.soldiers, r.armies[1].men);
        assert_eq!(d.soldiers, r.garrison_left);
    }

    #[test]
    fn a_short_raid_never_reaches_the_treasury() {
        let mut k = Kingdom::new(Kingdoms::Spain);
        k.treasury = 100;
        k.nobles = 5;
        let line = lay_out(&k, 1).remove(0);
        assert_eq!(line.ground(900).treasury, 0);
        assert_eq!(line.people(900).nobles, 0);
        assert_eq!(line.ground(5000).treasury, 8);
    }

    #[test]
    fn a_realm_crossed_to_its_capital_falls_to_the_army_that_took_the_most() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).soldiers = 5000;
        let s = game.kingdom_mut(Kingdoms::Spain);
        s.soldiers = 1;
        s.peasants = 5;
        rich(s);
        let r = simulate_front(&game, Kingdoms::Spain, &[Host::new(Kingdoms::France, 5000)]);
        assert_eq!(r.annexed_by, Some(0));
        game.kingdom_mut(Kingdoms::France).soldiers = 0;
        apply_front(&mut game, Kingdoms::Spain, &r);
        let d = game.kingdom(Kingdoms::Spain);
        assert!(d.is_dead);
        assert_eq!(d.fate, Some(Fate::Annexed(Kingdoms::France)));
        assert_eq!(
            (d.surface, d.peasants, d.treasury, d.grain_mills),
            (0, 0, 0, 0)
        );
        let f = game.kingdom(Kingdoms::France);
        let s = r.spoils();
        assert_eq!(f.surface, 20_000);
        assert_eq!(f.treasury, 6000 - s.treasury_lost);
        assert_eq!(f.merchants, 65 - s.killed.merchants);
        assert_eq!(f.nobles, 11 - s.killed.nobles);
        assert_eq!(f.grain_mills, 12 - s.burned[0]);
        assert_eq!(f.soldiers, r.armies[0].men);
    }

    #[test]
    fn an_annexed_attacker_brings_nothing_home() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).soldiers = 300;
        game.kingdom_mut(Kingdoms::Spain).soldiers = 2;
        let r = simulate_front(&game, Kingdoms::Spain, &[Host::new(Kingdoms::France, 300)]);
        game.kingdom_mut(Kingdoms::France)
            .fall(Fate::Annexed(Kingdoms::Germany));
        let before = game.kingdom(Kingdoms::France).clone();
        apply_front(&mut game, Kingdoms::Spain, &r);
        let f = game.kingdom(Kingdoms::France);
        assert_eq!(
            (f.surface, f.soldiers, f.treasury),
            (before.surface, before.soldiers, before.treasury)
        );
        assert_eq!(
            game.kingdom(Kingdoms::Spain).surface,
            10_000 - r.spoils().arpents
        );
    }
}
