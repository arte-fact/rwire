// A year of a table (arena.rs `watch`): every lane plays its own table,
// rules and networks. The rules stand where the CPU has them, roll for
// roll.

// The seats in the order the Intendances sit this year.
var<private> order: array<u32, 6>;

// The listing of last year joins the stall (kingdom.rs `open_market`).
fn open_market(k: u32) {
    if (K[k].listed > 0) {
        K[k].price = merged_price(K[k].to_sell, K[k].price, K[k].listed, K[k].listed_price);
        K[k].to_sell += K[k].listed;
        K[k].listed = 0;
        K[k].listed_price = 0;
    }
}

// The seed grain, the rats and the harvest (harvests.rs).
fn harvests(k: u32) {
    var sown = cultivated(k);
    if (K[k].stocks * 3 < sown) {
        sown = K[k].stocks * 3;
    }
    if (K[k].peasants * 5 < sown) {
        sown = K[k].peasants * 5;
    }
    K[k].stocks -= sown / 3;
    let rate = random(1, 30);
    K[k].stocks = max(0, K[k].stocks - K[k].stocks * rate / 100);
    K[k].to_sell = max(0, K[k].to_sell - K[k].to_sell * rate / 100);
    K[k].rats = rate;
    K[k].harvest = cultivated(k) * K[k].weather * 72 / 100 + random(1, 500) - K[k].foundries * 500;
    K[k].stocks += K[k].harvest;
}

fn open_year() {
    for (var k = 0u; k < 6u; k++) {
        K[k].weather = random(1, 7);
    }
    for (var k = 0u; k < 6u; k++) {
        open_market(k);
    }
    for (var k = 0u; k < 6u; k++) {
        if (K[k].dead == 0) {
            harvests(k);
        }
    }
    for (var i = 0u; i < 6u; i++) {
        order[i] = i;
    }
    for (var i = 5; i >= 1; i--) {
        let j = random(0, i + 1);
        let t = order[i];
        order[i] = order[j];
        order[j] = t;
    }
    n_expeditions = 0u;
}

// The plague (events.rs): one year in fifty, some of everyone, the
// hospices sparing their share, the last noble always spared.
fn plague(k: u32) -> bool {
    if (random(0, 100) >= 2) {
        return false;
    }
    let h = K[k].hospices;
    let peasants = spared(random(0, K[k].peasants / 2), h);
    let merchants = spared(random(0, K[k].merchants / 3), h);
    let soldiers = spared(random(0, K[k].soldiers / 3), h);
    let nobles = max(min(spared(random(0, K[k].nobles / 3), h), K[k].nobles - 1), 0);
    K[k].peasants -= peasants;
    K[k].merchants -= merchants;
    K[k].soldiers -= soldiers;
    K[k].nobles -= nobles;
    return true;
}

// How far a realm stands from the crown (arena.rs `progress`): the mean
// of its criteria, the land ratio counted once the barbarians are in
// play.
fn progress(k: u32) -> f32 {
    var sum = 0.0;
    var count = 0.0;
    for (var w = 0u; w < 9u; w++) {
        if (w == 1u && table_stage < STAGE_EMPEROR) {
            continue;
        }
        sum += min(f32(have(k, w)) / f32(need(EMPEROR, w)), 1.0);
        count += 1.0;
    }
    return sum / count;
}

fn close_year() {
    for (var i = 0u; i < 6u; i++) {
        if (O[i].crowned >= 0) {
            continue;
        }
        if (K[i].dead != 0) {
            if (O[i].fell < 0) {
                O[i].fell = year;
            }
            continue;
        }
        var starvation = 0;
        if (M[i].demo.has != 0) {
            starvation = M[i].demo.starvation;
        }
        M[i].plague = i32(plague(i));
        // The starving mother strikes the ruler (events.rs).
        let death = random(0, starvation) > random(0, 110);
        if (death) {
            barbarians += K[i].surface;
            K[i].surface = 0;
            K[i].dead = 1;
        }
        O[i].years = year;
        if (M[i].demo.has != 0) {
            O[i].born += M[i].demo.births;
            O[i].settled += M[i].demo.immigrants;
            O[i].nobles_come += M[i].demo.nobles_immigrants;
            O[i].starved += M[i].demo.malnutrition + M[i].demo.starvation;
            O[i].nobles_gone += M[i].demo.nobles_departed;
        }
        let p = progress(i);
        O[i].progress += (p - O[i].progress) / f32(year);
        O[i].walls += (f32(K[i].forts) - O[i].walls) / f32(year);
        let t = title(i);
        if (death) {
            O[i].fell = year;
            O[i].starved_out = 1;
        } else if (t == EMPEROR) {
            O[i].crowned = year;
        }
        if (t >= PRINCE && O[i].prince < 0) {
            O[i].prince = year;
        }
        if (t >= KING && O[i].king < 0) {
            O[i].king = year;
        }
    }
    year += 1;
    // The table ends at the first Emperor, or when every realm has fallen.
    var crowned = false;
    var fallen = true;
    for (var i = 0u; i < 6u; i++) {
        crowned = crowned || O[i].crowned >= 0;
        fallen = fallen && O[i].fell >= 0;
    }
    if (year > longest || crowned || fallen) {
        done = 1;
    }
}

// One year of the lane's table.
fn play_year(lane: u32) {
    open_year();
    for (var i = 0u; i < 6u; i++) {
        let s = order[i];
        if (K[s].dead != 0) {
            continue;
        }
        sight(s);
        forward_a(seat_genome[s]);
        intendance(s);
    }
    for (var s = 0u; s < 6u; s++) {
        if (K[s].dead != 0 || O[s].crowned >= 0) {
            continue;
        }
        sight_with_answer(s);
        forward_b(seat_genome[s]);
        missions(s);
        sight_with_answer(s);
        forward_b(seat_genome[s]);
        for (var j = 0u; j < B_OUT; j++) {
            M[s].orders[j] = y[j];
        }
        expeditions(s);
    }
    campaign(lane);
    close_year();
}

@compute @workgroup_size(32)
fn main(@builtin(workgroup_id) wg: vec3<u32>, @builtin(local_invocation_id) local: vec3<u32>) {
    let lane = local.x;
    let t = wg.x * 32u + lane;
    if (t >= arrayLength(&tables)) {
        return;
    }
    load(t);
    for (var i = 0u; i < params.years && done == 0; i++) {
        play_year(lane);
    }
    store(t);
}
