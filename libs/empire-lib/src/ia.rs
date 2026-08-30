use crate::game::EmpireGame;
use crate::kingdom::Kingdoms;
use crate::random::random;
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

    // Phase 1: resource growth
    let peasant_growth = random(0, 200) + (random(0, 200) as f32 * weather_ratio) as i32;
    let merchant_growth = (random(0, 25) as f32 * weather_ratio) as i32;
    let marketplace_growth = (random(0, 4) as f32 * weather_ratio) as i32;
    let mill_growth = (random(0, 2) as f32 * weather_ratio) as i32;
    let treasury_growth = (random(0, 1500) as f32 * weather_ratio) as i32;

    let (foundry_growth, shipyard_growth, palace_growth, noble_growth) = if weather_ratio >= 0.5 {
        let foundry = (random(0, 2) as f32 * weather_ratio) as i32;
        let shipyard = (random(0, 2) as f32 * weather_ratio) as i32;
        let (palace, noble) = if random(1, 2) == 1 {
            (
                (random(0, 2) as f32 * weather_ratio) as i32,
                (random(0, 2) as f32 * weather_ratio) as i32,
            )
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
    kingdom.palaces += palace_growth;
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

    // Phase 2: plan combat (decide only, don't execute)
    let mut attacks_allowed = kingdom.nobles / 4 + 1;
    let soldiers = kingdom.soldiers;

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
