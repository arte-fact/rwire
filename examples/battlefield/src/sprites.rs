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
    // Extra tilemaps for elevation variety
    pub const TILEMAP3: u8 = 48;
    pub const TILEMAP4: u8 = 49;
    // Monasteries
    pub const MONASTERY_BLUE: u8 = 50;
    pub const MONASTERY_RED: u8 = 51;
    pub const MONASTERY_BLACK: u8 = 52;
    // Water rocks (animated, 16 frames × 64×64)
    pub const WATER_ROCK1: u8 = 53;
    pub const WATER_ROCK2: u8 = 54;
    pub const WATER_ROCK3: u8 = 55;
    pub const WATER_ROCK4: u8 = 56;
    // Particles
    pub const DUST: u8 = 57;
    pub const ARROW: u8 = 58;
    pub const EXPLOSION: u8 = 59;
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
        (tex::WATER_ROCK1, "assets/terrain/water_rocks/wr1.png"),
        (tex::WATER_ROCK2, "assets/terrain/water_rocks/wr2.png"),
        (tex::WATER_ROCK3, "assets/terrain/water_rocks/wr3.png"),
        (tex::WATER_ROCK4, "assets/terrain/water_rocks/wr4.png"),
        (tex::TOWER_BLACK, "assets/buildings/black/tower.png"),
        (tex::TOWER_BLUE, "assets/buildings/blue/tower.png"),
        (tex::TOWER_RED, "assets/buildings/red/tower.png"),
        (tex::BARRACKS_BLUE, "assets/buildings/blue/barracks.png"),
        (tex::BARRACKS_RED, "assets/buildings/red/barracks.png"),
        (tex::ARCHERY_BLUE, "assets/buildings/blue/archery.png"),
        (tex::ARCHERY_RED, "assets/buildings/red/archery.png"),
        (tex::TILEMAP3, "assets/terrain/tileset/tilemap3.png"),
        (tex::TILEMAP4, "assets/terrain/tileset/tilemap4.png"),
        (tex::MONASTERY_BLUE, "assets/buildings/blue/monastery.png"),
        (tex::MONASTERY_RED, "assets/buildings/red/monastery.png"),
        (tex::MONASTERY_BLACK, "assets/buildings/black/monastery.png"),
        (tex::DUST, "assets/particles/dust.png"),
        (tex::ARROW, "assets/particles/arrow.png"),
        (tex::EXPLOSION, "assets/particles/explosion2.png"),
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

/// First sprite ID for decoration animations in the sprite table.
/// Layout: bush[4 variants × 8 frames] + foam[16 frames] + water_rock[4 × 16] + tree[4 × 8]
pub const DECOR_SPRITE_BASE: u16 = 0;

/// Sprite table: (texture_id, sx, sy, sw, sh) for all decoration animation frames.
pub fn decoration_sprite_table() -> Vec<(u8, u16, u16, u16, u16)> {
    let mut sprites = Vec::new();

    // Bush variants: 4 textures × 8 frames, each 128×128
    for v in 0..4u8 {
        for f in 0..8u16 {
            sprites.push((tex::BUSH1 + v, f * 128, 0, 128, 128));
        }
    }

    // Foam: 1 texture × 16 frames, each 192×192
    for f in 0..16u16 {
        sprites.push((tex::FOAM, f * 192, 0, 192, 192));
    }

    // Water rock variants: 4 textures × 16 frames, each 64×64
    for v in 0..4u8 {
        for f in 0..16u16 {
            sprites.push((tex::WATER_ROCK1 + v, f * 64, 0, 64, 64));
        }
    }

    // Tree variants: tree1,2 = 192×256 (8 frames), tree3,4 = 192×192 (8 frames)
    for v in 0..4u8 {
        let fh = if v < 2 { 256u16 } else { 192 };
        for f in 0..8u16 {
            sprites.push((tex::TREE1 + v, f * 192, 0, 192, fh));
        }
    }

    sprites
}

/// Sprite ID offsets for each decoration type.
pub const BUSH_SPRITE_BASE: u16 = DECOR_SPRITE_BASE; // 4 variants × 8 frames = 32
pub const FOAM_SPRITE_BASE: u16 = BUSH_SPRITE_BASE + 32; // 16 frames
pub const WATER_ROCK_SPRITE_BASE: u16 = FOAM_SPRITE_BASE + 16; // 4 variants × 16 frames = 64
pub const TREE_SPRITE_BASE: u16 = WATER_ROCK_SPRITE_BASE + 64; // 4 variants × 8 frames = 32

/// Get the first sprite ID for a bush variant (0-3). 8 frames per variant.
pub fn bush_first_sprite(variant: u8) -> u16 {
    BUSH_SPRITE_BASE + variant as u16 * 8
}

/// Get the first sprite ID for foam. 16 frames.
pub fn foam_first_sprite() -> u16 {
    FOAM_SPRITE_BASE
}

/// Get the first sprite ID for a water rock variant (0-3). 16 frames per variant.
pub fn water_rock_first_sprite(variant: u8) -> u16 {
    WATER_ROCK_SPRITE_BASE + variant as u16 * 16
}

/// Get the first sprite ID for a tree variant (0-3). 8 frames per variant.
pub fn tree_first_sprite(variant: u8) -> u16 {
    TREE_SPRITE_BASE + variant as u16 * 8
}

/// Unit sprite base — after all decoration sprites.
pub const UNIT_SPRITE_BASE: u16 = TREE_SPRITE_BASE + 32; // = 144

/// Add unit animation frames to sprite table.
/// Layout per unit type: idle frames + run frames + attack frames.
/// Returns (sprite_rects, unit_anim_offsets).
pub fn unit_sprite_table() -> Vec<(u8, u16, u16, u16, u16)> {
    let mut sprites = Vec::new();

    // For each faction × kind × anim: register all frames
    let anims = [
        // (texture_base_blue, texture_base_red, frame_w, frame_h, [(anim_type, frame_count)])
        // Warrior: idle=8, run=6, attack=4
        (0u8, 12u8, 192u16, 192u16, &[(8u16), (6), (4)] as &[u16]),
        // Archer: idle=6, run=4, attack=8
        (3, 15, 192, 192, &[6, 4, 8]),
        // Lancer: idle=12, run=6, attack=3
        (6, 18, 320, 320, &[12, 6, 3]),
        // Monk: idle=6, run=4, attack=11
        (9, 21, 192, 192, &[6, 4, 11]),
    ];

    for &(blue_base, red_base, fw, fh, frame_counts) in &anims {
        for &tex_base in &[blue_base, red_base] {
            for (anim_idx, &fc) in frame_counts.iter().enumerate() {
                let tex_id = tex_base + anim_idx as u8;
                for f in 0..fc {
                    sprites.push((tex_id, f * fw, 0, fw, fh));
                }
            }
        }
    }

    sprites
}

/// Get sprite ID for a unit animation frame.
/// kind: 0=warrior, 1=archer, 2=lancer, 3=monk
/// faction: 0=blue, 1=red
/// anim: 0=idle, 1=run, 2=attack
/// frame: animation frame index
pub fn unit_sprite_id(kind: UnitKind, faction: Faction, anim: u8, frame: u16) -> u16 {
    let kind_idx = match kind {
        UnitKind::Warrior => 0u16,
        UnitKind::Archer => 1,
        UnitKind::Lancer => 2,
        UnitKind::Monk => 3,
    };
    let faction_idx = if faction == Faction::Blue { 0u16 } else { 1 };

    // Frame counts per kind: [idle, run, attack]
    let frame_counts: &[u16] = match kind {
        UnitKind::Warrior => &[8, 6, 4],
        UnitKind::Archer => &[6, 4, 8],
        UnitKind::Lancer => &[12, 6, 3],
        UnitKind::Monk => &[6, 4, 11],
    };

    // Offset: sum of all prior kinds × 2 factions × all their frames
    let mut offset = UNIT_SPRITE_BASE;
    let all_frame_counts: &[&[u16]] = &[
        &[8, 6, 4],  // warrior
        &[6, 4, 8],  // archer
        &[12, 6, 3], // lancer
        &[6, 4, 11], // monk
    ];
    for k in 0..kind_idx as usize {
        let total: u16 = all_frame_counts[k].iter().sum();
        offset += total * 2; // × 2 factions
    }
    // Add faction offset
    if faction_idx > 0 {
        let total: u16 = frame_counts.iter().sum();
        offset += total;
    }
    // Add anim offset
    for a in 0..anim as usize {
        offset += frame_counts[a];
    }
    offset + frame
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
