//! Combat damage calculation and resolution.

use crate::unit::{Unit, UnitKind};
use crate::grid::Grid;

/// Calculate melee/ranged damage.
pub fn calc_damage(attacker: &Unit, defender: &Unit, grid: &Grid) -> i32 {
    let terrain_def = grid.tile_at_world(defender.x, defender.y).defense_bonus();
    let dmg = attacker.kind.attack() - defender.kind.defense() - terrain_def;
    dmg.max(1)
}

/// Resolve combat between two units. Returns damage dealt.
pub fn resolve_attack(attacker_idx: usize, defender_idx: usize, units: &mut [Unit], grid: &Grid) -> i32 {
    let dmg = {
        let atk = &units[attacker_idx];
        let def = &units[defender_idx];
        if !atk.alive || !def.alive || atk.faction == def.faction {
            return 0;
        }
        if !atk.can_attack() || !atk.in_range(def) {
            return 0;
        }
        calc_damage(atk, def, grid)
    };

    units[attacker_idx].cooldown = units[attacker_idx].kind.attack_cooldown();
    units[attacker_idx].anim = crate::unit::UnitAnim::Attack;

    // Face the target (AI only — player facing controlled by aim direction)
    let dx = units[defender_idx].x - units[attacker_idx].x;
    if dx > 0.0 {
        units[attacker_idx].facing = crate::unit::Facing::Right;
    } else if dx < 0.0 {
        units[attacker_idx].facing = crate::unit::Facing::Left;
    }

    units[defender_idx].hp -= dmg;
    units[defender_idx].hit_flash = 0.15;
    if units[defender_idx].hp <= 0 {
        units[defender_idx].alive = false;
        units[defender_idx].death_fade = 0.01; // start fading
    }

    dmg
}

/// Monk healing.
pub fn resolve_heal(healer_idx: usize, target_idx: usize, units: &mut [Unit]) -> i32 {
    let can_heal = {
        let h = &units[healer_idx];
        let t = &units[target_idx];
        h.alive && t.alive
            && h.kind == UnitKind::Monk
            && h.faction == t.faction
            && h.can_attack()
            && h.in_range(t)
            && t.hp < t.kind.max_hp()
    };

    if !can_heal { return 0; }

    units[healer_idx].cooldown = units[healer_idx].kind.attack_cooldown();
    let max_hp = units[target_idx].kind.max_hp();
    let heal = 3.min(max_hp - units[target_idx].hp);
    units[target_idx].hp += heal;
    heal
}
