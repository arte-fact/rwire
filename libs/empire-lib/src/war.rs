use std::cmp::max;

use crate::game::EmpireGame;
use crate::kingdom::{Kingdom, Kingdoms};
use crate::random::random;

/// Result of a battle between kingdoms.
#[derive(Debug, Clone)]
pub struct BattleResult {
    pub attacker_won: bool,
    pub attacker_remaining_soldiers: i32,
    pub surface_conquered: i32,
    pub defender_conquered: bool,
    pub defender_remaining_soldiers: i32,
    pub defender_remaining_peasants: i32,
    /// Damage dealt in a "big battle" scenario.
    pub collateral_damage: Option<CollateralDamage>,
}

#[derive(Debug, Clone)]
pub struct CollateralDamage {
    pub peasants_killed: i32,
    pub marketplaces_destroyed: i32,
    pub grain_destroyed: i32,
    pub grain_mills_destroyed: i32,
    pub foundries_destroyed: i32,
    pub shipyards_destroyed: i32,
    pub nobles_killed: i32,
}

/// Progress update emitted after every combat round.
#[derive(Debug, Clone)]
pub struct BattleProgress {
    pub attacker_soldiers: i32,
    pub defender_soldiers: i32,
    pub defender_peasants: i32,
    pub population_defending: bool,
}

/// Simulate a battle between two kingdoms without modifying state; apply the
/// result with [`apply_kingdom_battle_result`].
pub fn simulate_kingdom_battle(
    game: &EmpireGame,
    attacker: Kingdoms,
    defender: Kingdoms,
    soldiers_sent: i32,
    mut on_progress: impl FnMut(&BattleProgress),
) -> BattleResult {
    let attacker = game.kingdom(attacker);
    let defender = game.kingdom(defender);

    let mut attacker_soldiers = soldiers_sent;
    let attacking_strength = attacker.soldiers_efficiency;
    let mut defending_strength = defender.soldiers_efficiency;

    let mut defender_soldiers = defender.soldiers;
    let mut defender_peasants = defender.peasants;
    let defender_surface = defender.surface;

    // Peasants defend when there are no soldiers (at fixed efficiency 5).
    let population_defending = defender_soldiers <= 0;
    if population_defending {
        defending_strength = 5;
    }

    let mut surface_conquered = 0;

    loop {
        // All land conquered — remaining peasants are captured.
        if surface_conquered >= defender_surface {
            return BattleResult {
                attacker_won: true,
                attacker_remaining_soldiers: attacker_soldiers,
                surface_conquered: defender_surface,
                defender_conquered: true,
                defender_remaining_soldiers: 0,
                defender_remaining_peasants: defender_peasants,
                collateral_damage: None,
            };
        }

        // All defending peasants killed.
        if defender_peasants <= 0 && population_defending {
            return BattleResult {
                attacker_won: true,
                attacker_remaining_soldiers: attacker_soldiers,
                surface_conquered: defender_surface,
                defender_conquered: true,
                defender_remaining_soldiers: 0,
                defender_remaining_peasants: 0,
                collateral_damage: None,
            };
        }

        // The defending army is wiped out: the expedition ends with the land taken
        // so far. (Serfs only defend when the realm had no army to begin with.)
        if defender_soldiers <= 0 && !population_defending {
            return BattleResult {
                attacker_won: true,
                attacker_remaining_soldiers: attacker_soldiers,
                surface_conquered,
                defender_conquered: false,
                defender_remaining_soldiers: 0,
                defender_remaining_peasants: defender_peasants,
                collateral_damage: None,
            };
        }

        if attacker_soldiers <= 0 {
            let big_battle = surface_conquered >= defender_surface / 3;
            let collateral = big_battle.then(|| collateral_damage(defender));
            return BattleResult {
                attacker_won: false,
                attacker_remaining_soldiers: 0,
                surface_conquered,
                defender_conquered: false,
                defender_remaining_soldiers: defender_soldiers,
                defender_remaining_peasants: defender_peasants,
                collateral_damage: collateral,
            };
        }

        // Original: I7=INT(I1/15)+1
        let troop_unit = soldiers_sent / 15 + 1;

        // Original: IF INT(RND*I4+1)<INT(RND*I3+1) THEN 148 (defender wins the round)
        if random(1, attacking_strength) >= random(1, defending_strength) {
            // Original: I5=I5+INT(RND*I7*26+1)-INT(RND*(I7+5)+1)
            let land_gained = random(1, troop_unit * 26) - random(1, troop_unit + 5);
            surface_conquered = max(0, surface_conquered + land_gained);
            if population_defending {
                defender_peasants = max(0, defender_peasants - troop_unit);
            } else {
                defender_soldiers = max(0, defender_soldiers - troop_unit);
            }
        } else {
            attacker_soldiers = max(0, attacker_soldiers - troop_unit);
        }

        on_progress(&BattleProgress {
            attacker_soldiers,
            defender_soldiers,
            defender_peasants,
            population_defending,
        });
    }
}

/// Random collateral damage for a big battle (>= 1/3 of the land contested).
fn collateral_damage(defender: &Kingdom) -> CollateralDamage {
    CollateralDamage {
        peasants_killed: random(1, defender.peasants.max(1)),
        marketplaces_destroyed: random(1, defender.marketplaces.max(1)),
        grain_destroyed: random(1, defender.grain_stocks.max(1)),
        grain_mills_destroyed: random(1, defender.grain_mills.max(1)),
        foundries_destroyed: random(1, defender.foundries.max(1)),
        shipyards_destroyed: random(1, defender.shipyards.max(1)),
        nobles_killed: random(1, (defender.nobles / 2).max(1)),
    }
}

/// Result of a barbarian expedition.
#[derive(Debug, Clone)]
pub struct BarbarianBattleResult {
    pub attacker_won: bool,
    pub attacker_remaining_soldiers: i32,
    pub surface_conquered: i32,
    pub all_barbarians_conquered: bool,
}

/// Simulate an expedition against the barbarians without modifying state;
/// apply the result with [`apply_barbarian_battle_result`].
pub fn simulate_barbarian_battle(
    game: &EmpireGame,
    attacker: Kingdoms,
    soldiers_sent: i32,
    mut on_progress: impl FnMut(&BattleProgress),
) -> BarbarianBattleResult {
    let attacking_strength = game.kingdom(attacker).soldiers_efficiency;
    let barbarian_surface = game.barbarians_surface;

    // Original: I2=INT(RND*INT(RND*(I1+1)*3)+1)+INT(RND*INT(RND*(I1+1.5)+1)+1)
    let inner1 = random(0, (soldiers_sent + 1) * 3);
    let part1 = random(1, inner1.max(1) + 1);
    let inner2 = random(1, soldiers_sent + 2);
    let part2 = random(1, inner2.max(1) + 1);
    let mut defender_soldiers = part1 + part2;

    let mut attacker_soldiers = soldiers_sent;
    let mut surface_conquered = 0;

    loop {
        let troop_unit = soldiers_sent / 15 + 1;

        // Barbarians fight at fixed strength 9.
        if random(1, attacking_strength) < random(1, 9) {
            attacker_soldiers -= troop_unit;
        } else {
            let land_gained = random(1, troop_unit * 26) - random(1, troop_unit + 5);
            surface_conquered = max(0, surface_conquered + land_gained);
            defender_soldiers -= troop_unit;
        }

        on_progress(&BattleProgress {
            attacker_soldiers,
            defender_soldiers,
            defender_peasants: 0,
            population_defending: false,
        });

        let all_conquered = surface_conquered >= barbarian_surface;
        let surface_conquered = surface_conquered.min(barbarian_surface);

        if defender_soldiers <= 0 || all_conquered {
            return BarbarianBattleResult {
                attacker_won: true,
                attacker_remaining_soldiers: max(0, attacker_soldiers),
                surface_conquered,
                all_barbarians_conquered: all_conquered,
            };
        }

        if attacker_soldiers <= 0 {
            return BarbarianBattleResult {
                attacker_won: false,
                attacker_remaining_soldiers: 0,
                surface_conquered,
                all_barbarians_conquered: false,
            };
        }
    }
}

/// Apply the result of a kingdom battle to the game state.
pub fn apply_kingdom_battle_result(
    game: &mut EmpireGame,
    attacker: Kingdoms,
    defender: Kingdoms,
    soldiers_sent: i32,
    result: &BattleResult,
) {
    let attacker_remaining_home = game.kingdom(attacker).soldiers - soldiers_sent;
    let attacker_soldiers = attacker_remaining_home + result.attacker_remaining_soldiers;

    if result.defender_conquered {
        let d = game.kingdom_mut(defender);
        let (surface, treasury) = (d.surface, d.treasury);
        d.peasants = 0;
        d.surface = 0;
        d.treasury = 0;
        d.soldiers = 0;
        d.is_dead = true;

        let a = game.kingdom_mut(attacker);
        if result.defender_remaining_peasants > 0 {
            a.peasants += result.defender_remaining_peasants;
        }
        a.surface += surface;
        a.treasury += treasury;
        a.soldiers = attacker_soldiers;
    } else {
        let a = game.kingdom_mut(attacker);
        a.soldiers = attacker_soldiers;
        a.surface += result.surface_conquered;

        let d = game.kingdom_mut(defender);
        d.surface -= result.surface_conquered;
        d.soldiers = result.defender_remaining_soldiers;
        if let Some(damage) = &result.collateral_damage {
            apply_collateral_damage(d, damage);
        }
    }
}

fn apply_collateral_damage(kingdom: &mut Kingdom, damage: &CollateralDamage) {
    kingdom.peasants -= damage.peasants_killed;
    kingdom.marketplaces -= damage.marketplaces_destroyed;
    kingdom.grain_stocks -= damage.grain_destroyed;
    kingdom.grain_mills -= damage.grain_mills_destroyed;
    kingdom.foundries -= damage.foundries_destroyed;
    kingdom.shipyards -= damage.shipyards_destroyed;
    kingdom.nobles -= damage.nobles_killed;
}

/// Apply the result of a barbarian expedition to the game state.
pub fn apply_barbarian_battle_result(
    game: &mut EmpireGame,
    attacker: Kingdoms,
    soldiers_sent: i32,
    result: &BarbarianBattleResult,
) {
    game.barbarians_surface -= result.surface_conquered;
    let a = game.kingdom_mut(attacker);
    let remaining_home = a.soldiers - soldiers_sent;
    a.surface += result.surface_conquered;
    a.soldiers = remaining_home + result.attacker_remaining_soldiers;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kingdom_battle_terminates_and_reports_progress() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).soldiers = 300;
        let mut rounds = 0;
        let result = simulate_kingdom_battle(&game, Kingdoms::France, Kingdoms::Spain, 300, |_| {
            rounds += 1
        });
        assert!(rounds > 0);
        assert!(result.surface_conquered >= 0);
        assert!(result.attacker_remaining_soldiers >= 0);
    }

    #[test]
    fn expedition_ends_when_the_defending_army_is_wiped_out() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).soldiers = 5000;
        game.kingdom_mut(Kingdoms::Spain).soldiers = 5; // tiny army, huge land
        for _ in 0..30 {
            let r = simulate_kingdom_battle(&game, Kingdoms::France, Kingdoms::Spain, 5000, |p| {
                assert!(!p.population_defending, "serfs must not take over mid-battle");
            });
            if r.attacker_won && !r.defender_conquered {
                assert_eq!(r.defender_remaining_soldiers, 0);
                assert!(r.surface_conquered < 10000);
            }
        }
    }

    #[test]
    fn serfs_defend_a_realm_without_an_army() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).soldiers = 100;
        game.kingdom_mut(Kingdoms::Spain).soldiers = 0;
        let mut saw_serfs = false;
        simulate_kingdom_battle(&game, Kingdoms::France, Kingdoms::Spain, 100, |p| {
            saw_serfs |= p.population_defending;
        });
        assert!(saw_serfs);
    }

    #[test]
    fn conquest_absorbs_the_defender() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).soldiers = 100;
        let result = BattleResult {
            attacker_won: true,
            attacker_remaining_soldiers: 40,
            surface_conquered: 10000,
            defender_conquered: true,
            defender_remaining_soldiers: 0,
            defender_remaining_peasants: 500,
            collateral_damage: None,
        };
        apply_kingdom_battle_result(&mut game, Kingdoms::France, Kingdoms::Spain, 60, &result);
        let a = game.kingdom(Kingdoms::France);
        let d = game.kingdom(Kingdoms::Spain);
        assert_eq!(a.soldiers, 40 + 40);
        assert_eq!(a.surface, 20000);
        assert_eq!(a.treasury, 2000);
        assert_eq!(a.peasants, 2500);
        assert!(d.is_dead);
        assert_eq!(d.surface, 0);
        assert_eq!(game.alive_kingdoms().len(), 5);
    }

    #[test]
    fn partial_victory_moves_land_and_applies_collateral_damage() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).soldiers = 100;
        game.kingdom_mut(Kingdoms::Spain).marketplaces = 3;
        let result = BattleResult {
            attacker_won: false,
            attacker_remaining_soldiers: 0,
            surface_conquered: 4000,
            defender_conquered: false,
            defender_remaining_soldiers: 5,
            defender_remaining_peasants: 2000,
            collateral_damage: Some(CollateralDamage {
                peasants_killed: 100,
                marketplaces_destroyed: 2,
                grain_destroyed: 500,
                grain_mills_destroyed: 0,
                foundries_destroyed: 0,
                shipyards_destroyed: 0,
                nobles_killed: 0,
            }),
        };
        apply_kingdom_battle_result(&mut game, Kingdoms::France, Kingdoms::Spain, 100, &result);
        let a = game.kingdom(Kingdoms::France);
        let d = game.kingdom(Kingdoms::Spain);
        assert_eq!(a.soldiers, 0);
        assert_eq!(a.surface, 14000);
        assert_eq!(d.surface, 6000);
        assert_eq!(d.soldiers, 5);
        assert_eq!(d.peasants, 1900);
        assert_eq!(d.marketplaces, 1);
        assert!(!d.is_dead);
    }

    #[test]
    fn barbarian_conquest_never_exceeds_barbarian_land() {
        let mut game = EmpireGame {
            barbarians_surface: 50,
            ..Default::default()
        };
        game.kingdom_mut(Kingdoms::France).soldiers = 5000;
        for _ in 0..50 {
            let r = simulate_barbarian_battle(&game, Kingdoms::France, 5000, |_| {});
            assert!(r.surface_conquered <= 50);
            assert_eq!(r.all_barbarians_conquered, r.surface_conquered == 50);
        }
    }

    #[test]
    fn barbarian_expedition_updates_land_and_army() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::France).soldiers = 50;
        let result = simulate_barbarian_battle(&game, Kingdoms::France, 30, |_| {});
        assert!(result.surface_conquered <= game.barbarians_surface);
        apply_barbarian_battle_result(&mut game, Kingdoms::France, 30, &result);
        assert_eq!(game.barbarians_surface, 6000 - result.surface_conquered);
        let a = game.kingdom(Kingdoms::France);
        assert_eq!(a.surface, 10000 + result.surface_conquered);
        assert_eq!(a.soldiers, 20 + result.attacker_remaining_soldiers);
    }
}
