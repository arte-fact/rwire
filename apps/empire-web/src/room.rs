//! Shared game table: state, phases, battles and every handler.

use std::collections::BTreeMap;

use empire_lib::campaign::{apply_battle, forecast, march, Expedition, Fought};
use empire_lib::demography::{apply_feed, Council, YearDemography};
use empire_lib::economy::{apply_economy, apply_taxes, economy_report, Taxes, YearEconomy};
use empire_lib::events::{check_random_events, PlagueEvent, RulerDeathCause};
use empire_lib::front::{Army, BuildingKind, Forecast, FrontResult, Round, Spoils};
use empire_lib::harvests::{apply_grain_harvest, apply_rat_loss_rate, apply_seed_grain};
use empire_lib::ia::{plan_ai_intendance, plan_ai_war};
use empire_lib::investments::{apply_investment, InvestmentType};
use empire_lib::kingdom::RATION_SCALE;
use empire_lib::mind::SCOUT_CAUGHT;
pub use empire_lib::mind::SCOUT_PRICE;
use empire_lib::mind::{Mind, Seen, Temper};
use empire_lib::trade::{
    apply_trade, calculate_buy_cost, max_land_sale, Trade, LAND_SELL_PRICE, MAX_GRAIN_PRICE,
};
use empire_lib::{EmpireGame, Fate, Kingdom, Kingdoms, PlayerTitle, KINGDOMS};
use rand::Rng;
use rwire::{handler, EventContext, HandlerSpec, State};

use crate::ui::{coins, fmt, hommes_darmes};

/// Ticker period.
pub const TICK_MS: u64 = 50;
/// Ticks between two frames of a front being replayed.
const FRAME_TICKS: u32 = 3;
/// Ticks the order of battle stays before a front is fought: the lines draw
/// themselves, then the fronts not concerned fade.
const SCHEMA_TICKS: u32 = 50;
/// Ticks a verdict stays, and how many more for every army past the first.
const VERDICT_TICKS: u32 = 60;
const VERDICT_TICKS_PER_ARMY: u32 = 20;
/// A front is replayed over as many frames as it had rounds, within these bounds.
const BATTLE_MIN_FRAMES: usize = 25;
const BATTLE_MAX_FRAMES: usize = 50;
const JOURNAL_LEN: usize = 400;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stage {
    #[default]
    Lobby,
    Playing,
    Over,
}

/// The three parts of a year: every seigneur runs their own kingdom at the
/// same time, then everyone gives their orders at the same time, then the
/// armies march together and every battle is replayed for the whole table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Phase {
    /// Season, trade, council, census, treasury, purchases — in parallel; the
    /// phase ends once every living human seat has validated its purchases.
    #[default]
    Intendance,
    /// War: every seigneur orders their expeditions; the phase ends once
    /// every living human seat has given its orders.
    Exterieur,
    /// The armies have marched: the fronts are replayed one after the
    /// other, then the year ends.
    Campaign,
}

/// Where a seat's reading of the campaign stands: the order of battle with
/// the next front blinking, the front itself, its verdict, then the order of
/// battle again. The reading runs by itself; a tap skips to the next screen,
/// and once every front is told the last screen waits for the reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Staging {
    /// The schema, `current` about to be fought; ticks before it is.
    Schema(u32),
    /// `current` is being fought; ticks before its next frame.
    Fight(u32),
    /// `current` is settled; ticks before the next schema.
    Verdict(u32),
    /// Every front has been told.
    #[default]
    Done,
}

/// A seat's reading of the campaign — its own clock: which front it is at,
/// where it stands on it, and the frame of the front being fought.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Replay {
    pub current: usize,
    pub staging: Staging,
    pub cursor: usize,
}

impl Replay {
    /// About to read the campaign from its first front.
    fn opening() -> Self {
        Self {
            staging: Staging::Schema(SCHEMA_TICKS),
            ..Self::default()
        }
    }

    /// Every one of `n` fronts told.
    fn told(n: usize) -> Self {
        Self {
            current: n,
            ..Self::default()
        }
    }

    pub fn done(&self) -> bool {
        self.staging == Staging::Done
    }

    /// Whether front `i` has been told to its end.
    pub fn settled(&self, i: usize) -> bool {
        i < self.current || (i == self.current && matches!(self.staging, Staging::Verdict(_)))
    }

    /// The front is over: its verdict, held long enough to be read.
    fn settle(&mut self, fought: &Fought) {
        self.cursor = fought.result.rounds.len() - 1;
        let armies = fought.result.armies.len() as u32;
        self.staging = Staging::Verdict(VERDICT_TICKS + VERDICT_TICKS_PER_ARMY * (armies - 1));
    }

    /// On to the next of `n` fronts, or done.
    fn next_front(&mut self, n: usize) {
        self.current += 1;
        self.cursor = 0;
        self.staging = if self.current < n {
            Staging::Schema(SCHEMA_TICKS)
        } else {
            Staging::Done
        };
    }

    /// The round of `fought` being shown.
    pub fn frame<'a>(&self, fought: &'a Fought) -> &'a Round {
        let rounds = &fought.result.rounds;
        &rounds[self.cursor.min(rounds.len() - 1)]
    }

    fn finished(&self, fought: &Fought) -> bool {
        self.cursor + 1 >= fought.result.rounds.len()
    }
}

/// A player's position inside the year: the screens telling the year that
/// ended and the season that opens, then the roll where everything is
/// decided at once, then the reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Step {
    /// The Chronique: the year's facts since the last council.
    #[default]
    Chronicle,
    /// The Saison: the year's sky and the grain ledger it leaves.
    Season,
    /// The roll: market, rations, rates and purchases, sealed together by
    /// "Continuer".
    Intendance,
    /// Census: how the people fared under the council just promulgated.
    Report,
    /// The treasury's ledger: what each source brought in.
    Treasury,
    War,
}

impl Step {
    pub const LABELS: [&'static str; 6] = [
        "Chronique",
        "Saison",
        "Intendance",
        "Peuple",
        "Trésor",
        "Extérieur",
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        Self::LABELS[self.index()]
    }
}

/// One line of a table's journal.
#[derive(Clone, Debug)]
pub struct Entry {
    pub year: i32,
    /// Kingdoms this entry concerns (empty = everyone).
    pub about: Vec<Kingdoms>,
    /// The line as everyone reads it.
    pub text: String,
    /// The fuller line the kingdoms it concerns read instead: the figures
    /// that stay inside the walls (an army's headcount, a loot).
    pub secret: Option<String>,
}

impl Entry {
    /// The line as `reader` reads it.
    pub fn told_to(&self, reader: Option<Kingdoms>) -> &str {
        match &self.secret {
            Some(s) if reader.is_some_and(|r| self.about.contains(&r)) => s,
            _ => &self.text,
        }
    }
}

/// One of the council's five sliders.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Peasants,
    Soldiers,
    Customs,
    Sales,
    Income,
}

impl Field {
    pub const ALL: [Field; 5] = [
        Field::Peasants,
        Field::Soldiers,
        Field::Customs,
        Field::Sales,
        Field::Income,
    ];

    pub fn from_u8(n: u8) -> Option<Field> {
        Self::ALL.get(n as usize).copied()
    }

    /// The form field (and element id) of the slider.
    pub fn name(self) -> &'static str {
        match self {
            Field::Peasants => "peasants",
            Field::Soldiers => "soldiers",
            Field::Customs => "customs",
            Field::Sales => "sales",
            Field::Income => "income",
        }
    }
}

/// The council's decision as the sliders currently stand: the rations per
/// head in tenths of a bushel (see [`Council`]) and the tax rates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Draft {
    pub peasants: i32,
    pub soldiers: i32,
    pub customs: i32,
    pub sales: i32,
    pub income: i32,
}

impl Draft {
    /// The ration sliders' upper bounds, in tenths per head: beyond twice the
    /// people's ration immigration barely grows; beyond one and a half times
    /// the army's, efficiency is already maxed — so the sliders stop there,
    /// or earlier when the stocks can't stretch that far. With no men to feed
    /// the army's rate is free: it is what the recruits will be put on.
    pub fn bounds(k: &Kingdom) -> (i32, i32) {
        (
            Self::affordable(k, Council::PEASANTS_FULL * 2, k.mouths()),
            Self::affordable(k, Council::SOLDIERS_FULL * 3 / 2, k.soldiers),
        )
    }

    /// `cap`, or the rate the stocks can pay `heads` at when that is less.
    fn affordable(k: &Kingdom, cap: i32, heads: i32) -> i32 {
        match heads {
            0 => cap,
            heads => cap.min(k.grain_stocks.max(0) * RATION_SCALE / heads),
        }
    }

    /// Where the sliders start: full rations (the army on the rate it was
    /// last put on) and last year's rates.
    pub fn initial(k: &Kingdom) -> Draft {
        let (peasants_max, soldiers_max) = Self::bounds(k);
        let t = k.taxes();
        Draft {
            peasants: Council::PEASANTS_FULL.min(peasants_max),
            soldiers: k.soldiers_ration.min(soldiers_max),
            customs: t.customs,
            sales: t.sales,
            income: t.income,
        }
    }

    pub fn get(self, field: Field) -> i32 {
        match field {
            Field::Peasants => self.peasants,
            Field::Soldiers => self.soldiers,
            Field::Customs => self.customs,
            Field::Sales => self.sales,
            Field::Income => self.income,
        }
    }

    pub fn set(&mut self, field: Field, value: i32) {
        match field {
            Field::Peasants => self.peasants = value,
            Field::Soldiers => self.soldiers = value,
            Field::Customs => self.customs = value,
            Field::Sales => self.sales = value,
            Field::Income => self.income = value,
        }
    }

    /// Inside the sliders' bounds and the stocks (the army is served last:
    /// its rate is what the grain left after the people can pay), and the
    /// tax caps.
    pub fn clamped(self, k: &Kingdom) -> Draft {
        let (peasants_max, soldiers_max) = Self::bounds(k);
        let peasants = self.peasants.clamp(0, peasants_max);
        let mut left = k.clone();
        left.grain_stocks -= peasants * k.mouths() / RATION_SCALE;
        let soldiers_max = Self::affordable(&left, soldiers_max, k.soldiers);
        let t = self.taxes().clamped();
        Draft {
            peasants,
            soldiers: self.soldiers.clamp(0, soldiers_max),
            customs: t.customs,
            sales: t.sales,
            income: t.income,
        }
    }

    pub fn taxes(self) -> Taxes {
        Taxes {
            customs: self.customs,
            sales: self.sales,
            income: self.income,
        }
    }

    pub fn council(self) -> Council {
        Council {
            peasants_ration: self.peasants,
            soldiers_ration: self.soldiers,
            taxes: self.taxes(),
        }
    }
}

/// The war sheet's forecast for one target: the men sent, sampled evenly
/// from 1 to `max`, and what each sample brings back over [`FORECAST_DRAWS`]
/// fights. The sheet's slider reads it by interpolation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WarForecast {
    pub target: Option<Kingdoms>,
    pub max: i32,
    pub sent: Vec<i32>,
    pub rows: Vec<Forecast>,
    /// How wide of the truth the report it rests on may be, in per cent.
    pub spread: i32,
}

pub const FORECAST_DRAWS: usize = 24;
const FORECAST_POINTS: i32 = 16;

#[derive(Clone, Default)]
pub struct Seat {
    /// Token of the connection playing this kingdom; `None` = computer.
    pub owner: Option<u64>,
    /// Where this seigneur stands in the year.
    pub step: Step,
    /// Waiting for the other seigneurs: the council promulgated and its
    /// accounts read (Intendance), or the orders given (Extérieur).
    pub ready: bool,
    /// The expeditions ordered this year, marching together once every
    /// seigneur has given their orders (a computer decides with its
    /// intendance).
    pub planned: Vec<Expedition>,
    /// The council's sliders as last released; `None` = untouched this turn.
    pub draft: Option<Draft>,
    pub demo: Option<YearDemography>,
    pub eco: Option<YearEconomy>,
    /// Headcount and treasury as the last council left them (the Peuple and
    /// Trésor screens count up to these; war and plague may have moved the
    /// kingdom since).
    pub population_after: i32,
    pub treasury_after: i32,
    /// Bushels eaten by the rats at the start of the year (the season report
    /// shows the amount; the kingdom only keeps the rate).
    pub rats: i32,
    /// Bushels in the granaries once the harvest is in, before the year's
    /// dealings (the ledger's starting point).
    pub stocks_at_dawn: i32,
    /// This seigneur's reading of the campaign.
    pub replay: Replay,
    /// The tale of the game's end has been read; the ranking follows.
    pub epilogue_read: bool,
    /// Feedback from the player's last action, and where it shows.
    pub notice: Option<String>,
    pub notice_spot: Spot,
    /// The grain-sale sliders as last released, so the live figures
    /// re-centre on them; `None` = the sheet's defaults.
    pub deal_amount: Option<i32>,
    pub deal_price: Option<i32>,
    /// The spies sent this year, one realm each; they report as the next
    /// year opens.
    pub missions: Vec<Mission>,
    /// What this seat knows of every realm, by kingdom index.
    pub dossiers: [Dossier; 6],
    /// The target whose sheet is open (kingdom number, 0 = barbarians).
    pub target: Option<u8>,
    /// The open sheet is the war form (a kingdom's sheet opens on what is
    /// known of it first).
    pub attack: bool,
    /// What the open war sheet foretells, computed as it opens.
    pub forecast: Option<WarForecast>,
    /// The title held at the last year's end, so a change can be announced;
    /// `None` until the first year opens.
    pub title: Option<PlayerTitle>,
    /// Year the current title was won or fallen to; 0 = held since the start.
    pub title_year: i32,
    /// The year's facts since this seigneur's last council, oldest first
    /// (the Chronique). A fallen seat keeps its last year for good.
    pub news: Vec<News>,
    /// Bumped when a sheet's form concludes, so the sheet opened for the
    /// previous generation closes on its own.
    pub sheet_gen: u16,
    /// The computer's temperament and eye (kept, unused, under a seigneur).
    pub mind: Mind,
}

/// Where a seat's notice shows: under the roll's block it answers, inside
/// the open sheet (an action that failed), or at the top of the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Spot {
    #[default]
    Top,
    Sheet,
    Market,
    Purchases,
}

/// One fact of the year that concerns a seat, told on its Chronique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum News {
    /// Grain sold from this seat's stall; `left` is what stays on the market.
    Sold {
        buyer: Kingdoms,
        amount: i32,
        price: i32,
        left: i32,
    },
    /// This seat's expeditions of the year, as one running total.
    Expeditions {
        count: i32,
        spoils: Spoils,
        men_lost: i32,
        wiped: i32,
    },
    /// Other realms marched on this seat and it survived.
    Attacked {
        /// Every army of the front, with the men it sent.
        armies: Vec<(Kingdoms, i32)>,
        /// Not one army outlived the garrison.
        repelled: bool,
        /// The realm had no army: the people took up arms from the start.
        levy: bool,
        garrison_fallen: i32,
        spoils: Spoils,
    },
    Plague(PlagueEvent),
    Rank {
        before: PlayerTitle,
        now: PlayerTitle,
    },
    /// Something that changed the map or the hierarchy elsewhere.
    Elsewhere(Elsewhere),
    /// This seat's éclaireur sent to `at` last year.
    Scout {
        at: Kingdoms,
        outcome: Scouting,
    },
    /// This seat's agent in `at`, as the year ends.
    Agent {
        at: Kingdoms,
        outcome: Writing,
    },
    /// A spy of `by` was taken at this seat's court: an éclaireur on the
    /// roads, or an agent unmasked among the officials.
    SpyCaught {
        by: Kingdoms,
        agent: bool,
    },
    /// How this seat's realm fell; always the last line.
    Fallen(Fate),
    /// This seat took the imperial crown: the game is won.
    Crowned,
}

impl News {
    /// Told at the Extérieur, where it comes in, rather than in the
    /// Chronique a year later: the spies' returns and the spies taken.
    pub fn is_intelligence(&self) -> bool {
        matches!(
            self,
            News::Scout { .. } | News::Agent { .. } | News::SpyCaught { .. }
        )
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elsewhere {
    RulerDied {
        id: Kingdoms,
        cause: RulerDeathCause,
    },
    /// A war between two other realms, short of an annexation.
    Marched {
        by: Kingdoms,
        on: Kingdoms,
        victory: bool,
        arpents: i32,
    },
    Annexed {
        id: Kingdoms,
        by: Kingdoms,
    },
    Rank {
        id: Kingdoms,
        before: PlayerTitle,
        now: PlayerTitle,
    },
    Crowned(Kingdoms),
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

/// What an agent costs each year: about fifty men of arms.
pub const AGENT_PRICE: i32 = 400;
/// One chance in this, each year, of an agent being unmasked.
const AGENT_UNMASKED: u32 = 8;

/// A spy ordered at the Extérieur, resolved as the year ends: an éclaireur
/// rides to a realm and comes back; an agent is bought among its officials
/// and stays.
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

    fn price(self) -> i32 {
        match self {
            Mission::Scout(_) => SCOUT_PRICE,
            Mission::Agent(_) => AGENT_PRICE,
        }
    }
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
    fn read(k: &Kingdom, year: i32) -> Self {
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
    fn kingdom(&self, id: Kingdoms, jitter: i32) -> Kingdom {
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
    fn read(k: &Kingdom, year: i32, bought: [i32; 6]) -> Self {
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

/// One line of the trade step's list: buy from a seller, or one of the two sales.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deal {
    Buy(Kingdoms),
    Sell,
    Land,
}

impl Deal {
    /// Radio value: the seller's kingdom number, then the two sales.
    pub fn code(self) -> u8 {
        match self {
            Deal::Buy(o) => (o.index() + 1) as u8,
            Deal::Sell => 7,
            Deal::Land => 8,
        }
    }

    pub fn from_code(n: i32) -> Option<Deal> {
        match n {
            7 => Some(Deal::Sell),
            8 => Some(Deal::Land),
            n => Kingdoms::from_number(n).map(Deal::Buy),
        }
    }
}

/// Idle rooms are forgotten: empty lobbies after 10 minutes, anything after an hour.
const EMPTY_ROOM_TTL: u32 = (10 * 60 * 1000 / TICK_MS) as u32;
const IDLE_ROOM_TTL: u32 = (60 * 60 * 1000 / TICK_MS) as u32;
const CODE_LEN: usize = 5;
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// Every table on this server, keyed by share code. Shared by every connection.
#[derive(State, Default)]
#[storage(shared)]
pub struct Rooms {
    pub rooms: BTreeMap<String, Room>,
}

impl Rooms {
    pub fn get(&self, code: &str) -> Option<&Room> {
        self.rooms.get(code)
    }

    /// Tables where this connection holds a seat or is the host.
    pub fn mine(&self, token: u64) -> impl Iterator<Item = &Room> + '_ {
        self.rooms
            .values()
            .filter(move |r| r.host == token || r.seat_of(token).is_some())
    }

    fn create(&mut self, host: u64) -> String {
        let mut rng = rand::thread_rng();
        let code = loop {
            let code: String = (0..CODE_LEN)
                .map(|_| CODE_ALPHABET[rng.gen_range(0..CODE_ALPHABET.len())] as char)
                .collect();
            if !self.rooms.contains_key(&code) {
                break code;
            }
        };
        self.rooms.insert(
            code.clone(),
            Room {
                code: code.clone(),
                host,
                ..Default::default()
            },
        );
        code
    }

    /// Advance every table by one tick; returns true when any view changed.
    pub fn tick(&mut self) -> bool {
        let mut changed = false;
        for room in self.rooms.values_mut() {
            changed |= room.tick();
            room.idle = room.idle.saturating_add(1);
        }
        let before = self.rooms.len();
        self.rooms.retain(|_, r| {
            let empty = r.humans().next().is_none();
            !(r.idle > IDLE_ROOM_TTL
                || (empty && r.stage == Stage::Lobby && r.idle > EMPTY_ROOM_TTL))
        });
        changed || self.rooms.len() != before
    }
}

/// Upper-case a user-typed or URL code and drop anything outside the alphabet.
pub fn normalize_code(raw: &str) -> String {
    raw.trim()
        .to_ascii_uppercase()
        .chars()
        .filter(|c| CODE_ALPHABET.contains(&(*c as u8)))
        .take(CODE_LEN)
        .collect()
}

/// One game table.
#[derive(Default)]
pub struct Room {
    pub code: String,
    /// Token of the connection that created the table.
    pub host: u64,
    /// Ticks since the last player action.
    idle: u32,
    pub stage: Stage,
    pub seats: [Seat; 6],
    pub game: EmpireGame,
    pub log: Vec<Entry>,
    /// Every kingdom's surface at the start of each year (and once more at the
    /// end): the land curve of the game.
    pub history: Vec<[i32; 6]>,
    pub phase: Phase,
    /// The campaign's fronts, fought at the march and applied to the map once
    /// every seigneur has read them.
    pub battles: Vec<Fought>,
    /// The last campaign as the heralds tell it, until the next one marches.
    pub rumours: Vec<Rumour>,
    /// Bushels bought this year, by buyer then seller: what the agents read
    /// in the registers.
    bought: [[i32; 6]; 6],
}

impl Room {
    pub fn seat(&self, id: Kingdoms) -> &Seat {
        &self.seats[id.index()]
    }

    fn seat_mut(&mut self, id: Kingdoms) -> &mut Seat {
        &mut self.seats[id.index()]
    }

    pub fn seat_of(&self, token: u64) -> Option<Kingdoms> {
        KINGDOMS
            .into_iter()
            .find(|&id| self.seat(id).owner == Some(token))
    }

    /// Kingdoms played by humans, in canonical order.
    pub fn humans(&self) -> impl Iterator<Item = Kingdoms> + '_ {
        KINGDOMS
            .into_iter()
            .filter(|&id| self.seat(id).owner.is_some())
    }

    /// Whether `id` has a screen to play right now: their own intendance or
    /// their war orders, until they are ready (nobody plays while the armies
    /// march).
    pub fn playing(&self, id: Kingdoms) -> bool {
        self.stage == Stage::Playing
            && self.phase != Phase::Campaign
            && !self.is_computer(id)
            && !self.seat(id).ready
            && !self.game.kingdom(id).is_dead
    }

    /// Expeditions `id` may still order this year (original rule: one per
    /// four nobles, plus one).
    pub fn expeditions_left(&self, id: Kingdoms) -> i32 {
        self.game.kingdom(id).nobles / 4 + 1 - self.seat(id).planned.len() as i32
    }

    /// Men of `id` not yet ordered on an expedition.
    pub fn garrison(&self, id: Kingdoms) -> i32 {
        let ordered: i32 = self.seat(id).planned.iter().map(|e| e.soldiers).sum();
        self.game.kingdom(id).soldiers - ordered
    }

    /// The expedition `id` has ordered on `target`, with its place in the orders.
    pub fn planned_on(
        &self,
        id: Kingdoms,
        target: Option<Kingdoms>,
    ) -> Option<(usize, &Expedition)> {
        self.seat(id)
            .planned
            .iter()
            .enumerate()
            .find(|(_, e)| e.target == target)
    }

    /// Men `id` may send on `target`: the garrison, plus the army already
    /// ordered there (settling the target again replaces it).
    pub fn available(&self, id: Kingdoms, target: Option<Kingdoms>) -> i32 {
        self.garrison(id) + self.planned_on(id, target).map_or(0, |(_, e)| e.soldiers)
    }

    /// Foretell what `id` would bring back from `target`, for every size of
    /// army it can send. The bands are foretold on sight; a realm only as its
    /// report tells it, wider of the truth as the report ages — and not at
    /// all without one, or on one too old.
    pub fn war_forecast(&self, id: Kingdoms, target: Option<Kingdoms>) -> Option<WarForecast> {
        let mut games = Vec::new();
        let mut spread = 0;
        if let Some(t) = target {
            let report = self.dossier(id, t).report?;
            spread = report.spread(self.game.year)?;
            let mut rng = rand::thread_rng();
            games.extend((0..FORECAST_DRAWS).map(|_| {
                let mut g = self.game.clone();
                *g.kingdom_mut(t) = report.kingdom(t, rng.gen_range(-spread..=spread));
                g
            }));
        }
        let draws = || {
            if target.is_some() {
                games.iter().collect::<Vec<_>>()
            } else {
                vec![&self.game; FORECAST_DRAWS]
            }
        };
        let max = self.available(id, target).max(1);
        let points = max.min(FORECAST_POINTS);
        let sent: Vec<i32> = (0..points)
            .map(|i| {
                if points == 1 {
                    max
                } else {
                    1 + (i64::from(i) * i64::from(max - 1) / i64::from(points - 1)) as i32
                }
            })
            .collect();
        let rows = sent
            .iter()
            .map(|&n| forecast(draws(), target, id, n))
            .collect();
        Some(WarForecast {
            target,
            max,
            sent,
            rows,
            spread,
        })
    }

    pub fn dossier(&self, id: Kingdoms, on: Kingdoms) -> &Dossier {
        &self.seat(id).dossiers[on.index()]
    }

    /// How a computer's council leans; `None` under a seigneur.
    pub fn temper(&self, id: Kingdoms) -> Option<Temper> {
        self.is_computer(id).then(|| self.seat(id).mind.temper)
    }

    /// Whether `id` has an éclaireur leaving for `on` this year.
    pub fn scout_ordered(&self, id: Kingdoms, on: Kingdoms) -> bool {
        self.seat(id).missions.contains(&Mission::Scout(on))
    }

    /// Whether `id` is buying an agent in `on` this year.
    pub fn agent_ordered(&self, id: Kingdoms, on: Kingdoms) -> bool {
        self.seat(id).missions.contains(&Mission::Agent(on))
    }

    /// Send an éclaireur to `on`, or call back the one ordered there (the
    /// price comes back with him).
    fn toggle_scout(&mut self, id: Kingdoms, on: Kingdoms) -> Result<(), String> {
        if on == id || self.game.kingdom(on).is_dead {
            return Err("Il n'y a rien à éclairer là.".to_string());
        }
        self.toggle_mission(id, Mission::Scout(on), "Un éclaireur")
    }

    /// Buy an agent in `on`, or give up the purchase (the price comes back);
    /// once he is in place, dismiss him (the year's pay is spent).
    fn toggle_agent(&mut self, id: Kingdoms, on: Kingdoms) -> Result<(), String> {
        if on == id || self.game.kingdom(on).is_dead {
            return Err("Il n'y a personne à acheter là.".to_string());
        }
        if self.dossier(id, on).agent {
            self.seat_mut(id).dossiers[on.index()].agent = false;
            return Ok(());
        }
        let fresh = self
            .dossier(id, on)
            .report
            .is_some_and(|r| r.year == self.game.year);
        if !fresh && !self.agent_ordered(id, on) {
            return Err("Un agent ne s'achète que sur un rapport de l'année.".to_string());
        }
        self.toggle_mission(id, Mission::Agent(on), "Un agent")
    }

    /// Order a mission, paid up front, or cancel it and take the price back.
    fn toggle_mission(&mut self, id: Kingdoms, m: Mission, who: &str) -> Result<(), String> {
        if self.seat(id).missions.contains(&m) {
            self.seat_mut(id).missions.retain(|&o| o != m);
            self.game.kingdom_mut(id).treasury += m.price();
            return Ok(());
        }
        let k = self.game.kingdom_mut(id);
        if k.treasury < m.price() {
            return Err(format!(
                "{who} coûte {} francs ; le trésor n'en a que {}.",
                m.price(),
                k.treasury
            ));
        }
        k.treasury -= m.price();
        self.seat_mut(id).missions.push(m);
        Ok(())
    }

    /// The spies report as the Extérieur opens. Each éclaireur reads the realm as
    /// it stands, or is taken (one in six); the agents bought this year take
    /// their place, then every agent is unmasked (one in eight) or, paid for
    /// the year, writes his letter — a taken spy is told to that seigneur and
    /// to everyone reading the journal.
    fn resolve_missions(&mut self) {
        for id in KINGDOMS {
            let missions = std::mem::take(&mut self.seat_mut(id).missions);
            if self.game.kingdom(id).is_dead {
                continue;
            }
            let mut hired = Vec::new();
            for m in missions {
                match m {
                    Mission::Scout(on) => self.resolve_scout(id, on),
                    Mission::Agent(on) => {
                        self.seat_mut(id).dossiers[on.index()].agent = true;
                        hired.push(on);
                    }
                }
            }
            for on in KINGDOMS {
                if self.dossier(id, on).agent {
                    self.resolve_agent(id, on, !hired.contains(&on));
                }
            }
        }
    }

    fn resolve_scout(&mut self, id: Kingdoms, on: Kingdoms) {
        let year = self.game.year;
        let outcome = if self.game.kingdom(on).is_dead {
            Scouting::Gone
        } else if rand::thread_rng().gen_range(0..SCOUT_CAUGHT) == 0 {
            self.spy_taken(id, on, false);
            Scouting::Caught
        } else {
            let r = Report::read(self.game.kingdom(on), year);
            self.seat_mut(id).dossiers[on.index()].report = Some(r);
            Scouting::Back {
                garrison: r.garrison,
            }
        };
        self.report(id, News::Scout { at: on, outcome });
    }

    /// An agent's year: gone with the realm, unmasked, unpaid, or writing.
    /// `standing` agents pay their year now; a new one paid on purchase.
    fn resolve_agent(&mut self, id: Kingdoms, on: Kingdoms, standing: bool) {
        let year = self.game.year;
        let outcome = if self.game.kingdom(on).is_dead {
            Writing::Gone
        } else if rand::thread_rng().gen_range(0..AGENT_UNMASKED) == 0 {
            self.spy_taken(id, on, true);
            Writing::Unmasked
        } else if standing && self.game.kingdom(id).treasury < AGENT_PRICE {
            Writing::Unpaid
        } else {
            if standing {
                self.game.kingdom_mut(id).treasury -= AGENT_PRICE;
            }
            let k = self.game.kingdom(on);
            let report = Report::read(k, year);
            let ledger = Ledger::read(k, year, self.bought[on.index()]);
            let d = &mut self.seat_mut(id).dossiers[on.index()];
            let levied = d.report.map(|p| report.garrison - p.garrison);
            let spent = d.ledger.map(|p| p.treasury - ledger.treasury);
            d.report = Some(report);
            d.ledger = Some(ledger);
            Writing::Letter {
                ledger,
                levied,
                spent,
            }
        };
        if !matches!(outcome, Writing::Letter { .. }) {
            self.seat_mut(id).dossiers[on.index()].agent = false;
        }
        self.report(id, News::Agent { at: on, outcome });
    }

    /// A spy of `id` taken in `on`: `id` knows, `on` knows who sent him,
    /// and the journal tells everyone.
    fn spy_taken(&mut self, id: Kingdoms, on: Kingdoms, agent: bool) {
        let year = self.game.year;
        self.seat_mut(id).dossiers[on.index()].caught = Some(year);
        let line = if agent {
            format!(
                "Un agent de la {} a été démasqué en {}.",
                id.name(),
                on.name()
            )
        } else {
            format!(
                "Un éclaireur de la {} a été pris en {}.",
                id.name(),
                on.name()
            )
        };
        self.journal([id, on], line);
        self.report(on, News::SpyCaught { by: id, agent });
    }

    /// Whether `id` may act on `step` right now.
    pub fn may_act(&self, id: Kingdoms, step: Step) -> bool {
        self.playing(id) && self.seat(id).step == step
    }

    pub fn is_computer(&self, id: Kingdoms) -> bool {
        self.seat(id).owner.is_none()
    }

    /// The council's sliders for `id`, as last released or where they start.
    pub fn draft(&self, id: Kingdoms) -> Draft {
        self.seat(id)
            .draft
            .unwrap_or_else(|| Draft::initial(self.game.kingdom(id)))
    }

    /// Append a journal entry; `about` names the kingdoms it concerns so the
    /// viewer's own news can be marked.
    pub fn journal(&mut self, about: impl IntoIterator<Item = Kingdoms>, line: impl Into<String>) {
        self.confide(about, line, None);
    }

    /// Append a journal entry whose fuller telling is kept for the kingdoms
    /// it concerns; everyone else reads `line`.
    fn confide(
        &mut self,
        about: impl IntoIterator<Item = Kingdoms>,
        line: impl Into<String>,
        secret: Option<String>,
    ) {
        self.log.push(Entry {
            year: self.game.year,
            about: about.into_iter().collect(),
            text: line.into(),
            secret,
        });
        if self.log.len() > JOURNAL_LEN {
            self.log.remove(0);
        }
    }

    fn note_at(&mut self, id: Kingdoms, spot: Spot, msg: impl Into<String>) {
        let seat = self.seat_mut(id);
        seat.notice = Some(msg.into());
        seat.notice_spot = spot;
    }

    /// Close `id`'s open sheet (its form concluded).
    fn close_sheet(&mut self, id: Kingdoms) {
        let seat = self.seat_mut(id);
        seat.sheet_gen = seat.sheet_gen.wrapping_add(1);
    }

    /// Add a fact to `id`'s Chronique.
    fn report(&mut self, id: Kingdoms, news: News) {
        self.seat_mut(id).news.push(news);
    }

    /// Tell every living seat but `except` what happened elsewhere.
    fn report_others(&mut self, except: &[Kingdoms], fact: Elsewhere) {
        for id in KINGDOMS {
            if !except.contains(&id) && !self.game.kingdom(id).is_dead {
                self.report(id, News::Elsewhere(fact));
            }
        }
    }

    /// A grain sale concluded on the market: told to the seller, and kept
    /// for the registers.
    fn report_sale(&mut self, seller: Kingdoms, buyer: Kingdoms, amount: i32) {
        self.bought[buyer.index()][seller.index()] += amount;
        let s = self.game.kingdom(seller);
        let news = News::Sold {
            buyer,
            amount,
            price: s.grain_price.min(MAX_GRAIN_PRICE),
            left: s.grain_to_sell,
        };
        self.report(seller, news);
    }

    fn release(&mut self, id: Kingdoms) {
        self.seat_mut(id).owner = None;
        let k = self.game.kingdom_mut(id);
        k.is_player = false;
        k.player_name = id.default_king_name().to_string();
    }

    // -- year & turn order ---------------------------------------------------

    fn surfaces(&self) -> [i32; 6] {
        std::array::from_fn(|i| self.game.kingdoms[i].surface)
    }

    fn begin_year(&mut self) {
        self.history.push(self.surfaces());
        self.bought = [[0; 6]; 6];
        let weather = self.game.random_weather();
        self.journal([], weather.sentence());
        self.game.open_market();
        for id in KINGDOMS {
            let k = self.game.kingdom_mut(id);
            if k.is_dead {
                continue;
            }
            apply_seed_grain(k);
            let before_rats = k.grain_stocks;
            apply_rat_loss_rate(k);
            let rats = before_rats - k.grain_stocks;
            apply_grain_harvest(k, weather);
            let title = k.title();
            let stocks = k.grain_stocks;
            let seat = self.seat_mut(id);
            seat.rats = rats;
            seat.stocks_at_dawn = stocks;
            seat.title.get_or_insert(title);
        }
        self.phase = Phase::Intendance;
        // A computer's council sits the moment the year opens: its Chronique
        // covers the year (a seigneur who takes the seat over inherits it).
        for id in KINGDOMS {
            if self.is_computer(id) && !self.game.kingdom(id).is_dead {
                self.seat_mut(id).news.clear();
            }
        }
        for id in KINGDOMS {
            if self.game.kingdom(id).is_dead {
                continue;
            }
            if self.is_computer(id) {
                self.run_computer_intendance(id);
            } else {
                // The year opens on its Chronique; the first has nothing to
                // tell yet and opens on the season.
                let seat = self.seat_mut(id);
                seat.step = if seat.demo.is_some() {
                    Step::Chronicle
                } else {
                    Step::Season
                };
                seat.ready = false;
                seat.draft = None;
                seat.notice = None;
                seat.deal_amount = None;
                seat.deal_price = None;
                seat.target = None;
                seat.attack = false;
                seat.forecast = None;
                seat.planned.clear();
            }
        }
        self.try_open_exterior();
    }

    /// Whether a living seigneur has still to finish the current phase.
    fn someone_waits(&self) -> bool {
        self.humans()
            .any(|id| !self.game.kingdom(id).is_dead && !self.seat(id).ready)
    }

    /// Once every living seigneur has validated their purchases, everyone
    /// gives their war orders at the same time.
    fn try_open_exterior(&mut self) {
        if self.stage != Stage::Playing || self.phase != Phase::Intendance || self.someone_waits() {
            return;
        }
        self.phase = Phase::Exterieur;
        // The éclaireurs and letters come in as the Extérieur opens: every
        // court has held its intendance, the figures are those the campaign
        // will be fought with.
        self.resolve_missions();
        for id in KINGDOMS {
            if self.game.kingdom(id).is_dead {
                continue;
            }
            if self.is_computer(id) {
                self.run_computer_war(id);
                continue;
            }
            let seat = self.seat_mut(id);
            seat.step = Step::War;
            seat.ready = false;
            seat.notice = None;
            seat.target = None;
            seat.attack = false;
            seat.forecast = None;
            seat.planned.clear();
        }
        self.try_march();
    }

    /// Once every living seigneur has given their orders, the armies march:
    /// every front is fought at once, then replayed one after the other. A
    /// year without a single expedition has no campaign to show.
    fn try_march(&mut self) {
        if self.stage != Stage::Playing || self.phase != Phase::Exterieur || self.someone_waits() {
            return;
        }
        self.phase = Phase::Campaign;
        self.rumours.clear();
        let mut orders = Vec::new();
        for id in KINGDOMS {
            orders.append(&mut self.seat_mut(id).planned);
        }
        let fought = march(&mut self.game, orders);
        if fought.is_empty() {
            self.end_year();
            return;
        }
        // Everyone reads the campaign at their own pace; the year turns once
        // every living seigneur has asked for it. A computer has read.
        for id in KINGDOMS {
            let replay = if self.is_computer(id) {
                Replay::told(fought.len())
            } else {
                Replay::opening()
            };
            let seat = self.seat_mut(id);
            seat.ready = false;
            seat.replay = replay;
        }
        for f in fought {
            self.start_battle(f);
        }
    }

    /// How `me` reads the campaign: their own clock, or — with no seat at the
    /// table — everything told.
    pub fn replay(&self, me: Option<Kingdoms>) -> Replay {
        match me {
            Some(id) => self.seat(id).replay,
            None => Replay::told(self.battles.len()),
        }
    }

    /// Whether `id` may ask to turn the year: a living seigneur who has not
    /// yet, once they have read the whole campaign.
    pub fn may_continue(&self, id: Kingdoms) -> bool {
        self.stage == Stage::Playing
            && self.phase == Phase::Campaign
            && self.seat(id).replay.done()
            && !self.is_computer(id)
            && !self.seat(id).ready
            && !self.game.kingdom(id).is_dead
    }

    /// Living seigneurs still reading the campaign.
    pub fn still_reading(&self) -> impl Iterator<Item = Kingdoms> + '_ {
        self.humans()
            .filter(|&id| !self.game.kingdom(id).is_dead && !self.seat(id).ready)
    }

    /// A tap on the campaign screen skips ahead on `id`'s own clock: the
    /// order of battle gives way to the front at once, a front still moving
    /// jumps to its end, a verdict gives way to the next order of battle;
    /// once every front has been told, the tap is `id`'s "continue".
    pub fn tap_campaign(&mut self, id: Kingdoms) {
        if self.stage != Stage::Playing || self.phase != Phase::Campaign {
            return;
        }
        let n = self.battles.len();
        let replay = &mut self.seats[id.index()].replay;
        match replay.staging {
            Staging::Schema(_) => replay.staging = Staging::Fight(FRAME_TICKS),
            Staging::Fight(_) => replay.settle(&self.battles[replay.current]),
            Staging::Verdict(_) => replay.next_front(n),
            Staging::Done => self.continue_campaign(id),
        }
    }

    /// The year ends once every seigneur has read the campaign and asked for
    /// it: nobody's screen is taken away while they read. Only then do the
    /// fronts' outcomes reach the map, so nobody's screen tells them early.
    fn continue_campaign(&mut self, id: Kingdoms) {
        if !self.may_continue(id) {
            return;
        }
        self.seat_mut(id).ready = true;
        if self.someone_waits() {
            return;
        }
        for i in 0..self.battles.len() {
            self.finish_battle(i);
        }
        if !self.check_over() {
            self.end_year();
        }
    }

    /// The "next" button of `id`'s current step.
    fn advance(&mut self, id: Kingdoms) {
        if !self.playing(id) {
            return;
        }
        let seat = self.seat_mut(id);
        seat.notice = None;
        match seat.step {
            Step::Chronicle => seat.step = Step::Season,
            Step::Season => seat.step = Step::Intendance,
            Step::Intendance => {
                // "Continuer" is final: the council is promulgated, its
                // census and ledger follow at once, and the Chronique
                // starts over from this council.
                self.promulgate(id);
                let seat = self.seat_mut(id);
                seat.news.clear();
                seat.step = Step::Report;
            }
            Step::Report => seat.step = Step::Treasury,
            Step::Treasury => {
                // The accounts read, the seigneur waits for the others.
                seat.ready = true;
                self.try_open_exterior();
            }
            Step::War => {
                // The orders given, the seigneur waits for the others.
                seat.ready = true;
                self.try_march();
            }
        }
    }

    /// Promulgate the council's decision as the roll's sliders stand:
    /// rations served, taxes levied, the year's economy applied.
    fn promulgate(&mut self, id: Kingdoms) {
        let weather = self.game.weather;
        let k = self.game.kingdom(id);
        let council = self.draft(id).clamped(k).council();
        let k = self.game.kingdom_mut(id);
        apply_taxes(k, council.taxes);
        let demo = apply_feed(k, council);
        let eco = economy_report(k, weather, demo.immigrants);
        apply_economy(k, &eco);
        let (population_after, treasury_after) = (k.population(), k.treasury);
        let delta = demo.population_delta();
        self.journal(
            [id],
            format!("La {} a {}.", id.name(), subjects_delta(delta, "ses")),
        );
        let seat = self.seat_mut(id);
        seat.draft = None;
        seat.demo = Some(demo);
        seat.eco = Some(eco);
        seat.population_after = population_after;
        seat.treasury_after = treasury_after;
    }

    fn end_year(&mut self) {
        self.battles.clear();
        for id in KINGDOMS {
            // A computer's abstract growth knows no famine (original line 206).
            let starvation_deaths = self
                .seat(id)
                .demo
                .as_ref()
                .map_or(0, |d| d.starvation_victims);
            let computer = self.is_computer(id);
            let k = self.game.kingdom_mut(id);
            if k.is_dead {
                continue;
            }
            let title = k.full_title();
            let (plague, death) = check_random_events(k, starvation_deaths, computer);
            if let Some(plague) = plague {
                self.journal(
                    [id],
                    format!(
                        "La peste ravage la {} : {} morts.",
                        id.name(),
                        plague.serfs_killed
                            + plague.merchants_killed
                            + plague.soldiers_killed
                            + plague.nobles_killed
                    ),
                );
                self.report(id, News::Plague(plague));
            }
            if let Some(cause) = death {
                self.journal(
                    [id],
                    format!(
                        "{title} {}. Les autres nations ont envoyé des représentants aux funérailles.",
                        death_fr(&cause)
                    ),
                );
                self.report(id, News::Fallen(Fate::RulerDied(cause)));
                self.report_others(&[id], Elsewhere::RulerDied { id, cause });
            }
            self.judge_title(id);
        }
        if self.check_over() {
            return;
        }
        self.game.increment_year();
        self.begin_year();
    }

    /// Announce a rank won or lost since the last year's end.
    fn judge_title(&mut self, id: Kingdoms) {
        let k = self.game.kingdom(id);
        if k.is_dead {
            return;
        }
        let now = k.title();
        let year = self.game.year;
        let seat = self.seat_mut(id);
        let Some(before) = seat.title.replace(now) else {
            return;
        };
        if before == now {
            return;
        }
        seat.title_year = year;
        if now == PlayerTitle::Emperor {
            // The imperial crown ends the game: the original's only ending.
            seat.news.push(News::Crowned);
            self.report_others(&[id], Elsewhere::Crowned(id));
            let k = self.game.kingdom(id);
            let line = format!(
                "{} de {} est couronné {}.",
                k.player_name,
                k.name(),
                k.title_name()
            );
            self.journal([id], line);
            return;
        }
        seat.news.push(News::Rank { before, now });
        self.report_others(&[id], Elsewhere::Rank { id, before, now });
        let k = self.game.kingdom(id);
        let line = if now > before {
            format!(
                "{} de {} est fait {}.",
                k.player_name,
                k.name(),
                k.title_name()
            )
        } else {
            format!(
                "{} de {} n'est plus que {}.",
                k.player_name,
                k.name(),
                k.title_name()
            )
        };
        self.journal([id], line);
    }

    /// The game ends on the imperial crown — judged with the titles at
    /// year's end, human or computer, as in the original — or when no human
    /// is left to play it. A lone survivor plays on until the crown.
    fn check_over(&mut self) -> bool {
        let humans_alive = self.humans().any(|id| !self.game.kingdom(id).is_dead);
        if humans_alive && self.emperor().is_none() {
            return false;
        }
        self.stage = Stage::Over;
        self.battles.clear();
        self.history.push(self.surfaces());
        let why = match self.emperor() {
            Some(id) => format!(
                "{} de {} est couronné Empereur",
                self.game.kingdom(id).player_name,
                id.name()
            ),
            None => "plus aucun seigneur en vie".to_string(),
        };
        self.journal([], format!("La partie est terminée : {why}."));
        true
    }

    /// The realm crowned at the last judgement of the titles, if any.
    pub fn emperor(&self) -> Option<Kingdoms> {
        self.game
            .alive_kingdoms()
            .into_iter()
            .find(|&id| self.seat(id).title == Some(PlayerTitle::Emperor))
    }

    // -- ticker: battle replay -----------------------------------------------

    /// Advance every seat's clock by one tick: the order of battle gives way
    /// to its front, a front being fought moves a frame and reaches its
    /// verdict at its last, a verdict gives way to the next order of battle.
    /// Returns true when a view changed.
    pub fn tick(&mut self) -> bool {
        if self.stage != Stage::Playing || self.phase != Phase::Campaign {
            return false;
        }
        let n = self.battles.len();
        let mut changed = false;
        for id in KINGDOMS {
            let replay = &mut self.seats[id.index()].replay;
            match replay.staging {
                Staging::Schema(t) if t > 1 => replay.staging = Staging::Schema(t - 1),
                Staging::Schema(_) => {
                    replay.staging = Staging::Fight(FRAME_TICKS);
                    changed = true;
                }
                Staging::Fight(t) if t > 1 => replay.staging = Staging::Fight(t - 1),
                Staging::Fight(_) => {
                    replay.cursor += 1;
                    let fought = &self.battles[replay.current];
                    if replay.finished(fought) {
                        replay.settle(fought);
                    } else {
                        replay.staging = Staging::Fight(FRAME_TICKS);
                    }
                    changed = true;
                }
                Staging::Verdict(t) if t > 1 => replay.staging = Staging::Verdict(t - 1),
                Staging::Verdict(_) => {
                    replay.next_front(n);
                    changed = true;
                }
                Staging::Done => {}
            }
        }
        changed
    }

    /// The computer's council as the year opens: its growth and trade. Its
    /// war waits for the Extérieur, like everyone's — see `run_computer_war`.
    fn run_computer_intendance(&mut self, id: Kingdoms) {
        let decision = plan_ai_intendance(&mut self.game, id);
        if let Some((amount, price)) = decision.grain_listed {
            self.journal(
                [id],
                format!(
                    "La {} mettra {amount} boisseaux en vente à {price} le cent l'an prochain.",
                    id.name()
                ),
            );
        }
        if let Some((seller, amount)) = decision.grain_bought {
            self.journal(
                [id, seller],
                format!(
                    "La {} achète {amount} boisseaux à la {}.",
                    id.name(),
                    seller.name()
                ),
            );
            self.report_sale(seller, id, amount);
        }
    }

    /// The computer's war orders as the Extérieur opens, on the éclaireur's
    /// report of this very day; the spy it sends is ordered like a
    /// seigneur's (taken, journaled and told alike).
    fn run_computer_war(&mut self, id: Kingdoms) {
        let year = self.game.year;
        let mut mind = self.seat(id).mind;
        mind.seen = mind.eye.and_then(|on| {
            let report = self.dossier(id, on).report.filter(|r| r.year == year)?;
            Some(Seen {
                garrison: report.garrison,
                efficiency: report.efficiency,
                serfs: report.serfs,
            })
        });
        let war = plan_ai_war(&mut self.game, id, &mut mind);
        self.seat_mut(id).mind = mind;
        if let Some(on) = war.scout {
            self.seat_mut(id).missions.push(Mission::Scout(on));
        }
        let barbarians = war
            .barbarian_attacks
            .into_iter()
            .map(|soldiers| Expedition {
                attacker: id,
                target: None,
                soldiers,
            });
        let kingdoms = war
            .kingdom_attacks
            .into_iter()
            .map(|(target, soldiers)| Expedition {
                attacker: id,
                target: Some(target),
                soldiers,
            });
        self.seat_mut(id).planned = barbarians.chain(kingdoms).collect();
    }

    /// Add an expedition to a seigneur's orders.
    fn order(&mut self, e: Expedition) -> Result<(), String> {
        let id = e.attacker;
        if self.garrison(id) < 1 {
            return Err("Vous n'avez plus d'hommes d'armes.".into());
        }
        if self.expeditions_left(id) < 1 {
            return Err("Vos nobles ne peuvent mener davantage d'expéditions cette année.".into());
        }
        match e.target {
            Some(t) if t == id || self.game.kingdom(t).is_dead => {
                return Err("Ce royaume n'est plus.".into());
            }
            Some(_) if self.game.year < 3 => {
                return Err("Nul ne peut attaquer un autre royaume avant la 3ème année.".into());
            }
            _ => {}
        }
        let soldiers = e.soldiers.clamp(1, self.garrison(id));
        self.seat_mut(id).planned.push(Expedition { soldiers, ..e });
        Ok(())
    }

    /// Add a fought front to the campaign, replayed over as many ticks as it
    /// had rounds, within bounds.
    fn start_battle(&mut self, fought: Fought) {
        let foe = match fought.target {
            Some(t) => format!("la {}", t.name()),
            None => "les Barbares".to_string(),
        };
        for e in fought.expeditions() {
            let attacker = self.game.kingdom(e.attacker).titled_name();
            self.confide(
                [e.attacker].into_iter().chain(e.target),
                format!("{attacker} marche sur {foe}."),
                Some(format!(
                    "{attacker} marche sur {foe} avec {}.",
                    hommes_darmes(e.soldiers)
                )),
            );
        }
        let rounds = &fought.result.rounds;
        let n = rounds.len().clamp(BATTLE_MIN_FRAMES, BATTLE_MAX_FRAMES);
        let rounds = sample(rounds, n);
        self.battles.push(Fought {
            result: FrontResult {
                rounds,
                ..fought.result
            },
            ..fought
        });
    }

    /// Front `i` becomes real: its outcome reaches the map and is told.
    fn finish_battle(&mut self, i: usize) {
        let fought = self.battles[i].clone();
        let r = &fought.result;
        let annexed = fought.annexed_by();
        let realm = fought.target.map(|t| self.game.kingdom(t).surface);
        apply_battle(&mut self.game, &fought);
        let audience: Vec<Kingdoms> = r
            .armies
            .iter()
            .map(|a| a.attacker)
            .chain(fought.target)
            .collect();
        for a in &r.armies {
            let mut spoils = a.spoils();
            // The conqueror's share is everything the others left.
            if annexed == Some(a.attacker) {
                let others: i32 = r.armies.iter().map(|o| o.spoils().arpents).sum();
                spoils.arpents += realm.unwrap_or(0) - others;
            }
            self.tally_expedition(a.attacker, a.victory, spoils, a.lost());
            let k = self.game.kingdom(a.attacker);
            let name = k.titled_name();
            let line = verdict(&name, k.currency(), a);
            self.confide(
                audience.iter().copied(),
                verdict_heard(&name, a),
                Some(line),
            );
            self.rumours.push(Rumour {
                year: self.game.year,
                attacker: a.attacker,
                target: fought.target,
                victory: a.victory,
                arpents: spoils.arpents,
                annexed: annexed == Some(a.attacker),
            });
            if let Some(on) = fought.target.filter(|_| annexed.is_none()) {
                self.report_others(
                    &audience,
                    Elsewhere::Marched {
                        by: a.attacker,
                        on,
                        victory: a.victory,
                        arpents: spoils.arpents,
                    },
                );
            }
        }
        if let Some(t) = fought.target {
            match annexed {
                Some(by) => {
                    self.report(t, News::Fallen(Fate::Annexed(by)));
                    self.report_others(&[by, t], Elsewhere::Annexed { id: t, by });
                    self.journal(
                        audience.iter().copied(),
                        format!(
                            "Le pays de {} est conquis ! Ses serfs jurent fidélité à {}.",
                            self.game.kingdom(t).full_title(),
                            self.game.kingdom(by).titled_name()
                        ),
                    );
                }
                None => self.report(
                    t,
                    News::Attacked {
                        armies: r.armies.iter().map(|a| (a.attacker, a.sent)).collect(),
                        repelled: !r.garrison_fell(),
                        levy: fought.levy(),
                        garrison_fallen: r.garrison_fallen(),
                        spoils: r.spoils(),
                    },
                ),
            }
        }
    }
}

impl Room {
    /// Fold one expedition into the attacker's running total of the year.
    fn tally_expedition(&mut self, id: Kingdoms, won: bool, spoils: Spoils, men_lost: i32) {
        let news = &mut self.seat_mut(id).news;
        let total = news.iter_mut().rev().find_map(|n| match n {
            News::Expeditions {
                count,
                spoils,
                men_lost,
                wiped,
            } => Some((count, spoils, men_lost, wiped)),
            _ => None,
        });
        match total {
            Some((count, total_spoils, total_lost, wiped)) => {
                *count += 1;
                total_spoils.add(&spoils);
                *total_lost += men_lost;
                *wiped += i32::from(!won);
            }
            None => news.push(News::Expeditions {
                count: 1,
                spoils,
                men_lost,
                wiped: i32::from(!won),
            }),
        }
    }
}

/// One army's line of the journal.
/// The verdict as the other courts hear it: the outcome and the land, no
/// loot and no headcount.
fn verdict_heard(attacker: &str, a: &Army) -> String {
    let arpents = a.spoils().arpents;
    if !a.victory {
        format!("{attacker} perd toute son expédition sans garder un arpent.")
    } else if arpents == 0 {
        format!("{attacker} repousse l'ennemi sans gagner un arpent.")
    } else {
        format!("{attacker} gagne : {} arpents conquis.", fmt(arpents))
    }
}

fn verdict(attacker: &str, cur: &str, a: &Army) -> String {
    if !a.victory {
        // The expedition is wiped out: whatever it overran is lost with it.
        return format!("{attacker} perd toute son expédition sans garder un arpent.");
    }
    let s = a.spoils();
    if s.arpents == 0 {
        return format!("{attacker} repousse l'ennemi sans gagner un arpent.");
    }
    let mut m = format!("{attacker} gagne : {} arpents conquis.", fmt(s.arpents));
    let mut goods = goods_fr(&s, cur);
    goods.extend(people_fr(&s, Side::Attacker));
    if !goods.is_empty() {
        m.push_str(&format!(" Butin : {}.", goods.join(", ")));
    }
    let buildings = buildings_fr(&s);
    if !buildings.is_empty() {
        m.push_str(&format!(" {}.", buildings.join(", ")));
    }
    m
}

/// The goods carried off, the land aside: "1 200 livres", "3 000 boisseaux" —
/// zeros left out.
pub fn goods_fr(s: &Spoils, cur: &str) -> Vec<String> {
    let mut v = Vec::new();
    if s.treasury > 0 {
        v.push(coins(s.treasury, cur));
    }
    if s.grain > 0 {
        v.push(format!("{} boisseaux", fmt(s.grain)));
    }
    v
}

/// Whose story the people met on the way are told in.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Attacker,
    Defender,
}

/// The fate of those met on the way — "74 serfs ralliés", "1 marchand tué"
/// (or, told from the defender's side, "passés à l'ennemi", "tombés") — zeros
/// left out.
pub fn people_fr(s: &Spoils, side: Side) -> Vec<String> {
    let fates = match side {
        Side::Attacker => [("rallié", "ralliés"), ("tué", "tués")],
        Side::Defender => [
            ("passé à l'ennemi", "passés à l'ennemi"),
            ("tombé", "tombés"),
        ],
    };
    let mut v = Vec::new();
    for (p, (one_fate, many_fate)) in [&s.rallied, &s.killed].into_iter().zip(fates) {
        for (n, one, many) in [
            (p.peasants, "serf", "serfs"),
            (p.merchants, "marchand", "marchands"),
            (p.nobles, "noble", "nobles"),
        ] {
            match n {
                0 => {}
                1 => v.push(format!("1 {one} {one_fate}")),
                n => v.push(format!("{} {many} {many_fate}", fmt(n))),
            }
        }
    }
    v
}

/// The buildings' fate: "2 moulins pris", "1 foire brûlée" — zeros left out.
pub fn buildings_fr(s: &Spoils) -> Vec<String> {
    let mut v = Vec::new();
    for kind in BuildingKind::ALL {
        let n = s.taken[kind.index()];
        if n > 0 {
            v.push(format!(
                "{n} {} {}",
                building_fr(kind, n),
                fate_fr(kind, n, "pris")
            ));
        }
    }
    for kind in BuildingKind::ALL {
        let n = s.burned[kind.index()];
        if n > 0 {
            v.push(format!(
                "{n} {} {}",
                building_fr(kind, n),
                fate_fr(kind, n, "brûlé")
            ));
        }
    }
    v
}

pub fn building_fr(kind: BuildingKind, n: i32) -> &'static str {
    match (kind, n > 1) {
        (BuildingKind::Mill, false) => "moulin",
        (BuildingKind::Mill, true) => "moulins",
        (BuildingKind::Foundry, false) => "fonderie",
        (BuildingKind::Foundry, true) => "fonderies",
        (BuildingKind::Marketplace, false) => "foire",
        (BuildingKind::Marketplace, true) => "foires",
        (BuildingKind::Shipyard, false) => "chantier",
        (BuildingKind::Shipyard, true) => "chantiers",
    }
}

/// `adjective` agreed with `n` buildings of `kind` ("pris" / "brûlé").
fn fate_fr(kind: BuildingKind, n: i32, adjective: &str) -> String {
    let feminine = matches!(kind, BuildingKind::Foundry | BuildingKind::Marketplace);
    let mut a = adjective.to_string();
    if feminine {
        a.push('e');
    }
    if n > 1 && !a.ends_with('s') {
        a.push('s');
    }
    a
}

/// Resample a front's rounds to `n` frames, stretching a skirmish and
/// condensing a long siege alike.
fn sample(rounds: &[Round], n: usize) -> Vec<Round> {
    let n = n.max(2);
    if rounds.len() == n {
        return rounds.to_vec();
    }
    let last = rounds.len() - 1;
    (0..n).map(|i| rounds[i * last / (n - 1)].clone()).collect()
}

pub fn death_fr(cause: &RulerDeathCause) -> &'static str {
    match cause {
        RulerDeathCause::Assassination => "a été assassiné par un noble ambitieux",
        RulerDeathCause::HuntingAccident => {
            "s'est tué d'une chute pendant la chasse au renard annuelle"
        }
        RulerDeathCause::FoodPoisoning => {
            "est mort d'un empoisonnement alimentaire foudroyant ; le cuisinier royal a été exécuté sur-le-champ"
        }
        RulerDeathCause::NaturalCauses => "s'est éteint cet hiver, le cœur fatigué",
        RulerDeathCause::StarvationAssassination => {
            "a été assassiné par une mère folle de douleur dont l'enfant était mort de faim"
        }
    }
}

pub fn invest_fr(kind: InvestmentType) -> &'static str {
    match kind {
        InvestmentType::Marketplaces => "champs de foire",
        InvestmentType::GrainMills => "moulins à grain",
        InvestmentType::Foundries => "fonderies",
        InvestmentType::Shipyards => "chantiers navals",
        InvestmentType::Soldiers => "hommes d'armes",
        InvestmentType::Palaces => "dixièmes de palais",
    }
}

// ---------------------------------------------------------------------------
// Handler plumbing: the caller's token travels as the handler's param bytes.
// ---------------------------------------------------------------------------

/// Bind a handler to the calling connection and a table (plus optional extra
/// bytes). Layout: `[token: 8][code_len: 1][code][extra]`.
pub fn by(spec: HandlerSpec, token: u64, code: &str, extra: &[u8]) -> HandlerSpec {
    let mut bytes = token.to_le_bytes().to_vec();
    bytes.push(code.len() as u8);
    bytes.extend_from_slice(code.as_bytes());
    bytes.extend_from_slice(extra);
    spec.with_param_bytes(bytes)
}

fn caller(ctx: &EventContext) -> (u64, &str, &[u8]) {
    let p = ctx.param_bytes();
    if p.len() < 9 {
        return (0, "", &[]);
    }
    let mut t = [0u8; 8];
    t.copy_from_slice(&p[..8]);
    let end = (9 + p[8] as usize).min(p.len());
    let code = std::str::from_utf8(&p[9..end]).unwrap_or("");
    (u64::from_le_bytes(t), code, &p[end..])
}

/// The caller's token, extra bytes and table, if the table exists.
fn table<'a>(rooms: &'a mut Rooms, ctx: &'a EventContext) -> Option<(u64, &'a [u8], &'a mut Room)> {
    let (token, code, extra) = caller(ctx);
    let room = rooms.rooms.get_mut(code)?;
    room.idle = 0;
    Some((token, extra, room))
}

fn num(ctx: &EventContext, field: &str) -> i32 {
    ctx.field(field)
        .and_then(|v| v.trim().parse::<i32>().ok())
        .unwrap_or(0)
}

/// The caller's kingdom if they are playing `step` right now.
fn acting(room: &Room, token: u64, step: Step) -> Option<Kingdoms> {
    let id = room.seat_of(token)?;
    room.may_act(id, step).then_some(id)
}

// ---------------------------------------------------------------------------
// Home
// ---------------------------------------------------------------------------

#[handler]
pub fn create_room(rooms: &mut Rooms, ctx: &EventContext) {
    let (token, _, _) = caller(ctx);
    let code = rooms.create(token);
    ctx.navigate(format!("/r/{code}"));
}

#[handler]
pub fn enter_code(rooms: &mut Rooms, ctx: &EventContext) {
    // Navigate even when the table is unknown: the room page then says so.
    let code = normalize_code(ctx.field("code").unwrap_or(""));
    if !code.is_empty() {
        ctx.navigate(format!("/r/{code}"));
    }
    let _ = rooms;
}

// ---------------------------------------------------------------------------
// Lobby
// ---------------------------------------------------------------------------

#[handler]
pub fn join(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, args, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = args
        .first()
        .and_then(|&i| KINGDOMS.get(i as usize))
        .copied()
    else {
        return;
    };
    if room.stage != Stage::Lobby || room.seat(id).owner.is_some_and(|o| o != token) {
        return;
    }
    if let Some(prev) = room.seat_of(token) {
        room.release(prev);
    }
    room.seats[id.index()].owner = Some(token);
    room.game.kingdom_mut(id).is_player = true;
}

#[handler]
pub fn leave(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, _, room)) = table(rooms, ctx) else {
        return;
    };
    if room.stage == Stage::Lobby {
        if let Some(id) = room.seat_of(token) {
            room.release(id);
        }
    }
}

/// Take over a human seat — the recovery path when a device loses its
/// session (app switch, PWA vs browser, reboot). Among friends, trust rules.
#[handler]
pub fn reclaim(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, args, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = args
        .first()
        .and_then(|&i| KINGDOMS.get(i as usize))
        .copied()
    else {
        return;
    };
    if room.seat(id).owner.is_none() {
        return; // computer seats are joined via the lobby, not reclaimed
    }
    if let Some(prev) = room.seat_of(token) {
        room.release(prev);
    }
    room.seats[id.index()].owner = Some(token);
    room.game.kingdom_mut(id).is_player = true;
}

#[handler]
pub fn rename(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, _, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = room.seat_of(token) else {
        return;
    };
    let name: String = ctx
        .field("name")
        .unwrap_or("")
        .trim()
        .chars()
        .take(24)
        .collect();
    if !name.is_empty() {
        room.game.kingdom_mut(id).player_name = name;
        let seat = room.seat_mut(id);
        seat.sheet_gen = seat.sheet_gen.wrapping_add(1);
    }
}

#[handler]
pub fn start(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, _, room)) = table(rooms, ctx) else {
        return;
    };
    if room.stage == Stage::Lobby && room.seat_of(token).is_some() {
        room.stage = Stage::Playing;
        room.log.clear();
        room.history.clear();
        room.begin_year();
    }
}

/// A tap on the campaign screen by any seated player: the replay moves on
/// for the whole table; once it is over, the seat asks to turn the year.
#[handler]
pub fn tap_campaign(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, _, room)) = table(rooms, ctx) else {
        return;
    };
    if let Some(id) = room.seat_of(token) {
        room.tap_campaign(id);
    }
}

/// A tap on the Épilogue: this seigneur moves on to the ranking.
#[handler]
pub fn tap_epilogue(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, _, room)) = table(rooms, ctx) else {
        return;
    };
    if room.stage != Stage::Over {
        return;
    }
    if let Some(id) = room.seat_of(token) {
        room.seat_mut(id).epilogue_read = true;
    }
}

#[handler]
pub fn new_game(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, _, room)) = table(rooms, ctx) else {
        return;
    };
    if room.stage != Stage::Over || room.seat_of(token).is_none() {
        return;
    }
    let owners: Vec<(Kingdoms, u64, String)> = room
        .humans()
        .filter_map(|id| {
            room.seat(id)
                .owner
                .map(|o| (id, o, room.game.kingdom(id).player_name.clone()))
        })
        .collect();
    *room = Room {
        code: room.code.clone(),
        host: room.host,
        ..Default::default()
    };
    for (id, owner, name) in owners {
        room.seats[id.index()].owner = Some(owner);
        let k = room.game.kingdom_mut(id);
        k.is_player = true;
        k.player_name = name;
    }
}

// ---------------------------------------------------------------------------
// Turn
// ---------------------------------------------------------------------------

/// Move to the next step; from the war step this gives the orders.
#[handler]
pub fn advance(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, _, room)) = table(rooms, ctx) else {
        return;
    };
    if let Some(id) = room.seat_of(token) {
        room.advance(id);
    }
}

/// A grain-sale slider released: keep its value so the live figures
/// re-centre on it. The first extra param byte says which (0 = amount,
/// 1 = price).
#[handler]
pub fn deal_draft(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, extra, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = acting(room, token, Step::Intendance) else {
        return;
    };
    let Some(value) = ctx.text().and_then(|v| v.trim().parse::<i32>().ok()) else {
        return;
    };
    let seat = room.seat_mut(id);
    match extra.first() {
        Some(0) => seat.deal_amount = Some(value),
        Some(1) => seat.deal_price = Some(value),
        _ => {}
    }
}

/// Conclude a market sheet: buy from a seller, list grain, or sell land to
/// the barbarians. The first extra param byte is the [`Deal::code`]. Done,
/// the sheet closes and the market says so; refused, it says why.
#[handler]
pub fn trade(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, extra, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = acting(room, token, Step::Intendance) else {
        return;
    };
    let Some(deal) = extra.first().and_then(|&c| Deal::from_code(c as i32)) else {
        return;
    };
    let amount = num(ctx, "amount");
    let outcome = match deal {
        Deal::Buy(seller) => buy_grain(room, id, seller, amount),
        Deal::Sell => sell_grain(room, id, amount, num(ctx, "price")),
        Deal::Land => sell_land(room, id, amount),
    };
    let seat = room.seat_mut(id);
    seat.deal_amount = None;
    seat.deal_price = None;
    conclude(room, id, Spot::Market, outcome);
}

/// Tell the outcome of a sheet's form: done, the sheet closes and the
/// notice goes under `spot`; refused, it stays open with the reason.
fn conclude(room: &mut Room, id: Kingdoms, spot: Spot, outcome: Result<String, String>) {
    match outcome {
        Ok(msg) => {
            room.close_sheet(id);
            room.note_at(id, spot, msg);
        }
        Err(msg) => room.note_at(id, Spot::Sheet, msg),
    }
}

fn buy_grain(
    room: &mut Room,
    id: Kingdoms,
    seller: Kingdoms,
    amount: i32,
) -> Result<String, String> {
    if seller == id {
        return Err("On n'achète pas son propre grain.".to_string());
    }
    let s = room.game.kingdom(seller);
    let price = s.grain_price.min(MAX_GRAIN_PRICE);
    let on_sale = s.grain_to_sell;
    if on_sale < 1 || price < 1 {
        return Err(format!("La {} n'a pas de grain à vendre.", seller.name()));
    }
    let amount = amount.clamp(1, on_sale);
    let cost = calculate_buy_cost(amount, price);
    if cost > room.game.kingdom(id).treasury {
        return Err(format!(
            "Le trésor ne couvre pas les {cost} {} demandés.",
            id.currency()
        ));
    }
    apply_trade(&mut room.game, id, Trade::Buy { amount, seller });
    room.journal(
        [id, seller],
        format!(
            "La {} achète {amount} boisseaux à la {}.",
            id.name(),
            seller.name()
        ),
    );
    room.report_sale(seller, id, amount);
    Ok(format!(
        "{} boisseaux achetés à la {} pour {} {}.",
        fmt(amount),
        seller.name(),
        fmt(cost),
        id.currency()
    ))
}

fn sell_grain(room: &mut Room, id: Kingdoms, amount: i32, price: i32) -> Result<String, String> {
    let stocks = room.game.kingdom(id).grain_stocks;
    if stocks < 1 {
        return Err("Les greniers sont vides.".to_string());
    }
    let amount = amount.clamp(1, stocks);
    let price = price.clamp(1, MAX_GRAIN_PRICE);
    apply_trade(&mut room.game, id, Trade::Sell { amount, price });
    Ok(format!(
        "{} boisseaux mis en vente à {price} le cent ; ils seront au marché l'an prochain.",
        fmt(amount)
    ))
}

fn sell_land(room: &mut Room, id: Kingdoms, amount: i32) -> Result<String, String> {
    let max = max_land_sale(room.game.kingdom(id).surface);
    if max < 1 {
        return Err("Il ne reste pas assez de terres à vendre.".to_string());
    }
    let arpents = amount.clamp(1, max);
    apply_trade(&mut room.game, id, Trade::SellLand { arpents });
    Ok(format!(
        "{} arpents vendus aux Barbares pour {} {}.",
        fmt(arpents),
        fmt(arpents * LAND_SELL_PRICE),
        id.currency()
    ))
}

/// A council sheet validated: its one slider (the [`Field`] in the first
/// extra param byte) is set in the draft, which "Continuer" promulgates.
#[handler]
pub fn settle(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, extra, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = acting(room, token, Step::Intendance) else {
        return;
    };
    let Some(field) = extra.first().and_then(|&f| Field::from_u8(f)) else {
        return;
    };
    let mut draft = room.draft(id);
    draft.set(field, num(ctx, field.name()));
    let draft = draft.clamped(room.game.kingdom(id));
    let seat = room.seat_mut(id);
    seat.draft = Some(draft);
    seat.notice = None;
    room.close_sheet(id);
}

/// The chronicle's telling of a population delta, after "a" / "avez";
/// `own` is the possessive ("ses", "vos") for a year that changed nothing.
pub fn subjects_delta(delta: i32, own: &str) -> String {
    let taillables = "sujets taillables et corvéables à merci";
    match delta {
        n if n > 0 => format!("gagné {} {taillables}", fmt(n)),
        n if n < 0 => format!("perdu {} {taillables}", fmt(-n)),
        _ => format!("conservé {own} {taillables}"),
    }
}

/// A row touched: the barbarians' war sheet opens, a kingdom's sheet opens
/// on what is known of it (the extra param byte is the kingdom number, 0 =
/// the barbarians; 0xFF closes).
#[handler]
pub fn pick_target(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, extra, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = acting(room, token, Step::War) else {
        return;
    };
    let picked = extra.first().copied().filter(|&n| n <= 6);
    let attack = picked == Some(0);
    let forecast = attack.then(|| room.war_forecast(id, None)).flatten();
    let seat = room.seat_mut(id);
    seat.target = picked;
    seat.attack = attack;
    seat.forecast = forecast;
    seat.notice = None;
}

/// "Attaquer" on a kingdom's sheet: the war form opens on it.
#[handler]
pub fn open_attack(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, _, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = acting(room, token, Step::War) else {
        return;
    };
    let Some(target) = sheet_kingdom(room, id) else {
        return;
    };
    let forecast = room.war_forecast(id, Some(target));
    let seat = room.seat_mut(id);
    seat.attack = true;
    seat.forecast = forecast;
    seat.notice = None;
}

/// "Envoyer un éclaireur" on a kingdom's sheet, or his recall.
#[handler]
pub fn scout(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, _, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = acting(room, token, Step::War) else {
        return;
    };
    let Some(target) = sheet_kingdom(room, id) else {
        return;
    };
    room.seat_mut(id).notice = None;
    if let Err(msg) = room.toggle_scout(id, target) {
        room.note_at(id, Spot::Sheet, msg);
    }
}

/// "Acheter un agent" on a kingdom's sheet, his recall, or his dismissal.
#[handler]
pub fn agent(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, _, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = acting(room, token, Step::War) else {
        return;
    };
    let Some(target) = sheet_kingdom(room, id) else {
        return;
    };
    room.seat_mut(id).notice = None;
    if let Err(msg) = room.toggle_agent(id, target) {
        room.note_at(id, Spot::Sheet, msg);
    }
}

/// The living kingdom whose sheet `id` has open.
fn sheet_kingdom(room: &Room, id: Kingdoms) -> Option<Kingdoms> {
    let n = room.seat(id).target.filter(|&n| n != 0)?;
    let o = Kingdoms::from_number(i32::from(n))?;
    (o != id && !room.game.kingdom(o).is_dead).then_some(o)
}

/// A purchase sheet validated: the kind is the first extra param byte (its
/// [`InvestmentType::from_number`] number), the quantity a form field.
#[handler]
pub fn invest(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, extra, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = acting(room, token, Step::Intendance) else {
        return;
    };
    let Some(kind) = extra
        .first()
        .and_then(|&n| InvestmentType::from_number(n as i32))
    else {
        return;
    };
    let amount = num(ctx, "invest_amount").max(0);
    if amount < 1 {
        return;
    }
    let result = apply_investment(room.game.kingdom_mut(id), kind, amount);
    let outcome = match result.error {
        Some(err) => Err(err),
        None => {
            let fx = result.side_effects;
            let mut m = format!(
                "{} × {} pour {} {}.",
                fmt(amount),
                invest_fr(kind),
                fmt(result.total_cost),
                id.currency()
            );
            if fx.merchants_attracted > 0 {
                m.push_str(&format!(
                    " {} serfs sont devenus marchands.",
                    fx.merchants_attracted
                ));
            }
            if fx.nobles_attracted > 0 {
                m.push_str(&format!(
                    " {} nobles rejoignent la cour.",
                    fx.nobles_attracted
                ));
            }
            Ok(m)
        }
    };
    conclude(room, id, Spot::Purchases, outcome);
}

/// The war sheet validated: the army on its target (the extra param byte)
/// is settled, replacing the one ordered there before, and the sheet closes.
#[handler]
pub fn attack(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, extra, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = acting(room, token, Step::War) else {
        return;
    };
    let Some(target) = extra.first().filter(|&&n| n <= 6) else {
        return;
    };
    let target = Kingdoms::from_number(i32::from(*target));
    let was = room.planned_on(id, target).map(|(i, e)| (i, *e));
    if let Some((i, _)) = was {
        room.seat_mut(id).planned.remove(i);
    }
    let e = Expedition {
        attacker: id,
        target,
        soldiers: num(ctx, "soldiers"),
    };
    room.seat_mut(id).notice = None;
    match room.order(e) {
        Ok(()) => {
            let seat = room.seat_mut(id);
            seat.target = None;
            seat.attack = false;
            seat.forecast = None;
        }
        Err(msg) => {
            if let Some((i, e)) = was {
                room.seat_mut(id).planned.insert(i, e);
            }
            room.note_at(id, Spot::Sheet, msg);
        }
    }
}

/// An ordered expedition struck off (its position is the extra param byte).
#[handler]
pub fn withdraw(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, extra, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = acting(room, token, Step::War) else {
        return;
    };
    let seat = room.seat_mut(id);
    if let Some(&i) = extra
        .first()
        .filter(|&&i| (i as usize) < seat.planned.len())
    {
        seat.planned.remove(i as usize);
        seat.notice = None;
        seat.target = None;
        seat.attack = false;
        seat.forecast = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table in play with the given kingdoms seated by humans (token = index + 1).
    fn playing(humans: &[Kingdoms]) -> Room {
        let mut room = Room::default();
        for &id in humans {
            room.seats[id.index()].owner = Some(id.index() as u64 + 1);
            room.game.kingdom_mut(id).is_player = true;
        }
        room.stage = Stage::Playing;
        room.begin_year();
        room
    }

    /// Play `id` through the intendance: the council is promulgated with the
    /// default draft, every other step is skipped with its "next" button.
    fn finish_intendance(room: &mut Room, id: Kingdoms) {
        // The first year opens on the season; the others tell the year first.
        if room.seat(id).step == Step::Chronicle {
            room.advance(id); // Chronicle → Season
        }
        assert_eq!(room.seat(id).step, Step::Season);
        room.advance(id); // Season → Intendance
        room.advance(id); // Intendance → promulgated, Report
        room.advance(id); // Report → Treasury
        room.advance(id); // Treasury → ready
    }

    #[test]
    fn the_year_opens_on_everyones_intendance() {
        let room = playing(&[Kingdoms::France, Kingdoms::Spain]);
        assert_eq!(room.phase, Phase::Intendance);
        for id in [Kingdoms::France, Kingdoms::Spain] {
            assert!(room.playing(id));
            assert!(room.may_act(id, Step::Season));
            assert!(!room.may_act(id, Step::War));
        }
        // The computers have already run their kingdoms.
        assert!(!room.playing(Kingdoms::Germany));
    }

    #[test]
    fn war_waits_for_every_seigneur() {
        let mut room = playing(&[Kingdoms::France, Kingdoms::Spain]);
        finish_intendance(&mut room, Kingdoms::France);
        assert!(room.seat(Kingdoms::France).ready);
        assert_eq!(room.phase, Phase::Intendance);
        assert!(!room.playing(Kingdoms::France));
        // Being ready is final: the button does nothing more.
        room.advance(Kingdoms::France);
        assert_eq!(room.phase, Phase::Intendance);

        finish_intendance(&mut room, Kingdoms::Spain);
        // Everyone gives their war orders at once.
        assert_eq!(room.phase, Phase::Exterieur);
        for id in [Kingdoms::France, Kingdoms::Spain] {
            assert_eq!(room.seat(id).step, Step::War);
            assert!(room.playing(id));
            assert!(room.may_act(id, Step::War));
            assert!(!room.seat(id).ready);
        }
    }

    #[test]
    fn the_armies_march_together_once_everyone_has_ordered() {
        let mut room = playing(&[Kingdoms::France, Kingdoms::Spain]);
        let (f, s) = (Kingdoms::France, Kingdoms::Spain);
        finish_intendance(&mut room, f);
        finish_intendance(&mut room, s);
        room.game.year = 3;
        room.game.kingdom_mut(f).soldiers = 100;
        room.game.kingdom_mut(s).soldiers = 100;
        room.order(Expedition {
            attacker: f,
            target: Some(s),
            soldiers: 30,
        })
        .unwrap();
        assert_eq!(room.garrison(f), 70);
        room.advance(f);
        // France waits for Spain: nothing has marched yet.
        assert_eq!(room.phase, Phase::Exterieur);
        assert!(room.battles.is_empty());
        assert_eq!(room.game.kingdom(f).soldiers, 100);
        room.order(Expedition {
            attacker: s,
            target: Some(f),
            soldiers: 40,
        })
        .unwrap();
        room.advance(s);
        // Both armies have left their garrisons before any battle is told.
        assert_eq!(room.phase, Phase::Campaign);
        assert_eq!(room.game.kingdom(f).soldiers, 70);
        assert_eq!(room.game.kingdom(s).soldiers, 60);
        let human: Vec<Kingdoms> = room
            .battles
            .iter()
            .flat_map(|b| b.expeditions().map(|e| e.attacker))
            .filter(|a| [f, s].contains(a))
            .collect();
        assert_eq!(human, vec![f, s]);
        // Each front was fought against the reduced garrison.
        assert_eq!(room.battles[0].result.garrison_start, 60);
        // Every seigneur reads on their own clock: the order of battle holds
        // a moment, then the first front moves by itself.
        assert_eq!(room.seat(f).replay, Replay::opening());
        assert_eq!(room.seat(s).replay, Replay::opening());
        assert!(room.seat(Kingdoms::Britanny).replay.done());
        for _ in 1..SCHEMA_TICKS {
            assert!(!room.tick());
        }
        assert!(room.tick());
        assert_eq!(room.seat(f).replay.staging, Staging::Fight(FRAME_TICKS));
        assert_eq!(room.seat(s).replay.staging, Staging::Fight(FRAME_TICKS));
        for _ in 1..FRAME_TICKS {
            assert!(!room.tick());
        }
        assert!(room.tick());
        assert_eq!(room.seat(f).replay.cursor, 1);
        // Nobody may continue before they have read the last front.
        assert!(!room.may_continue(f));
        room.continue_campaign(f);
        assert_eq!(room.phase, Phase::Campaign);
        // Spain taps: a tap during the front ends it, held for its verdict.
        room.tap_campaign(s);
        let frames = room.battles[0].result.rounds.len();
        let armies = room.battles[0].result.armies.len() as u32;
        let held = VERDICT_TICKS + VERDICT_TICKS_PER_ARMY * (armies - 1);
        assert_eq!(room.seat(s).replay.staging, Staging::Verdict(held));
        assert_eq!(room.seat(s).replay.cursor, frames - 1);
        assert!(room.seat(s).replay.settled(0));
        // A tap on the verdict moves Spain on at once.
        room.tap_campaign(s);
        assert_eq!(room.seat(s).replay.current, 1);
        assert_eq!(room.seat(s).replay.staging, Staging::Schema(SCHEMA_TICKS));
        // Left alone, France's front runs to the same verdict, which holds,
        // then gives way by itself.
        for _ in 0..(frames * FRAME_TICKS as usize) {
            room.tick();
        }
        assert!(matches!(room.seat(f).replay.staging, Staging::Verdict(_)));
        assert_eq!(room.seat(f).replay.cursor, frames - 1);
        assert_eq!(room.seat(f).replay.current, 0);
        let mut ticks = 0;
        while matches!(room.seat(f).replay.staging, Staging::Verdict(_)) {
            room.tick();
            ticks += 1;
        }
        assert!(ticks <= held);
        assert_eq!(room.seat(f).replay.current, 1);
        assert_eq!(room.seat(f).replay.staging, Staging::Schema(SCHEMA_TICKS));
        // A spectator sees everything told.
        assert!(room.replay(None).done());
        assert!(room.replay(None).settled(room.battles.len() - 1));
        // Both read the rest of the campaign, France first.
        while !room.seat(f).replay.done() {
            room.tap_campaign(f);
        }
        // Nothing has reached the map yet: the year still waits for Spain.
        let year = room.game.year;
        assert_eq!(room.game.kingdom(f).soldiers, 70);
        assert_eq!(room.game.kingdom(s).soldiers, 60);
        assert!(room.may_continue(f));
        room.tap_campaign(f);
        assert!(!room.may_continue(f));
        assert_eq!(room.phase, Phase::Campaign);
        assert_eq!(room.game.year, year);
        assert_eq!(room.still_reading().collect::<Vec<_>>(), vec![s]);
        assert!(!room.may_continue(s));
        while !room.seat(s).replay.done() {
            room.tap_campaign(s);
        }
        assert_eq!(room.game.year, year);
        // Spain done reading, the fronts reach the map and the year turns.
        room.tap_campaign(s);
        assert_eq!(room.game.year, year + 1);
        assert!(room.battles.is_empty());
        assert!(room
            .seat(f)
            .news
            .iter()
            .any(|n| matches!(n, News::Expeditions { .. })));
    }

    #[test]
    fn an_order_is_checked_and_may_be_withdrawn() {
        let mut room = playing(&[Kingdoms::France]);
        let f = Kingdoms::France;
        finish_intendance(&mut room, f);
        let k = room.game.kingdom_mut(f);
        k.soldiers = 10;
        k.nobles = 4;
        assert_eq!(room.expeditions_left(f), 2);
        // Ten men only, whatever is asked.
        room.order(Expedition {
            attacker: f,
            target: None,
            soldiers: 50,
        })
        .unwrap();
        assert_eq!(room.seat(f).planned[0].soldiers, 10);
        assert_eq!(room.garrison(f), 0);
        assert!(room
            .order(Expedition {
                attacker: f,
                target: None,
                soldiers: 1,
            })
            .is_err());
        room.seat_mut(f).planned.remove(0);
        assert_eq!(room.garrison(f), 10);
        // A neighbour may not be attacked before the third year.
        assert!(room
            .order(Expedition {
                attacker: f,
                target: Some(Kingdoms::Spain),
                soldiers: 5,
            })
            .is_err());
    }

    #[test]
    fn a_fallen_kingdom_no_longer_blocks_the_war() {
        let mut room = playing(&[Kingdoms::France, Kingdoms::Spain]);
        room.game.kingdom_mut(Kingdoms::Spain).is_dead = true;
        finish_intendance(&mut room, Kingdoms::France);
        assert_eq!(room.phase, Phase::Exterieur);
    }

    #[test]
    fn a_solo_year_runs_end_to_end() {
        let mut room = playing(&[Kingdoms::France]);
        let year = room.game.year;
        finish_intendance(&mut room, Kingdoms::France);
        assert_eq!(room.seat(Kingdoms::France).step, Step::War);
        // The orders are given. If any computer marched, the battles are told
        // side by side and France turns the year; a year without a campaign
        // turns by itself…
        room.advance(Kingdoms::France);
        if room.phase == Phase::Campaign {
            while !room.seat(Kingdoms::France).replay.done() {
                room.tap_campaign(Kingdoms::France);
            }
            assert_eq!(room.game.year, year);
            room.tap_campaign(Kingdoms::France);
        }
        // …and the next year opens on France's intendance again.
        assert_eq!(room.game.year, year + 1);
        assert_eq!(room.phase, Phase::Intendance);
        assert!(room.battles.is_empty(), "last year's battles linger");
        assert_eq!(room.seat(Kingdoms::France).step, Step::Chronicle);
        assert!(!room.seat(Kingdoms::France).ready);
        // …with the last council's census to tell.
        assert!(room.seat(Kingdoms::France).demo.is_some());
    }

    #[test]
    fn a_rank_won_or_lost_is_announced_once() {
        let mut room = playing(&[Kingdoms::France]);
        let id = Kingdoms::France;
        assert_eq!(room.seat(id).title, Some(PlayerTitle::Duke));
        let k = room.game.kingdom_mut(id);
        k.peasants = 2300;
        k.surface = 2300 * 5;
        k.nobles = 10;
        k.grain_mills = 4;
        k.marketplaces = 8;
        k.palaces = 2;
        room.judge_title(id);
        let seat = room.seat(id);
        assert_eq!(seat.title, Some(PlayerTitle::Prince));
        assert_eq!(
            seat.news.last(),
            Some(&News::Rank {
                before: PlayerTitle::Duke,
                now: PlayerTitle::Prince
            })
        );
        assert_eq!(seat.title_year, room.game.year);
        assert_eq!(
            room.log.last().unwrap().text,
            "Hugues de France est fait Duc."
        );
        let lines = room.log.len();
        // Nothing changed: nothing to say.
        let facts = room.seat(id).news.len();
        room.judge_title(id);
        assert_eq!(room.seat(id).news.len(), facts);
        assert_eq!(room.log.len(), lines);
        // A famine empties the countryside: the title goes with it.
        room.game.kingdom_mut(id).peasants = 1500;
        room.judge_title(id);
        assert_eq!(
            room.seat(id).news.last(),
            Some(&News::Rank {
                before: PlayerTitle::Prince,
                now: PlayerTitle::Duke
            })
        );
        assert_eq!(
            room.log.last().unwrap().text,
            "Hugues de France n'est plus que Baron."
        );
    }

    fn make_emperor(k: &mut Kingdom) {
        k.peasants = 3200;
        k.surface = 3200 * 6;
        k.nobles = 41;
        k.grain_mills = 6;
        k.marketplaces = 14;
        k.foundries = 1;
        k.palaces = 10;
    }

    #[test]
    fn the_imperial_crown_wins_the_game() {
        let mut room = playing(&[Kingdoms::France]);
        let id = Kingdoms::France;
        make_emperor(room.game.kingdom_mut(id));
        room.judge_title(id);
        assert_eq!(room.seat(id).title, Some(PlayerTitle::Emperor));
        assert_eq!(room.seat(id).title_year, room.game.year);
        assert_eq!(room.seat(id).news.last(), Some(&News::Crowned));
        assert!(room
            .seat(Kingdoms::Spain)
            .news
            .contains(&News::Elsewhere(Elsewhere::Crowned(id))));
        assert_eq!(room.emperor(), Some(id));
        assert!(room.check_over());
        assert_eq!(room.stage, Stage::Over);
        let texts: Vec<&str> = room.log.iter().map(|e| e.text.as_str()).collect();
        assert!(texts.contains(&"Hugues de France est couronné Empereur."));
        assert_eq!(
            texts.last(),
            Some(&"La partie est terminée : Hugues de France est couronné Empereur.")
        );
    }

    #[test]
    fn a_computer_may_take_the_crown_too() {
        let mut room = playing(&[Kingdoms::France]);
        let id = Kingdoms::Spain;
        make_emperor(room.game.kingdom_mut(id));
        room.judge_title(id);
        assert_eq!(room.emperor(), Some(id));
        assert!(room.check_over());
        assert!(room
            .seat(Kingdoms::France)
            .news
            .contains(&News::Elsewhere(Elsewhere::Crowned(id))));
    }

    #[test]
    fn the_last_realm_standing_plays_on_until_the_crown() {
        let mut room = playing(&[Kingdoms::France]);
        let id = Kingdoms::France;
        for other in KINGDOMS.into_iter().filter(|&o| o != id) {
            room.game
                .kingdom_mut(other)
                .fall(Fate::Annexed(Kingdoms::France));
        }
        assert!(!room.check_over());
        assert_eq!(room.stage, Stage::Playing);
        assert_eq!(room.seat(id).title, Some(PlayerTitle::Duke));
        assert_eq!(room.emperor(), None);
    }

    #[test]
    fn the_crown_is_judged_at_years_end_not_on_the_field() {
        let mut room = playing(&[Kingdoms::France]);
        // The ledgers already earn the crown, but the titles have not been
        // judged since: the campaign's check leaves the game running.
        make_emperor(room.game.kingdom_mut(Kingdoms::France));
        assert!(!room.check_over());
        assert_eq!(room.stage, Stage::Playing);
    }

    #[test]
    fn a_shared_map_crowns_nobody() {
        let mut room = playing(&[Kingdoms::France]);
        room.game.kingdom_mut(Kingdoms::France).is_dead = true;
        assert!(room.check_over());
        assert_eq!(room.emperor(), None);
        assert_eq!(
            room.log.last().map(|e| e.text.as_str()),
            Some("La partie est terminée : plus aucun seigneur en vie.")
        );
    }

    #[test]
    fn a_battle_is_told_to_both_sides() {
        let mut room = playing(&[Kingdoms::France, Kingdoms::Spain]);
        let (a, d) = (Kingdoms::France, Kingdoms::Spain);
        room.game.kingdom_mut(a).soldiers = 100;
        let def = room.game.kingdom_mut(d);
        def.soldiers = 400;
        def.surface = 20_000;
        room.seat_mut(a).news.clear();
        room.seat_mut(d).news.clear();
        for n in 1..=2 {
            let attacker_before = room.game.kingdom(a).soldiers;
            let defender_before = room.game.kingdom(d).soldiers;
            let fought = march(
                &mut room.game,
                [Expedition {
                    attacker: a,
                    target: Some(d),
                    soldiers: 20,
                }],
            )
            .remove(0);
            room.start_battle(fought);
            room.finish_battle(0);
            room.battles.clear();
            let attacker_after = room.game.kingdom(a).soldiers;
            let defender_after = room.game.kingdom(d).soldiers;
            // One running total on the attacker's side…
            let totals: Vec<&News> = room
                .seat(a)
                .news
                .iter()
                .filter(|n| matches!(n, News::Expeditions { .. }))
                .collect();
            assert_eq!(totals.len(), 1);
            let News::Expeditions {
                count, men_lost, ..
            } = totals[0]
            else {
                unreachable!()
            };
            assert_eq!(*count, n);
            if n == 1 {
                assert_eq!(*men_lost, attacker_before - attacker_after);
            }
            // …one line per front on the defender's.
            let Some(News::Attacked {
                armies,
                garrison_fallen,
                levy,
                ..
            }) = room.seat(d).news.last()
            else {
                panic!("the defender was not told");
            };
            assert_eq!((armies.as_slice(), *levy), (&[(a, 20)][..], false));
            assert_eq!(*garrison_fallen, defender_before - defender_after);
        }
        assert_eq!(room.seat(d).news.len(), 2);
    }

    #[test]
    fn a_war_is_heard_everywhere_but_its_headcount_stays_on_the_field() {
        let mut room = playing(&[Kingdoms::France, Kingdoms::Spain, Kingdoms::Germany]);
        let (a, d, other) = (Kingdoms::France, Kingdoms::Spain, Kingdoms::Germany);
        room.game.kingdom_mut(a).soldiers = 100;
        room.game.kingdom_mut(d).soldiers = 5;
        room.seat_mut(other).news.clear();
        let fought = march(
            &mut room.game,
            [Expedition {
                attacker: a,
                target: Some(d),
                soldiers: 60,
            }],
        )
        .remove(0);
        room.start_battle(fought);
        room.finish_battle(0);
        // The march: the field knows the headcount, the court next door not.
        let march_line = room
            .log
            .iter()
            .find(|e| e.text.contains("marche sur"))
            .expect("the march is journaled");
        assert!(march_line.told_to(Some(a)).contains("60 hommes d'armes"));
        assert!(march_line.told_to(Some(d)).contains("60 hommes d'armes"));
        assert!(!march_line.told_to(Some(other)).contains("hommes"));
        assert!(!march_line.told_to(None).contains("hommes"));
        // The verdict: the land is public, the loot is not.
        let verdict_line = room.log.last().unwrap();
        assert!(verdict_line.told_to(Some(other)).contains("arpent"));
        assert!(!verdict_line.told_to(Some(other)).contains("Butin"));
        // The heralds keep the front, without a headcount…
        assert_eq!(room.rumours.len(), 1);
        let r = room.rumours[0];
        assert_eq!((r.attacker, r.target, r.year), (a, Some(d), room.game.year));
        // …and the third court reads it in its chronicle.
        assert!(room.seat(other).news.iter().any(|n| matches!(
            n,
            News::Elsewhere(Elsewhere::Marched { by, on, .. }) if *by == a && *on == d
        )));
        assert!(!room
            .seat(a)
            .news
            .iter()
            .any(|n| matches!(n, News::Elsewhere(Elsewhere::Marched { .. }))));
        // The rumours last until the next campaign marches.
        room.end_year();
        assert_eq!(room.rumours.len(), 1);
        room.try_open_exterior();
        assert_eq!(room.rumours.len(), 1);
    }

    #[test]
    fn a_rank_is_told_to_the_others_too() {
        let mut room = playing(&[Kingdoms::France, Kingdoms::Spain]);
        let id = Kingdoms::France;
        let k = room.game.kingdom_mut(id);
        k.peasants = 2300;
        k.surface = 2300 * 5;
        k.nobles = 10;
        k.grain_mills = 4;
        k.marketplaces = 8;
        k.palaces = 2;
        room.judge_title(id);
        assert_eq!(
            room.seat(Kingdoms::Spain).news.last(),
            Some(&News::Elsewhere(Elsewhere::Rank {
                id,
                before: PlayerTitle::Duke,
                now: PlayerTitle::Prince
            }))
        );
        assert!(!room
            .seat(id)
            .news
            .iter()
            .any(|n| matches!(n, News::Elsewhere(_))));
    }

    #[test]
    fn a_sale_is_told_to_the_seller() {
        let mut room = playing(&[Kingdoms::France, Kingdoms::Spain]);
        let (buyer, seller) = (Kingdoms::France, Kingdoms::Spain);
        let s = room.game.kingdom_mut(seller);
        s.grain_to_sell = 1000;
        s.grain_price = 12;
        room.game.kingdom_mut(buyer).treasury = 10_000;
        room.seat_mut(seller).news.clear();
        buy_grain(&mut room, buyer, seller, 300).unwrap();
        assert_eq!(
            room.seat(seller).news,
            vec![News::Sold {
                buyer,
                amount: 300,
                price: 12,
                left: 700
            }]
        );
        assert_eq!(
            room.log.last().unwrap().text,
            "La France achète 300 boisseaux à la Castille."
        );
        assert!(room.seat(buyer).news.is_empty());
    }

    #[test]
    fn the_chronicle_runs_from_council_to_council_and_outlives_a_fall() {
        let mut room = playing(&[Kingdoms::France]);
        let id = Kingdoms::France;
        room.report(id, News::Crowned);
        // The first year opens on the season; the news waits for next year's
        // Chronique.
        assert_eq!(room.seat(id).step, Step::Season);
        assert_eq!(room.seat(id).news.len(), 1);
        room.advance(id); // Season → Intendance
        room.advance(id); // Intendance → promulgated: the tale starts over
        assert!(room.seat(id).news.is_empty());
        // A fallen seat keeps its epitaph through the years.
        let fate = Fate::Annexed(Kingdoms::Spain);
        room.game.kingdom_mut(id).fall(fate);
        room.report(id, News::Fallen(fate));
        // Computers start afresh with each year (their intendance may
        // already sell grain to one another as the year opens).
        room.report(Kingdoms::Spain, News::Crowned);
        room.begin_year();
        assert_eq!(room.seat(id).news, vec![News::Fallen(fate)]);
        assert!(!room.seat(Kingdoms::Spain).news.contains(&News::Crowned));
    }

    /// Play `id` from its war orders to the next year's Extérieur.
    fn turn_year(room: &mut Room, id: Kingdoms) {
        room.advance(id);
        if room.phase == Phase::Campaign {
            while !room.seat(id).replay.done() {
                room.tap_campaign(id);
            }
            room.tap_campaign(id);
        }
        finish_intendance(room, id);
    }

    #[test]
    fn an_eclaireur_sent_at_the_exterior_reports_at_the_next_one() {
        let mut room = playing(&[Kingdoms::France]);
        let (id, on) = (Kingdoms::France, Kingdoms::Spain);
        finish_intendance(&mut room, id);
        room.game.kingdom_mut(id).treasury = 100_000;
        room.toggle_scout(id, on).unwrap();
        turn_year(&mut room, id);
        assert_eq!(room.phase, Phase::Exterieur);
        assert!(!room.scout_ordered(id, on), "the mission lingers");
        let d = room.dossier(id, on);
        let year = room.game.year;
        let read = d.report.is_some_and(|r| r.year == year);
        let taken = d.caught == Some(year);
        assert!(read || taken, "{d:?}");
        // The figures are of this very day, after Spain's council sat.
        if read {
            let r = d.report.unwrap();
            let k = room.game.kingdom(on);
            assert_eq!((r.garrison, r.serfs), (k.soldiers, k.peasants));
        }
        // Told at the Extérieur, not in a Chronique a year late.
        assert!(room.seat(id).news.iter().any(News::is_intelligence));
    }

    #[test]
    fn computers_give_their_war_orders_as_the_exterior_opens() {
        let mut room = playing(&[Kingdoms::France]);
        // A bold council, year three, with a fresh report on a weak realm:
        // it strikes for sure (a coup de sang may march first and take men).
        let (id, on) = (Kingdoms::Britanny, Kingdoms::Spain);
        room.game.year = 3;
        room.seat_mut(id).mind = Mind {
            temper: Temper::Bold,
            eye: Some(on),
            seen: None,
        };
        let k = room.game.kingdom_mut(id);
        k.soldiers = 60;
        k.soldiers_efficiency = 100;
        room.seat_mut(id).dossiers[on.index()].report = Some(Report {
            year: 3,
            garrison: 20,
            efficiency: 100,
            nobles: 1,
            merchants: 100,
            serfs: 1000,
            surface: 10_000,
        });
        assert!(room.seat(id).planned.is_empty());
        finish_intendance(&mut room, Kingdoms::France);
        assert_eq!(room.phase, Phase::Exterieur);
        assert!(room.seat(id).planned.iter().any(|e| e.attacker == id));
        room.advance(Kingdoms::France);
        assert_eq!(room.phase, Phase::Campaign);
        assert!(room
            .battles
            .iter()
            .any(|b| b.expeditions().any(|e| e.attacker == id)));
    }
}

#[cfg(test)]
mod scouting {
    use super::*;

    fn table() -> Room {
        let mut room = Room::default();
        for id in [Kingdoms::France, Kingdoms::Spain] {
            room.seats[id.index()].owner = Some(id.index() as u64 + 1);
            room.game.kingdom_mut(id).is_player = true;
        }
        room.stage = Stage::Playing;
        room.begin_year();
        room
    }

    #[test]
    fn a_scout_is_paid_on_order_and_refunded_on_recall() {
        let mut room = table();
        let (id, on) = (Kingdoms::France, Kingdoms::Spain);
        let before = room.game.kingdom(id).treasury;
        assert_eq!(room.toggle_scout(id, on), Ok(()));
        assert!(room.scout_ordered(id, on));
        assert_eq!(room.game.kingdom(id).treasury, before - SCOUT_PRICE);
        assert_eq!(room.toggle_scout(id, on), Ok(()));
        assert!(!room.scout_ordered(id, on));
        assert_eq!(room.game.kingdom(id).treasury, before);
        // Nothing to scout at home, and no scout without the price.
        assert!(room.toggle_scout(id, id).is_err());
        room.game.kingdom_mut(id).treasury = SCOUT_PRICE - 1;
        assert!(room.toggle_scout(id, on).is_err());
        assert!(!room.scout_ordered(id, on));
    }

    #[test]
    fn a_scout_comes_home_with_a_report_or_is_taken() {
        let (id, on) = (Kingdoms::France, Kingdoms::Spain);
        let (mut back, mut caught) = (false, false);
        for _ in 0..200 {
            let mut room = table();
            room.game.kingdom_mut(on).soldiers = 77;
            room.game.kingdom_mut(id).treasury = 10_000;
            room.seat_mut(id).news.clear();
            room.seat_mut(on).news.clear();
            room.toggle_scout(id, on).unwrap();
            room.game.increment_year();
            room.resolve_missions();
            let year = room.game.year;
            assert!(room.seat(id).missions.is_empty());
            match room.seat(id).news.as_slice() {
                [News::Scout {
                    at,
                    outcome: Scouting::Back { garrison },
                }] => {
                    back = true;
                    assert_eq!((*at, *garrison), (on, 77));
                    let r = room.dossier(id, on).report.expect("a report");
                    assert_eq!((r.year, r.garrison), (year, 77));
                    assert!(room.seat(on).news.is_empty());
                    assert!(room.dossier(id, on).caught.is_none());
                }
                [News::Scout {
                    at,
                    outcome: Scouting::Caught,
                }] => {
                    caught = true;
                    assert_eq!(*at, on);
                    assert!(room.dossier(id, on).report.is_none());
                    assert_eq!(room.dossier(id, on).caught, Some(year));
                    assert!(matches!(
                        room.seat(on).news.as_slice(),
                        [News::SpyCaught { by, agent: false }] if *by == id
                    ));
                    let line = room.log.last().expect("a journal line");
                    let heard = line.told_to(Some(Kingdoms::Germany));
                    assert!(heard.contains("éclaireur de la France"), "{heard}");
                    assert!(line.secret.is_none());
                }
                other => panic!("unexpected news {other:?}"),
            }
            if back && caught {
                return;
            }
        }
        panic!("both outcomes should show up in 200 draws (back {back}, caught {caught})");
    }

    /// A fresh report on `on`, as an éclaireur home this year would leave.
    fn fresh_report(room: &mut Room, id: Kingdoms, on: Kingdoms) {
        let year = room.game.year;
        room.seat_mut(id).dossiers[on.index()].report =
            Some(Report::read(room.game.kingdom(on), year));
    }

    #[test]
    fn an_agent_is_bought_on_a_fresh_report_only() {
        let mut room = table();
        let (id, on) = (Kingdoms::France, Kingdoms::Spain);
        room.game.kingdom_mut(id).treasury = 1_000;
        assert!(room.toggle_agent(id, on).is_err());
        fresh_report(&mut room, id, on);
        assert_eq!(room.toggle_agent(id, on), Ok(()));
        assert!(room.agent_ordered(id, on));
        assert_eq!(room.game.kingdom(id).treasury, 1_000 - AGENT_PRICE);
        assert_eq!(room.toggle_agent(id, on), Ok(()));
        assert!(!room.agent_ordered(id, on));
        assert_eq!(room.game.kingdom(id).treasury, 1_000);
        room.game.kingdom_mut(id).treasury = AGENT_PRICE - 1;
        assert!(room.toggle_agent(id, on).is_err());
        assert!(room.toggle_agent(id, id).is_err());
    }

    #[test]
    fn an_agent_writes_each_year_he_is_paid_or_is_unmasked() {
        let (id, on) = (Kingdoms::France, Kingdoms::Spain);
        let (mut wrote, mut unmasked) = (false, false);
        for _ in 0..200 {
            let mut room = table();
            room.game.kingdom_mut(id).treasury = 10_000;
            room.game.kingdom_mut(on).soldiers = 30;
            fresh_report(&mut room, id, on);
            room.toggle_agent(id, on).unwrap();
            room.game.kingdom_mut(on).soldiers = 70;
            room.game.kingdom_mut(on).treasury = 6_200;
            room.report_sale(Kingdoms::Germany, on, 300);
            room.seat_mut(id).news.clear();
            room.seat_mut(on).news.clear();
            room.game.increment_year();
            room.resolve_missions();
            let year = room.game.year;
            assert!(room.seat(id).missions.is_empty());
            match room.seat(id).news.as_slice() {
                [News::Agent {
                    at,
                    outcome:
                        Writing::Letter {
                            ledger,
                            levied,
                            spent,
                        },
                }] => {
                    wrote = true;
                    assert_eq!(*at, on);
                    // The first letter is paid by the purchase.
                    assert_eq!(room.game.kingdom(id).treasury, 10_000 - AGENT_PRICE);
                    assert_eq!((ledger.year, ledger.treasury), (year, 6_200));
                    assert_eq!(ledger.bought[Kingdoms::Germany.index()], 300);
                    assert_eq!((*levied, *spent), (Some(40), None));
                    let d = room.dossier(id, on);
                    assert!(d.agent);
                    assert_eq!(d.report.map(|r| (r.year, r.garrison)), Some((year, 70)));
                    assert_eq!(d.ledger, Some(*ledger));
                    // The next year he is paid as he writes; unpaid, he leaves.
                    room.game.kingdom_mut(id).treasury = AGENT_PRICE - 1;
                    room.seat_mut(id).news.clear();
                    room.game.increment_year();
                    room.resolve_missions();
                    assert!(matches!(
                        room.seat(id).news.as_slice(),
                        [News::Agent {
                            outcome: Writing::Unpaid | Writing::Unmasked,
                            ..
                        }]
                    ));
                    assert!(!room.dossier(id, on).agent);
                }
                [News::Agent {
                    at,
                    outcome: Writing::Unmasked,
                }] => {
                    unmasked = true;
                    assert_eq!(*at, on);
                    let d = room.dossier(id, on);
                    assert!(!d.agent);
                    assert!(d.ledger.is_none());
                    assert_eq!(d.caught, Some(year));
                    assert!(matches!(
                        room.seat(on).news.as_slice(),
                        [News::SpyCaught { by, agent: true }] if *by == id
                    ));
                    let heard = room.log.last().unwrap().told_to(Some(Kingdoms::Germany));
                    assert!(
                        heard.contains("agent de la France a été démasqué"),
                        "{heard}"
                    );
                }
                other => panic!("unexpected news {other:?}"),
            }
            if wrote && unmasked {
                return;
            }
        }
        panic!("both outcomes should show up in 200 draws (wrote {wrote}, unmasked {unmasked})");
    }

    #[test]
    fn a_standing_agent_pays_his_year_and_tells_what_moved() {
        let (id, on) = (Kingdoms::France, Kingdoms::Spain);
        for _ in 0..100 {
            let mut room = table();
            room.game.kingdom_mut(id).treasury = 10_000;
            let year = room.game.year;
            let report = Report::read(room.game.kingdom(on), year);
            let ledger = Ledger::read(room.game.kingdom(on), year, [0; 6]);
            let d = &mut room.seat_mut(id).dossiers[on.index()];
            d.agent = true;
            d.report = Some(report);
            d.ledger = Some(ledger);
            room.game.kingdom_mut(on).treasury -= 500;
            room.seat_mut(id).news.clear();
            room.game.increment_year();
            room.resolve_missions();
            if let [News::Agent {
                outcome: Writing::Letter { spent, .. },
                ..
            }] = room.seat(id).news.as_slice()
            {
                assert_eq!(room.game.kingdom(id).treasury, 10_000 - AGENT_PRICE);
                assert_eq!(*spent, Some(500));
                return;
            }
        }
        panic!("an agent should write at least once in 100 draws");
    }

    #[test]
    fn an_agent_is_dismissed_at_once_and_lost_with_the_realm() {
        let mut room = table();
        let (id, on) = (Kingdoms::France, Kingdoms::Spain);
        room.seat_mut(id).dossiers[on.index()].agent = true;
        assert_eq!(room.toggle_agent(id, on), Ok(()));
        assert!(!room.dossier(id, on).agent);
        room.seat_mut(id).dossiers[on.index()].agent = true;
        room.game.kingdom_mut(on).is_dead = true;
        room.seat_mut(id).news.clear();
        room.resolve_missions();
        assert!(matches!(
            room.seat(id).news.as_slice(),
            [News::Agent {
                outcome: Writing::Gone,
                ..
            }]
        ));
        assert!(!room.dossier(id, on).agent);
    }

    #[test]
    fn a_scout_finds_only_ruins_in_an_annexed_realm() {
        let mut room = table();
        let (id, on) = (Kingdoms::France, Kingdoms::Spain);
        room.toggle_scout(id, on).unwrap();
        room.game.kingdom_mut(on).is_dead = true;
        room.seat_mut(id).news.clear();
        room.resolve_missions();
        assert!(matches!(
            room.seat(id).news.as_slice(),
            [News::Scout {
                outcome: Scouting::Gone,
                ..
            }]
        ));
        assert!(room.dossier(id, on).report.is_none());
    }

    #[test]
    fn a_forecast_needs_a_report_and_the_report_ages_out() {
        let mut room = table();
        let (id, on) = (Kingdoms::France, Kingdoms::Spain);
        assert!(room.war_forecast(id, Some(on)).is_none());
        assert!(room.war_forecast(id, None).is_some());
        let year = room.game.year;
        let report = Report::read(room.game.kingdom(on), year);
        assert_eq!(report.spread(year), Some(0));
        assert_eq!(report.spread(year + 1), Some(20));
        assert_eq!(report.spread(year + 2), Some(40));
        assert_eq!(report.spread(year + 3), None);
        room.seat_mut(id).dossiers[on.index()].report = Some(report);
        let fc = room.war_forecast(id, Some(on)).expect("a forecast");
        assert_eq!((fc.target, fc.spread), (Some(on), 0));
        for _ in 0..3 {
            room.game.increment_year();
        }
        assert!(room.war_forecast(id, Some(on)).is_none());
    }

    /// A computer at year three with a report of the year on its eye.
    fn watching(temper: Temper, garrison: i32) -> (Room, Kingdoms, Kingdoms) {
        let mut room = table();
        let (id, on) = (Kingdoms::Germany, Kingdoms::Spain);
        room.game.year = 3;
        room.seat_mut(id).mind = Mind {
            temper,
            eye: Some(on),
            seen: None,
        };
        let k = room.game.kingdom_mut(id);
        k.soldiers = 60;
        k.soldiers_efficiency = 100;
        k.treasury = 1_000;
        room.seat_mut(id).dossiers[on.index()].report = Some(Report {
            year: 3,
            garrison,
            efficiency: 100,
            nobles: 1,
            merchants: 100,
            serfs: 1000,
            surface: 10_000,
        });
        (room, id, on)
    }

    #[test]
    fn a_computer_reads_its_dossier_and_strikes_on_a_favourable_report() {
        let (mut room, id, on) = watching(Temper::Bold, 20);
        room.run_computer_war(id);
        // 1.5 × 20 × 100 / 100 = 30 men, or three quarters of the 60: 45.
        let planned = &room.seat(id).planned;
        assert!(
            planned
                .iter()
                .any(|e| e.target == Some(on) && e.soldiers == 45),
            "{planned:?}"
        );
        assert!(room.scout_ordered(id, on));
        assert_eq!(room.seat(id).mind.eye, Some(on));
        assert_eq!(room.seat(id).mind.seen, None);
    }

    #[test]
    fn a_computer_looks_elsewhere_on_an_unfavourable_report() {
        let (mut room, id, on) = watching(Temper::Cautious, 200);
        // The watched realm is a speck: the new eye all but surely lands
        // elsewhere.
        room.game.kingdom_mut(on).surface = 1;
        room.run_computer_war(id);
        let eye = room.seat(id).mind.eye.expect("an eye");
        assert!(eye != id && eye != on, "{eye:?}");
        assert!(room.scout_ordered(id, eye));
        assert!(!room.scout_ordered(id, on));
    }

    #[test]
    fn a_stale_report_is_not_read() {
        let (mut room, id, on) = watching(Temper::Bold, 1);
        room.seat_mut(id).dossiers[on.index()]
            .report
            .as_mut()
            .unwrap()
            .year = 2;
        room.run_computer_war(id);
        // Only the coup de sang could have marched: never an ost cut to the
        // report (1.5 × 100 / 100 = 2 men).
        assert!(!room.seat(id).planned.iter().any(|e| e.soldiers == 2));
    }

    #[test]
    fn a_computer_scout_is_taken_and_told_like_a_seigneur_s() {
        let (id, on) = (Kingdoms::Germany, Kingdoms::Spain);
        let (mut back, mut caught) = (false, false);
        for _ in 0..200 {
            let (mut room, _, _) = watching(Temper::Measured, 20);
            room.seat_mut(on).news.clear();
            room.run_computer_war(id);
            assert!(room.scout_ordered(id, on));
            room.game.increment_year();
            room.resolve_missions();
            let d = room.dossier(id, on);
            if d.report.is_some_and(|r| r.year == 4) {
                back = true;
            } else {
                assert_eq!(d.caught, Some(4));
                assert!(room.seat(on).news.contains(&News::SpyCaught {
                    by: id,
                    agent: false
                }));
                caught = true;
            }
        }
        assert!(back && caught);
    }

    #[test]
    fn the_letter_tells_the_council_s_temper() {
        let (room, id, _) = watching(Temper::Cautious, 20);
        assert_eq!(room.temper(id), Some(Temper::Cautious));
        assert_eq!(room.temper(Kingdoms::France), None);
    }
}
