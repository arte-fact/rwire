use crate::game::EmpireGame;
use crate::intel::{SCOUT_CAUGHT, SCOUT_PRICE};
use crate::kingdom::{Kingdom, Kingdoms};
use crate::mind::{Mind, Seen};
use crate::random::random;
use crate::trade::{apply_trade, calculate_buy_cost, Trade, MAX_GRAIN_PRICE};
use crate::war::{
    apply_barbarian_battle_result, apply_kingdom_battle_result, simulate_barbarian_battle,
    simulate_kingdom_battle,
};

/// The computer trades by the hundred: no one-bushel stalls or purchases.
const MIN_LOT: i32 = 100;

/// What the AI decided to do over its year: the intendance, then the war.
#[derive(Debug, Clone, Default)]
pub struct AiTurnDecision {
    /// Barbarian attacks: soldiers to send per attack.
    pub barbarian_attacks: Vec<i32>,
    /// Kingdom attacks: (target, soldiers).
    pub kingdom_attacks: Vec<(Kingdoms, i32)>,
    /// Resource/infrastructure growth applied.
    pub growth: AiGrowth,
    /// Grain put on the market this turn: `(amount, price)`.
    pub grain_listed: Option<(i32, i32)>,
    /// Grain bought from another kingdom's market: `(seller, amount)`.
    pub grain_bought: Option<(Kingdoms, i32)>,
    /// The realm an éclaireur rides to this year (paid for).
    pub scout: Option<Kingdoms>,
}

/// The war orders of one year, given once the éclaireur is back.
#[derive(Debug, Clone, Default)]
pub struct AiWar {
    /// Barbarian attacks: soldiers to send per attack.
    pub barbarian_attacks: Vec<i32>,
    /// Kingdom attacks: (target, soldiers).
    pub kingdom_attacks: Vec<(Kingdoms, i32)>,
    /// The realm an éclaireur rides to this year (paid for).
    pub scout: Option<Kingdoms>,
}

#[derive(Debug, Clone, Default)]
pub struct AiGrowth {
    pub peasants: i32,
    pub merchants: i32,
    pub marketplaces: i32,
    pub grain_mills: i32,
    pub foundries: i32,
    pub shipyards: i32,
    pub palaces: i32,
    pub nobles: i32,
    pub treasury: i32,
    pub soldiers: i32,
    pub soldiers_efficiency: i32,
}

/// The computer's intendance as the year opens: growth applied at once, the
/// market mirrored, grain bought. The war comes later, at the Extérieur —
/// see [`plan_ai_war`]; [`plan_ai_turn`] does both in one go.
pub fn plan_ai_intendance(game: &mut EmpireGame, id: Kingdoms) -> AiTurnDecision {
    if game.kingdom(id).is_player || game.kingdom(id).is_dead {
        return AiTurnDecision::default();
    }

    let weather_ratio = game.kingdom(id).weather.value() as f32 / 6.0;
    let kingdom = game.kingdom_mut(id);

    // Phase 1: resource growth. Weather scales each roll; rounded, not
    // truncated — truncation silently zeroed every small roll (a 0-or-1 noble
    // roll times a ratio < 1.0 always truncated to 0, freezing AI nobles at 1).
    let scaled = |n: i32| (n as f32 * weather_ratio).round() as i32;
    let peasant_growth = random(0, 200) + scaled(random(0, 200));
    let merchant_growth = scaled(random(0, 25));
    let marketplace_growth = scaled(random(0, 4));
    let mill_growth = scaled(random(0, 2));
    let treasury_growth = scaled(random(0, 1500));

    let (foundry_growth, shipyard_growth, palace_growth, noble_growth) = if weather_ratio >= 0.5 {
        let foundry = scaled(random(0, 2));
        let shipyard = scaled(random(0, 2));
        let (palace, noble) = if random(1, 3) == 1 {
            (scaled(random(0, 2)), scaled(random(0, 2)))
        } else {
            (0, 0)
        };
        (foundry, shipyard, palace, noble)
    } else {
        (0, 0, 0, 0)
    };

    let efficiency_growth = random(5, 10) + (5.0 * weather_ratio) as i32;
    let max_soldiers = kingdom.nobles * 20;
    let soldier_growth = if max_soldiers > kingdom.soldiers {
        random(0, max_soldiers - kingdom.soldiers)
    } else {
        0
    };

    kingdom.peasants += peasant_growth;
    kingdom.merchants += merchant_growth;
    kingdom.marketplaces += marketplace_growth;
    kingdom.grain_mills += mill_growth;
    kingdom.treasury += treasury_growth;
    kingdom.foundries += foundry_growth;
    kingdom.shipyards += shipyard_growth;
    kingdom.palaces = (kingdom.palaces + palace_growth).min(10);
    kingdom.nobles += noble_growth;
    kingdom.soldiers_efficiency = (kingdom.soldiers_efficiency + efficiency_growth * 10).min(150);
    kingdom.soldiers += soldier_growth;

    // Original army size limit:
    // if soldiers / serfs > foundries * 0.01 + 0.05 → soldiers = soldiers / 2
    let serfs = kingdom.peasants.max(1);
    let army_limit = kingdom.foundries as f32 * 0.01 + 0.05;
    if kingdom.soldiers as f32 / serfs as f32 > army_limit {
        kingdom.soldiers /= 2;
    }

    let mut decision = AiTurnDecision {
        growth: AiGrowth {
            peasants: peasant_growth,
            merchants: merchant_growth,
            marketplaces: marketplace_growth,
            grain_mills: mill_growth,
            foundries: foundry_growth,
            shipyards: shipyard_growth,
            palaces: palace_growth,
            nobles: noble_growth,
            treasury: treasury_growth,
            soldiers: soldier_growth,
            soldiers_efficiency: efficiency_growth,
        },
        ..Default::default()
    };

    // Market, per original Empire.bas lines 209–232: the computer mirrors the
    // human players' market rather than running its own economy.
    let humans: Vec<(i32, i32)> = game
        .kingdoms
        .iter()
        .filter(|k| k.is_player && !k.is_dead)
        .map(|k| (k.for_sale(), k.grain_price))
        .collect();
    if !humans.is_empty() {
        let n = humans.len() as i32;
        // Q3/Q4: human averages with noise (±1000 bushels, ±5 on the price
        // of the hundred — the original's ±1 on the bushel).
        let q3 = (humans.iter().map(|h| h.0).sum::<i32>() / n + random(1, 1001) - random(1, 1001))
            .max(0);
        let mut q4 = (humans.iter().map(|h| h.1).sum::<i32>() / n + random(0, 6) - random(0, 6))
            .clamp(0, MAX_GRAIN_PRICE);
        // Bad years push the asking price up (original: +RND/1.5 when NW<3).
        if game.kingdom(id).weather.value() < 3 {
            q4 = (q4 + 10).min(MAX_GRAIN_PRICE);
        }
        let k = game.kingdom_mut(id);
        // 1-in-3 years it matches the humans' volume (bounded by real stocks);
        // like theirs, the listing reaches the stall next year.
        let listed = k.offered();
        if q3 > listed && random(1, 10) > 6 {
            let add = (q3 - listed).min(k.grain_stocks - listed) / MIN_LOT * MIN_LOT;
            if add > 0 {
                k.list_grain(add, q4.max(1));
                decision.grain_listed = Some((add, q4.max(1)));
            }
        }
        // Grain already on the stall follows the market's price.
        k.grain_price = if k.for_sale() > 0 { q4.max(1) } else { q4 };
    }

    // Original lines 228–232: shop at one random kingdom's stall, buying a
    // random affordable amount (a couple of tries before giving up).
    let stalls: Vec<Kingdoms> = game
        .alive_kingdoms()
        .into_iter()
        .filter(|&o| o != id)
        .collect();
    if !stalls.is_empty() {
        let seller = stalls[random(0, stalls.len() as i32) as usize];
        let (on_sale, price) = {
            let s = game.kingdom(seller);
            (s.for_sale(), s.grain_price.min(MAX_GRAIN_PRICE))
        };
        if on_sale >= MIN_LOT && price > 0 {
            for _ in 0..3 {
                let amount = (random(MIN_LOT, on_sale + 1) / MIN_LOT * MIN_LOT).max(MIN_LOT);
                if calculate_buy_cost(amount, price) <= game.kingdom(id).treasury {
                    apply_trade(game, id, Trade::Buy { amount, seller });
                    decision.grain_bought = Some((seller, amount));
                    break;
                }
            }
        }
    }

    decision
}

/// The computer's war orders, given at the Extérieur once the éclaireur is
/// back: the first two years are the original's barbarian raids; from the
/// third the mind decides on what `mind.seen` tells it (the caller feeds it
/// with the éclaireur's report and sends the éclaireur asked for, paid here).
pub fn plan_ai_war(game: &mut EmpireGame, id: Kingdoms, mind: &mut Mind) -> AiWar {
    let mut war = AiWar::default();
    let k = game.kingdom(id);
    if k.is_player || k.is_dead {
        return war;
    }
    let (nobles, soldiers) = (k.nobles, k.soldiers);
    if game.year < 3 {
        let mut attacks_allowed = nobles / 4 + 1;
        while random(1, 5) >= 2 && attacks_allowed > 0 && soldiers > 0 {
            war.barbarian_attacks.push(random(1, soldiers).max(1));
            attacks_allowed -= 1;
        }
        return war;
    }
    let enemies: Vec<&Kingdom> = game
        .kingdoms
        .iter()
        .filter(|k| k.id != id && !k.is_dead)
        .collect();
    let orders = mind.campaign(game.kingdom(id), &enemies);
    war.kingdom_attacks
        .extend(orders.blind.into_iter().chain(orders.aimed));
    if orders.scout.is_some() {
        game.kingdom_mut(id).treasury -= SCOUT_PRICE;
        war.scout = orders.scout;
    }
    war
}

/// The whole year at once — intendance, then war — for a table where the
/// computers play one after the other. Battles are returned for the caller
/// to run (so a UI can animate them) — see [`execute_ai_turn`].
pub fn plan_ai_turn(game: &mut EmpireGame, id: Kingdoms, mind: &mut Mind) -> AiTurnDecision {
    let mut decision = plan_ai_intendance(game, id);
    let war = plan_ai_war(game, id, mind);
    decision.barbarian_attacks = war.barbarian_attacks;
    decision.kingdom_attacks = war.kingdom_attacks;
    decision.scout = war.scout;
    decision
}

/// Plan and fully execute the AI turn, running every planned battle silently;
/// the éclaireur reads the realm at once (or is taken, one in six) for next
/// year's orders.
pub fn execute_ai_turn(game: &mut EmpireGame, id: Kingdoms, mind: &mut Mind) -> AiTurnDecision {
    let decision = plan_ai_turn(game, id, mind);

    for &soldiers in &decision.barbarian_attacks {
        let soldiers = soldiers.min(game.kingdom(id).soldiers);
        if soldiers <= 0 {
            break;
        }
        let result = simulate_barbarian_battle(game, id, soldiers, |_| {});
        apply_barbarian_battle_result(game, id, soldiers, &result);
    }

    for &(target, soldiers) in &decision.kingdom_attacks {
        if game.kingdom(target).is_dead {
            continue;
        }
        let soldiers = soldiers.min(game.kingdom(id).soldiers);
        if soldiers <= 0 {
            break;
        }
        let result = simulate_kingdom_battle(game, id, target, soldiers, |_| {});
        apply_kingdom_battle_result(game, id, target, soldiers, &result);
    }

    if let Some(on) = decision.scout {
        if random(0, SCOUT_CAUGHT) != 0 {
            mind.seen = Some(Seen::read(game.kingdom(on)));
        }
    }

    decision
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mind::Temper;
    use crate::trade::grain_value;

    fn mind() -> Mind {
        Mind {
            temper: Temper::Measured,
            eye: None,
            seen: None,
        }
    }

    #[test]
    fn players_and_dead_kingdoms_do_nothing() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).is_player = true;
        game.kingdom_mut(Kingdoms::Spain).is_dead = true;
        let before = game.clone();
        plan_ai_turn(&mut game, Kingdoms::France, &mut mind());
        plan_ai_turn(&mut game, Kingdoms::Spain, &mut mind());
        assert_eq!(game, before);
    }

    #[test]
    fn ai_grows_and_only_raids_barbarians_before_year_three() {
        let mut game = EmpireGame::default();
        let d = plan_ai_turn(&mut game, Kingdoms::Germany, &mut mind());
        assert!(d.kingdom_attacks.is_empty());
        let k = game.kingdom(Kingdoms::Germany);
        assert_eq!(k.peasants, 2000 + d.growth.peasants);
        assert!(k.soldiers_efficiency <= 150);
    }

    #[test]
    fn ai_nobles_grow_in_decent_weather() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::Germany).weather = crate::Weather::Good;
        for _ in 0..60 {
            plan_ai_turn(&mut game, Kingdoms::Germany, &mut mind());
        }
        let k = game.kingdom(Kingdoms::Germany);
        assert!(k.nobles > 1, "nobles stuck at {}", k.nobles);
        assert!(k.palaces <= 10);
    }

    #[test]
    fn ai_mirrors_the_human_market() {
        let mut game = EmpireGame::default();
        let h = game.kingdom_mut(Kingdoms::France);
        h.is_player = true;
        h.grain_to_sell = 5000;
        h.grain_price = 30;
        let mut listed = false;
        for _ in 0..60 {
            let before = game.kingdom(Kingdoms::Germany).clone();
            let d = plan_ai_turn(&mut game, Kingdoms::Germany, &mut mind());
            if let Some((amount, price)) = d.grain_listed {
                listed = true;
                let k = game.kingdom(Kingdoms::Germany);
                assert!(amount >= MIN_LOT && amount % MIN_LOT == 0, "{amount}");
                assert!((1..=MAX_GRAIN_PRICE).contains(&price));
                // The listing waits for next year's market.
                assert_eq!(k.grain_to_sell, before.grain_to_sell);
                assert_eq!(
                    k.listing.map(|l| l.0),
                    Some(before.listing.map_or(0, |l| l.0) + amount)
                );
            }
        }
        assert!(listed, "the computer never matched the human market");
    }

    #[test]
    fn ai_buys_from_a_stall_it_can_afford() {
        let mut game = EmpireGame::default();
        let h = game.kingdom_mut(Kingdoms::France);
        h.is_player = true;
        h.grain_to_sell = 2000;
        h.grain_price = 20;
        let mut bought = false;
        for _ in 0..40 {
            game.kingdom_mut(Kingdoms::Germany).treasury = 50_000;
            let before = game.kingdom(Kingdoms::France).clone();
            let d = plan_ai_turn(&mut game, Kingdoms::Germany, &mut mind());
            if let Some((seller, amount)) = d.grain_bought {
                if seller == Kingdoms::France {
                    bought = true;
                    assert!(amount >= MIN_LOT && amount % MIN_LOT == 0, "{amount}");
                    let f = game.kingdom(Kingdoms::France);
                    assert_eq!(f.grain_to_sell, before.grain_to_sell - amount);
                    assert_eq!(
                        f.treasury,
                        before.treasury + grain_value(amount, before.grain_price)
                    );
                    break;
                }
            }
        }
        assert!(bought, "the computer never bought from the human stall");
    }

    #[test]
    fn ai_never_targets_itself_or_the_dead() {
        let mut game = EmpireGame {
            year: 5,
            ..Default::default()
        };
        game.kingdom_mut(Kingdoms::Spain).is_dead = true;
        let mut m = Mind {
            temper: Temper::Bold,
            ..mind()
        };
        for _ in 0..50 {
            game.kingdom_mut(Kingdoms::Germany).treasury = 1000;
            let d = plan_ai_turn(&mut game, Kingdoms::Germany, &mut m);
            for (target, soldiers) in d.kingdom_attacks {
                assert_ne!(target, Kingdoms::Germany);
                assert_ne!(target, Kingdoms::Spain);
                assert!(soldiers >= 1);
            }
            assert!(d
                .scout
                .is_some_and(|s| s != Kingdoms::Germany && s != Kingdoms::Spain));
            if d.grain_bought.is_none() {
                let k = game.kingdom(Kingdoms::Germany);
                assert_eq!(k.treasury, 1000 + d.growth.treasury - SCOUT_PRICE);
            }
        }
    }

    #[test]
    fn a_scout_sent_reads_the_realm_for_next_year() {
        let mut game = EmpireGame {
            year: 4,
            ..Default::default()
        };
        let mut m = mind();
        let (mut sent, mut read) = (0, 0);
        while sent < 60 && game.alive_kingdoms().len() > 1 {
            // Money for the éclaireur, no men to march (the council raises
            // a few: the table may thin out).
            let k = game.kingdom_mut(Kingdoms::Germany);
            k.treasury = 100_000;
            k.soldiers = 0;
            let d = execute_ai_turn(&mut game, Kingdoms::Germany, &mut m);
            let on = d.scout.expect("an éclaireur every year with money");
            assert_eq!(m.eye, Some(on));
            sent += 1;
            if let Some(seen) = m.seen {
                read += 1;
                assert_eq!(seen, Seen::read(game.kingdom(on)));
            }
        }
        assert!(sent >= 20 && read * 3 >= sent * 2, "{read} of {sent}");
    }

    #[test]
    fn execute_ai_turn_keeps_state_consistent() {
        let mut game = EmpireGame {
            year: 4,
            ..Default::default()
        };
        for id in crate::KINGDOMS {
            execute_ai_turn(&mut game, id, &mut mind());
        }
        for k in &game.kingdoms {
            assert!(k.soldiers >= 0, "{:?}", k);
            assert!(k.surface >= 0, "{:?}", k);
        }
    }
}
