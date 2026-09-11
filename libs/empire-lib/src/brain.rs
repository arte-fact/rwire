//! The computer as a player: two small neural networks that see what a
//! seigneur sees and answer the two phases of the year — the Intendance
//! (rations, rates, market, purchases) in one breath, the Extérieur in two
//! (the éclaireur and the agents first, the expeditions once they have
//! answered). The networks only choose; the decoders bound
//! every answer by the rules and the game applies it exactly as it does a
//! human's. Nothing here knows how to play: the weights are found by
//! evolution (see `apps/empire-train`) on tables of [`crate::arena`].

use std::sync::LazyLock;

use crate::campaign::{Expedition, FIRST_WAR_YEAR};
use crate::demography::{affordable_ration, Council, YearDemography};
use crate::economy::{Taxes, YearEconomy};
use crate::game::{EmpireGame, BARBARIAN_LANDS};
use crate::intel::{Dossier, Heard, SCOUT_PRICE};
use crate::investments::{InvestmentType, TENTHS};
use crate::kingdom::{Kingdom, Kingdoms, PlayerTitle, Requirement, KINGDOMS, RATION_SCALE};
use crate::trade::{
    calculate_buy_cost, max_land_sale, GRAIN_LOT, MAX_GRAIN_PRICE, MIN_GRAIN_PRICE,
};

/// Neurons of the hidden layer of each network.
pub const HIDDEN: usize = 32;
/// An answer under this is "nothing": sigmoids never quite reach zero.
const DEADBAND: f32 = 0.05;

// -- what a seigneur sees ---------------------------------------------------

/// Figures of the seigneur's own realm, the year opened (harvest in), the
/// barbarian lands left to take, then — appended when the walls came —
/// the last two criteria, the walls, the hospice and the rams in store.
const OWN: usize = 43;
/// The Chronique: last year's census and ledger.
const CHRONICLE: usize = 12;
/// Per rival: what is public, the stall, the rumours, the éclaireur's
/// report, the agent's letter.
pub const RIVAL: usize = 19;
/// The rivals, in the order they sit from the seigneur's own seat.
pub const RIVALS: usize = 5;
/// The Intendance's answer of the year, the Extérieur's of the year before.
pub const A_OUT: usize = 18;
pub const B_OUT: usize = 19;
/// What both networks see.
pub const SIGHT: usize = OWN + CHRONICLE + RIVALS * RIVAL + B_OUT;
/// The Intendance sees the sight; the Extérieur sees it and the Intendance's
/// answer of the same year.
pub const A_IN: usize = SIGHT;
pub const B_IN: usize = SIGHT + A_OUT;

/// What the seat remembers from one year to the next, as a seigneur would
/// from the Chronique and the last campaign.
#[derive(Debug, Clone, Default)]
pub struct Memory {
    /// The last Intendance sealed, as applied.
    pub intendance: Option<Intendance>,
    pub demo: Option<YearDemography>,
    pub eco: Option<YearEconomy>,
    pub plague: bool,
    /// What everyone heard of the last campaign, indexed by realm.
    pub heard: [Heard; 6],
    /// The Intendance's answer of the year, read again by the Extérieur.
    pub answer: [f32; A_OUT],
    /// The Extérieur's raw answer of last year.
    pub last_orders: [f32; B_OUT],
    /// What the seat knows of each realm.
    pub dossiers: [Dossier; 6],
}

/// The rivals of `id`, in the order they sit from its seat.
pub fn rivals(id: Kingdoms) -> [Kingdoms; RIVALS] {
    std::array::from_fn(|j| KINGDOMS[(id.index() + 1 + j) % 6])
}

fn share(n: i32, of: i32) -> f32 {
    if of <= 0 {
        0.0
    } else {
        n as f32 / of as f32
    }
}

/// A count read on a log scale: `scale` reads 0.7, ten times it 2.4, a
/// hundred times 4.6 — a realm ten times the size it was schooled at still
/// reads within range.
fn count(n: i32, scale: f32) -> f32 {
    (n.max(0) as f32 / scale).ln_1p()
}

/// [`count`] for a figure that may be negative.
fn signed_count(n: i32, scale: f32) -> f32 {
    if n < 0 {
        -count(-n, scale)
    } else {
        count(n, scale)
    }
}

fn title_level(t: PlayerTitle) -> f32 {
    t as usize as f32 / 3.0
}

/// Where the realm stands on each requirement of its next rank, as a share
/// of what is asked (1 once reached); all ones for an Emperor.
fn criteria(k: &Kingdom) -> [f32; 9] {
    let Some(next) = k.title().next() else {
        return [1.0; 9];
    };
    let progress = k.progress(next);
    std::array::from_fn(|i| {
        progress
            .iter()
            .find(|c| c.what == Requirement::ALL[i])
            .map_or(1.0, |c| share(c.have, c.need).min(1.0))
    })
}

/// What a seigneur sees as the year's Intendance opens, as the networks
/// read it: shares as they are, counts on a log scale (see [`count`]).
pub fn sight(game: &EmpireGame, id: Kingdoms, m: &Memory) -> Vec<f32> {
    let kingdoms = &game.kingdoms;
    let year = game.year;
    let k = &kingdoms[id.index()];
    let mut v = Vec::with_capacity(SIGHT);
    let needs = k.peasants_grain_needs() + k.soldiers_grain_needs();
    v.extend([
        year as f32 / 100.0,
        k.weather.value() as f32 / 6.0,
        count(k.surface, 10_000.0),
        count(k.peasants, 2_000.0),
        count(k.nobles, 40.0),
        count(k.merchants, 100.0),
        count(k.soldiers, 400.0),
        k.soldiers_efficiency as f32 / 150.0,
        k.soldiers_ration as f32 / Council::SOLDIERS_FULL as f32,
        signed_count(k.treasury, 10_000.0),
        count(k.grain_stocks, 16_000.0),
        signed_count(k.grain_harvest, 10_000.0),
        k.rats_loss_rate as f32 / 30.0,
        count(k.marketplaces, 14.0),
        count(k.grain_mills, 6.0),
        count(k.foundries, 3.0),
        count(k.shipyards, 3.0),
        k.palaces as f32 / 10.0,
        k.immigration_taxes as f32 / Taxes::MAX_CUSTOMS as f32,
        k.commercial_taxes as f32 / Taxes::MAX_SALES as f32,
        k.income_taxes as f32 / Taxes::MAX_INCOME as f32,
        title_level(k.title()),
        count(k.land_ratio(), 100.0),
        (share(k.grain_stocks, needs) / 4.0).min(1.0),
        share(k.cultivated_surface(), k.surface),
        share(k.soldiers, k.nobles * 20),
        count(k.for_sale(), 10_000.0),
        k.grain_price.min(MAX_GRAIN_PRICE) as f32 / MAX_GRAIN_PRICE as f32,
        k.listing.map_or(0.0, |(a, _)| count(a, 10_000.0)),
        k.listing
            .map_or(0.0, |(_, p)| p as f32 / MAX_GRAIN_PRICE as f32),
    ]);
    let criteria = criteria(k);
    v.extend(&criteria[..7]);
    v.push(count(game.barbarians_surface, BARBARIAN_LANDS as f32));
    // What came with the walls, after what was schooled before them.
    v.extend(&criteria[7..]);
    v.extend([
        k.fortifications as f32 / 10.0,
        k.hospices as f32 / 10.0,
        count(k.rams, 4.0),
    ]);
    debug_assert_eq!(v.len(), OWN);
    // The Chronique.
    let pop = k.population().max(1);
    match &m.demo {
        Some(d) => v.extend([
            share(d.births, pop),
            share(d.immigrants, pop),
            share(d.nobles_departed, k.nobles + d.nobles_departed),
            share(d.merchants_departed, k.merchants + d.merchants_departed),
            share(d.disease_victims, pop),
            share(d.malnutrition_victims, pop),
            share(d.starvation_victims, pop),
            share(
                d.soldiers_starvation_victims + d.soldiers_desertion_victims,
                k.soldiers + d.soldiers_starvation_victims + d.soldiers_desertion_victims,
            ),
            share(d.population_delta(), pop),
        ]),
        None => v.extend([0.0; 9]),
    }
    match &m.eco {
        Some(e) => v.extend([
            signed_count(e.net(), 10_000.0),
            count(e.soldiers_maintenance, 10_000.0),
        ]),
        None => v.extend([0.0; 2]),
    }
    v.push(f32::from(u8::from(m.plague)));
    debug_assert_eq!(v.len(), OWN + CHRONICLE);
    // The rivals.
    for o in rivals(id) {
        let r = &kingdoms[o.index()];
        let i = o.index();
        let mine = &m.heard[id.index()];
        let theirs = &m.heard[i];
        let others = |flags: &[bool; 6]| {
            flags
                .iter()
                .enumerate()
                .filter(|&(j, &f)| f && j != id.index())
                .count() as f32
                / 2.0
        };
        let report = m.dossiers[i].report.filter(|_| !r.is_dead);
        let ledger = m.dossiers[i].ledger.filter(|_| !r.is_dead);
        let age = |y: i32| 1.0 / (1.0 + (year - y) as f32);
        v.extend([
            f32::from(u8::from(!r.is_dead)),
            title_level(r.title()),
            // A realm's surface is not heard of: an éclaireur reads it.
            report.map_or(0.0, |r| count(r.surface, 10_000.0)),
            count(r.for_sale(), 10_000.0),
            r.grain_price.min(MAX_GRAIN_PRICE) as f32 / MAX_GRAIN_PRICE as f32,
            f32::from(u8::from(mine.marched_by[i])),
            f32::from(u8::from(theirs.marched_by[id.index()])),
            f32::from(u8::from(theirs.beaten_by[id.index()])),
            others(&theirs.marched_on),
            others(&theirs.marched_by),
            count(theirs.lost_to[id.index()], 1_000.0),
            // The report's age rides on the "known" flag: a report read
            // this year reads 1, an old one fades. The letter likewise.
            report.map_or(0.0, |r| age(r.year)),
            report.map_or(0.0, |r| count(r.garrison, 400.0)),
            report.map_or(0.0, |r| r.efficiency as f32 / 150.0),
            report.map_or(0.0, |r| r.fortifications as f32 / 10.0),
            ledger.map_or(0.0, |l| age(l.year)),
            ledger.map_or(0.0, |l| signed_count(l.treasury, 10_000.0)),
            ledger.map_or(0.0, |l| count(l.grain_stocks, 10_000.0)),
            ledger.map_or(0.0, |l| {
                l.aim.map_or(1.0, |(_, met, all)| met as f32 / all as f32)
            }),
        ]);
    }
    v.extend(m.last_orders);
    debug_assert_eq!(v.len(), SIGHT);
    v
}

// -- the networks ------------------------------------------------------------

/// A dense network with one hidden layer: `tanh` inside, sigmoids out.
#[derive(Debug, Clone)]
pub struct Net {
    inputs: usize,
    outputs: usize,
    w: Vec<f32>,
}

impl Net {
    /// Weights a network of this shape holds.
    pub const fn len(inputs: usize, outputs: usize) -> usize {
        (inputs + 1) * HIDDEN + (HIDDEN + 1) * outputs
    }

    pub fn new(inputs: usize, outputs: usize, w: &[f32]) -> Net {
        assert_eq!(w.len(), Self::len(inputs, outputs));
        Net {
            inputs,
            outputs,
            w: w.to_vec(),
        }
    }

    /// The scale each weight is best drawn at to start with: `1/√fan_in`,
    /// so the hidden layer opens neither saturated nor dead.
    pub fn scales(inputs: usize, outputs: usize) -> Vec<f32> {
        let mut s = vec![1.0 / (inputs as f32).sqrt(); (inputs + 1) * HIDDEN];
        s.extend(vec![1.0 / (HIDDEN as f32).sqrt(); (HIDDEN + 1) * outputs]);
        s
    }

    pub fn forward(&self, x: &[f32]) -> Vec<f32> {
        debug_assert_eq!(x.len(), self.inputs);
        let (w1, w2) = self.w.split_at((self.inputs + 1) * HIDDEN);
        let mut h = [0.0f32; HIDDEN];
        for (j, hj) in h.iter_mut().enumerate() {
            let row = &w1[j * (self.inputs + 1)..(j + 1) * (self.inputs + 1)];
            let mut s = row[self.inputs];
            for (wi, xi) in row[..self.inputs].iter().zip(x) {
                s += wi * xi;
            }
            *hj = s.tanh();
        }
        (0..self.outputs)
            .map(|o| {
                let row = &w2[o * (HIDDEN + 1)..(o + 1) * (HIDDEN + 1)];
                let s = row[HIDDEN]
                    + row[..HIDDEN]
                        .iter()
                        .zip(&h)
                        .map(|(w, h)| w * h)
                        .sum::<f32>();
                1.0 / (1.0 + (-s).exp())
            })
            .collect()
    }
}

/// The two networks of one computer; [`Brain::schooled`] by default.
#[derive(Debug, Clone)]
pub struct Brain {
    pub intendance: Net,
    pub exterieur: Net,
}

impl Default for Brain {
    fn default() -> Brain {
        Brain::schooled().clone()
    }
}

/// The widths a genome is laid out for: the seigneur's own entries of the
/// sight, the entries per rival, the Intendance's and the Extérieur's
/// answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
    pub own: usize,
    pub rival: usize,
    pub a_out: usize,
    pub b_out: usize,
}

impl Shape {
    /// Today's widths.
    pub const NOW: Shape = Shape {
        own: OWN,
        rival: RIVAL,
        a_out: A_OUT,
        b_out: B_OUT,
    };

    const fn sight(self) -> usize {
        self.own + CHRONICLE + RIVALS * self.rival + self.b_out
    }

    /// The length of a genome laid out this way.
    pub const fn genome(self) -> usize {
        Net::len(self.sight(), self.a_out) + Net::len(self.sight() + self.a_out, self.b_out)
    }

    fn fits(self, now: Shape) -> bool {
        self.own <= now.own
            && self.rival <= now.rival
            && self.a_out <= now.a_out
            && self.b_out <= now.b_out
    }
}

impl Brain {
    /// Weights of both networks laid end to end: the genome evolution works on.
    pub const GENOME: usize = Shape::NOW.genome();

    pub fn from_genome(g: &[f32]) -> Brain {
        assert_eq!(g.len(), Self::GENOME);
        let (a, b) = g.split_at(Net::len(A_IN, A_OUT));
        Brain {
            intendance: Net::new(A_IN, A_OUT, a),
            exterieur: Net::new(B_IN, B_OUT, b),
        }
    }

    /// A genome schooled at the widths `was`, laid out for today's
    /// [`Shape::NOW`]: the weights it had where they were, nothing on the
    /// entries it never saw and nothing into the answers it never gave — so
    /// it plays on exactly as it did until evolution finds a use for them.
    /// New entries and answers are appended to their block.
    pub fn grown(g: &[f32], was: Shape) -> Vec<f32> {
        let now = Shape::NOW;
        assert!(was.fits(now), "{was:?} does not fit {now:?}");
        assert_eq!(g.len(), was.genome());
        // Where an input of the old sight (and, for the Extérieur, of the
        // Intendance's answer after it) sits in the new one: its block
        // begins where the new block does, at the same offset within.
        let moved = |i: usize| {
            let blocks = [
                (was.own, now.own),
                (CHRONICLE, CHRONICLE),
                (RIVALS * was.rival, RIVALS * now.rival),
                (was.b_out, now.b_out),
                (was.a_out, now.a_out),
            ];
            let (mut from, mut to) = (0, 0);
            for (width_was, width) in blocks {
                if i < from + width_was {
                    let off = i - from;
                    return if width_was == RIVALS * was.rival && was.rival != now.rival {
                        to + off / was.rival * now.rival + off % was.rival
                    } else {
                        to + off
                    };
                }
                from += width_was;
                to += width;
            }
            unreachable!("input {i} beyond the old sight")
        };
        let net =
            |w: &[f32], inputs_was: usize, inputs: usize, outputs_was: usize, outputs: usize| {
                let (w1, w2) = w.split_at((inputs_was + 1) * HIDDEN);
                let mut out = vec![0.0; (inputs + 1) * HIDDEN];
                for j in 0..HIDDEN {
                    let row = &w1[j * (inputs_was + 1)..(j + 1) * (inputs_was + 1)];
                    let to = &mut out[j * (inputs + 1)..(j + 1) * (inputs + 1)];
                    for (i, &x) in row[..inputs_was].iter().enumerate() {
                        to[moved(i)] = x;
                    }
                    to[inputs] = row[inputs_was];
                }
                debug_assert_eq!(w2.len(), (HIDDEN + 1) * outputs_was);
                out.extend_from_slice(w2);
                out.extend(vec![0.0; (HIDDEN + 1) * (outputs - outputs_was)]);
                out
            };
        let (a, b) = g.split_at(Net::len(was.sight(), was.a_out));
        let mut grown = net(a, was.sight(), A_IN, was.a_out, A_OUT);
        grown.extend(net(b, was.sight() + was.a_out, B_IN, was.b_out, B_OUT));
        assert_eq!(grown.len(), Self::GENOME);
        grown
    }

    /// The brain the computers sit down with: schooled at the arena
    /// (`apps/empire-train`) from scratch up to the war rung — 3 500 generations, six
    /// tables of six, a hall of eight, a rank cost of forty, no letters —
    /// and kept as `brains/schooled.f32`, its genome in little-endian floats.
    pub fn schooled() -> &'static Brain {
        static SCHOOLED: LazyLock<Brain> = LazyLock::new(|| {
            let genome: Vec<f32> = include_bytes!("../brains/schooled.f32")
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                .collect();
            Brain::from_genome(&genome)
        });
        &SCHOOLED
    }

    /// The Intendance's answer as the year opens, kept in `m` for the
    /// Extérieur to read again.
    pub fn answer(&self, game: &EmpireGame, id: Kingdoms, m: &mut Memory) -> [f32; A_OUT] {
        let a = self.intendance.forward(&sight(game, id, m));
        m.answer.copy_from_slice(&a);
        m.answer
    }

    /// The Extérieur's first reading, on the year's sight and the
    /// Intendance's answer: the éclaireur and the agents to send. They
    /// answer at once; the expeditions are read after, on what they saw.
    pub fn missions(
        &self,
        game: &EmpireGame,
        id: Kingdoms,
        m: &Memory,
        stage: Stage,
        letters: Letters,
    ) -> Missions {
        let o = self.exterieur.forward(&self.exterieur_sight(game, id, m));
        decode_missions(&o, game.kingdom(id), game, stage, letters)
    }

    /// The Extérieur's second reading, the spies home: the expeditions.
    /// The raw answer is kept in `m` for next year's sight.
    pub fn expeditions(
        &self,
        game: &EmpireGame,
        id: Kingdoms,
        m: &mut Memory,
        stage: Stage,
    ) -> Vec<Expedition> {
        let o = self.exterieur.forward(&self.exterieur_sight(game, id, m));
        m.last_orders.copy_from_slice(&o);
        decode_expeditions(&o, game.kingdom(id), game, stage)
    }

    fn exterieur_sight(&self, game: &EmpireGame, id: Kingdoms, m: &Memory) -> Vec<f32> {
        let mut x = sight(game, id, m);
        x.extend(m.answer);
        x
    }

    /// The starting scale of every weight of the genome.
    pub fn scales() -> Vec<f32> {
        let mut s = Net::scales(A_IN, A_OUT);
        s.extend(Net::scales(B_IN, B_OUT));
        s
    }

    /// Where in the genome the Extérieur's output `o` lives: its `HIDDEN`
    /// weights, then its bias. So a school may open an answer the brain
    /// never gave, or seed one it should.
    pub fn exterieur_output(o: usize) -> std::ops::Range<usize> {
        assert!(o < B_OUT);
        let start = Net::len(A_IN, A_OUT) + (B_IN + 1) * HIDDEN + o * (HIDDEN + 1);
        start..start + HIDDEN + 1
    }
}

// -- what the answers may touch ---------------------------------------------

/// How much of the game a brain is let play: the rungs of its schooling.
/// Each rung opens more of the answers; the rest are read as "nothing".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// Rations, rates, purchases, land: live.
    Survive,
    /// Raids on the barbarians: grow.
    Emperor,
    /// The grain market.
    Market,
    /// The éclaireur and the agents, a garrison kept against the
    /// neighbours' armies: hold.
    Guard,
    /// War on the neighbours.
    War,
}

impl Stage {
    pub fn barbarians(self) -> bool {
        self >= Stage::Emperor
    }

    pub fn market(self) -> bool {
        self >= Stage::Market
    }

    /// Spies are sent, and the neighbours' armies may come.
    pub fn guard(self) -> bool {
        self >= Stage::Guard
    }

    pub fn war(self) -> bool {
        self >= Stage::War
    }
}

// -- the Intendance ------------------------------------------------------------

/// The year's Intendance, as decoded from the network's answer: legal by
/// construction against the realm it was read on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intendance {
    /// Grain listed: `(bushels, centimes the bushel)`.
    pub listed: Option<(i32, i32)>,
    /// Grain bought: `(seller, bushels)`.
    pub bought: Option<(Kingdoms, i32)>,
    /// Arpents sold to the barbarians.
    pub land_sold: i32,
    /// Purchases, in the order they are made.
    pub purchases: Vec<(InvestmentType, i32)>,
    /// Rations and rates, to be clamped to the stocks once the market and the
    /// purchases are done.
    pub council: Council,
}

/// The purchases, in the order of the Intendance's answers `9..18`.
const PURCHASES: [InvestmentType; 9] = [
    InvestmentType::Marketplaces,
    InvestmentType::GrainMills,
    InvestmentType::Foundries,
    InvestmentType::Shipyards,
    InvestmentType::Palaces,
    InvestmentType::Soldiers,
    InvestmentType::Fortifications,
    InvestmentType::Hospices,
    InvestmentType::Rams,
];

fn lots(bushels: i32) -> i32 {
    bushels / GRAIN_LOT * GRAIN_LOT
}

/// Read the Intendance's answer `out` for `k`, whose rivals' stalls are
/// `stalls` (`(seller, bushels, price)`, living realms only). Purchases are
/// costed one after the other on a running treasury; the council's rations
/// are bounded afterwards by [`bound_council`] on the stocks left.
pub fn decode_intendance(
    out: &[f32],
    k: &Kingdom,
    stalls: &[(Kingdoms, i32, i32)],
    stage: Stage,
) -> Intendance {
    debug_assert_eq!(out.len(), A_OUT);
    let on = |i: usize| if out[i] < DEADBAND { 0.0 } else { out[i] };
    let mut stocks = k.grain_stocks.max(0);
    let mut treasury = k.treasury;

    let listed = if stage.market() {
        let bushels = lots((on(5) * stocks as f32) as i32);
        (bushels > 0).then(|| {
            stocks -= bushels;
            let span = (MAX_GRAIN_PRICE - MIN_GRAIN_PRICE) as f32;
            (bushels, MIN_GRAIN_PRICE + (out[6] * span) as i32)
        })
    } else {
        None
    };

    let bought = if stage.market() && on(7) > 0.0 {
        // The cheapest stall with a lot on it.
        stalls
            .iter()
            .filter(|&&(_, bushels, price)| bushels >= GRAIN_LOT && price > 0)
            .min_by_key(|&&(_, _, price)| price)
            .and_then(|&(seller, bushels, price)| {
                let per_lot = calculate_buy_cost(GRAIN_LOT, price.min(MAX_GRAIN_PRICE));
                let budget = (on(7) * treasury.max(0) as f32) as i32;
                let amount = lots(bushels).min(budget / per_lot * GRAIN_LOT);
                (amount > 0).then(|| {
                    treasury -= calculate_buy_cost(amount, price.min(MAX_GRAIN_PRICE));
                    (seller, amount)
                })
            })
    } else {
        None
    };

    let land_sold = (on(8) * max_land_sale(k.surface) as f32) as i32;
    treasury += land_sold * crate::trade::LAND_SELL_PRICE;

    // The strongest wants are served first, each on what the treasury still
    // holds; the palace, the walls and the hospice stop at their roof, the
    // army at what the nobles can command.
    let mut wants: Vec<(usize, f32)> = (0..PURCHASES.len())
        .map(|i| (i, on(9 + i)))
        .filter(|w| w.1 > 0.0)
        .collect();
    wants.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut purchases = Vec::new();
    for (i, want) in wants {
        let kind = PURCHASES[i];
        let mut cap = treasury / kind.cost();
        cap = match (kind, kind.tenths(k)) {
            (InvestmentType::Soldiers, _) => cap.min(k.nobles * 20 - k.soldiers),
            (_, Some(built)) => cap.min(TENTHS - built),
            (_, None) => cap,
        };
        let n = (want * cap as f32).round() as i32;
        if n > 0 {
            treasury -= n * kind.cost();
            purchases.push((kind, n));
        }
    }

    let council = Council {
        peasants_ration: (out[0] * 2.0 * Council::PEASANTS_FULL as f32).round() as i32,
        soldiers_ration: (out[1] * 1.5 * Council::SOLDIERS_FULL as f32).round() as i32,
        taxes: Taxes {
            customs: (out[2] * Taxes::MAX_CUSTOMS as f32).round() as i32,
            sales: (out[3] * Taxes::MAX_SALES as f32).round() as i32,
            income: (out[4] * Taxes::MAX_INCOME as f32).round() as i32,
        },
    };
    Intendance {
        listed,
        bought,
        land_sold,
        purchases,
        council,
    }
}

/// The council as the stocks can pay it, the people served first: the
/// web's sliders' rule.
pub fn bound_council(c: Council, k: &Kingdom) -> Council {
    let peasants = c.peasants_ration.clamp(
        0,
        affordable_ration(k, Council::PEASANTS_FULL * 2, k.mouths()),
    );
    let mut left = k.clone();
    left.grain_stocks -= peasants * k.mouths() / RATION_SCALE;
    let soldiers = c.soldiers_ration.clamp(
        0,
        affordable_ration(&left, Council::SOLDIERS_FULL * 3 / 2, k.soldiers),
    );
    Council {
        peasants_ration: peasants,
        soldiers_ration: soldiers,
        taxes: c.taxes.clamped(),
    }
}

// -- the Extérieur -----------------------------------------------------------

/// The Extérieur's first reading: an éclaireur the realm can pay, the
/// agents it sends.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Missions {
    pub scout: Option<Kingdoms>,
    /// The living rivals an agent is sent to this year.
    pub agents: Vec<Kingdoms>,
}

/// Which intelligence the table gives away — the school of letters, where
/// a brain learns to read reports before it has to pay for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Letters {
    #[default]
    None,
    /// The éclaireurs go for nothing.
    Scouts,
    /// The éclaireurs go and the agents are sent for nothing.
    All,
}

impl Letters {
    pub fn scouts(self) -> bool {
        self != Letters::None
    }

    pub fn agents(self) -> bool {
        self == Letters::All
    }
}

/// What the éclaireur himself costs, bought before he rides when none is in
/// reserve.
const SCOUT_COST: i32 = InvestmentType::Scouts.cost();

/// Read the expeditions in the Extérieur's answer `out` for `k`: within
/// the realm's right and its garrison, the rams (a share of the store)
/// with the largest army marching on a realm.
pub fn decode_expeditions(
    out: &[f32],
    k: &Kingdom,
    game: &EmpireGame,
    stage: Stage,
) -> Vec<Expedition> {
    debug_assert_eq!(out.len(), B_OUT);
    let kingdoms = &game.kingdoms;
    let rivals = rivals(k.id);
    // Targets by the men wanted on them, the strongest first.
    let mut wants: Vec<(Option<Kingdoms>, f32)> = Vec::new();
    if stage.war() && game.year >= FIRST_WAR_YEAR {
        wants.extend(
            rivals
                .iter()
                .enumerate()
                .filter(|&(_, o)| !kingdoms[o.index()].is_dead)
                .map(|(j, &o)| (Some(o), out[j])),
        );
    }
    if stage.barbarians() && game.barbarians_surface > 0 {
        wants.push((None, out[5]));
    }
    wants.retain(|w| w.1 >= DEADBAND);
    wants.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut garrison = k.soldiers;
    let allowed = k.nobles / 4 + 1;
    let mut expeditions = Vec::new();
    for (target, want) in wants.into_iter().take(allowed.max(0) as usize) {
        let men = ((want * k.soldiers as f32).round() as i32).min(garrison);
        if men < 1 {
            break;
        }
        garrison -= men;
        expeditions.push(Expedition::new(k.id, target, men));
    }
    let rams = (out[18] * k.rams as f32).round() as i32;
    if rams > 0 {
        if let Some(e) = expeditions.iter_mut().find(|e| e.target.is_some()) {
            e.rams = rams;
        }
    }
    expeditions
}

/// Read the missions in the Extérieur's answer `out` for `k`.
pub fn decode_missions(
    out: &[f32],
    k: &Kingdom,
    game: &EmpireGame,
    stage: Stage,
    letters: Letters,
) -> Missions {
    debug_assert_eq!(out.len(), B_OUT);
    let kingdoms = &game.kingdoms;
    let rivals = rivals(k.id);
    let mut missions = Missions::default();
    if stage.guard() && (letters.scouts() || k.treasury >= SCOUT_PRICE + SCOUT_COST) {
        // The loudest of the five rivals still standing and "no one" wins.
        let (best, _) = out[6..12]
            .iter()
            .enumerate()
            .filter(|&(j, _)| j == RIVALS || !kingdoms[rivals[j].index()].is_dead)
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap();
        if best < RIVALS {
            missions.scout = Some(rivals[best]);
        }
    }
    if stage.guard() {
        missions.agents = rivals
            .iter()
            .enumerate()
            .filter(|&(j, o)| out[12 + j] >= 0.5 && !kingdoms[o.index()].is_dead)
            .map(|(_, &o)| o)
            .collect();
    }
    missions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::EmpireGame;

    fn brain(fill: f32) -> Brain {
        Brain::from_genome(&vec![fill; Brain::GENOME])
    }

    #[test]
    fn the_sight_has_its_length_and_stays_bounded() {
        let game = EmpireGame::default();
        for id in KINGDOMS {
            let v = sight(&game, id, &Memory::default());
            assert_eq!(v.len(), SIGHT);
            assert!(v.iter().all(|x| x.is_finite() && (-1.0..=2.0).contains(x)));
        }
        // A realm a hundred times its starting size still reads within a
        // few units.
        let mut game = EmpireGame::default();
        let k = game.kingdom_mut(Kingdoms::France);
        k.surface = 1_000_000;
        k.grain_stocks = 2_000_000;
        k.treasury = -50_000;
        k.grain_to_sell = 500_000;
        let v = sight(&game, Kingdoms::France, &Memory::default());
        assert!(
            v.iter().all(|x| x.is_finite() && (-3.0..=5.0).contains(x)),
            "{v:?}"
        );
    }

    #[test]
    fn the_networks_answer_in_zero_one() {
        let game = EmpireGame::default();
        let b = brain(0.3);
        let s = sight(&game, Kingdoms::France, &Memory::default());
        let a = b.intendance.forward(&s);
        assert_eq!(a.len(), A_OUT);
        let mut x = s.clone();
        x.extend(&a);
        let o = b.exterieur.forward(&x);
        assert_eq!(o.len(), B_OUT);
        assert!(a.iter().chain(&o).all(|y| (0.0..=1.0).contains(y)));
    }

    #[test]
    fn a_greedy_answer_never_overspends() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.treasury = 12_345;
        k.nobles = 3;
        let stalls = [(Kingdoms::Spain, 5_000, 40), (Kingdoms::Persia, 300, 20)];
        let i = decode_intendance(&[1.0; A_OUT], &k, &stalls, Stage::War);
        let mut spent: i32 = i.purchases.iter().map(|&(kind, n)| n * kind.cost()).sum();
        let (seller, bought) = i.bought.unwrap();
        assert_eq!(seller, Kingdoms::Persia);
        spent += calculate_buy_cost(bought, 20);
        assert!(spent <= k.treasury + i.land_sold * crate::trade::LAND_SELL_PRICE);
        assert_eq!(i.land_sold, max_land_sale(k.surface));
        assert_eq!(i.listed.unwrap().0, lots(k.grain_stocks));
        let soldiers = i
            .purchases
            .iter()
            .find(|p| p.0 == InvestmentType::Soldiers)
            .map_or(0, |p| p.1);
        assert!(k.soldiers + soldiers <= k.nobles * 20);
        assert!(i
            .purchases
            .iter()
            .all(|&(kind, n)| kind != InvestmentType::Palaces || n <= 10));
    }

    #[test]
    fn the_stage_closes_what_is_not_taught_yet() {
        let k = Kingdom::new(Kingdoms::France);
        let stalls = [(Kingdoms::Spain, 5_000, 40)];
        let i = decode_intendance(&[1.0; A_OUT], &k, &stalls, Stage::Survive);
        assert!(i.listed.is_none() && i.bought.is_none());
        assert!(!i.purchases.is_empty());
        let game = EmpireGame::default();
        let all = [1.0; B_OUT];
        assert!(decode_expeditions(&all, &k, &game, Stage::Survive).is_empty());
        let m = decode_missions(&all, &k, &game, Stage::Survive, Letters::None);
        assert!(m.scout.is_none() && m.agents.is_empty());
        assert_eq!(
            decode_expeditions(&all, &k, &game, Stage::Emperor),
            vec![Expedition::new(k.id, None, k.soldiers)]
        );
    }

    #[test]
    fn expeditions_stay_within_the_right_and_the_garrison() {
        let mut game = EmpireGame {
            year: FIRST_WAR_YEAR,
            ..EmpireGame::default()
        };
        let k = game.kingdom_mut(Kingdoms::France);
        k.nobles = 8; // three expeditions
        k.soldiers = 100;
        k.rams = 3;
        k.treasury = SCOUT_PRICE + SCOUT_COST;
        let k = game.kingdom(Kingdoms::France).clone();
        let mut out = [0.0; B_OUT];
        out[0] = 0.6; // Britanny
        out[1] = 0.5; // Germany
        out[5] = 0.7; // barbarians
        out[2] = 0.4; // Spain, fourth: dropped
        out[7] = 0.9; // scout Germany
        out[13] = 0.7; // an agent in Germany
        out[14] = 0.4; // none in Spain
        out[18] = 0.7; // two of the three rams
        let e = decode_expeditions(&out, &k, &game, Stage::War);
        // The barbarians first (70 of 100), Britanny on what is left with
        // the rams, no one for Germany; Spain would have been a fourth
        // anyway.
        assert_eq!(
            e,
            vec![
                Expedition::new(k.id, None, 70),
                Expedition::new(k.id, Some(Kingdoms::Britanny), 30).with_rams(2)
            ]
        );
        let m = decode_missions(&out, &k, &game, Stage::War, Letters::None);
        assert_eq!(m.scout, Some(Kingdoms::Germany));
        assert_eq!(m.agents, vec![Kingdoms::Germany]);
        let m = decode_missions(&out, &k, &game, Stage::Market, Letters::None);
        assert!(m.agents.is_empty());
        // Before the third year only the barbarians can be marched on: the
        // rams stay home.
        game.year = FIRST_WAR_YEAR - 1;
        let e = decode_expeditions(&out, &k, &game, Stage::War);
        assert_eq!(e, vec![Expedition::new(k.id, None, 70)]);
    }

    #[test]
    fn a_grown_genome_plays_as_it_did() {
        // A genome laid out narrower on every side, grown: the new entries
        // weigh nothing and the new answers are blank, so on a sight where
        // the new entries are zero the old answers are the narrower net's
        // on the narrower sight.
        let was = Shape {
            own: OWN - 1,
            rival: RIVAL - 2,
            a_out: A_OUT - 3,
            b_out: B_OUT - 1,
        };
        let old: Vec<f32> = (0..was.genome())
            .map(|i| ((i * 7919) % 101) as f32 / 101.0 - 0.5)
            .collect();
        let grown = Brain::grown(&old, was);
        let brain = Brain::from_genome(&grown);
        let (a, b) = old.split_at(Net::len(was.sight(), was.a_out));
        let old_a = Net::new(was.sight(), was.a_out, a);
        let old_b = Net::new(was.sight() + was.a_out, was.b_out, b);
        let game = EmpireGame {
            year: 5,
            ..Default::default()
        };
        let mut last_orders = [0.3; B_OUT];
        last_orders[B_OUT - 1] = 0.0;
        let m = Memory {
            last_orders,
            ..Default::default()
        };
        let full = sight(&game, Kingdoms::France, &m);
        // The narrower sight: the own block without its last entry, each
        // rival block without its last two, the last orders without the
        // last.
        let base = OWN + CHRONICLE;
        let mut narrow: Vec<f32> = full[..was.own].to_vec();
        narrow.extend(&full[OWN..base]);
        for r in 0..RIVALS {
            narrow.extend(&full[base + r * RIVAL..base + r * RIVAL + was.rival]);
        }
        narrow.extend(&full[base + RIVALS * RIVAL..SIGHT - 1]);
        let full_zeroed: Vec<f32> = full
            .iter()
            .enumerate()
            .map(|(i, &x)| {
                let in_rivals = (base..base + RIVALS * RIVAL).contains(&i);
                if (was.own..OWN).contains(&i) || in_rivals && (i - base) % RIVAL >= was.rival {
                    0.0
                } else {
                    x
                }
            })
            .collect();
        let a_new = brain.intendance.forward(&full_zeroed);
        let a_old = old_a.forward(&narrow);
        assert_eq!(&a_new[..was.a_out], &a_old[..]);
        assert!(a_new[was.a_out..].iter().all(|&y| y == 0.5));
        let mut x_new = full_zeroed.clone();
        x_new.extend(&a_old);
        x_new.extend(vec![0.0; A_OUT - was.a_out]);
        let mut x_old = narrow.clone();
        x_old.extend(&a_old);
        let b_new = brain.exterieur.forward(&x_new);
        assert_eq!(&b_new[..was.b_out], &old_b.forward(&x_old)[..]);
        assert_eq!(b_new[was.b_out], 0.5);
        assert_eq!(Brain::grown(&grown, Shape::NOW), grown);
    }

    #[test]
    fn the_council_is_bounded_by_the_stocks_people_first() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.grain_stocks = k.mouths() * 3; // 3 bushels a mouth, nothing for the army
        k.soldiers = 50;
        let c = bound_council(
            Council {
                peasants_ration: 100,
                soldiers_ration: 100,
                taxes: Taxes {
                    customs: 99,
                    sales: 99,
                    income: 99,
                },
            },
            &k,
        );
        assert_eq!(c.peasants_ration, 30);
        assert_eq!(c.soldiers_ration, 0);
        assert_eq!(
            c.taxes,
            Taxes {
                customs: 50,
                sales: 20,
                income: 35
            }
        );
    }
}
