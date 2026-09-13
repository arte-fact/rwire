// The table as its lane holds it: the game, the six memories, what was
// heard, the outcomes — the same records as `state.rs`, word for word.
// Every lane plays its own table, rules and networks, in the registers
// and scratch of its own.

struct Kingdom {
    dead: i32,
    surface: i32,
    peasants: i32,
    nobles: i32,
    merchants: i32,
    soldiers: i32,
    efficiency: i32,
    ration: i32,
    treasury: i32,
    stocks: i32,
    harvest: i32,
    weather: i32,
    price: i32,
    to_sell: i32,
    listed: i32,
    listed_price: i32,
    rats: i32,
    marketplaces: i32,
    mills: i32,
    foundries: i32,
    shipyards: i32,
    palaces: i32,
    forts: i32,
    hospices: i32,
    rams: i32,
    scouts: i32,
    customs: i32,
    sales: i32,
    income: i32,
}

struct Demo {
    has: i32,
    births: i32,
    immigrants: i32,
    nobles_immigrants: i32,
    merchants_immigrants: i32,
    merchants_settled: i32,
    nobles_departed: i32,
    merchants_departed: i32,
    disease: i32,
    malnutrition: i32,
    starvation: i32,
    efficiency: i32,
    soldiers_starvation: i32,
    soldiers_desertion: i32,
}

struct Dossier {
    report: i32,
    report_year: i32,
    surface: i32,
    garrison: i32,
    forts: i32,
    efficiency: i32,
    ledger: i32,
    ledger_year: i32,
    treasury: i32,
    stocks: i32,
    aim: i32,
    met: i32,
    all: i32,
}

// What a seat's éclaireur read of one realm in one year.
struct Sighted {
    read: i32,
    surface: i32,
    garrison: i32,
    forts: i32,
    efficiency: i32,
}

struct Memory {
    demo: Demo,
    eco: i32,
    net: i32,
    maintenance: i32,
    plague: i32,
    answer: array<f32, A_OUT>,
    orders: array<f32, B_OUT>,
    dossiers: array<Dossier, 6>,
    sighted: array<array<Sighted, 6>, JOURNAL_YEARS>,
}

// A realm as the record kept it for one year (brain.rs `Recorded`).
struct Recorded {
    title: i32,
    alive: i32,
    soldiers: i32,
    treasury: i32,
    starved: i32,
    marched_by: array<i32, 6>,
    beaten_by: array<i32, 6>,
    lost_to: array<i32, 6>,
}

struct RecordedYear {
    realms: array<Recorded, 6>,
}

struct Heard {
    marched_on: array<i32, 6>,
    marched_by: array<i32, 6>,
    beaten_by: array<i32, 6>,
    lost_to: array<i32, 6>,
}

struct Outcome {
    crowned: i32,
    fell: i32,
    starved_out: i32,
    years: i32,
    progress: f32,
    born: i32,
    settled: i32,
    nobles_come: i32,
    starved: i32,
    nobles_gone: i32,
    prince: i32,
    king: i32,
    readings: i32,
    walls: f32,
}

struct Table {
    year: i32,
    barbarians: i32,
    done: i32,
    rng: array<u32, 4>,
    genome: array<u32, 6>,
    reads: array<u32, 6>,
    seat: array<u32, 6>,
    stage: u32,
    longest: i32,
    letters: u32,
    kingdoms: array<Kingdom, 6>,
    memories: array<Memory, 6>,
    heard: array<Heard, 6>,
    outcomes: array<Outcome, 6>,
    journal: array<RecordedYear, JOURNAL_YEARS>,
}

struct Params {
    years: u32,
}

@group(0) @binding(0) var<storage, read> genomes: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> tables: array<Table>;
@group(0) @binding(2) var<storage, read> params: Params;

var<private> K: array<Kingdom, 6>;
var<private> M: array<Memory, 6>;
var<private> H: array<Heard, 6>;
var<private> O: array<Outcome, 6>;
var<private> J: array<RecordedYear, JOURNAL_YEARS>;
var<private> year: i32;
var<private> barbarians: i32;
var<private> done: i32;
var<private> seat_genome: array<u32, 6>;
var<private> seat_reads: array<u32, 6>;
var<private> seat_stage: array<u32, 6>;
var<private> table_stage: u32;
var<private> longest: i32;
var<private> letters: u32;

// The rungs (brain::Stage) and the letters (brain::Letters), in order.
const STAGE_EMPEROR: u32 = 1u;
const STAGE_MARKET: u32 = 2u;
const STAGE_GUARD: u32 = 3u;
const STAGE_WAR: u32 = 4u;
const LETTERS_SCOUTS: u32 = 1u;
const LETTERS_ALL: u32 = 2u;

// The titles, lowest first.
const DUKE: i32 = 0;
const PRINCE: i32 = 1;
const KING: i32 = 2;
const EMPEROR: i32 = 3;

// What each title asks, in Requirement::ALL order: peasants, land ratio,
// nobles, mills, marketplaces, foundries, palaces, forts, hospices.
var<private> REQUIREMENTS: array<array<i32, 9>, 3> = array(
    array(2300, 48, 10, 4, 8, 0, 2, 1, 1),
    array(2600, 50, 25, 6, 14, 1, 6, 3, 3),
    array(3100, 50, 40, 6, 14, 1, 10, 5, 5),
);

fn load(t: u32) {
    for (var i = 0u; i < 6u; i++) {
        K[i] = tables[t].kingdoms[i];
        M[i] = tables[t].memories[i];
        H[i] = tables[t].heard[i];
        O[i] = tables[t].outcomes[i];
        seat_genome[i] = tables[t].genome[i];
        seat_reads[i] = tables[t].reads[i];
        seat_stage[i] = tables[t].seat[i];
    }
    for (var y = 0u; y < JOURNAL_YEARS; y++) {
        J[y] = tables[t].journal[y];
    }
    year = tables[t].year;
    barbarians = tables[t].barbarians;
    done = tables[t].done;
    table_stage = tables[t].stage;
    longest = tables[t].longest;
    letters = tables[t].letters;
    for (var i = 0u; i < 4u; i++) {
        rng[i] = tables[t].rng[i];
    }
}

fn store(t: u32) {
    for (var i = 0u; i < 6u; i++) {
        tables[t].kingdoms[i] = K[i];
        tables[t].memories[i] = M[i];
        tables[t].heard[i] = H[i];
        tables[t].outcomes[i] = O[i];
    }
    for (var y = 0u; y < JOURNAL_YEARS; y++) {
        tables[t].journal[y] = J[y];
    }
    tables[t].year = year;
    tables[t].barbarians = barbarians;
    tables[t].done = done;
    for (var i = 0u; i < 4u; i++) {
        tables[t].rng[i] = rng[i];
    }
}

// --- The realm's figures (kingdom.rs) ---

fn mouths(k: u32) -> i32 {
    return K[k].peasants + K[k].merchants + K[k].nobles * 3;
}

fn population(k: u32) -> i32 {
    return K[k].peasants + K[k].nobles + K[k].merchants;
}

fn cultivated(k: u32) -> i32 {
    return K[k].surface - K[k].peasants - 2 * K[k].nobles - K[k].palaces - K[k].merchants
        - 2 * K[k].soldiers;
}

fn land_ratio(k: u32) -> i32 {
    if (K[k].peasants > 0) {
        return K[k].surface * 10 / K[k].peasants;
    }
    return 0;
}

fn for_sale(k: u32) -> i32 {
    return max(K[k].to_sell, 0);
}

fn have(k: u32, what: u32) -> i32 {
    switch (what) {
        case 0u: { return K[k].peasants; }
        case 1u: { return land_ratio(k); }
        case 2u: { return K[k].nobles; }
        case 3u: { return K[k].mills; }
        case 4u: { return K[k].marketplaces; }
        case 5u: { return K[k].foundries; }
        case 6u: { return K[k].palaces; }
        case 7u: { return K[k].forts; }
        default: { return K[k].hospices; }
    }
}

fn need(title: i32, what: u32) -> i32 {
    return REQUIREMENTS[title - 1][what];
}

// Criterion::met: the land ratio must be exceeded, the rest reached.
fn met(k: u32, title: i32, what: u32) -> bool {
    let h = have(k, what);
    let n = need(title, what);
    if (what == 1u) {
        return h > n;
    }
    return h >= n;
}

fn meets(k: u32, title: i32) -> bool {
    if (title == DUKE) {
        return true;
    }
    for (var w = 0u; w < 9u; w++) {
        if (need(title, w) > 0 && !met(k, title, w)) {
            return false;
        }
    }
    return true;
}

fn title(k: u32) -> i32 {
    for (var t = EMPEROR; t > DUKE; t--) {
        if (meets(k, t)) {
            return t;
        }
    }
    return DUKE;
}

fn round_away(x: f32) -> f32 {
    return sign(x) * floor(abs(x) + 0.5);
}

fn to_i32(x: f32) -> i32 {
    if (x != x) {
        return 0;
    }
    return i32(x);
}
