use crate::game::EmpireGame;
use crate::kingdom::Kingdoms;
use crate::random::random;
use crate::trade::{apply_trade, calculate_buy_cost, Trade, MAX_GRAIN_PRICE};
use crate::war::{
    apply_barbarian_battle_result, apply_kingdom_battle_result, simulate_barbarian_battle,
    simulate_kingdom_battle,
};

/// What the AI decided to do.
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

/// Plan the AI turn: growth is applied immediately, battles are returned for
/// the caller to run (so a UI can animate them) — see [`execute_ai_turn`].
pub fn plan_ai_turn(game: &mut EmpireGame, id: Kingdoms) -> AiTurnDecision {
    if game.kingdom(id).is_player || game.kingdom(id).is_dead {
        return AiTurnDecision::default();
    }

    let weather_ratio = game.weather.value() as f32 / 6.0;
    let year = game.year;
    let barbarians_surface = game.barbarians_surface;
    let enemies: Vec<Kingdoms> = game
        .alive_kingdoms()
        .into_iter()
        .filter(|&k| k != id)
        .collect();

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
    kingdom.soldiers_efficiency = (kingdom.soldiers_efficiency + efficiency_growth).min(15);
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
    let (nobles, soldiers) = (kingdom.nobles, kingdom.soldiers);
    let humans: Vec<(i32, i32)> = game
        .kingdoms
        .iter()
        .filter(|k| k.is_player && !k.is_dead)
        .map(|k| (k.grain_to_sell, k.grain_price))
        .collect();
    if !humans.is_empty() {
        let n = humans.len() as i32;
        // Q3/Q4: human averages with noise (±1000 bushels, ±1 on the price).
        let q3 = (humans.iter().map(|h| h.0).sum::<i32>() / n + random(1, 1001) - random(1, 1001))
            .max(0);
        let mut q4 = (humans.iter().map(|h| h.1).sum::<i32>() / n + random(0, 2) - random(0, 2))
            .clamp(0, MAX_GRAIN_PRICE);
        // Bad years push the asking price up (original: +RND/1.5 when NW<3).
        if game.weather.value() < 3 {
            q4 = (q4 + 1).min(MAX_GRAIN_PRICE);
        }
        let k = game.kingdom_mut(id);
        // 1-in-3 years it matches the humans' volume (bounded by real stocks).
        if q3 > k.grain_to_sell && random(1, 10) > 6 {
            let add = (q3 - k.grain_to_sell).min(k.grain_stocks.max(0));
            if add > 0 {
                k.grain_to_sell += add;
                k.grain_stocks -= add;
                decision.grain_listed = Some((add, q4.max(1)));
            }
        }
        k.grain_price = if k.grain_to_sell > 0 { q4.max(1) } else { q4 };
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
            (s.grain_to_sell, s.grain_price.min(MAX_GRAIN_PRICE))
        };
        if on_sale > 0 && price > 0 {
            for _ in 0..3 {
                let amount = random(1, on_sale + 1).max(1);
                if calculate_buy_cost(amount, price) <= game.kingdom(id).treasury {
                    apply_trade(game, id, Trade::Buy { amount, seller });
                    decision.grain_bought = Some((seller, amount));
                    break;
                }
            }
        }
    }

    // Phase 2: plan combat (decide only, don't execute)
    let mut attacks_allowed = nobles / 4 + 1;

    loop {
        if random(1, 5) < 2 || attacks_allowed <= 0 || soldiers <= 0 {
            break;
        }

        if year < 3 {
            if barbarians_surface <= 0 {
                break;
            }
            decision.barbarian_attacks.push(random(1, soldiers).max(1));
        } else {
            if enemies.is_empty() {
                break;
            }
            let target = enemies[random(0, enemies.len() as i32) as usize];
            let soldiers_to_send = random(soldiers / 3, soldiers).max(1);
            decision.kingdom_attacks.push((target, soldiers_to_send));
        }
        attacks_allowed -= 1;
    }

    decision
}

/// Plan and fully execute the AI turn, running every planned battle silently.
pub fn execute_ai_turn(game: &mut EmpireGame, id: Kingdoms) -> AiTurnDecision {
    let decision = plan_ai_turn(game, id);

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

    decision
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn players_and_dead_kingdoms_do_nothing() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).is_player = true;
        game.kingdom_mut(Kingdoms::Spain).is_dead = true;
        let before = game.clone();
        plan_ai_turn(&mut game, Kingdoms::France);
        plan_ai_turn(&mut game, Kingdoms::Spain);
        assert_eq!(game, before);
    }

    #[test]
    fn ai_grows_and_only_raids_barbarians_before_year_three() {
        let mut game = EmpireGame::default();
        let d = plan_ai_turn(&mut game, Kingdoms::Germany);
        assert!(d.kingdom_attacks.is_empty());
        let k = game.kingdom(Kingdoms::Germany);
        assert_eq!(k.peasants, 2000 + d.growth.peasants);
        assert!(k.soldiers_efficiency <= 15);
    }

    #[test]
    fn ai_nobles_grow_in_decent_weather() {
        let mut game = EmpireGame {
            weather: crate::Weather::Good,
            ..Default::default()
        };
        for _ in 0..60 {
            plan_ai_turn(&mut game, Kingdoms::Germany);
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
        h.grain_price = 8;
        let mut listed = false;
        for _ in 0..60 {
            let before = game.kingdom(Kingdoms::Germany).clone();
            let d = plan_ai_turn(&mut game, Kingdoms::Germany);
            if let Some((amount, price)) = d.grain_listed {
                listed = true;
                let k = game.kingdom(Kingdoms::Germany);
                assert!(amount > 0);
                assert!((1..=MAX_GRAIN_PRICE).contains(&price));
                // Honest books: the listing came out of real stocks.
                assert_eq!(k.grain_to_sell, before.grain_to_sell + amount);
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
        h.grain_price = 3;
        let mut bought = false;
        for _ in 0..40 {
            game.kingdom_mut(Kingdoms::Germany).treasury = 50_000;
            let before = game.kingdom(Kingdoms::France).clone();
            let d = plan_ai_turn(&mut game, Kingdoms::Germany);
            if let Some((seller, amount)) = d.grain_bought {
                if seller == Kingdoms::France {
                    bought = true;
                    let f = game.kingdom(Kingdoms::France);
                    assert_eq!(f.grain_to_sell, before.grain_to_sell - amount);
                    assert_eq!(f.treasury, before.treasury + amount * before.grain_price);
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
        game.kingdom_mut(Kingdoms::Germany).nobles = 40; // many attacks allowed
        for _ in 0..50 {
            let d = plan_ai_turn(&mut game, Kingdoms::Germany);
            for (target, soldiers) in d.kingdom_attacks {
                assert_ne!(target, Kingdoms::Germany);
                assert_ne!(target, Kingdoms::Spain);
                assert!(soldiers >= 1);
            }
        }
    }

    #[test]
    fn execute_ai_turn_keeps_state_consistent() {
        let mut game = EmpireGame {
            year: 4,
            ..Default::default()
        };
        for id in crate::KINGDOMS {
            execute_ai_turn(&mut game, id);
        }
        for k in &game.kingdoms {
            assert!(k.soldiers >= 0, "{:?}", k);
            assert!(k.surface >= 0, "{:?}", k);
        }
        assert!(game.barbarians_surface >= 0);
    }
}
