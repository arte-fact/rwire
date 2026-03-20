//! Sprite sheet definitions and texture/sprite table generation.
//!
//! Maps all game assets to texture IDs, defines sprite rects for
//! the sprite table, and provides animation frame lookups.

use crate::unit::{Faction, UnitKind};

/// Texture IDs — each maps to one PNG file loaded by the browser.
pub mod tex {
    // Unit sprites: Blue (0-11), Red (12-23)
    pub const BLUE_WARRIOR_IDLE: u8 = 0;
    pub const BLUE_WARRIOR_RUN: u8 = 1;
    pub const BLUE_WARRIOR_ATK: u8 = 2;
    pub const BLUE_ARCHER_IDLE: u8 = 3;
    pub const BLUE_ARCHER_RUN: u8 = 4;
    pub const BLUE_ARCHER_ATK: u8 = 5;
    pub const BLUE_LANCER_IDLE: u8 = 6;
    pub const BLUE_LANCER_RUN: u8 = 7;
    pub const BLUE_LANCER_ATK: u8 = 8;
    pub const BLUE_MONK_IDLE: u8 = 9;
    pub const BLUE_MONK_RUN: u8 = 10;
    pub const BLUE_MONK_ATK: u8 = 11;
    pub const RED_WARRIOR_IDLE: u8 = 12;
    pub const RED_WARRIOR_RUN: u8 = 13;
    pub const RED_WARRIOR_ATK: u8 = 14;
    pub const RED_ARCHER_IDLE: u8 = 15;
    pub const RED_ARCHER_RUN: u8 = 16;
    pub const RED_ARCHER_ATK: u8 = 17;
    pub const RED_LANCER_IDLE: u8 = 18;
    pub const RED_LANCER_RUN: u8 = 19;
    pub const RED_LANCER_ATK: u8 = 20;
    pub const RED_MONK_IDLE: u8 = 21;
    pub const RED_MONK_RUN: u8 = 22;
    pub const RED_MONK_ATK: u8 = 23;
    // Terrain
    pub const TILEMAP1: u8 = 24;
    pub const TILEMAP2: u8 = 25;
    pub const WATER: u8 = 26;
    pub const FOAM: u8 = 27;
    pub const SHADOW: u8 = 28;
    // Trees (4 variants)
    pub const TREE1: u8 = 29;
    pub const TREE2: u8 = 30;
    pub const TREE3: u8 = 31;
    pub const TREE4: u8 = 32;
    // Rocks
    pub const ROCK1: u8 = 33;
    pub const ROCK2: u8 = 34;
    pub const ROCK3: u8 = 35;
    pub const ROCK4: u8 = 36;
    // Bushes
    pub const BUSH1: u8 = 37;
    pub const BUSH2: u8 = 38;
    pub const BUSH3: u8 = 39;
    pub const BUSH4: u8 = 40;
    // Buildings
    pub const TOWER_BLACK: u8 = 41;
    pub const TOWER_BLUE: u8 = 42;
    pub const TOWER_RED: u8 = 43;
    pub const BARRACKS_BLUE: u8 = 44;
    pub const BARRACKS_RED: u8 = 45;
    pub const ARCHERY_BLUE: u8 = 46;
    pub const ARCHERY_RED: u8 = 47;
    // Particles
    pub const DUST: u8 = 48;
    pub const ARROW: u8 = 49;
}

/// Build the texture table for the setup message.
/// Returns (id, http_path) pairs.
pub fn texture_table() -> Vec<(u8, &'static str)> {
    vec![
        (tex::BLUE_WARRIOR_IDLE, "assets/units/blue/warrior/idle.png"),
        (tex::BLUE_WARRIOR_RUN, "assets/units/blue/warrior/run.png"),
        (tex::BLUE_WARRIOR_ATK, "assets/units/blue/warrior/attack.png"),
        (tex::BLUE_ARCHER_IDLE, "assets/units/blue/archer/idle.png"),
        (tex::BLUE_ARCHER_RUN, "assets/units/blue/archer/run.png"),
        (tex::BLUE_ARCHER_ATK, "assets/units/blue/archer/attack.png"),
        (tex::BLUE_LANCER_IDLE, "assets/units/blue/lancer/idle.png"),
        (tex::BLUE_LANCER_RUN, "assets/units/blue/lancer/run.png"),
        (tex::BLUE_LANCER_ATK, "assets/units/blue/lancer/attack.png"),
        (tex::BLUE_MONK_IDLE, "assets/units/blue/monk/idle.png"),
        (tex::BLUE_MONK_RUN, "assets/units/blue/monk/run.png"),
        (tex::BLUE_MONK_ATK, "assets/units/blue/monk/attack.png"),
        (tex::RED_WARRIOR_IDLE, "assets/units/red/warrior/idle.png"),
        (tex::RED_WARRIOR_RUN, "assets/units/red/warrior/run.png"),
        (tex::RED_WARRIOR_ATK, "assets/units/red/warrior/attack.png"),
        (tex::RED_ARCHER_IDLE, "assets/units/red/archer/idle.png"),
        (tex::RED_ARCHER_RUN, "assets/units/red/archer/run.png"),
        (tex::RED_ARCHER_ATK, "assets/units/red/archer/attack.png"),
        (tex::RED_LANCER_IDLE, "assets/units/red/lancer/idle.png"),
        (tex::RED_LANCER_RUN, "assets/units/red/lancer/run.png"),
        (tex::RED_LANCER_ATK, "assets/units/red/lancer/attack.png"),
        (tex::RED_MONK_IDLE, "assets/units/red/monk/idle.png"),
        (tex::RED_MONK_RUN, "assets/units/red/monk/run.png"),
        (tex::RED_MONK_ATK, "assets/units/red/monk/attack.png"),
        (tex::TILEMAP1, "assets/terrain/tileset/tilemap1.png"),
        (tex::TILEMAP2, "assets/terrain/tileset/tilemap2.png"),
        (tex::WATER, "assets/terrain/tileset/water.png"),
        (tex::FOAM, "assets/terrain/tileset/foam.png"),
        (tex::SHADOW, "assets/terrain/tileset/shadow.png"),
        (tex::TREE1, "assets/terrain/trees/tree1.png"),
        (tex::TREE2, "assets/terrain/trees/tree2.png"),
        (tex::TREE3, "assets/terrain/trees/tree3.png"),
        (tex::TREE4, "assets/terrain/trees/tree4.png"),
        (tex::ROCK1, "assets/terrain/rocks/rock1.png"),
        (tex::ROCK2, "assets/terrain/rocks/rock2.png"),
        (tex::ROCK3, "assets/terrain/rocks/rock3.png"),
        (tex::ROCK4, "assets/terrain/rocks/rock4.png"),
        (tex::BUSH1, "assets/terrain/bushes/bush1.png"),
        (tex::BUSH2, "assets/terrain/bushes/bush2.png"),
        (tex::BUSH3, "assets/terrain/bushes/bush3.png"),
        (tex::BUSH4, "assets/terrain/bushes/bush4.png"),
        (tex::TOWER_BLACK, "assets/buildings/black/tower.png"),
        (tex::TOWER_BLUE, "assets/buildings/blue/tower.png"),
        (tex::TOWER_RED, "assets/buildings/red/tower.png"),
        (tex::BARRACKS_BLUE, "assets/buildings/blue/barracks.png"),
        (tex::BARRACKS_RED, "assets/buildings/red/barracks.png"),
        (tex::ARCHERY_BLUE, "assets/buildings/blue/archery.png"),
        (tex::ARCHERY_RED, "assets/buildings/red/archery.png"),
        (tex::DUST, "assets/particles/dust.png"),
        (tex::ARROW, "assets/particles/arrow.png"),
    ]
}

/// Unit animation definition.
#[derive(Clone, Copy)]
pub struct UnitAnim {
    pub texture: u8,
    pub frame_w: u16,
    pub frame_h: u16,
    pub frame_count: u16,
}

/// Get the idle animation for a unit.
pub fn unit_idle(faction: Faction, kind: UnitKind) -> UnitAnim {
    let base = if faction == Faction::Blue { 0 } else { 12 };
    match kind {
        UnitKind::Warrior => UnitAnim { texture: base, frame_w: 192, frame_h: 192, frame_count: 8 },
        UnitKind::Archer => UnitAnim { texture: base + 3, frame_w: 192, frame_h: 192, frame_count: 6 },
        UnitKind::Lancer => UnitAnim { texture: base + 6, frame_w: 320, frame_h: 320, frame_count: 12 },
        UnitKind::Monk => UnitAnim { texture: base + 9, frame_w: 192, frame_h: 192, frame_count: 6 },
    }
}

/// Get the run animation for a unit.
pub fn unit_run(faction: Faction, kind: UnitKind) -> UnitAnim {
    let base = if faction == Faction::Blue { 0 } else { 12 };
    match kind {
        UnitKind::Warrior => UnitAnim { texture: base + 1, frame_w: 192, frame_h: 192, frame_count: 6 },
        UnitKind::Archer => UnitAnim { texture: base + 4, frame_w: 192, frame_h: 192, frame_count: 4 },
        UnitKind::Lancer => UnitAnim { texture: base + 7, frame_w: 320, frame_h: 320, frame_count: 6 },
        UnitKind::Monk => UnitAnim { texture: base + 10, frame_w: 192, frame_h: 192, frame_count: 4 },
    }
}

/// Get the attack animation for a unit.
pub fn unit_attack(faction: Faction, kind: UnitKind) -> UnitAnim {
    let base = if faction == Faction::Blue { 0 } else { 12 };
    match kind {
        UnitKind::Warrior => UnitAnim { texture: base + 2, frame_w: 192, frame_h: 192, frame_count: 4 },
        UnitKind::Archer => UnitAnim { texture: base + 5, frame_w: 192, frame_h: 192, frame_count: 8 },
        UnitKind::Lancer => UnitAnim { texture: base + 8, frame_w: 320, frame_h: 320, frame_count: 3 },
        UnitKind::Monk => UnitAnim { texture: base + 11, frame_w: 192, frame_h: 192, frame_count: 11 },
    }
}
