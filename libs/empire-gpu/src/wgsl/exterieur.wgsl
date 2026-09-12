// The Extérieur of one seat (brain.rs `decode_missions` and
// `decode_expeditions`, arena.rs `watch` and `send_scout`, intel.rs
// `Dossier`): the éclaireur and the agents sent on the first reading of
// `y`, the expeditions listed on the second. Each lane on its table.

const SCOUT_PRICE: i32 = 150;
const SCOUT_COST: i32 = 16;
const AGENT_PRICE: i32 = 400;
const CAUGHT: i32 = 6;
const FIRST_WAR_YEAR: i32 = 3;

// The year's expeditions, in the order the seats gave them: a foe of
// -1 is the barbarians.
struct Expedition {
    attacker: i32,
    foe: i32,
    men: i32,
    rams: i32,
}

var<private> E: array<Expedition, 36>;
var<private> n_expeditions: u32;

fn rival(s: u32, j: u32) -> u32 {
    return (s + 1u + j) % 6u;
}

fn send_scout(k: u32, free: bool) -> bool {
    if (free) {
        return true;
    }
    var man = SCOUT_COST;
    if (K[k].scouts > 0) {
        man = 0;
    }
    if (K[k].treasury < SCOUT_PRICE + man) {
        return false;
    }
    if (K[k].scouts > 0) {
        K[k].scouts -= 1;
    }
    K[k].treasury -= SCOUT_PRICE + man;
    return true;
}

// The éclaireur of `s` on `o`: caught one time in six, else a report.
fn scout(s: u32, o: u32) {
    if (random(0, CAUGHT) == 0) {
        return;
    }
    M[s].dossiers[o].report = 1;
    M[s].dossiers[o].report_year = year;
    M[s].dossiers[o].surface = K[o].surface;
    M[s].dossiers[o].garrison = K[o].soldiers;
    M[s].dossiers[o].forts = K[o].forts;
    M[s].dossiers[o].efficiency = K[o].efficiency;
}

// The agent of `s` in `o`: caught one time in six, else a ledger with
// how far `o` stands from its next title.
fn agent(s: u32, o: u32) {
    if (random(0, CAUGHT) == 0) {
        return;
    }
    M[s].dossiers[o].ledger = 1;
    M[s].dossiers[o].ledger_year = year;
    M[s].dossiers[o].treasury = K[o].treasury;
    M[s].dossiers[o].stocks = K[o].stocks;
    let next = title(o) + 1;
    if (next > EMPEROR) {
        M[s].dossiers[o].aim = -1;
        M[s].dossiers[o].met = 0;
        M[s].dossiers[o].all = 0;
        return;
    }
    var all = 0;
    var got = 0;
    for (var w = 0u; w < 9u; w++) {
        if (need(next, w) > 0) {
            all += 1;
            if (met(o, next, w)) {
                got += 1;
            }
        }
    }
    M[s].dossiers[o].aim = next;
    M[s].dossiers[o].met = got;
    M[s].dossiers[o].all = all;
}

// The missions read on `y`: the éclaireur on the rival wanted most (the
// last of equals, "no one" always in the running), the agents on every
// rival asked for.
fn missions(s: u32) {
    if (seat_stage[s] < STAGE_GUARD) {
        return;
    }
    if (letters >= LETTERS_SCOUTS || K[s].treasury >= SCOUT_PRICE + SCOUT_COST) {
        var best = 6u;
        var most = 0.0;
        for (var j = 0u; j < 6u; j++) {
            if (j < 5u && K[rival(s, j)].dead != 0) {
                continue;
            }
            if (best == 6u || y[6u + j] >= most) {
                best = j;
                most = y[6u + j];
            }
        }
        if (best < 5u) {
            let on = rival(s, best);
            if (send_scout(s, letters >= LETTERS_SCOUTS)) {
                scout(s, on);
                O[s].readings += 1;
            }
        }
    }
    let free = letters == LETTERS_ALL;
    for (var j = 0u; j < 5u; j++) {
        let on = rival(s, j);
        if (y[12u + j] < 0.5 || K[on].dead != 0) {
            continue;
        }
        if (!free) {
            if (K[s].treasury < AGENT_PRICE) {
                continue;
            }
            K[s].treasury -= AGENT_PRICE;
        }
        agent(s, on);
        O[s].readings += 1;
    }
}

// The expeditions read on `y`, appended to `E`: the targets wanted at
// least at the deadband, strongest want first (equals in rival order),
// as many as the nobles allow, each taking its share of the soldiers
// while some remain; the rams go with the first on a realm.
fn expeditions(s: u32) {
    var foe: array<i32, 6>;
    var want: array<f32, 6>;
    var n = 0u;
    if (seat_stage[s] >= STAGE_WAR && year >= FIRST_WAR_YEAR) {
        for (var j = 0u; j < 5u; j++) {
            let o = rival(s, j);
            if (K[o].dead == 0 && y[j] >= DEADBAND) {
                foe[n] = i32(o);
                want[n] = y[j];
                n += 1u;
            }
        }
    }
    if (seat_stage[s] >= STAGE_EMPEROR && barbarians > 0 && y[5] >= DEADBAND) {
        foe[n] = -1;
        want[n] = y[5];
        n += 1u;
    }
    for (var i = 1u; i < n; i++) {
        let t = foe[i];
        let w = want[i];
        var j = i;
        for (; j > 0u && want[j - 1u] < w; j--) {
            foe[j] = foe[j - 1u];
            want[j] = want[j - 1u];
        }
        foe[j] = t;
        want[j] = w;
    }
    let first = n_expeditions;
    var garrison = K[s].soldiers;
    let allowed = max(K[s].nobles / 4 + 1, 0);
    for (var i = 0u; i < n && i32(i) < allowed; i++) {
        let men = min(to_i32(round_away(want[i] * f32(K[s].soldiers))), garrison);
        if (men < 1) {
            break;
        }
        garrison -= men;
        E[n_expeditions] = Expedition(i32(s), foe[i], men, 0);
        n_expeditions += 1u;
    }
    let rams = to_i32(round_away(y[18] * f32(K[s].rams)));
    if (rams > 0) {
        for (var i = first; i < n_expeditions; i++) {
            if (E[i].foe >= 0) {
                E[i].rams = rams;
                break;
            }
        }
    }
}
