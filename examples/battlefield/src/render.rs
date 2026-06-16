//! Server-side rendering: GameState → CanvasBuffer draw commands.

use crate::autotile;
use crate::grid::{Decoration, GRID_SIZE, TILE_SIZE, TileType};
use crate::particle::ParticleKind;
use crate::sprites::tex;
use crate::state::{BuildingKind, GamePhase, GameState};
use crate::unit::Faction;
use rwire_canvas::CanvasBuffer;

/// Fallback canvas dimensions (used before client reports viewport).
const CW: u16 = 960;
const CH: u16 = 640;

/// Get viewport dimensions from state, with fallback.
fn vp(state: &GameState) -> (u16, u16) {
    let w = if state.viewport_w > 0.0 { state.viewport_w as u16 } else { CW };
    let h = if state.viewport_h > 0.0 { state.viewport_h as u16 } else { CH };
    (w, h)
}
const TS: f32 = TILE_SIZE;

/// Layer IDs for cached terrain (water below, land above, foam drawn between them).
const LAYER_WATER: u8 = 0;
const LAYER_LAND: u8 = 1;

/// Flags: cacheable + visible + world-space.
const LAYER_FLAGS_CACHED_WORLD: u8 = 0x01 | 0x02 | 0x04;

/// Check if a land tile is adjacent to water (cardinal directions).
fn is_water_adjacent(grid: &crate::grid::Grid, tx: usize, ty: usize) -> bool {
    [(0i32, -1), (0, 1), (-1, 0), (1, 0)].iter().any(|&(ddx, ddy)| {
        let nx = tx as i32 + ddx;
        let ny = ty as i32 + ddy;
        if nx < 0 || ny < 0 { return false; }
        grid.get(nx as usize, ny as usize) == TileType::Water
    })
}

/// Send static terrain into two cached layers (called once at setup).
/// LAYER_WATER: water tiles + background (drawn first)
/// LAYER_LAND: grass/forest/rock autotiling + elevation + decorations + borders
/// Foam is drawn per-frame between these two layers.
pub fn send_terrain_layer(state: &GameState, buf: &mut CanvasBuffer) {
    let ts = TS as u16;
    let world_px = (GRID_SIZE as f32 * TS) as u16;

    // === LAYER_WATER: water background ===
    buf.layer_create(LAYER_WATER, LAYER_FLAGS_CACHED_WORLD);
    buf.layer_target(LAYER_WATER);

    // Dark background beyond world edges
    buf.set_fill_rgb(15, 15, 25);
    let margin = 1000u16;
    buf.fill_rect(-(margin as i16), -(margin as i16), world_px + margin * 2, world_px + margin * 2);

    // Water on water tiles AND land tiles adjacent to water
    for ty in 0..GRID_SIZE {
        for tx in 0..GRID_SIZE {
            let is_water = state.grid.get(tx, ty) == TileType::Water;
            if !is_water && !is_water_adjacent(&state.grid, tx, ty) { continue; }
            let dx = (tx as f32 * TS) as i16;
            let dy = (ty as f32 * TS) as i16;
            buf.draw_image(tex::WATER, 0, 0, 64, 64, dx, dy, ts + 1, ts + 1);
        }
    }
    buf.layer_target_main();

    // === LAYER_LAND: grass, forest, elevation, rocks (transparent bg so water/foam show through) ===
    buf.layer_create(LAYER_LAND, LAYER_FLAGS_CACHED_WORLD);
    buf.layer_target(LAYER_LAND);

    // Terrain (autotiled) — transparent background lets water show at edges
    for ty in 0..GRID_SIZE {
        for tx in 0..GRID_SIZE {
            let tile = state.grid.get(tx, ty);
            if tile == TileType::Water { continue; }
            let dx = (tx as f32 * TS) as i16;
            let dy = (ty as f32 * TS) as i16;
            let draw_ts = ts + 1;
            match tile {
                TileType::Grass | TileType::Forest => {
                    let (sx, sy) = autotile::flat_ground_src(&state.grid, tx, ty);
                    let is_center = sx == 64 && sy == 64;
                    let flip = is_center && state.tile_rng.flip(tx, ty);
                    if flip {
                        buf.save();
                        buf.translate(dx + draw_ts as i16, dy);
                        buf.scale(-256, 256);
                        buf.draw_image(tex::TILEMAP1, sx, sy, 64, 64, 0, 0, draw_ts, draw_ts);
                        buf.restore();
                    } else {
                        buf.draw_image(tex::TILEMAP1, sx, sy, 64, 64, dx, dy, draw_ts, draw_ts);
                    }
                    // Forest tint removed — on transparent layer canvas the fill_rect
                    // creates visible dark squares at autotile edges. Trees provide
                    // enough visual darkening.
                }
                TileType::Rock => {
                    let (sx, sy) = autotile::flat_ground_src(&state.grid, tx, ty);
                    buf.draw_image(tex::TILEMAP1, sx, sy, 64, 64, dx, dy, draw_ts, draw_ts);
                }
                _ => {}
            }
        }
    }

    // Elevation — draw grass base first, then elevated surface on top
    for ty in 0..GRID_SIZE {
        for tx in 0..GRID_SIZE {
            if state.grid.elev(tx, ty) == 0 { continue; }
            let dx = (tx as f32 * TS) as i16;
            let dy = (ty as f32 * TS) as i16;
            let draw_ts = ts + 1;

            // Grass base under elevated tile (fills transparent areas in elevation sprite)
            let (gsx, gsy) = autotile::flat_ground_src(&state.grid, tx, ty);
            buf.draw_image(tex::TILEMAP1, gsx, gsy, 64, 64, dx, dy, draw_ts, draw_ts);

            if ty + 1 < GRID_SIZE && state.grid.elev(tx, ty + 1) == 0 {
                buf.set_alpha(127);
                let sdx = (tx as f32 * TS + TS / 2.0 - 96.0) as i16;
                let sdy = ((ty + 1) as f32 * TS + TS / 2.0 - 96.0) as i16;
                buf.draw_image(tex::SHADOW, 0, 0, 192, 192, sdx, sdy, 192, 192);
                buf.set_alpha(255);
            }
            let (sx, sy) = autotile::elevated_src(&state.grid, tx, ty);
            buf.draw_image(tex::TILEMAP1, sx, sy, 64, 64, dx, dy, draw_ts, draw_ts);
            if ty + 1 < GRID_SIZE && state.grid.elev(tx, ty + 1) == 0 {
                let (csx, csy) = autotile::cliff_src(&state.grid, tx, ty + 1);
                let cdx = (tx as f32 * TS) as i16;
                let cdy = ((ty + 1) as f32 * TS) as i16;
                buf.draw_image(tex::TILEMAP1, csx, csy, 64, 64, cdx, cdy, draw_ts, draw_ts);
            }
        }
    }

    // Static decorations: rocks at 1:1 (skip elevated tiles)
    for ty in 0..GRID_SIZE {
        for tx in 0..GRID_SIZE {
            if state.grid.elev(tx, ty) > 0 { continue; }
            let dx = (tx as f32 * TS) as i16;
            let dy = (ty as f32 * TS) as i16;
            if state.grid.get(tx, ty) == TileType::Rock {
                let v = state.tile_rng.variant(tx, ty, 4);
                buf.draw_image(tex::ROCK1 + v, 0, 0, 64, 64, dx, dy, 64, 64);
            }
            // Water rocks moved to per-frame overlay (animated)
        }
    }

    buf.layer_target_main();
}

/// Decoration layers for SPRITE_ANIM sprites — separate for z-ordering.
const LAYER_FOAM: u8 = 2;    // foam + water rocks: between water and land
const LAYER_BUSHES: u8 = 3;  // bushes: above land, behind units
const LAYER_TREES: u8 = 4;   // trees: ABOVE units, with player-proximity alpha fade

/// Send all animated decoration sprites (bushes, foam, water rocks, trees).
/// These are SPRITE_ANIM — the client auto-animates them. Zero per-frame cost.
pub fn send_decoration_sprites(state: &mut GameState, buf: &mut CanvasBuffer) {
    use crate::sprites;

    // Decoration layers + unit layer
    buf.layer_create(LAYER_FOAM, 0x02 | 0x04);
    buf.layer_create(LAYER_BUSHES, 0x02 | 0x04);
    buf.layer_create(LAYER_TREES, 0x02 | 0x04);
    buf.layer_create(LAYER_UNITS, 0x02 | 0x04);

    let mut next_id: u16 = 0;
    let flag_vis: u8 = 0x02;
    let flag_wave: u8 = 0x04;

    // Foam: between water and land
    for ty in 0..GRID_SIZE {
        for tx in 0..GRID_SIZE {
            if state.grid.get(tx, ty) == TileType::Water { continue; }
            if !is_water_adjacent(&state.grid, tx, ty) { continue; }
            let phase = state.tile_rng.phase_offset(tx, ty, 255) as u8;
            let dx = (tx as f32 * TS + TS / 2.0 - 96.0) as i16;
            let dy = (ty as f32 * TS + TS / 2.0 - 96.0) as i16;
            buf.sprite_anim(next_id, LAYER_FOAM, sprites::foam_first_sprite(), 16, 8, phase, dx, dy, flag_vis | flag_wave);
            next_id += 1;
        }
    }

    // Water rocks: also between water and land
    for ty in 0..GRID_SIZE {
        for tx in 0..GRID_SIZE {
            if state.grid.decoration(tx, ty) != Some(Decoration::WaterRock) { continue; }
            let v = state.tile_rng.variant(tx, ty, 4);
            let phase = state.tile_rng.phase_offset(tx, ty, 255) as u8;
            let dx = (tx as f32 * TS) as i16;
            let dy = (ty as f32 * TS) as i16;
            let mut flags = flag_vis | flag_wave;
            if state.tile_rng.flip(tx, ty) { flags |= 0x01; }
            buf.sprite_anim(next_id, LAYER_FOAM, sprites::water_rock_first_sprite(v), 16, 10, phase, dx, dy, flags);
            next_id += 1;
        }
    }

    // Bushes: above land, behind units
    for ty in 0..GRID_SIZE {
        for tx in 0..GRID_SIZE {
            if state.grid.decoration(tx, ty) != Some(Decoration::Bush) { continue; }
            let v = state.tile_rng.variant(tx, ty, 4);
            let phase = state.tile_rng.phase_offset(tx, ty, 255) as u8;
            let dx = (tx as f32 * TS + TS / 2.0 - 64.0) as i16;
            let dy = (ty as f32 * TS + TS / 2.0 - 64.0) as i16;
            let mut flags = flag_vis | flag_wave;
            if state.tile_rng.flip(tx, ty) { flags |= 0x01; }
            buf.sprite_anim(next_id, LAYER_BUSHES, sprites::bush_first_sprite(v), 8, 10, phase, dx, dy, flags);
            next_id += 1;
        }
    }

    // Trees: ABOVE units (every forest tile). Record IDs for per-frame alpha updates.
    for ty in 0..GRID_SIZE {
        for tx in 0..GRID_SIZE {
            if state.grid.get(tx, ty) != TileType::Forest { continue; }
            let v = state.tile_rng.variant(tx, ty, 4);
            let (fw, fh): (f32, f32) = if v < 2 { (192.0, 256.0) } else { (192.0, 192.0) };
            let phase = state.tile_rng.phase_offset(tx, ty, 255) as u8;
            let draw_w = TS * 3.0;
            let draw_h = draw_w * (fh / fw);
            let dx = (tx as f32 * TS + TS / 2.0 - draw_w / 2.0) as i16;
            let dy = ((ty + 1) as f32 * TS - draw_h) as i16;
            let tree_cx = tx as f32 * TS + TS / 2.0;
            let tree_cy = ty as f32 * TS + TS / 2.0;
            state.tree_sprites.push((next_id, tree_cx, tree_cy));
            buf.sprite_anim(next_id, LAYER_TREES, sprites::tree_first_sprite(v), 8, 10, phase, dx, dy, flag_vis | flag_wave);
            next_id += 1;
        }
    }
}

/// Layer for retained unit sprites.
const LAYER_UNITS: u8 = 5;

/// Sprite ID range for units (offset to avoid collision with decoration sprite IDs).
/// Decoration sprites use IDs 0..N (assigned in send_decoration_sprites).
/// Unit sprites use IDs 10000+ (keyed by unit game ID).
const UNIT_SPRITE_ID_BASE: u16 = 10000;

/// Update the scene graph with current unit states.
/// The diff engine computes minimal SPRITE_CREATE/UPDATE/DELETE opcodes.
pub fn update_unit_sprites(state: &GameState, scene: &mut rwire_canvas::Scene) {
    use crate::sprites;
    use crate::unit::{UnitAnim, DEATH_FADE_DURATION};

    let mut alive_ids = std::collections::HashSet::new();

    for u in &state.units {
        if !u.alive && u.death_fade >= DEATH_FADE_DURATION { continue; }
        // Skip red units not visible (fog of war)
        if u.faction == crate::unit::Faction::Red {
            let visible = state.units.iter()
                .filter(|f| f.alive && f.faction == crate::unit::Faction::Blue)
                .any(|f| {
                    let d = ((f.x - u.x).powi(2) + (f.y - u.y).powi(2)).sqrt() / TILE_SIZE;
                    d < 15.0
                });
            if !visible { continue; }
        }

        let sprite_id = UNIT_SPRITE_ID_BASE + u.id as u16;
        alive_ids.insert(sprite_id);

        let anim_idx: u8 = match u.anim {
            UnitAnim::Idle => 0,
            UnitAnim::Run => 1,
            UnitAnim::Attack => 2,
        };
        let frame_sprite = sprites::unit_sprite_id(u.kind, u.faction, anim_idx, u.anim_frame);

        let mut flags: u8 = 0x02; // visible
        if u.facing == crate::unit::Facing::Left { flags |= 0x01; }

        let x = u.x as i16;
        let y = u.y as i16;

        let alpha = if !u.alive {
            let fade = 1.0 - (u.death_fade / DEATH_FADE_DURATION);
            (fade * 255.0) as u8
        } else if u.hit_flash > 0.0 {
            let flash_frame = (u.hit_flash * 30.0) as i32;
            if flash_frame % 2 == 0 { 77 } else { 255 }
        } else {
            255
        };

        // Create or update
        if scene.get(sprite_id).is_some() {
            scene.update(sprite_id, x, y, frame_sprite, flags);
        } else {
            scene.create_with_id(sprite_id, LAYER_UNITS, frame_sprite, x, y, flags);
        }
        scene.set_alpha(sprite_id, alpha);
    }

    // Delete sprites for units no longer visible
    let to_delete: Vec<u16> = scene.sprites.keys().copied()
        .filter(|id| *id >= UNIT_SPRITE_ID_BASE && !alive_ids.contains(id))
        .collect();
    for id in to_delete {
        scene.delete(id);
    }
}

/// Per-frame overlay: foreground entities, fog, HUD (everything except cached terrain).
pub fn render_overlay(state: &GameState, buf: &mut CanvasBuffer) {
    let (vw, vh) = vp(state);
    let cx = state.camera_x;
    let cy = state.camera_y;
    let zoom = state.camera_zoom;

    // World-space content (single camera transform for all layers + entities)
    buf.save();
    let offset_x = (vw as f32 / 2.0 - zoom * cx).round() as i16;
    let offset_y = (vh as f32 / 2.0 - zoom * cy).round() as i16;
    buf.translate(offset_x, offset_y);
    buf.scale_uniform(zoom);

    // Z-order: water → foam/water rocks → land → bushes → zones → entities → trees (with player alpha fade) → fog
    buf.layer_draw(LAYER_WATER);
    buf.draw_anim_sprites(LAYER_FOAM);
    buf.layer_draw(LAYER_LAND);
    buf.draw_anim_sprites(LAYER_BUSHES);

    render_zones(state, buf);
    render_aim_cone(state, buf);

    // Player highlight (under unit sprite)
    if let Some(p) = state.units.iter().find(|u| u.id == state.player_id && u.alive) {
        buf.set_stroke_rgba(255, 215, 0, 200);
        buf.set_line_width(8);
        buf.begin_path();
        buf.arc_full(p.x as i16, p.y as i16 + 12, 18);
        buf.stroke();
    }

    // Units as retained sprites (diff'd by scene engine) + buildings/projectiles/particles (immediate)
    buf.draw_sprites(LAYER_UNITS);
    render_foreground(state, buf, cx, cy, zoom);

    // Trees ON TOP of units — with alpha fade near player
    update_tree_alpha(state, buf);
    buf.draw_anim_sprites(LAYER_TREES);
    render_hp_bars(state, buf);
    render_order_labels(state, buf);
    render_fog(state, buf, cx, cy);

    buf.restore();

    // Screen-space overlay
    render_minimap(state, buf);
    render_hud(state, buf);
    render_victory_progress(state, buf);

    let (sw, sh) = vp(state);
    match &state.phase {
        GamePhase::Menu => render_menu(sw, sh, buf),
        GamePhase::Victory(f) => render_result(sw, sh, buf, *f),
        GamePhase::Dead => render_death(sw, sh, buf),
        GamePhase::Playing => {}
    }
}

fn vis_zoom(vw: f32, vh: f32, cx: f32, cy: f32, zoom: f32) -> (usize, usize, usize, usize) {
    let half_w = vw / (2.0 * zoom) + TS * 3.0;
    let half_h = vh / (2.0 * zoom) + TS * 3.0;
    let stx = (((cx - half_w) / TS).floor() as i32).max(0) as usize;
    let sty = (((cy - half_h) / TS).floor() as i32).max(0) as usize;
    let etx = (((cx + half_w) / TS).ceil() as i32 + 1).min(GRID_SIZE as i32) as usize;
    let ety = (((cy + half_h) / TS).ceil() as i32 + 1).min(GRID_SIZE as i32) as usize;
    (stx, sty, etx, ety)
}

fn vis(vw: f32, vh: f32, cx: f32, cy: f32) -> (usize, usize, usize, usize) {
    vis_zoom(vw, vh, cx, cy, 0.5)
}

/// Send SPRITE_ALPHA updates for trees near the player (transparency fade).
/// Also resets trees just outside the fade zone to full opacity.
fn update_tree_alpha(state: &GameState, buf: &mut CanvasBuffer) {
    let player = match state.units.iter().find(|u| u.id == state.player_id && u.alive) {
        Some(p) => p,
        None => return,
    };
    let px = player.x;
    let py = player.y;
    let fade_start = TS * 2.5;
    let fade_end = TS * 1.0;
    let reset_dist = fade_start + TS; // reset alpha for trees just outside fade zone

    for &(sprite_id, tree_cx, tree_cy) in &state.tree_sprites {
        let dist = ((px - tree_cx).powi(2) + (py - tree_cy).powi(2)).sqrt();
        if dist < fade_start {
            let t = ((dist - fade_end) / (fade_start - fade_end)).clamp(0.0, 1.0);
            let alpha = ((0.3 + t * 0.7) * 255.0) as u8;
            buf.sprite_alpha(sprite_id, alpha);
        } else if dist < reset_dist {
            buf.sprite_alpha(sprite_id, 255);
        }
    }
}

fn render_aim_cone(state: &GameState, buf: &mut CanvasBuffer) {
    let player = match state.units.iter().find(|u| u.id == state.player_id && u.alive) {
        Some(p) => p,
        None => return,
    };
    if state.phase != GamePhase::Playing { return; }

    let px = player.x as i16;
    let py = player.y as i16;

    // Position indicator circle (matching original: 24px radius, yellow 20% alpha)
    buf.set_fill_rgba(255, 255, 51, 51); // rgba(255,255,51,0.2)
    buf.begin_path();
    buf.arc_full(px, py, 24);
    buf.fill();

    // Aim cone wedge (matching original: 40px radius, PI/3 half-angle)
    let aim_angle = state.aim_y.atan2(state.aim_x);
    let half_cone = std::f32::consts::FRAC_PI_3;
    let radius: u16 = 40;

    buf.set_fill_rgba(255, 255, 100, 30); // rgba(255,255,100,0.12)
    buf.begin_path();
    buf.move_to(px, py);
    buf.arc(px, py, radius, aim_angle - half_cone, aim_angle + half_cone);
    buf.close_path();
    buf.fill();

    buf.set_stroke_rgba(255, 255, 100, 89); // rgba(255,255,100,0.35)
    buf.set_line_width(4); // 1.0 at native
    buf.begin_path();
    buf.move_to(px, py);
    buf.arc(px, py, radius, aim_angle - half_cone, aim_angle + half_cone);
    buf.close_path();
    buf.stroke();
}

fn render_foreground(state: &GameState, buf: &mut CanvasBuffer, cx: f32, cy: f32, zoom: f32) {
    // Units are now retained sprites (drawn by DRAW_SPRITES).
    // Only buildings, projectiles, particles remain in immediate-mode.
    enum D { Building(usize), Projectile(usize), Particle(usize) }
    let mut items: Vec<(f32, D)> = Vec::new();

    for (i, p) in state.projectiles.iter().enumerate() {
        let cur_y = p.start_y + (p.target_y - p.start_y) * p.progress;
        items.push((cur_y, D::Projectile(i)));
    }

    for (i, p) in state.particles.iter().enumerate() {
        items.push((p.y + TS * 0.5, D::Particle(i)));
    }

    let (vw, vh) = vp(state);
    let hvw = vw as f32 / (2.0 * zoom) + 200.0;
    let hvh = vh as f32 / (2.0 * zoom) + 200.0;

    for (i, b) in state.buildings.iter().enumerate() {
        if (b.x - cx).abs() < hvw + 100.0 && (b.y - cy).abs() < hvh + 100.0 {
            items.push((b.y + TS, D::Building(i)));
        }
    }

    items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    for (_, item) in items {
        match item {
            D::Building(i) => draw_building(state, buf, i),
            D::Projectile(i) => draw_projectile(state, buf, i),
            D::Particle(i) => draw_particle(state, buf, i),
        }
    }
}

fn draw_projectile(_state: &GameState, buf: &mut CanvasBuffer, idx: usize) {
    let p = &_state.projectiles[idx];
    let t = p.progress;

    // Interpolate position along trajectory
    let cur_x = p.start_x + (p.target_x - p.start_x) * t;
    let cur_y = p.start_y + (p.target_y - p.start_y) * t;

    // Parabolic arc: z = arc_height * 4 * t * (1 - t), peaks at t=0.5
    let arc_y = cur_y - p.arc_height * 4.0 * t * (1.0 - t);

    // Rotation from tangent of arc trajectory
    let dx = p.target_x - p.start_x;
    let dy = p.target_y - p.start_y;
    // Vertical component includes arc derivative: -arc_height * 4 * (1 - 2t)
    let arc_slope = -p.arc_height * 4.0 * (1.0 - 2.0 * t);
    let angle = (dy + arc_slope).atan2(dx);

    buf.save();
    buf.translate(cur_x as i16, arc_y as i16);
    buf.rotate(angle);
    buf.draw_image(tex::ARROW, 0, 0, 64, 64, -32, -32, 64, 64);
    buf.restore();
}

fn draw_particle(state: &GameState, buf: &mut CanvasBuffer, idx: usize) {
    let p = &state.particles[idx];
    let (tex_id, frame_size) = match p.kind {
        ParticleKind::Dust => (tex::DUST, 64u16),
        ParticleKind::ExplosionLarge => (tex::EXPLOSION, 192),
    };
    let sx = p.frame * frame_size;
    let half = frame_size as i16 / 2;
    buf.draw_image(tex_id, sx, 0, frame_size, frame_size,
        p.x as i16 - half, p.y as i16 - half, frame_size, frame_size);
}

fn draw_building(state: &GameState, buf: &mut CanvasBuffer, idx: usize) {
    let b = &state.buildings[idx];
    let (tex_id, sw, sh) = match (b.faction, b.kind) {
        (Faction::Blue, BuildingKind::Barracks) => (tex::BARRACKS_BLUE, 192u16, 256u16),
        (Faction::Red, BuildingKind::Barracks) => (tex::BARRACKS_RED, 192, 256),
        (Faction::Blue, BuildingKind::Archery) => (tex::ARCHERY_BLUE, 192, 256),
        (Faction::Red, BuildingKind::Archery) => (tex::ARCHERY_RED, 192, 256),
        (Faction::Blue, BuildingKind::Monastery) => (tex::MONASTERY_BLUE, 192, 256),
        (Faction::Red, BuildingKind::Monastery) => (tex::MONASTERY_RED, 192, 256),
    };
    // Draw at 1:1 source size (ts*3 width = 192 = source width), bottom-aligned
    let dw = (TS * 3.0) as u16; // 192 = source width
    let dh = (dw as f32 * (sh as f32 / sw as f32)) as u16; // proportional height
    let dx = b.x as i16 - dw as i16 / 2;
    let dy = b.y as i16 - dh as i16 + (TS * 0.5) as i16;
    buf.draw_image(tex_id, 0, 0, sw, sh, dx, dy, dw, dh);
}

fn render_zones(state: &GameState, buf: &mut CanvasBuffer) {
    for zone in &state.zones {
        let zcx = zone.cx as i16;
        let zcy = zone.cy as i16;
        let r = (zone.radius * TS) as u16;

        // Exact zone colors from original (very low fill opacity)
        let (zr, zg, zb, za) = match zone.owner {
            Some(Faction::Blue) => (60, 120, 255, 30),  // 0.12 * 255
            Some(Faction::Red) => (255, 60, 60, 30),
            None => (200, 200, 200, 15),  // 0.06 * 255
        };
        buf.set_fill_rgba(zr, zg, zb, za);
        buf.begin_path().arc_full(zcx, zcy, r).fill();

        let (sr, sg, sb, sa) = match zone.owner {
            Some(Faction::Blue) => (60, 120, 255, 127),  // 0.5
            Some(Faction::Red) => (255, 60, 60, 127),
            None => (200, 200, 200, 64),  // 0.25
        };
        buf.set_stroke_rgba(sr, sg, sb, sa);
        buf.set_line_width(8);
        buf.set_line_dash(&[8, 4]);
        buf.begin_path().arc_full(zcx, zcy, r).stroke();
        buf.clear_line_dash();

        // Zone label (matching original: zone name like "Zone A")
        buf.set_fill_rgba(255, 255, 255, 180);
        buf.set_font(1);
        buf.set_text_align(1);
        buf.set_text_baseline(2);
        buf.fill_text(zcx, zcy - r as i16 - 22, zone.name);

        // Zone tower building
        let tower_tex = match zone.owner {
            Some(Faction::Blue) => tex::TOWER_BLUE,
            Some(Faction::Red) => tex::TOWER_RED,
            None => tex::TOWER_BLACK,
        };
        // Tower: 128×256 source, draw at 2×4 tiles (matching original)
        let tw = (TS * 2.0) as u16;
        let th = (TS * 4.0) as u16;
        let tdx = zcx - tw as i16 / 2;
        let tdy = zcy - th as i16 + (TS * 0.5) as i16;
        // Tower alpha: pulse based on capture progress (matching original)
        let tower_alpha = if zone.owner.is_none() {
            let a = (zone.progress.abs() * 0.5 + 0.5).clamp(0.5, 1.0);
            (a * 255.0) as u8
        } else {
            255
        };
        buf.set_alpha(tower_alpha);
        buf.draw_image(tower_tex, 0, 0, 128, 256, tdx, tdy, tw, th);
        buf.set_alpha(255);

        // Progress bar
        let bw: u16 = 60;
        let bh: u16 = 6;
        let bx = zcx - bw as i16 / 2;
        let by = tdy - 12;
        buf.set_fill_rgba(0, 0, 0, 100);
        buf.fill_rect(bx, by, bw, bh);
        if zone.progress > 0.01 {
            buf.set_fill_rgba(60, 120, 255, 200);
            buf.fill_rect(bx + bw as i16 / 2, by, (zone.progress * bw as f32 / 2.0) as u16, bh);
        } else if zone.progress < -0.01 {
            buf.set_fill_rgba(255, 60, 60, 200);
            let fw = (-zone.progress * bw as f32 / 2.0) as u16;
            buf.fill_rect(bx + bw as i16 / 2 - fw as i16, by, fw, bh);
        }
    }
}

fn render_hp_bars(state: &GameState, buf: &mut CanvasBuffer) {
    let bw: u16 = 48;
    let bh: u16 = 6;

    for u in &state.units {
        if !u.alive || u.hp >= u.kind.max_hp() { continue; }

        // Position: 0.85 tiles above unit center (matching original)
        let bx = u.x as i16 - bw as i16 / 2;
        let by = u.y as i16 - (TS * 0.85) as i16;
        let ratio = u.hp as f32 / u.kind.max_hp() as f32;

        // Background (80% alpha, matching original)
        buf.set_alpha(204);
        buf.set_fill_rgb(51, 51, 51);
        buf.fill_rect(bx, by - bh as i16 / 2, bw, bh);

        // Health fill with 2px inset (matching original: bar_height - 2)
        let (cr, cg, cb) = if ratio > 0.5 {
            (51, 204, 51)
        } else if ratio > 0.25 {
            (230, 179, 26)
        } else {
            (230, 51, 26)
        };
        buf.set_alpha(230);
        buf.set_fill_rgb(cr, cg, cb);
        let fill_h = bh - 2; // 2px inset
        buf.fill_rect(bx, by - fill_h as i16 / 2, (ratio * bw as f32) as u16, fill_h);
        buf.set_alpha(255);
    }
}

fn render_order_labels(state: &GameState, buf: &mut CanvasBuffer) {
    for u in &state.units {
        if !u.alive || u.order_flash <= 0.0 { continue; }
        if let Some(order) = u.order {
            let label = match order {
                crate::unit::OrderKind::Hold => "HOLD",
                crate::unit::OrderKind::Go => "GO",
                crate::unit::OrderKind::Retreat => "RETREAT",
                crate::unit::OrderKind::Follow => "FOLLOW",
            };
            let alpha = (u.order_flash * 255.0).min(255.0) as u8;
            // Gold text with black outline (matching original)
            buf.set_font(0); // bold 14px sans-serif
            buf.set_text_align(1); // center
            buf.set_text_baseline(2); // bottom
            let oy = u.y as i16 - (TS * 1.0) as i16;
            buf.set_stroke_rgba(0, 0, 0, (alpha as f32 * 0.9) as u8);
            buf.set_line_width(12); // 3px
            buf.stroke_text(u.x as i16, oy, label);
            buf.set_fill_rgba(255, 215, 0, alpha);
            buf.fill_text(u.x as i16, oy, label);
        }
    }
}

fn render_fog(state: &GameState, buf: &mut CanvasBuffer, cx: f32, cy: f32) {
    let (vw, vh) = vp(state);
    let (stx, sty, etx, ety) = vis(vw as f32, vh as f32, cx, cy);
    let fog_w = etx - stx;
    let fog_h = ety - sty;
    if fog_w == 0 || fog_h == 0 { return; }

    // Collect all friendly unit positions
    let friendly_positions: Vec<(f32, f32)> = state.units.iter()
        .filter(|u| u.alive && u.faction == Faction::Blue)
        .map(|u| (u.x, u.y))
        .collect();

    let fov = 15.0f32;
    let soft_edge = 4.0f32;

    // Build fog alpha grid (1 byte per tile)
    let mut alphas = vec![0u8; fog_w * fog_h];
    for gy in 0..fog_h {
        for gx in 0..fog_w {
            let tx = stx + gx;
            let ty = sty + gy;
            let wx = tx as f32 * TS + TS / 2.0;
            let wy = ty as f32 * TS + TS / 2.0;

            let min_dist = friendly_positions.iter()
                .map(|&(px, py)| ((wx - px).powi(2) + (wy - py).powi(2)).sqrt() / TS)
                .fold(f32::MAX, f32::min);

            let alpha = if min_dist > fov {
                180u8 // fully fogged
            } else if min_dist > fov - soft_edge {
                let t = (min_dist - (fov - soft_edge)) / soft_edge;
                (t * t * 160.0) as u8
            } else {
                // Check for soft edge near fog (like original: fog neighbors affect visible tiles)
                let vis_edge = fov - soft_edge - 1.0;
                if min_dist > vis_edge {
                    let t = (min_dist - vis_edge) / 1.0;
                    (t * 20.0) as u8 // very subtle darkening at far vision edge
                } else {
                    0
                }
            };
            alphas[gy * fog_w + gx] = alpha;
        }
    }

    // Send as FOG_GRID — rendered with bilinear smoothing on client
    buf.fog_grid(
        (stx as f32 * TS) as i16,
        (sty as f32 * TS) as i16,
        fog_w as u16,
        fog_h as u16,
        TS as u8,
        &alphas,
    );
}

/// Send the minimap terrain image once at connection start.
/// This generates an 80×80 RGB pixel buffer from the 160×160 grid (step_by(2))
/// and sends it via MINIMAP_DATA opcode. The client caches it in an offscreen canvas.
pub fn send_minimap_base(state: &GameState, buf: &mut CanvasBuffer) {
    let mw: u16 = (GRID_SIZE / 2) as u16; // 80
    let mh: u16 = (GRID_SIZE / 2) as u16;
    let mx: i16 = 8;
    let (_, vh_mm) = vp(state);
    let my: i16 = vh_mm as i16 - MINIMAP_DISPLAY_SIZE as i16 - 8;

    let mut pixels = Vec::with_capacity(mw as usize * mh as usize);
    for ty in (0..GRID_SIZE).step_by(2) {
        for tx in (0..GRID_SIZE).step_by(2) {
            let t = state.grid.get(tx, ty);
            let (r, g, b) = match t {
                TileType::Grass => (86, 125, 70),
                TileType::Water => (40, 80, 150),
                TileType::Forest => (50, 95, 45),
                TileType::Rock => (100, 95, 85),
            };
            pixels.push((r, g, b));
        }
    }
    buf.minimap_data(mx, my, mw, mh, &pixels);
}

const MINIMAP_DISPLAY_SIZE: u16 = 120;

fn render_minimap(state: &GameState, buf: &mut CanvasBuffer) {
    let (_, vh) = vp(state);
    let msize = MINIMAP_DISPLAY_SIZE;
    let mx: i16 = 8;
    let my: i16 = vh as i16 - msize as i16 - 8;
    let scale = msize as f32 / GRID_SIZE as f32;

    // Background
    buf.set_fill_rgba(0, 0, 0, 180);
    buf.fill_rect(mx - 2, my - 2, msize + 4, msize + 4);

    // Draw cached minimap terrain (sent once via MINIMAP_DATA in setup)
    buf.minimap_draw(mx, my, msize, msize);

    // Zones (dynamic — ownership changes)
    for zone in &state.zones {
        let (r, g, b) = match zone.owner {
            Some(Faction::Blue) => (60, 120, 255),
            Some(Faction::Red) => (255, 60, 60),
            None => (200, 200, 100),
        };
        buf.set_fill_rgba(r, g, b, 150);
        let zx = mx + (zone.cx / TS * scale) as i16;
        let zy = my + (zone.cy / TS * scale) as i16;
        buf.begin_path().arc_full(zx, zy, (zone.radius * scale) as u16).fill();
    }

    // Units (dynamic — move every tick)
    for u in &state.units {
        if !u.alive { continue; }
        let (r, g, b) = match u.faction {
            Faction::Blue => (80, 140, 255),
            Faction::Red => (255, 80, 80),
        };
        buf.set_fill_rgb(r, g, b);
        let ux = mx + (u.x / TS * scale) as i16;
        let uy = my + (u.y / TS * scale) as i16;
        buf.fill_rect(ux - 1, uy - 1, 2, 2);
    }

    // Camera viewport
    buf.set_stroke_rgba(255, 255, 255, 180);
    buf.set_line_width(4);
    let (cvw, cvh) = vp(state);
    let half_vw = cvw as f32 / (2.0 * state.camera_zoom);
    let half_vh = cvh as f32 / (2.0 * state.camera_zoom);
    let vx = mx + ((state.camera_x - half_vw) / TS * scale) as i16;
    let vy = my + ((state.camera_y - half_vh) / TS * scale) as i16;
    let vw = (half_vw * 2.0 / TS * scale) as u16;
    let vh = (half_vh * 2.0 / TS * scale) as u16;
    buf.stroke_rect(vx, vy, vw, vh);
}

fn render_hud(state: &GameState, buf: &mut CanvasBuffer) {
    let (hw, hh) = vp(state);
    // Zone pips
    let ps: i16 = 24;
    let sx: i16 = hw as i16 - 20 - (state.zones.len() as i16 - 1) * ps;
    for (i, z) in state.zones.iter().enumerate() {
        let (r, g, b) = match z.owner {
            Some(Faction::Blue) => (60, 120, 255),
            Some(Faction::Red) => (255, 60, 60),
            None => (100, 100, 100),
        };
        buf.set_fill_rgb(r, g, b);
        buf.begin_path().arc_full(sx + i as i16 * ps, 20, 8).fill();
    }

    // Player HP
    if let Some(p) = state.units.iter().find(|u| u.id == state.player_id && u.alive) {
        buf.set_fill_rgb(255, 255, 255);
        buf.set_font(0);
        buf.set_text_align(0);
        buf.set_text_baseline(0);
        buf.fill_text(140, hh as i16 - 24, &format!("HP: {}/{}", p.hp, p.kind.max_hp()));
    }

    // Unit counts
    let bc = state.units.iter().filter(|u| u.alive && u.faction == Faction::Blue).count();
    let rc = state.units.iter().filter(|u| u.alive && u.faction == Faction::Red).count();
    buf.set_fill_rgb(200, 200, 200);
    buf.set_font(1);
    buf.set_text_align(2);
    buf.fill_text(hw as i16 - 12, hh as i16 - 12, &format!("Blue: {}  Red: {}", bc, rc));

    // Controls hint
    buf.set_fill_rgba(200, 200, 200, 100);
    buf.set_font(4);
    buf.set_text_align(0);
    buf.set_text_baseline(0);
    buf.fill_text(12, 12, "WASD: Move  Space: Attack  H/G/R: Orders");
}

fn render_victory_progress(state: &GameState, buf: &mut CanvasBuffer) {
    if state.phase != GamePhase::Playing { return; }

    if state.victory_hold_timer <= 0.0 { return; }

    let all_blue = state.zones.iter().all(|z| z.owner == Some(Faction::Blue));
    let all_red = state.zones.iter().all(|z| z.owner == Some(Faction::Red));
    if !all_blue && !all_red { return; }

    // Victory progress bar (300×24, top-center)
    let bw: u16 = 300;
    let bh: u16 = 24;
    let (svw, _) = vp(state);
    let bx = svw as i16 / 2 - bw as i16 / 2;
    let by: i16 = 50;

    buf.set_fill_rgba(0, 0, 0, 150);
    buf.round_rect(bx, by, bw, bh, 6);

    // Use actual hold timer
    let hold_progress = (state.victory_hold_timer / 120.0).min(1.0);
    let fill_w = (hold_progress * (bw - 4) as f32) as u16;

    let (fr, fg, fb) = if all_blue { (70, 130, 230) } else { (220, 60, 60) };
    buf.set_fill_rgba(fr, fg, fb, 230);
    buf.round_rect(bx + 2, by + 2, fill_w, bh - 4, 4);

    // Label with time remaining
    let remaining = (120.0 - state.victory_hold_timer).max(0.0) as u32;
    buf.set_fill_rgb(255, 255, 255);
    buf.set_font(0);
    buf.set_text_align(1);
    buf.set_text_baseline(1);
    let label = if all_blue {
        format!("Holding all zones... {}s", remaining)
    } else {
        format!("Enemy holds all zones! {}s", remaining)
    };
    buf.fill_text(svw as i16 / 2, by + bh as i16 / 2, &label);
}

fn render_menu(w: u16, h: u16, buf: &mut CanvasBuffer) {
    buf.set_fill_rgba(0, 0, 0, 191);
    buf.fill_rect(0, 0, w, h);

    let cx = w as i16 / 2;
    let cy = h as i16 / 2;

    // Title with shadow (matching original: shadow at +3,+3)
    buf.set_font(2);
    buf.set_text_align(1);
    buf.set_text_baseline(1);
    buf.set_fill_rgba(0, 0, 0, 178); // shadow
    buf.fill_text(cx + 3, cy - 57, "THE BATTLEFIELD");
    buf.set_fill_rgba(255, 215, 0, 255); // gold
    buf.fill_text(cx, cy - 60, "THE BATTLEFIELD");

    // Play button (matching original: green with white border)
    buf.set_fill_rgba(70, 150, 70, 217);
    buf.round_rect(cx - 100, cy - 20, 200, 50, 10);
    buf.set_stroke_rgba(255, 255, 255, 102);
    buf.set_line_width(8); // 2px
    buf.stroke_rect(cx - 100, cy - 20, 200, 50);
    buf.set_fill_rgb(255, 255, 255);
    buf.set_font(3);
    buf.fill_text(cx, cy + 5, "PLAY");

    // Controls hint (matching original: 50% white, monospace)
    buf.set_fill_rgba(255, 255, 255, 127);
    buf.set_font(4);
    buf.fill_text(cx, cy + 55, "WASD: Move  Space: Attack  H/G/R/F: Orders");
    buf.fill_text(cx, cy + 75, "Press SPACE or click to start");
}

fn render_death(w: u16, h: u16, buf: &mut CanvasBuffer) {
    let cx = w as i16 / 2;
    let cy = h as i16 / 2;

    buf.set_fill_rgba(80, 0, 0, 153);
    buf.fill_rect(0, 0, w, h);

    // Title with shadow
    buf.set_font(5);
    buf.set_text_align(1);
    buf.set_text_baseline(1);
    buf.set_fill_rgba(0, 0, 0, 178);
    buf.fill_text(cx + 3, cy - 17, "YOU DIED");
    buf.set_fill_rgba(204, 34, 34, 255);
    buf.fill_text(cx, cy - 20, "YOU DIED");

    // Retry button
    buf.set_fill_rgba(180, 80, 40, 217);
    buf.round_rect(cx - 100, cy + 10, 200, 50, 10);
    buf.set_stroke_rgba(255, 255, 255, 102);
    buf.set_line_width(8);
    buf.stroke_rect(cx - 100, cy + 10, 200, 50);
    buf.set_fill_rgb(255, 255, 255);
    buf.set_font(3);
    buf.fill_text(cx, cy + 35, "RETRY (Space)");
}

fn render_result(w: u16, h: u16, buf: &mut CanvasBuffer, winner: Faction) {
    let cx = w as i16 / 2;
    let cy = h as i16 / 2;

    let (overlay_r, overlay_g, overlay_b) = match winner {
        Faction::Blue => (0, 30, 60),
        Faction::Red => (40, 0, 0),
    };
    buf.set_fill_rgba(overlay_r, overlay_g, overlay_b, 153);
    buf.fill_rect(0, 0, w, h);

    let (text, r, g, b) = match winner {
        Faction::Blue => ("VICTORY!", 78, 168, 255),
        Faction::Red => ("DEFEAT", 255, 85, 85),
    };

    // Title with shadow
    buf.set_font(5);
    buf.set_text_align(1);
    buf.set_text_baseline(1);
    buf.set_fill_rgba(0, 0, 0, 178);
    buf.fill_text(cx + 3, cy - 27, text);
    buf.set_fill_rgb(r, g, b);
    buf.fill_text(cx, cy - 30, text);

    // Subtitle
    buf.set_fill_rgba(255, 255, 255, 178);
    buf.set_font(0);
    let subtitle = match winner {
        Faction::Blue => "All zones captured and held!",
        Faction::Red => "The enemy has conquered all zones.",
    };
    buf.fill_text(cx, cy + 10, subtitle);

    // Replay button
    buf.set_fill_rgba(60, 120, 180, 217);
    buf.round_rect(cx - 100, cy + 30, 200, 50, 10);
    buf.set_stroke_rgba(255, 255, 255, 102);
    buf.set_line_width(8);
    buf.stroke_rect(cx - 100, cy + 30, 200, 50);
    buf.set_fill_rgb(255, 255, 255);
    buf.set_font(3);
    buf.fill_text(cx, cy + 55, "PLAY AGAIN (Space)");
}
