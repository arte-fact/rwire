//! The computer as a player: two small neural networks that see what a
//! seigneur sees and answer the two phases of the year in one breath each —
//! the Intendance (rations, rates, market, purchases) and the Extérieur
//! (expeditions, éclaireur). The networks only choose; the decoders bound
//! every answer by the rules and the game applies it exactly as it does a
//! human's. Nothing here knows how to play: the weights are found by
//! evolution (see `apps/empire-train`) on tables of [`crate::arena`].

use std::sync::LazyLock;

use crate::campaign::FIRST_WAR_YEAR;
use crate::demography::{affordable_ration, Council, YearDemography};
use crate::economy::{Taxes, YearEconomy};
use crate::game::{EmpireGame, BARBARIAN_LANDS};
use crate::intel::{Dossier, Heard, SCOUT_PRICE};
use crate::investments::InvestmentType;
use crate::kingdom::{Kingdom, Kingdoms, PlayerTitle, Requirement, KINGDOMS, RATION_SCALE};
use crate::trade::{
    calculate_buy_cost, max_land_sale, GRAIN_LOT, MAX_GRAIN_PRICE, MIN_GRAIN_PRICE,
};

/// Neurons of the hidden layer of each network.
pub const HIDDEN: usize = 32;
/// An answer under this is "nothing": sigmoids never quite reach zero.
const DEADBAND: f32 = 0.05;
/// Surfaces are heard by the five hundred, from the third year.
const HEARD_ROUNDING: i32 = 500;

// -- what a seigneur sees ---------------------------------------------------

/// Figures of the seigneur's own realm, the year opened (harvest in), and
/// the barbarian lands left to take.
const OWN: usize = 38;
/// The Chronique: last year's census and ledger.
const CHRONICLE: usize = 12;
/// Per rival: what is public, the stall, the rumours, the éclaireur's
/// report, the agent's letter.
pub const RIVAL: usize = 19;
/// The rivals, in the order they sit from the seigneur's own seat.
pub const RIVALS: usize = 5;
/// The Intendance's answer of the year, the Extérieur's of the year before.
pub const A_OUT: usize = 15;
pub const B_OUT: usize = 18;
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
fn criteria(k: &Kingdom) -> [f32; 7] {
    let Some(next) = k.title().next() else {
        return [1.0; 7];
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
    v.extend(criteria(k));
    v.push(count(game.barbarians_surface, BARBARIAN_LANDS as f32));
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
        let heard = if year >= 3 {
            (r.surface + HEARD_ROUNDING / 2) / HEARD_ROUNDING * HEARD_ROUNDING
        } else {
            0
        };
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
            count(heard, 10_000.0),
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
            report.map_or(0.0, |r| count(r.subjects(), 3_000.0)),
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

impl Brain {
    /// Weights of both networks laid end to end: the genome evolution works on.
    pub const GENOME: usize = Self::genome_with(OWN, RIVAL);

    pub fn from_genome(g: &[f32]) -> Brain {
        assert_eq!(g.len(), Self::GENOME);
        let (a, b) = g.split_at(Net::len(A_IN, A_OUT));
        Brain {
            intendance: Net::new(A_IN, A_OUT, a),
            exterieur: Net::new(B_IN, B_OUT, b),
        }
    }

    /// Today's [`OWN`] and [`RIVAL`] widths.
    pub const OWN: usize = OWN;
    pub const RIVAL: usize = RIVAL;

    /// The length of a genome laid out for `own` entries of the seigneur's
    /// own and `rival` per rival.
    pub const fn genome_with(own: usize, rival: usize) -> usize {
        let sight = own + CHRONICLE + RIVALS * rival + B_OUT;
        Net::len(sight, A_OUT) + Net::len(sight + A_OUT, B_OUT)
    }

    /// A genome schooled when the seigneur's own took `own_was` entries of
    /// the sight and each rival `rival_was`, laid out for today's [`OWN`]
    /// and [`RIVAL`]: the weights it had where they were, nothing on the
    /// entries it never saw — so it plays on exactly as it did until
    /// evolution finds a use for them. New entries are appended to their
    /// block.
    pub fn grown(g: &[f32], own_was: usize, rival_was: usize) -> Vec<f32> {
        assert!(own_was <= OWN && rival_was <= RIVAL);
        assert_eq!(g.len(), Self::genome_with(own_was, rival_was));
        let sight_was = own_was + CHRONICLE + RIVALS * rival_was + B_OUT;
        let moved = |i: usize| {
            let base_was = own_was + CHRONICLE;
            let base = OWN + CHRONICLE;
            if i < own_was {
                i
            } else if i < base_was {
                i + OWN - own_was
            } else if i < base_was + RIVALS * rival_was {
                base + (i - base_was) / rival_was * RIVAL + (i - base_was) % rival_was
            } else {
                i - base_was - RIVALS * rival_was + base + RIVALS * RIVAL
            }
        };
        let net = |w: &[f32], inputs_was: usize, inputs: usize, outputs: usize| {
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
            debug_assert_eq!(w2.len(), (HIDDEN + 1) * outputs);
            out.extend_from_slice(w2);
            out
        };
        let (a, b) = g.split_at(Net::len(sight_was, A_OUT));
        let mut grown = net(a, sight_was, A_IN, A_OUT);
        grown.extend(net(b, sight_was + A_OUT, B_IN, B_OUT));
        assert_eq!(grown.len(), Self::GENOME);
        grown
    }

    /// The brain the computers sit down with: schooled at the arena
    /// (`apps/empire-train`) up to the war rung — 2 300 generations, six
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

    /// The Extérieur's orders, on the year's sight and the Intendance's
    /// answer; the raw answer is kept in `m` for next year's sight.
    pub fn orders(
        &self,
        game: &EmpireGame,
        id: Kingdoms,
        m: &mut Memory,
        stage: Stage,
        letters: Letters,
    ) -> Orders {
        let mut x = sight(game, id, m);
        x.extend(m.answer);
        let o = self.exterieur.forward(&x);
        m.last_orders.copy_from_slice(&o);
        decode_orders(&o, game.kingdom(id), game, stage, letters)
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

const PURCHASES: [InvestmentType; 6] = [
    InvestmentType::Marketplaces,
    InvestmentType::GrainMills,
    InvestmentType::Foundries,
    InvestmentType::Shipyards,
    InvestmentType::Palaces,
    InvestmentType::Soldiers,
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
    // holds; the palace stops at its roof, the army at what the nobles can
    // command.
    let mut wants: Vec<(usize, f32)> = (0..6)
        .map(|i| (i, on(9 + i)))
        .filter(|w| w.1 > 0.0)
        .collect();
    wants.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut purchases = Vec::new();
    for (i, want) in wants {
        let kind = PURCHASES[i];
        let mut cap = treasury / kind.cost();
        cap = match kind {
            InvestmentType::Palaces => cap.min(10 - k.palaces),
            InvestmentType::Soldiers => cap.min(k.nobles * 20 - k.soldiers),
            _ => cap,
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

/// The year's Extérieur, decoded: expeditions within the realm's right and
/// its garrison, an éclaireur it can pay, the agents it means to keep.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Orders {
    /// `(target, men)`; `None` = the barbarians.
    pub expeditions: Vec<(Option<Kingdoms>, i32)>,
    pub scout: Option<Kingdoms>,
    /// The living rivals an agent is wanted in next year: kept where he
    /// stands, bought where a fresh report allows, dismissed elsewhere.
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
    /// The éclaireurs go and the agents are kept for nothing.
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

/// Read the Extérieur's answer `out` for `k` among `kingdoms`.
pub fn decode_orders(
    out: &[f32],
    k: &Kingdom,
    game: &EmpireGame,
    stage: Stage,
    letters: Letters,
) -> Orders {
    debug_assert_eq!(out.len(), B_OUT);
    let kingdoms = &game.kingdoms;
    let mut orders = Orders::default();
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
    for (target, want) in wants.into_iter().take(allowed.max(0) as usize) {
        let men = ((want * k.soldiers as f32).round() as i32).min(garrison);
        if men < 1 {
            break;
        }
        garrison -= men;
        orders.expeditions.push((target, men));
    }
    if stage.guard() && (letters.scouts() || k.treasury >= SCOUT_PRICE) {
        // The loudest of the five rivals still standing and "no one" wins.
        let (best, _) = out[6..12]
            .iter()
            .enumerate()
            .filter(|&(j, _)| j == RIVALS || !kingdoms[rivals[j].index()].is_dead)
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap();
        if best < RIVALS {
            orders.scout = Some(rivals[best]);
        }
    }
    if stage.guard() {
        orders.agents = rivals
            .iter()
            .enumerate()
            .filter(|&(j, o)| out[12 + j] >= 0.5 && !kingdoms[o.index()].is_dead)
            .map(|(_, &o)| o)
            .collect();
    }
    orders
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
        let o = decode_orders(&[1.0; B_OUT], &k, &game, Stage::Survive, Letters::None);
        assert!(o.expeditions.is_empty() && o.scout.is_none());
        let o = decode_orders(&[1.0; B_OUT], &k, &game, Stage::Emperor, Letters::None);
        assert_eq!(o.expeditions, vec![(None, k.soldiers)]);
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
        k.treasury = SCOUT_PRICE;
        let k = game.kingdom(Kingdoms::France).clone();
        let mut out = [0.0; B_OUT];
        out[0] = 0.6; // Britanny
        out[1] = 0.5; // Germany
        out[5] = 0.7; // barbarians
        out[2] = 0.4; // Spain, fourth: dropped
        out[7] = 0.9; // scout Germany
        out[13] = 0.7; // an agent in Germany
        out[14] = 0.4; // none in Spain
        let o = decode_orders(&out, &k, &game, Stage::War, Letters::None);
        // The barbarians first (70 of 100), Britanny on what is left, no
        // one for Germany; Spain would have been a fourth anyway.
        assert_eq!(
            o.expeditions,
            vec![(None, 70), (Some(Kingdoms::Britanny), 30)]
        );
        assert_eq!(o.scout, Some(Kingdoms::Germany));
        assert_eq!(o.agents, vec![Kingdoms::Germany]);
        let o = decode_orders(&out, &k, &game, Stage::Market, Letters::None);
        assert!(o.agents.is_empty());
        // Before the third year only the barbarians can be marched on.
        game.year = FIRST_WAR_YEAR - 1;
        let o = decode_orders(&out, &k, &game, Stage::War, Letters::None);
        assert_eq!(o.expeditions, vec![(None, 70)]);
    }

    #[test]
    fn a_grown_genome_plays_as_it_did() {
        // A genome laid out for narrower rivals, grown: the new entries
        // weigh nothing, so on a sight where they are zero the answers
        // are the same as the narrower net's on the narrower sight.
        let was = RIVAL - 2;
        let own_was = OWN - 1;
        let sight_was = own_was + CHRONICLE + RIVALS * was + B_OUT;
        let old: Vec<f32> = (0..Net::len(sight_was, A_OUT) + Net::len(sight_was + A_OUT, B_OUT))
            .map(|i| ((i * 7919) % 101) as f32 / 101.0 - 0.5)
            .collect();
        let grown = Brain::grown(&old, own_was, was);
        let brain = Brain::from_genome(&grown);
        let (a, b) = old.split_at(Net::len(sight_was, A_OUT));
        let old_a = Net::new(sight_was, A_OUT, a);
        let old_b = Net::new(sight_was + A_OUT, B_OUT, b);
        let game = EmpireGame {
            year: 5,
            ..Default::default()
        };
        let m = Memory {
            last_orders: [0.3; B_OUT],
            ..Default::default()
        };
        let full = sight(&game, Kingdoms::France, &m);
        // The narrower sight: the own block without its last entry, each
        // rival block without its last two.
        let base = OWN + CHRONICLE;
        let mut narrow: Vec<f32> = full[..own_was].to_vec();
        narrow.extend(&full[OWN..base]);
        for r in 0..RIVALS {
            narrow.extend(&full[base + r * RIVAL..base + r * RIVAL + was]);
        }
        narrow.extend(&full[base + RIVALS * RIVAL..]);
        let full_zeroed: Vec<f32> = full
            .iter()
            .enumerate()
            .map(|(i, &x)| {
                let in_rivals = (base..base + RIVALS * RIVAL).contains(&i);
                if (own_was..OWN).contains(&i) || in_rivals && (i - base) % RIVAL >= was {
                    0.0
                } else {
                    x
                }
            })
            .collect();
        let a_new = brain.intendance.forward(&full_zeroed);
        let a_old = old_a.forward(&narrow);
        assert_eq!(a_new, a_old);
        let mut x_new = full_zeroed.clone();
        x_new.extend(&a_new);
        let mut x_old = narrow.clone();
        x_old.extend(&a_old);
        assert_eq!(brain.exterieur.forward(&x_new), old_b.forward(&x_old));
        assert_eq!(Brain::grown(&grown, OWN, RIVAL), grown);
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
