// The campaign (campaign.rs `march_with`, `raid`, `apply_battle`;
// front.rs `fight`, `apply_front`; war.rs `simulate_barbarian_battle`):
// the expeditions of `E` leave their realms, the fronts are fought on a
// copy of the realms as they stood then, and each front's result is
// applied to the table as soon as it is fought — it never reads the
// table. Each lane on its table.

const GOLD_PER_MAN: i32 = 10;
const GRAIN_PER_MAN: i32 = 20;
const MILITIA_EFFICIENCY: i32 = 50;
const RAM_PACE: i32 = 8;
const RAM_ESCORT: i32 = 10;
const ARPENTS_PER_MAN: i32 = 20;

// The realms as they stood when the armies left, in what the fronts
// read of them.
struct Field {
    surface: i32,
    peasants: i32,
    nobles: i32,
    merchants: i32,
    soldiers: i32,
    efficiency: i32,
    treasury: i32,
    stocks: i32,
    forts: i32,
    buildings: array<i32, 4>,
}

var<private> F: array<Field, 6>;
var<private> field_barbarians: i32;

// A line of the defender's realm, one per host, and what lies on it.
struct Line {
    arpents: i32,
    peasants: i32,
    grain: i32,
    merchants: i32,
    treasury: i32,
    nobles: i32,
    buildings: array<i32, 4>,
    lot: u32,
}

// A host on its line.
struct Stand {
    attacker: u32,
    men: i32,
    rams: i32,
    rams_broken: i32,
    advance: i32,
    victory: bool,
    rallied: array<i32, 3>,
    killed: array<i32, 3>,
}

var<private> lines: array<Line, 6>;
var<private> stands: array<Stand, 6>;

fn affordable(k: u32) -> i32 {
    return max(min(min(K[k].treasury / GOLD_PER_MAN, K[k].stocks / GRAIN_PER_MAN), K[k].soldiers), 0);
}

fn building(k: u32, kind: u32) -> i32 {
    switch (kind) {
        case 0u: { return K[k].mills; }
        case 1u: { return K[k].foundries; }
        case 2u: { return K[k].marketplaces; }
        default: { return K[k].shipyards; }
    }
}

fn add_building(k: u32, kind: u32, n: i32) {
    switch (kind) {
        case 0u: { K[k].mills = max(K[k].mills + n, 0); }
        case 1u: { K[k].foundries = max(K[k].foundries + n, 0); }
        case 2u: { K[k].marketplaces = max(K[k].marketplaces + n, 0); }
        default: { K[k].shipyards = max(K[k].shipyards + n, 0); }
    }
}

fn set_building(k: u32, kind: u32, n: i32) {
    switch (kind) {
        case 0u: { K[k].mills = n; }
        case 1u: { K[k].foundries = n; }
        case 2u: { K[k].marketplaces = n; }
        default: { K[k].shipyards = n; }
    }
}

// The i-th of n equal shares of a total, the remainder on the first.
fn share_of(total: i32, n: i32, i: i32) -> i32 {
    var part = total / n;
    if (i < total % n) {
        part += 1;
    }
    return part;
}

// The defender's realm laid out in n lines, one lot drawn per line.
fn lay_out(d: u32, n: i32) {
    for (var i = 0; i < n; i++) {
        lines[i].arpents = share_of(F[d].surface, n, i);
        lines[i].peasants = share_of(max(F[d].peasants, 0), n, i);
        lines[i].grain = share_of(F[d].stocks, n, i);
        lines[i].merchants = share_of(F[d].merchants, n, i);
        lines[i].treasury = share_of(F[d].treasury, n, i);
        lines[i].nobles = share_of(F[d].nobles, n, i);
        lines[i].lot = next_word();
    }
    for (var kind = 0u; kind < 4u; kind++) {
        let count = F[d].buildings[kind];
        for (var i = 0; i < n; i++) {
            lines[i].buildings[kind] = share_of(count, n, i);
        }
    }
}

fn any_standing(n: u32) -> bool {
    for (var i = 0u; i < n; i++) {
        if (stands[i].men > 0) {
            return true;
        }
    }
    return false;
}

fn garrison_strength(efficiency: i32, walls: i32) -> i32 {
    return efficiency * 3 * (TENTHS + clamp(walls, 0, TENTHS)) / 20;
}

// How many of c buildings spread along the line stand within x arpents.
fn crossed(i: u32, c: i32, x: i32) -> i32 {
    if (c <= 0 || x < 1) {
        return 0;
    }
    let len = lines[i].arpents;
    let n = 2 * c * (x + 1) - len;
    let d = 2 * len;
    if (n <= 0) {
        return 0;
    }
    return min((n + d - 1) / d, c);
}

fn burns(i: u32, kind: u32, j: i32) -> bool {
    return mix(lines[i].lot ^ (kind << 28u) ^ u32(j)) % 3u != 0u;
}

fn even(i: u32, total: i32, x: i32) -> i32 {
    return mul_div(total, x, max(lines[i].arpents, 1));
}

fn deep(i: u32, total: i32, x: i32) -> i32 {
    return even(i, even(i, total, x), x);
}

fn people(i: u32, advance: i32) -> vec3<i32> {
    let x = clamp(advance, 0, lines[i].arpents);
    return vec3<i32>(
        even(i, lines[i].peasants, x),
        even(i, lines[i].merchants, x),
        deep(i, lines[i].nobles, x),
    );
}

// The barbarians met by `men` (war.rs): the land they yield, whether all
// of it, the men left — after `field_barbarians` is read for the raid.
fn raid(e: u32) {
    let a = u32(E[e].attacker);
    let men = E[e].men;
    let strength = F[a].efficiency;
    let land = field_barbarians;
    let inner1 = random(0, (men + 1) * 3);
    let part1 = random(1, max(inner1, 1) + 1);
    let inner2 = random(1, men + 2);
    let part2 = random(1, max(inner2, 1) + 1);
    var defenders = part1 + part2;
    var attackers = men;
    var conquered = 0;
    let unit = men / 15 + 1;
    var won = false;
    var all = false;
    var taken = 0;
    loop {
        if (random(1, strength) < random(1, 90)) {
            attackers -= unit;
        } else {
            let gained = random(1, unit * 26) - random(1, unit + 5);
            conquered = max(0, conquered + gained);
            defenders -= unit;
        }
        all = conquered >= land;
        taken = min(conquered, land);
        if (defenders <= 0 || all) {
            won = true;
            attackers = max(0, attackers);
            break;
        }
        if (attackers <= 0) {
            attackers = 0;
            // The spoils of a defeat: a share of the land taken, drawn
            // but never brought home (war.rs `defeat_spoils`).
            if (taken >= 2) {
                random(1, 4);
            }
            break;
        }
    }
    var advance = 0;
    if (won) {
        advance = taken;
    }
    let arpents = clamp(advance, 0, land);
    field_barbarians -= arpents;
    let home = min(arpents, barbarians);
    barbarians -= home;
    if (K[a].dead == 0) {
        K[a].surface += home;
        K[a].soldiers += attackers;
    }
}

// A front on realm `d` (front.rs `fight`, then `apply_front`): the siege
// of the garrison, the rams at the walls, the march over the lines and
// the duels with those met on the way; then the losses and the spoils.
fn fight(d: u32, n: u32) {
    lay_out(d, i32(n));
    let strength_d = F[d].efficiency;
    var garrison = max(F[d].soldiers, 0);
    let garrison_start = garrison;
    let walls_start = clamp(F[d].forts, 0, TENTHS);
    var walls = walls_start;
    var exchange = 0;
    var units: array<i32, 6>;
    var strength: array<i32, 6>;
    for (var i = 0u; i < n; i++) {
        units[i] = stands[i].men / 15 + 1;
        strength[i] = F[stands[i].attacker].efficiency;
        stands[i].victory = garrison == 0;
    }
    var standing = any_standing(n);
    while (garrison > 0 && standing) {
        for (var i = 0u; i < n; i++) {
            if (garrison == 0 || stands[i].men == 0) {
                continue;
            }
            if (random(1, strength[i]) > random(1, garrison_strength(strength_d, walls))) {
                garrison = max(0, garrison - units[i]);
                if (garrison == 0) {
                    for (var j = 0u; j < n; j++) {
                        stands[j].victory = stands[j].victory || stands[j].men > 0;
                    }
                }
            } else {
                stands[i].men = max(0, stands[i].men - units[i]);
            }
        }
        exchange += 1;
        if (garrison > 0 && exchange % RAM_PACE == 0) {
            for (var i = 0u; i < n; i++) {
                if (stands[i].rams <= 0) {
                    continue;
                }
                let escort = stands[i].men / stands[i].rams;
                let missing = max(RAM_ESCORT - escort, 0);
                var broken = 0;
                for (var r = 0; r < stands[i].rams; r++) {
                    walls = max(0, walls - 1);
                    if (random(1, RAM_ESCORT) <= missing) {
                        broken += 1;
                    }
                }
                stands[i].rams -= broken;
                stands[i].rams_broken += broken;
            }
        }
        standing = any_standing(n);
    }
    var marching = garrison == 0;
    while (marching) {
        marching = false;
        for (var i = 0u; i < n; i++) {
            let reach = min(lines[i].arpents, stands[i].men * ARPENTS_PER_MAN);
            if (stands[i].men <= 0 || stands[i].advance >= reach) {
                continue;
            }
            let tu = units[i];
            let gained = max(random(1, tu * 26) - random(1, tu + 5), 0);
            let start = stands[i].advance;
            let to = min(start + gained, reach);
            let met3 = people(i, to) - people(i, start);
            let total = met3.x + met3.y + met3.z;
            var reached = to;
            var k = 0;
            var broken = false;
            for (var who = 0u; who < 3u && !broken; who++) {
                var ardour = MILITIA_EFFICIENCY;
                if (who == 2u) {
                    ardour = strength_d;
                }
                for (var m = 0; m < max(met3[who], 0); m++) {
                    if (random(0, 3) == 0) {
                        stands[i].rallied[who] += 1;
                    } else if (random(1, strength[i]) > random(1, ardour)) {
                        stands[i].killed[who] += 1;
                    } else {
                        stands[i].men -= 1;
                        if (stands[i].men == 0) {
                            reached = start + mul_div(to - start, k, total);
                            broken = true;
                            break;
                        }
                    }
                    k += 1;
                }
            }
            stands[i].advance = reached;
        }
        for (var i = 0u; i < n; i++) {
            let reach = min(lines[i].arpents, stands[i].men * ARPENTS_PER_MAN);
            marching = marching || (stands[i].men > 0 && stands[i].advance < reach);
        }
    }
    var fallen = garrison == 0;
    for (var i = 0u; i < n; i++) {
        fallen = fallen && stands[i].advance >= lines[i].arpents;
    }
    var annexed_by = -1;
    if (fallen) {
        for (var i = 0u; i < n; i++) {
            if (!stands[i].victory) {
                continue;
            }
            if (annexed_by < 0
                || stands[i].advance > stands[annexed_by].advance
                || (stands[i].advance == stands[annexed_by].advance
                    && stands[i].men >= stands[annexed_by].men)) {
                annexed_by = i32(i);
            }
        }
    }
    // The spoils of every army, added up on the defender.
    var arpents = 0;
    var grain = 0;
    var treasury = 0;
    var rallied = vec3<i32>(0);
    var killed = vec3<i32>(0);
    var gone: array<i32, 4>;
    for (var i = 0u; i < n; i++) {
        let x = clamp(stands[i].advance, 0, lines[i].arpents);
        arpents += x;
        grain += even(i, lines[i].grain, x) / 3 * 2;
        treasury += deep(i, lines[i].treasury, x) / 3 * 2;
        for (var who = 0u; who < 3u; who++) {
            rallied[who] += stands[i].rallied[who];
            killed[who] += stands[i].killed[who];
        }
        for (var kind = 0u; kind < 4u; kind++) {
            gone[kind] += crossed(i, lines[i].buildings[kind], x);
        }
    }
    K[d].soldiers = max(K[d].soldiers - (garrison_start - garrison), 0);
    K[d].forts = max(K[d].forts - (walls_start - walls), 0);
    K[d].peasants = max(K[d].peasants - rallied.x - killed.x, 0);
    K[d].merchants = max(K[d].merchants - rallied.y - killed.y, 0);
    K[d].nobles = max(K[d].nobles - rallied.z - killed.z, 0);
    K[d].surface = max(K[d].surface - arpents, 0);
    K[d].stocks = max(K[d].stocks - grain, 0);
    K[d].treasury = max(K[d].treasury - treasury, 0);
    for (var kind = 0u; kind < 4u; kind++) {
        add_building(d, kind, -gone[kind]);
    }
    for (var i = 0u; i < n; i++) {
        let a = stands[i].attacker;
        let x = clamp(stands[i].advance, 0, lines[i].arpents);
        // What was heard of this army.
        H[a].marched_on[d] = 1;
        H[d].marched_by[a] = 1;
        if (stands[i].victory) {
            H[d].beaten_by[a] = 1;
        }
        H[d].lost_to[a] += x;
        if (K[a].dead != 0) {
            continue;
        }
        K[a].soldiers += stands[i].men;
        // The rams left come home with the men, none without them.
        if (stands[i].men > 0) {
            K[a].rams += stands[i].rams;
        }
        K[a].surface += x;
        K[a].peasants += stands[i].rallied[0];
        K[a].merchants += stands[i].rallied[1];
        K[a].nobles += stands[i].rallied[2];
        K[a].stocks += even(i, lines[i].grain, x) / 3;
        K[a].treasury += deep(i, lines[i].treasury, x) / 3;
        for (var kind = 0u; kind < 4u; kind++) {
            var taken = 0;
            for (var j = 0; j < crossed(i, lines[i].buildings[kind], x); j++) {
                if (!burns(i, kind, j)) {
                    taken += 1;
                }
            }
            add_building(a, kind, taken);
        }
    }
    if (annexed_by >= 0) {
        let winner = stands[annexed_by].attacker;
        let rest = K[d];
        var rest_buildings: array<i32, 4>;
        for (var kind = 0u; kind < 4u; kind++) {
            rest_buildings[kind] = building(d, kind);
        }
        K[d].surface = 0;
        K[d].peasants = 0;
        K[d].stocks = 0;
        K[d].merchants = 0;
        K[d].treasury = 0;
        K[d].soldiers = 0;
        K[d].nobles = 0;
        K[d].forts = 0;
        K[d].hospices = 0;
        K[d].rams = 0;
        K[d].scouts = 0;
        for (var kind = 0u; kind < 4u; kind++) {
            set_building(d, kind, 0);
        }
        K[d].dead = 1;
        if (K[winner].dead == 0) {
            K[winner].surface += rest.surface;
            K[winner].peasants += rest.peasants;
            K[winner].stocks += rest.stocks;
            K[winner].merchants += rest.merchants;
            K[winner].treasury += rest.treasury;
            K[winner].nobles += rest.nobles;
            for (var kind = 0u; kind < 4u; kind++) {
                add_building(winner, kind, rest_buildings[kind]);
            }
        }
    }
}

// The campaign of the year: the expeditions grouped by realm, the realms
// in a random order, each expedition cut to what its realm can pay and
// dropped when it makes no sense, the fronts fought in the order the
// targets first appear.
fn campaign(lane: u32) {
    for (var k = 0u; k < 6u; k++) {
        for (var o = 0u; o < 6u; o++) {
            H[k].marched_on[o] = 0;
            H[k].marched_by[o] = 0;
            H[k].beaten_by[o] = 0;
            H[k].lost_to[o] = 0;
        }
    }
    // The realms, in order of first appearance (the seats gave their
    // expeditions in turn, so each realm's are contiguous).
    var first: array<u32, 6>;
    var count: array<u32, 6>;
    var order: array<u32, 6>;
    var groups = 0u;
    for (var e = 0u; e < n_expeditions; e++) {
        if (e == 0u || E[e].attacker != E[e - 1u].attacker) {
            first[groups] = e;
            count[groups] = 0u;
            order[groups] = groups;
            groups += 1u;
        }
        count[groups - 1u] += 1u;
    }
    for (var i = i32(groups) - 1; i >= 1; i--) {
        let j = random(0, i + 1);
        let t = order[i];
        order[i] = order[j];
        order[j] = t;
    }
    var front_target: array<i32, 7>;
    var front_n: array<u32, 7>;
    var front_e: array<array<u32, 6>, 7>;
    var fronts = 0u;
    for (var g = 0u; g < groups; g++) {
        let o = order[g];
        for (var e = first[o]; e < first[o] + count[o]; e++) {
            let a = u32(E[e].attacker);
            let t = E[e].foe;
            var sensible = K[a].dead == 0;
            if (t >= 0) {
                sensible = sensible && year >= FIRST_WAR_YEAR && t != E[e].attacker
                    && K[t].dead == 0;
            } else {
                sensible = sensible && barbarians > 0;
            }
            E[e].men = min(E[e].men, affordable(a));
            if (t >= 0) {
                E[e].rams = clamp(E[e].rams, 0, K[a].rams);
            } else {
                E[e].rams = 0;
            }
            if (!sensible || E[e].men < 1) {
                continue;
            }
            K[a].soldiers -= E[e].men;
            K[a].rams -= E[e].rams;
            K[a].treasury -= E[e].men * GOLD_PER_MAN;
            K[a].stocks -= E[e].men * GRAIN_PER_MAN;
            var f = 0u;
            for (; f < fronts && front_target[f] != t; f++) {}
            if (f == fronts) {
                front_target[f] = t;
                front_n[f] = 0u;
                fronts += 1u;
            }
            front_e[f][front_n[f]] = e;
            front_n[f] += 1u;
        }
    }
    // Read through the subgroup so the copy is taken now: RADV (Mesa
    // 25.2) otherwise forwards the later reads of `F` to `K`, which the
    // fronts change as they go.
    for (var k = 0u; k < 6u; k++) {
        F[k] = Field(
            subgroupShuffle(K[k].surface, lane),
            subgroupShuffle(K[k].peasants, lane),
            subgroupShuffle(K[k].nobles, lane),
            subgroupShuffle(K[k].merchants, lane),
            subgroupShuffle(K[k].soldiers, lane),
            subgroupShuffle(K[k].efficiency, lane),
            subgroupShuffle(K[k].treasury, lane),
            subgroupShuffle(K[k].stocks, lane),
            subgroupShuffle(K[k].forts, lane),
            array<i32, 4>(
                subgroupShuffle(K[k].mills, lane),
                subgroupShuffle(K[k].foundries, lane),
                subgroupShuffle(K[k].marketplaces, lane),
                subgroupShuffle(K[k].shipyards, lane),
            ),
        );
    }
    field_barbarians = barbarians;
    for (var f = 0u; f < fronts; f++) {
        let t = front_target[f];
        if (t < 0) {
            for (var i = 0u; i < front_n[f]; i++) {
                raid(front_e[f][i]);
            }
            continue;
        }
        for (var i = 0u; i < front_n[f]; i++) {
            let e = front_e[f][i];
            stands[i] = Stand(u32(E[e].attacker), E[e].men, max(E[e].rams, 0), 0, 0, false,
                array<i32, 3>(0, 0, 0), array<i32, 3>(0, 0, 0));
        }
        fight(u32(t), front_n[f]);
    }
}
