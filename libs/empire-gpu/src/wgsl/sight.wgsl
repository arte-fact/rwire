// What a seat sees (brain.rs `sight`), written into the lane's `x` for
// the networks to read: the realm's own figures, the Chronique, the
// rivals, then last year's orders and the recall.

fn share(n: i32, whole: i32) -> f32 {
    if (whole <= 0) {
        return 0.0;
    }
    return f32(n) / f32(whole);
}

fn count(n: i32, scale: f32) -> f32 {
    return log(1.0 + f32(max(n, 0)) / scale);
}

fn signed_count(n: i32, scale: f32) -> f32 {
    if (n < 0) {
        return -count(-n, scale);
    }
    return count(n, scale);
}

fn title_level(t: i32) -> f32 {
    return f32(t) / 3.0;
}

fn criterion(k: u32, next: i32, what: u32) -> f32 {
    if (next > EMPEROR) {
        return 1.0;
    }
    let n = need(next, what);
    if (n <= 0) {
        return 1.0;
    }
    return min(share(have(k, what), n), 1.0);
}

fn flag(b: bool) -> f32 {
    if (b) {
        return 1.0;
    }
    return 0.0;
}

// How many realms other than `me` the flags name, halved.
fn others_on(flags: array<i32, 6>, me: u32) -> f32 {
    var n = 0;
    for (var j = 0u; j < 6u; j++) {
        if (flags[j] != 0 && j != me) {
            n += 1;
        }
    }
    return f32(n) / 2.0;
}

fn sight(s: u32) {
    let told = seat_told[s] != 0u;
    let needs = mouths(s) * 5 + K[s].soldiers * 8;
    let next = title(s) + 1;
    var i = 0u;
    x[0] = f32(year) / 100.0;
    x[1] = f32(K[s].weather) / 6.0;
    x[2] = count(K[s].surface, 10000.0);
    x[3] = count(K[s].peasants, 2000.0);
    x[4] = count(K[s].nobles, 40.0);
    x[5] = count(K[s].merchants, 100.0);
    x[6] = count(K[s].soldiers, 400.0);
    x[7] = f32(K[s].efficiency) / 150.0;
    x[8] = f32(K[s].ration) / 80.0;
    x[9] = signed_count(K[s].treasury, 10000.0);
    x[10] = count(K[s].stocks, 16000.0);
    x[11] = signed_count(K[s].harvest, 10000.0);
    x[12] = f32(K[s].rats) / 30.0;
    x[13] = count(K[s].marketplaces, 14.0);
    x[14] = count(K[s].mills, 6.0);
    x[15] = count(K[s].foundries, 3.0);
    x[16] = count(K[s].shipyards, 3.0);
    x[17] = f32(K[s].palaces) / 10.0;
    x[18] = f32(K[s].customs) / 50.0;
    x[19] = f32(K[s].sales) / 20.0;
    x[20] = f32(K[s].income) / 35.0;
    x[21] = title_level(title(s));
    x[22] = count(land_ratio(s), 100.0);
    x[23] = min(share(K[s].stocks, needs) / 4.0, 1.0);
    x[24] = share(cultivated(s), K[s].surface);
    x[25] = share(K[s].soldiers, K[s].nobles * 20);
    x[26] = count(for_sale(s), 10000.0);
    x[27] = f32(min(K[s].price, 100)) / 100.0;
    if (K[s].listed > 0) {
        x[28] = count(K[s].listed, 10000.0);
        x[29] = f32(K[s].listed_price) / 100.0;
    } else {
        x[28] = 0.0;
        x[29] = 0.0;
    }
    for (var w = 0u; w < 7u; w++) {
        x[30u + w] = criterion(s, next, w);
    }
    x[37] = count(barbarians, 6000.0);
    x[38] = criterion(s, next, 7u);
    x[39] = criterion(s, next, 8u);
    x[40] = f32(K[s].forts) / 10.0;
    x[41] = f32(K[s].hospices) / 10.0;
    x[42] = count(K[s].rams, 4.0);
    // The Chronique.
    let pop = max(population(s), 1);
    i = 43u;
    if (told && M[s].demo.has != 0) {
        let d = M[s].demo;
        let delta = d.births + d.immigrants + d.merchants_settled - d.disease - d.nobles_departed
            - d.merchants_departed - d.malnutrition - d.starvation;
        x[i] = share(d.births, pop);
        x[i + 1u] = share(d.immigrants, pop);
        x[i + 2u] = share(d.nobles_departed, K[s].nobles + d.nobles_departed);
        x[i + 3u] = share(d.merchants_departed, K[s].merchants + d.merchants_departed);
        x[i + 4u] = share(d.disease, pop);
        x[i + 5u] = share(d.malnutrition, pop);
        x[i + 6u] = share(d.starvation, pop);
        x[i + 7u] = share(
            d.soldiers_starvation + d.soldiers_desertion,
            K[s].soldiers + d.soldiers_starvation + d.soldiers_desertion,
        );
        x[i + 8u] = share(delta, pop);
    } else {
        for (var j = 0u; j < 9u; j++) {
            x[i + j] = 0.0;
        }
    }
    i += 9u;
    if (told && M[s].eco != 0) {
        x[i] = signed_count(M[s].net, 10000.0);
        x[i + 1u] = count(M[s].maintenance, 10000.0);
    } else {
        x[i] = 0.0;
        x[i + 1u] = 0.0;
    }
    x[i + 2u] = flag(told && M[s].plague != 0);
    i += 3u;
    // The rivals.
    for (var j = 0u; j < 5u; j++) {
        let o = (s + 1u + j) % 6u;
        let alive = K[o].dead == 0;
        let d = M[s].dossiers[o];
        let report = d.report != 0 && alive && (told || d.report_year == year);
        let ledger = d.ledger != 0 && alive && (told || d.ledger_year == year);
        x[i] = flag(alive);
        x[i + 1u] = title_level(title(o));
        x[i + 2u] = select(0.0, count(d.surface, 10000.0), report);
        x[i + 3u] = count(for_sale(o), 10000.0);
        x[i + 4u] = f32(min(K[o].price, 100)) / 100.0;
        x[i + 5u] = flag(H[s].marched_by[o] != 0);
        x[i + 6u] = flag(H[o].marched_by[s] != 0);
        x[i + 7u] = flag(H[o].beaten_by[s] != 0);
        x[i + 8u] = others_on(H[o].marched_on, s);
        x[i + 9u] = others_on(H[o].marched_by, s);
        x[i + 10u] = count(H[o].lost_to[s], 1000.0);
        x[i + 11u] = select(0.0, 1.0 / (1.0 + f32(year - d.report_year)), report);
        x[i + 12u] = select(0.0, count(d.garrison, 400.0), report);
        x[i + 13u] = select(0.0, f32(d.efficiency) / 150.0, report);
        x[i + 14u] = select(0.0, f32(d.forts) / 10.0, report);
        x[i + 15u] = select(0.0, 1.0 / (1.0 + f32(year - d.ledger_year)), ledger);
        x[i + 16u] = select(0.0, signed_count(d.treasury, 10000.0), ledger);
        x[i + 17u] = select(0.0, count(d.stocks, 10000.0), ledger);
        var aim = 0.0;
        if (ledger) {
            aim = 1.0;
            if (d.aim >= 0) {
                aim = f32(d.met) / f32(d.all);
            }
        }
        x[i + 18u] = aim;
        i += 19u;
    }
    // Last year's orders, then the recall.
    for (var j = 0u; j < 19u; j++) {
        x[i + j] = select(0.0, M[s].orders[j], told);
    }
    i += 19u;
    for (var j = 19u; j < B_OUT; j++) {
        x[i] = M[s].orders[j];
        i += 1u;
    }
}

// The Extérieur reads the sight and the Intendance's answer.
fn sight_with_answer(s: u32) {
    sight(s);
    for (var j = 0u; j < A_OUT; j++) {
        x[A_IN + j] = M[s].answer[j];
    }
}
