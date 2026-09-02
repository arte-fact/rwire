//! Shared game table: state, turn order, battles and every handler.

use std::collections::{BTreeMap, VecDeque};

use empire_lib::demography::{apply_feed, Council, YearDemography};
use empire_lib::economy::{apply_economy, apply_taxes, economy_report, Taxes, YearEconomy};
use empire_lib::events::{check_random_events, RulerDeathCause};
use empire_lib::harvests::{apply_grain_harvest, apply_rat_loss_rate, apply_seed_grain};
use empire_lib::ia::plan_ai_turn;
use empire_lib::investments::{apply_investment, InvestmentType};
use empire_lib::trade::{apply_trade, calculate_buy_cost, max_land_sale, Trade, MAX_GRAIN_PRICE};
use empire_lib::war::{
    apply_barbarian_battle_result, apply_kingdom_battle_result, simulate_barbarian_battle,
    simulate_kingdom_battle, BarbarianBattleResult, BattleProgress, BattleResult,
};
use empire_lib::{EmpireGame, Kingdom, Kingdoms, KINGDOMS};
use rand::Rng;
use rwire::{handler, EventContext, HandlerSpec, State};

use crate::ui::fmt;

/// Ticker period; battle frames and computer pauses are counted in ticks.
pub const TICK_MS: u64 = 100;
const BATTLE_FRAMES: usize = 40;
const BATTLE_LINGER: u8 = 25;
const AI_PAUSE: u8 = 15;
const JOURNAL_LEN: usize = 40;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Stage {
    #[default]
    Lobby,
    Playing,
    Over,
}

/// The active player's position inside their turn.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Step {
    #[default]
    Weather,
    Trade,
    /// Grain rations and tax rates, decided together.
    Council,
    /// Census: how the people fared this year.
    Report,
    /// The treasury's ledger: what each source brought in.
    Treasury,
    /// Purchases: buildings, soldiers, palaces.
    Invest,
    War,
}

impl Step {
    pub const LABELS: [&'static str; 7] = [
        "Saison", "Commerce", "Conseil", "Peuple", "Trésor", "Achats", "Guerre",
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
    pub text: String,
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

/// The council's decision as the sliders currently stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Draft {
    pub peasants: i32,
    pub soldiers: i32,
    pub customs: i32,
    pub sales: i32,
    pub income: i32,
}

impl Draft {
    /// The grain sliders' upper bounds: beyond 2× the people's needs
    /// immigration barely grows; beyond 1.5× the army's needs efficiency is
    /// already maxed — so the sliders stop there.
    pub fn bounds(k: &Kingdom) -> (i32, i32) {
        let stocks = k.grain_stocks.max(0);
        (
            (k.peasants_grain_needs() * 2).min(stocks).max(0),
            (k.soldiers_grain_needs() * 3 / 2).min(stocks).max(0),
        )
    }

    /// Where the sliders start: full rations and last year's rates.
    pub fn initial(k: &Kingdom) -> Draft {
        let (peasants_max, soldiers_max) = Self::bounds(k);
        let t = k.taxes();
        Draft {
            peasants: k.peasants_grain_needs().min(peasants_max),
            soldiers: k.soldiers_grain_needs().min(soldiers_max),
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

    /// Inside the stocks (the army is served last) and the tax caps.
    pub fn clamped(self, k: &Kingdom) -> Draft {
        let stocks = k.grain_stocks.max(0);
        let peasants = self.peasants.clamp(0, stocks);
        let t = self.taxes().clamped();
        Draft {
            peasants,
            soldiers: self.soldiers.clamp(0, stocks - peasants),
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
            grain_for_peasants: self.peasants,
            grain_for_soldiers: self.soldiers,
            taxes: self.taxes(),
        }
    }
}

#[derive(Clone, Default)]
pub struct Seat {
    /// Token of the connection playing this kingdom; `None` = computer.
    pub owner: Option<u64>,
    /// The council's sliders as last released; `None` = untouched this turn.
    pub draft: Option<Draft>,
    pub demo: Option<YearDemography>,
    pub eco: Option<YearEconomy>,
    /// Bushels eaten by the rats at the start of the year (the season report
    /// shows the amount; the kingdom only keeps the rate).
    pub rats: i32,
    /// Feedback from the player's last action.
    pub notice: Option<String>,
    /// Investment type currently selected in the investment form.
    pub invest_kind: Option<InvestmentType>,
    /// Expeditions left this turn (original rule: nobles / 4 + 1).
    pub attacks_left: i32,
}

#[derive(Clone)]
pub struct Attack {
    pub attacker: Kingdoms,
    /// `None` = the barbarians.
    pub target: Option<Kingdoms>,
    pub soldiers: i32,
}

pub enum Outcome {
    Kingdom(Kingdoms, BattleResult),
    Barbarians(BarbarianBattleResult),
}

/// A battle being replayed for everyone: precomputed, applied when the
/// animation ends so nobody sees the outcome early.
pub struct Battle {
    pub attack: Attack,
    pub frames: Vec<BattleProgress>,
    pub cursor: usize,
    linger: u8,
    outcome: Outcome,
    /// Defender head-count at the first frame (for the progress bar).
    pub defender_start: i32,
    pub summary: String,
}

impl Battle {
    pub fn frame(&self) -> &BattleProgress {
        &self.frames[self.cursor.min(self.frames.len() - 1)]
    }

    pub fn finished(&self) -> bool {
        self.cursor + 1 >= self.frames.len()
    }

    /// Land taken by the end of the battle (the outcome is known from the start).
    pub fn surface_conquered(&self) -> i32 {
        match &self.outcome {
            Outcome::Kingdom(_, r) => r.surface_conquered,
            Outcome::Barbarians(r) => r.surface_conquered,
        }
    }

    /// The land taken so far in the replay: the final spoils, grown frame by frame.
    pub fn surface_so_far(&self) -> i32 {
        let last = (self.frames.len() - 1).max(1);
        (self.surface_conquered() as i64 * self.cursor.min(last) as i64 / last as i64) as i32
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
    /// Index into [`KINGDOMS`] of the kingdom whose turn it is.
    pub turn: usize,
    /// Step of the active (human) player.
    pub step: Step,
    pub battle: Option<Battle>,
    queue: VecDeque<Attack>,
    /// Ticks before the computer plays ("Un moment…").
    pause: u8,
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

    /// The kingdom whose turn it is.
    pub fn active(&self) -> Option<Kingdoms> {
        (self.stage == Stage::Playing)
            .then(|| KINGDOMS.get(self.turn).copied())
            .flatten()
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
        self.log.push(Entry {
            year: self.game.year,
            about: about.into_iter().collect(),
            text: line.into(),
        });
        if self.log.len() > JOURNAL_LEN {
            self.log.remove(0);
        }
    }

    fn note(&mut self, id: Kingdoms, msg: impl Into<String>) {
        self.seat_mut(id).notice = Some(msg.into());
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
        let weather = self.game.random_weather();
        self.journal([], weather.sentence());
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
            self.seat_mut(id).rats = rats;
        }
        self.turn = 0;
    }

    /// Hand the turn to `self.turn`, skipping the dead and rolling the year over.
    fn start_turn(&mut self) {
        loop {
            if self.stage != Stage::Playing {
                return;
            }
            let Some(&id) = KINGDOMS.get(self.turn) else {
                self.end_year();
                continue;
            };
            if self.game.kingdom(id).is_dead {
                self.turn += 1;
                continue;
            }
            self.step = Step::Weather;
            if self.is_computer(id) {
                self.pause = AI_PAUSE;
            } else {
                let attacks = self.game.kingdom(id).nobles / 4 + 1;
                let seat = self.seat_mut(id);
                seat.draft = None;
                seat.demo = None;
                seat.eco = None;
                seat.notice = None;
                seat.attacks_left = attacks;
            }
            return;
        }
    }

    fn next_turn(&mut self) {
        self.turn += 1;
        self.start_turn();
    }

    fn end_year(&mut self) {
        for id in KINGDOMS {
            let starved = self
                .seat(id)
                .demo
                .as_ref()
                .is_some_and(|d| d.starvation_victims > 0);
            let k = self.game.kingdom_mut(id);
            if k.is_dead {
                continue;
            }
            let title = k.full_title();
            let (plague, death) = check_random_events(k, starved);
            if plague.occurred {
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
            }
            if death.occurred {
                self.journal([id], format!("{title} {}.", death_fr(&death.cause)));
            }
        }
        if self.check_over() {
            return;
        }
        self.game.increment_year();
        self.begin_year();
    }

    fn check_over(&mut self) -> bool {
        let humans_alive = self.humans().any(|id| !self.game.kingdom(id).is_dead);
        if humans_alive && self.game.alive_kingdoms().len() > 1 {
            return false;
        }
        self.stage = Stage::Over;
        self.battle = None;
        self.queue.clear();
        self.history.push(self.surfaces());
        self.journal([], "La partie est terminée.");
        true
    }

    // -- ticker: computer turns and battle replay ----------------------------

    /// Advance the table clock by one tick. Returns true when the view changed.
    pub fn tick(&mut self) -> bool {
        if self.stage != Stage::Playing {
            return false;
        }
        if let Some(b) = &mut self.battle {
            if !b.finished() {
                b.cursor += 1;
                return true;
            }
            if b.linger > 0 {
                b.linger -= 1;
                return b.linger == 0;
            }
            self.finish_battle();
            self.after_battle();
            return true;
        }
        if let Some(attack) = self.queue.pop_front() {
            self.start_battle(attack);
            if self.battle.is_none() {
                self.after_battle();
            }
            return true;
        }
        if self.pause > 0 {
            self.pause -= 1;
            if self.pause > 0 {
                return false;
            }
            self.run_computer_turn();
            return true;
        }
        false
    }

    /// Once the queue drains, a computer's turn is over.
    fn after_battle(&mut self) {
        if self.stage == Stage::Playing
            && self.battle.is_none()
            && self.queue.is_empty()
            && self.active().is_some_and(|id| self.is_computer(id))
        {
            self.next_turn();
        }
    }

    fn run_computer_turn(&mut self) {
        let Some(id) = self.active() else { return };
        let decision = plan_ai_turn(&mut self.game, id);
        if let Some((amount, price)) = decision.grain_listed {
            self.journal(
                [id],
                format!(
                    "La {} met {amount} boisseaux en vente à {price} la mesure.",
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
        }
        for soldiers in decision.barbarian_attacks {
            self.queue.push_back(Attack {
                attacker: id,
                target: None,
                soldiers,
            });
        }
        for (target, soldiers) in decision.kingdom_attacks {
            self.queue.push_back(Attack {
                attacker: id,
                target: Some(target),
                soldiers,
            });
        }
        if self.queue.is_empty() {
            self.next_turn();
        }
    }

    /// Validate an attack for the active player.
    fn launch(&mut self, attack: Attack) -> Result<(), String> {
        let a = self.game.kingdom(attack.attacker);
        if a.soldiers < 1 {
            return Err("Vous n'avez plus d'hommes d'armes.".into());
        }
        if self.seat(attack.attacker).attacks_left < 1 {
            return Err("Vos nobles ne peuvent mener davantage d'expéditions cette année.".into());
        }
        match attack.target {
            None if self.game.barbarians_surface <= 0 => {
                return Err("Toutes les terres barbares ont déjà été conquises.".into());
            }
            Some(t) if t == attack.attacker || self.game.kingdom(t).is_dead => {
                return Err("Ce royaume n'est plus.".into());
            }
            Some(_) if self.game.year < 3 => {
                return Err("Nul ne peut attaquer un autre royaume avant la 3ème année.".into());
            }
            _ => {}
        }
        let attacker = attack.attacker;
        self.start_battle(attack);
        if self.battle.is_some() {
            self.seat_mut(attacker).attacks_left -= 1;
        }
        Ok(())
    }

    /// Precompute a battle and start replaying it. Silently drops attacks that
    /// no longer make sense (a computer's plan can be stale by the time it runs).
    fn start_battle(&mut self, attack: Attack) {
        let a = self.game.kingdom(attack.attacker);
        let soldiers = attack.soldiers.min(a.soldiers);
        if a.is_dead || soldiers < 1 {
            return;
        }
        let attacker = a.titled_name();
        let mut frames = Vec::new();
        let (outcome, defender_start, foe) = match attack.target {
            Some(t) => {
                let d = self.game.kingdom(t);
                if t == attack.attacker || d.is_dead {
                    return;
                }
                let start = if d.soldiers > 0 {
                    d.soldiers
                } else {
                    d.peasants
                };
                let r = simulate_kingdom_battle(&self.game, attack.attacker, t, soldiers, |p| {
                    frames.push(p.clone())
                });
                (Outcome::Kingdom(t, r), start, format!("la {}", t.name()))
            }
            None => {
                if self.game.barbarians_surface <= 0 {
                    return;
                }
                let r = simulate_barbarian_battle(&self.game, attack.attacker, soldiers, |p| {
                    frames.push(p.clone())
                });
                let start = frames.iter().map(|f| f.defender_soldiers).max();
                (
                    Outcome::Barbarians(r),
                    start.unwrap_or(1),
                    "les Barbares".to_string(),
                )
            }
        };
        if frames.is_empty() {
            frames.push(BattleProgress {
                attacker_soldiers: soldiers,
                defender_soldiers: defender_start,
                defender_peasants: 0,
                population_defending: false,
            });
        }
        let summary = match &outcome {
            Outcome::Kingdom(t, r) => {
                let defender = self.game.kingdom(*t).full_title();
                if r.defender_conquered {
                    format!("Le pays de {defender} est conquis ! Ses serfs jurent fidélité à {attacker}.")
                } else if r.attacker_won {
                    won(&attacker, r.surface_conquered)
                } else {
                    let mut m = format!(
                        "{attacker} perd, mais arrache tout de même {} arpents.",
                        r.surface_conquered
                    );
                    if let Some(d) = &r.collateral_damage {
                        m.push_str(&format!(
                            " Grande bataille : {} serfs, {} foires et {} nobles ennemis anéantis.",
                            d.peasants_killed, d.marketplaces_destroyed, d.nobles_killed
                        ));
                    }
                    m
                }
            }
            Outcome::Barbarians(r) => {
                if r.all_barbarians_conquered {
                    format!("{attacker} conquiert les dernières terres barbares ; les survivants ont fui.")
                } else if r.attacker_won {
                    won(&attacker, r.surface_conquered)
                } else {
                    format!(
                        "{attacker} perd, mais arrache tout de même {} arpents.",
                        r.surface_conquered
                    )
                }
            }
        };
        self.journal(
            [attack.attacker].into_iter().chain(attack.target),
            format!("{attacker} marche sur {foe} avec {soldiers} hommes d'armes."),
        );
        self.battle = Some(Battle {
            attack: Attack { soldiers, ..attack },
            frames: sample(frames),
            cursor: 0,
            linger: BATTLE_LINGER,
            outcome,
            defender_start: defender_start.max(1),
            summary,
        });
    }

    fn finish_battle(&mut self) {
        let Some(b) = self.battle.take() else { return };
        let id = b.attack.attacker;
        match &b.outcome {
            Outcome::Kingdom(t, r) => {
                apply_kingdom_battle_result(&mut self.game, id, *t, b.attack.soldiers, r);
            }
            Outcome::Barbarians(r) => {
                apply_barbarian_battle_result(&mut self.game, id, b.attack.soldiers, r);
            }
        }
        self.journal([id].into_iter().chain(b.attack.target), b.summary.clone());
        if !self.is_computer(id) {
            self.note(id, b.summary);
        }
        self.check_over();
    }
}

fn won(attacker: &str, arpents: i32) -> String {
    if arpents > 0 {
        format!("{attacker} gagne : {arpents} arpents conquis.")
    } else {
        format!("{attacker} repousse l'ennemi sans gagner un arpent.")
    }
}

/// Keep a battle replay to a few seconds whatever its length.
fn sample(frames: Vec<BattleProgress>) -> Vec<BattleProgress> {
    if frames.len() <= BATTLE_FRAMES {
        return frames;
    }
    let last = frames.len() - 1;
    (0..BATTLE_FRAMES)
        .map(|i| frames[i * last / (BATTLE_FRAMES - 1)].clone())
        .collect()
}

fn death_fr(cause: &RulerDeathCause) -> &'static str {
    match cause {
        RulerDeathCause::None => "",
        RulerDeathCause::Assassination => "a été assassiné par un noble ambitieux",
        RulerDeathCause::HuntingAccident => "est mort dans un accident de chasse",
        RulerDeathCause::FoodPoisoning => "est mort empoisonné (le cuisinier a été exécuté)",
        RulerDeathCause::NaturalCauses => "est mort de sa belle mort",
        RulerDeathCause::StarvationAssassination => "a été assassiné par une mère affamée",
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

/// The caller's kingdom if it is their turn, at `step`, with no battle running.
fn acting(room: &Room, token: u64, step: Step) -> Option<Kingdoms> {
    let id = room.seat_of(token)?;
    (room.active() == Some(id) && room.step == step && room.battle.is_none()).then_some(id)
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
        room.start_turn();
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

/// Move to the next step; from the war step this ends the turn.
#[handler]
pub fn advance(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, _, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = room.seat_of(token) else {
        return;
    };
    if room.active() != Some(id) || room.battle.is_some() {
        return;
    }
    room.seat_mut(id).notice = None;
    match room.step {
        Step::Weather => room.step = Step::Trade,
        Step::Trade => room.step = Step::Council,
        Step::Council => {} // only the council form advances this step
        Step::Report => room.step = Step::Treasury,
        Step::Treasury => room.step = Step::Invest,
        Step::Invest => room.step = Step::War,
        Step::War => room.next_turn(),
    }
}

/// Buy grain from the seller named by the first extra param byte (kingdom number).
#[handler]
pub fn buy_grain(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, extra, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = acting(room, token, Step::Trade) else {
        return;
    };
    let Some(seller) = extra
        .first()
        .and_then(|&n| Kingdoms::from_number(i32::from(n)))
    else {
        return;
    };
    if seller == id {
        return;
    }
    let s = room.game.kingdom(seller);
    let price = s.grain_price.min(MAX_GRAIN_PRICE);
    let on_sale = s.grain_to_sell;
    if on_sale < 1 || price < 1 {
        room.note(
            id,
            format!("La {} n'a pas de grain à vendre.", seller.name()),
        );
        return;
    }
    let amount = num(ctx, "buy_amount").clamp(1, 500.min(on_sale));
    let cost = calculate_buy_cost(amount, price);
    if cost > room.game.kingdom(id).treasury {
        room.note(
            id,
            format!(
                "Le trésor ne couvre pas les {cost} {} demandés.",
                id.currency()
            ),
        );
        return;
    }
    apply_trade(&mut room.game, id, Trade::Buy { amount, seller });
    room.note(
        id,
        format!(
            "{amount} boisseaux achetés à la {} pour {cost} {}.",
            seller.name(),
            id.currency()
        ),
    );
}

#[handler]
pub fn sell_grain(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, _, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = acting(room, token, Step::Trade) else {
        return;
    };
    let stocks = room.game.kingdom(id).grain_stocks;
    if stocks < 1 {
        return;
    }
    let amount = num(ctx, "sell_amount").clamp(1, stocks);
    let price = num(ctx, "price").clamp(1, MAX_GRAIN_PRICE);
    apply_trade(&mut room.game, id, Trade::Sell { amount, price });
    room.note(
        id,
        format!("{amount} boisseaux mis en vente à {price} le boisseau."),
    );
}

#[handler]
pub fn sell_land(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, _, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = acting(room, token, Step::Trade) else {
        return;
    };
    let max = max_land_sale(room.game.kingdom(id).surface);
    if max < 1 {
        return;
    }
    let arpents = num(ctx, "arpents").clamp(1, max);
    apply_trade(&mut room.game, id, Trade::SellLand { arpents });
    room.note(id, format!("{arpents} arpents vendus aux Barbares."));
}

/// A council slider released: keep its value so the forecast re-centres on
/// it. The first extra param byte names the slider ([`Field`]).
#[handler]
pub fn draft(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, extra, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = acting(room, token, Step::Council) else {
        return;
    };
    let Some(field) = extra.first().and_then(|&f| Field::from_u8(f)) else {
        return;
    };
    let Some(value) = ctx.text().and_then(|v| v.trim().parse::<i32>().ok()) else {
        return;
    };
    let mut draft = room.draft(id);
    draft.set(field, value);
    let draft = draft.clamped(room.game.kingdom(id));
    room.seat_mut(id).draft = Some(draft);
}

/// Promulgate the council's decision: rations and rates for the year.
#[handler]
pub fn feed(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, _, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = acting(room, token, Step::Council) else {
        return;
    };
    let weather = room.game.weather;
    let mut draft = room.draft(id);
    for field in Field::ALL {
        if let Some(v) = ctx.field(field.name()).and_then(|v| v.trim().parse().ok()) {
            draft.set(field, v);
        }
    }
    let k = room.game.kingdom(id);
    let council = draft.clamped(k).council();
    let k = room.game.kingdom_mut(id);
    apply_taxes(k, council.taxes);
    let demo = apply_feed(k, council);
    let eco = economy_report(k, weather, demo.immigrants);
    apply_economy(k, &eco);
    let delta = demo.population_delta();
    room.journal(
        [id],
        format!(
            "La {} a {} {} sujets taillables et corvéables à merci.",
            id.name(),
            delta_verb(delta),
            delta.abs()
        ),
    );
    let seat = room.seat_mut(id);
    seat.draft = None;
    seat.demo = Some(demo);
    seat.eco = Some(eco);
    seat.notice = None;
    room.step = Step::Report;
}

/// The chronicle's verb for a population delta.
pub fn delta_verb(delta: i32) -> &'static str {
    match delta {
        n if n > 0 => "gagné",
        n if n < 0 => "perdu",
        _ => "conservé",
    }
}

/// The investment type picked in the economy form (drives the quantity slider).
#[handler]
pub fn pick_investment(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, _, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = room.seat_of(token) else {
        return;
    };
    let kind = ctx
        .text()
        .and_then(|v| v.trim().parse::<i32>().ok())
        .and_then(InvestmentType::from_number);
    room.seat_mut(id).invest_kind = kind;
}

#[handler]
pub fn invest(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, _, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = acting(room, token, Step::Invest) else {
        return;
    };
    let Some(kind) = InvestmentType::from_number(num(ctx, "kind")) else {
        return;
    };
    let amount = num(ctx, "invest_amount").max(0);
    let result = apply_investment(room.game.kingdom_mut(id), kind, amount);
    let msg = match result.error {
        Some(err) => err,
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
            m
        }
    };
    room.note(id, msg);
}

#[handler]
pub fn attack(rooms: &mut Rooms, ctx: &EventContext) {
    let Some((token, _, room)) = table(rooms, ctx) else {
        return;
    };
    let Some(id) = acting(room, token, Step::War) else {
        return;
    };
    let soldiers = num(ctx, "soldiers").clamp(1, room.game.kingdom(id).soldiers.max(1));
    let attack = Attack {
        attacker: id,
        target: Kingdoms::from_number(num(ctx, "target")),
        soldiers,
    };
    room.seat_mut(id).notice = None;
    if let Err(msg) = room.launch(attack) {
        room.note(id, msg);
    }
}
