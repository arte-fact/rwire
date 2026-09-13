// The Intendance of one seat (brain.rs `decode_intendance`, arena.rs
// `intendance`): the answer in `y[0..A_OUT]` read into a market roll,
// purchases and a council, then applied in the seigneur's order — the
// listing, the purchase, the land, the buildings, the council promulgated,
// the census read, the economy. Each lane on its table.

const GRAIN_LOT: i32 = 100;
const MIN_GRAIN_PRICE: i32 = 10;
const MAX_GRAIN_PRICE: i32 = 100;
const LAND_SELL_PRICE: i32 = 4;
const TENTHS: i32 = 10;
const RATION_SCALE: i32 = 10;
const PEASANTS_FULL: i32 = 50;
const SOLDIERS_FULL: i32 = 80;
const MAX_CUSTOMS: i32 = 50;
const MAX_SALES: i32 = 20;
const MAX_INCOME: i32 = 35;

// The purchases in the order of the answers 9..18, their costs, and
// which are bought by tenths.
const P_MARKETPLACES: u32 = 0u;
const P_MILLS: u32 = 1u;
const P_FOUNDRIES: u32 = 2u;
const P_SHIPYARDS: u32 = 3u;
const P_PALACES: u32 = 4u;
const P_SOLDIERS: u32 = 5u;
const P_FORTS: u32 = 6u;
const P_HOSPICES: u32 = 7u;
const P_RAMS: u32 = 8u;
var<private> COSTS: array<i32, 9> = array(1000, 2000, 7000, 8000, 5000, 8, 1000, 5000, 1500);

fn lots(bushels: i32) -> i32 {
    return bushels / GRAIN_LOT * GRAIN_LOT;
}

// The buyer's cost with the broker's tenth, rounded up.
fn buy_cost(amount: i32, price: i32) -> i32 {
    return (amount * price * 10 + 9 * GRAIN_LOT - 1) / (9 * GRAIN_LOT);
}

fn tenths_built(k: u32, kind: u32) -> i32 {
    switch (kind) {
        case P_PALACES: { return K[k].palaces; }
        case P_FORTS: { return K[k].forts; }
        case P_HOSPICES: { return K[k].hospices; }
        default: { return -1; }
    }
}

// --- the market (trade.rs, kingdom.rs) ---

// `amount` bushels at `price` joined to a `(have, have_price)` lot at the
// volume-weighted price.
fn merged_price(have: i32, have_price: i32, amount: i32, price: i32) -> i32 {
    let total = have + amount;
    if (total > 0) {
        return (have_price * have + price * amount) / total;
    }
    return price;
}

fn list_grain(k: u32, amount0: i32, price: i32) {
    let amount = max(min(amount0, K[k].stocks), 0);
    if (amount == 0) {
        return;
    }
    K[k].stocks -= amount;
    var have = 0;
    var have_price = price;
    if (K[k].listed > 0) {
        have = K[k].listed;
        have_price = K[k].listed_price;
    }
    K[k].listed_price = merged_price(have, have_price, amount, price);
    K[k].listed = have + amount;
}

fn buy_grain(buyer: u32, seller: u32, amount0: i32) {
    let price = clamp(K[seller].price, MIN_GRAIN_PRICE, MAX_GRAIN_PRICE);
    let amount = clamp(amount0, 0, for_sale(seller));
    K[seller].to_sell -= amount;
    K[seller].treasury += amount * price / GRAIN_LOT;
    K[buyer].stocks += amount;
    K[buyer].treasury -= buy_cost(amount, price);
}

fn sell_land(k: u32, arpents0: i32) {
    let arpents = clamp(arpents0, 0, K[k].surface / 10);
    K[k].surface -= arpents;
    K[k].treasury += arpents * LAND_SELL_PRICE;
    barbarians += arpents;
}

// --- the purchases (investments.rs) ---

fn invest(k: u32, kind: u32, n: i32) {
    let total = n * COSTS[kind];
    if (total > K[k].treasury) {
        return;
    }
    if (kind == P_SOLDIERS && K[k].soldiers + n > K[k].nobles * 20) {
        return;
    }
    let built = tenths_built(k, kind);
    if (built >= 0 && built + n > TENTHS) {
        return;
    }
    switch (kind) {
        case P_MARKETPLACES: {
            K[k].marketplaces += n;
            var gained = 0;
            for (var i = 0; i < n; i++) {
                gained += random(1, 7);
            }
            K[k].merchants += gained;
            K[k].peasants -= gained;
        }
        case P_MILLS: { K[k].mills += n; }
        case P_FOUNDRIES: { K[k].foundries += n; }
        case P_SHIPYARDS: { K[k].shipyards += n; }
        case P_SOLDIERS: { K[k].soldiers += n; }
        case P_PALACES: {
            K[k].palaces += n;
            var gained = 0;
            for (var i = 0; i < n; i++) {
                gained += random(1, 4);
            }
            K[k].nobles += gained;
        }
        case P_FORTS: { K[k].forts += n; }
        case P_HOSPICES: { K[k].hospices += n; }
        default: { K[k].rams += n; }
    }
    K[k].treasury -= total;
}

// --- the council (demography.rs) ---

// `cap`, or the rate `stocks` can pay `heads` at when that is less.
fn affordable_ration(stocks: i32, cap: i32, heads: i32) -> i32 {
    if (heads == 0) {
        return cap;
    }
    return min(cap, mul_div(max(stocks, 0), RATION_SCALE, heads));
}

fn ration(tenths: i32, full: i32) -> f32 {
    return f32(max(tenths, 0)) / f32(full);
}

// `rate × (1 − r)^exp` of `count` below a full ration, nothing above.
fn shortfall(count: i32, r: f32, rate: f32, exp: f32) -> f32 {
    if (r < 1.0) {
        return f32(count) * rate * pow(1.0 - r, exp);
    }
    return 0.0;
}

// A draw uniform within `±spread` of `expected`, the bounds rounded.
fn draw_around(expected: f32, spread: f32) -> i32 {
    let low = to_i32(round_away(expected * (1.0 - spread)));
    let high = to_i32(round_away(expected * (1.0 + spread)));
    return random(low, high + 1);
}

fn logistic(v: f32, mid: f32, width: f32) -> f32 {
    return 1.0 / (1.0 + exp(-(v - mid) / width));
}

fn immigrants_expected(pop: i32, r: f32, x: f32) -> f32 {
    let floor = logistic(1.0, 1.5, 0.12);
    let s = max((logistic(r, 1.5, 0.12) - floor) / (1.0 - floor), 0.0);
    return f32(pop) * 0.06 * s * max(1.0 - pow(x, 1.6), 0.0);
}

fn nobles_among(immigrants: i32, x: f32) -> i32 {
    return to_i32(f32(immigrants) * 0.02 * 2.0 * (1.0 - x * x));
}

fn merchants_among(immigrants: i32, y: f32) -> i32 {
    return to_i32(f32(immigrants) * 0.004 * 2.0 * (1.0 - y * y));
}

fn army_efficiency(soldiers_ration: i32) -> i32 {
    let r = ration(soldiers_ration, SOLDIERS_FULL);
    if (r <= 0.0) {
        return 50;
    }
    return clamp(to_i32(round_away(100.0 + 50.0 * log(r) / LN_1_5)), 50, 150);
}

// `victims` less the share the hospice spares.
fn spared(victims: i32, hospices: i32) -> i32 {
    return victims * (20 - clamp(hospices, 0, 10)) / 20;
}

// The council promulgated: the rations eaten, the census drawn and read
// into `M[k].demo`, the people counted again.
fn apply_feed(k: u32, peasants_ration: i32, soldiers_ration: i32) {
    let pop = population(k);
    let x = f32(K[k].customs) / f32(MAX_CUSTOMS);
    let y = f32(K[k].sales) / f32(MAX_SALES);
    let z = f32(K[k].income) / f32(MAX_INCOME);
    let r = ration(peasants_ration, PEASANTS_FULL);
    let hunger_all = shortfall(pop, r, 0.6, 2.2);
    let hunger_maln = shortfall(pop, max(r, 0.5), 0.6, 2.2);
    let ra = ration(soldiers_ration, SOLDIERS_FULL);
    let army_all = shortfall(K[k].soldiers, ra, 0.7, 2.0);
    let army_desertion = shortfall(K[k].soldiers, max(ra, 0.5), 0.7, 2.0);
    let disease_cap = spared(pop / 22, K[k].hospices);
    let trickle = 1.0 - y * y;
    let taxed_away = to_i32(round_away(f32(K[k].nobles) * 0.25 * logistic(z, 0.75, 0.05)));
    let flight = shortfall(K[k].nobles, r, 0.7, 1.3);
    let flight_low = to_i32(round_away(flight * (1.0 - 0.4)));
    let flight_high = to_i32(round_away(flight * (1.0 + 0.4)));
    let merchants_departed = to_i32(round_away(f32(K[k].merchants) * 0.08 * y * y * y));

    // The draws, in the report's order.
    let immigrants = draw_around(immigrants_expected(pop, r, x), 0.6);
    let nobles_departed = random(
        min(flight_low + taxed_away, K[k].nobles),
        min(flight_high + taxed_away, K[k].nobles) + 1,
    );
    let disease = random(min(disease_cap, 1), disease_cap + 1);
    let malnutrition = draw_around(hunger_maln, 0.2);
    let left = pop - nobles_departed - merchants_departed;
    let starvation = max(
        min(draw_around(hunger_all - hunger_maln, 0.2), left - disease - malnutrition),
        0,
    );
    let desertion = draw_around(army_desertion, 0.3);
    let army_starvation = max(
        min(draw_around(army_all - army_desertion, 0.3), K[k].soldiers - desertion),
        0,
    );
    let births = draw_around(
        f32(pop) * 0.053 * pow(min(r, 1.0), 0.7) * (1.0 - 0.35 * z * z),
        0.8,
    );
    let nobles_immigrants = random(0, nobles_among(immigrants, x) + 1);
    let merchants_immigrants = random(0, merchants_among(immigrants, y) + 1);
    let merchants_settled = to_i32(round_away(f32(random(1, 7)) * trickle));
    let efficiency = army_efficiency(soldiers_ration);

    M[k].demo.has = 1;
    M[k].demo.births = births;
    M[k].demo.immigrants = immigrants;
    M[k].demo.nobles_immigrants = nobles_immigrants;
    M[k].demo.merchants_immigrants = merchants_immigrants;
    M[k].demo.merchants_settled = merchants_settled;
    M[k].demo.nobles_departed = nobles_departed;
    M[k].demo.merchants_departed = merchants_departed;
    M[k].demo.disease = disease;
    M[k].demo.malnutrition = malnutrition;
    M[k].demo.starvation = starvation;
    M[k].demo.efficiency = efficiency;
    M[k].demo.soldiers_starvation = army_starvation;
    M[k].demo.soldiers_desertion = desertion;

    K[k].stocks -= peasants_ration * mouths(k) / RATION_SCALE
        + soldiers_ration * K[k].soldiers / RATION_SCALE;
    K[k].ration = max(soldiers_ration, 0);
    let peasant_change = births + immigrants - nobles_immigrants - merchants_immigrants
        - starvation - malnutrition - disease;
    K[k].peasants = max(0, K[k].peasants + peasant_change);
    K[k].soldiers = max(0, K[k].soldiers - (army_starvation + desertion));
    K[k].efficiency = efficiency;
    K[k].nobles = max(0, K[k].nobles + nobles_immigrants - nobles_departed);
    K[k].merchants = max(
        0,
        K[k].merchants + merchants_immigrants + merchants_settled - merchants_departed,
    );
}

// --- the economy (economy.rs) ---

fn apply_economy(k: u32, immigrants: i32) {
    let fair_dice = random(1, 35) + random(1, 35);
    let mill_die = random(1, 250);
    let foundry_die = random(1, 150);
    let sales = f32(K[k].sales);
    let income = f32(K[k].income);
    let trade = f32(K[k].merchants + fair_dice) / (sales + 2.0);
    let fairs = pow(f32(K[k].marketplaces) * (trade * 12.0 + 5.0), 0.9);
    let divisor = income * 20.0 + sales * 40.0 + 200.0;
    let flour = 5.8 * f32(K[k].harvest + mill_die) / divisor;
    let mills = pow(f32(K[k].mills) * (flour + 150.0), 0.9);
    let foundries = pow(f32(K[k].foundries + K[k].soldiers + foundry_die + 400), 0.9);
    let cargo = f32(K[k].merchants) * 4.0 + f32(K[k].marketplaces) * 9.0
        + f32(K[k].foundries) * 15.0;
    let ships = pow(f32(K[k].shipyards) * cargo * f32(K[k].weather), 0.9);
    let maintenance = K[k].soldiers * 8;
    let customs = to_i32(round_away(f32(immigrants) * 0.4 * f32(K[k].customs)));
    let goods = f32(K[k].merchants) * 1.8 + fairs * 33.0 + mills * 17.0 + foundries * 50.0
        + ships * 70.0;
    let base = pow(goods, 0.85) + f32(K[k].nobles) * 5.0 + f32(K[k].peasants);
    let commercial = to_i32(sales / 100.0 * base);
    let wealth = f32(K[k].peasants) * 1.3 + f32(K[k].nobles) * 145.0
        + f32(K[k].merchants) * 39.0 + f32(K[k].marketplaces) * 99.0
        + f32(K[k].mills) * 99.0 + f32(K[k].foundries) * 425.0
        + f32(K[k].shipyards) * 965.0;
    let income_taxes = to_i32(pow(income / 100.0 * wealth, 0.97));
    let net = to_i32(fairs) + to_i32(mills) + to_i32(ships) + to_i32(foundries) + income_taxes
        + commercial + customs - maintenance;
    M[k].eco = 1;
    M[k].net = net;
    M[k].maintenance = maintenance;
    K[k].treasury += net;
}

// --- the Intendance read and applied ---

fn on(i: u32) -> f32 {
    if (y[i] < DEADBAND) {
        return 0.0;
    }
    return y[i];
}

fn intendance(s: u32) {
    for (var i = 0u; i < A_OUT; i++) {
        M[s].answer[i] = y[i];
    }
    let market = seat_stage[s] >= STAGE_MARKET;
    var stocks = max(K[s].stocks, 0);
    var treasury = K[s].treasury;

    // The listing.
    var listed = 0;
    var listed_price = 0;
    if (market) {
        listed = lots(to_i32(on(5u) * f32(stocks)));
        if (listed > 0) {
            stocks -= listed;
            listed_price = MIN_GRAIN_PRICE
                + to_i32(y[6] * f32(MAX_GRAIN_PRICE - MIN_GRAIN_PRICE));
        }
    }

    // The cheapest stall with a lot on it, the first of a tie.
    var seller = 6u;
    var bought = 0;
    if (market && on(7u) > 0.0) {
        var best = 0;
        for (var o = 0u; o < 6u; o++) {
            if (o == s || K[o].dead != 0) {
                continue;
            }
            let bushels = for_sale(o);
            let price = K[o].price;
            if (bushels >= GRAIN_LOT && price > 0 && (seller == 6u || price < best)) {
                seller = o;
                best = price;
            }
        }
        if (seller < 6u) {
            let price = min(best, MAX_GRAIN_PRICE);
            let per_lot = buy_cost(GRAIN_LOT, price);
            let budget = to_i32(on(7u) * f32(max(treasury, 0)));
            let amount = min(lots(for_sale(seller)), budget / per_lot * GRAIN_LOT);
            if (amount > 0) {
                treasury -= buy_cost(amount, price);
                bought = amount;
            } else {
                seller = 6u;
            }
        }
    }

    let land_sold = to_i32(on(8u) * f32(K[s].surface / 10));
    treasury += land_sold * LAND_SELL_PRICE;

    // The strongest wants first, a stable order on ties.
    var want_kind: array<u32, 9>;
    var want_of: array<f32, 9>;
    var wants = 0u;
    for (var i = 0u; i < 9u; i++) {
        let w = on(9u + i);
        if (w > 0.0) {
            var j = wants;
            while (j > 0u && want_of[j - 1u] < w) {
                want_kind[j] = want_kind[j - 1u];
                want_of[j] = want_of[j - 1u];
                j -= 1u;
            }
            want_kind[j] = i;
            want_of[j] = w;
            wants += 1u;
        }
    }
    var buy_kind: array<u32, 9>;
    var buy_n: array<i32, 9>;
    var buys = 0u;
    for (var i = 0u; i < wants; i++) {
        let kind = want_kind[i];
        var cap = treasury / COSTS[kind];
        if (kind == P_SOLDIERS) {
            cap = min(cap, K[s].nobles * 20 - K[s].soldiers);
        } else {
            let built = tenths_built(s, kind);
            if (built >= 0) {
                cap = min(cap, TENTHS - built);
            }
        }
        let w2 = want_of[i] * want_of[i];
        let n = to_i32(round_away(w2 * w2 * f32(cap)));
        if (n > 0) {
            treasury -= n * COSTS[kind];
            buy_kind[buys] = kind;
            buy_n[buys] = n;
            buys += 1u;
        }
    }

    let peasants_ration = to_i32(round_away(y[0] * 2.0 * f32(PEASANTS_FULL)));
    let soldiers_ration = to_i32(round_away(y[1] * 1.5 * f32(SOLDIERS_FULL)));
    let customs = to_i32(round_away(y[2] * f32(MAX_CUSTOMS)));
    let sales = to_i32(round_away(y[3] * f32(MAX_SALES)));
    let income = to_i32(round_away(y[4] * f32(MAX_INCOME)));

    // Applied in the seigneur's order.
    if (listed > 0) {
        list_grain(s, listed, clamp(listed_price, MIN_GRAIN_PRICE, MAX_GRAIN_PRICE));
    }
    if (seller < 6u) {
        buy_grain(s, seller, bought);
    }
    if (land_sold > 0) {
        sell_land(s, land_sold);
    }
    for (var i = 0u; i < buys; i++) {
        invest(s, buy_kind[i], buy_n[i]);
    }
    // The council as the stocks can pay it, the people served first.
    let heads = mouths(s);
    let peasants = clamp(
        peasants_ration,
        0,
        affordable_ration(K[s].stocks, PEASANTS_FULL * 2, heads),
    );
    let left = K[s].stocks - peasants * heads / RATION_SCALE;
    let soldiers = clamp(
        soldiers_ration,
        0,
        affordable_ration(left, SOLDIERS_FULL * 3 / 2, K[s].soldiers),
    );
    K[s].customs = clamp(customs, 0, MAX_CUSTOMS);
    K[s].sales = clamp(sales, 0, MAX_SALES);
    K[s].income = clamp(income, 0, MAX_INCOME);
    apply_feed(s, peasants, soldiers);
    apply_economy(s, M[s].demo.immigrants);
}
