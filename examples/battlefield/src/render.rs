//! Server-side rendering: GameState → CanvasBuffer draw commands.

use crate::autotile;
use crate::grid::{Decoration, GRID_SIZE, TILE_SIZE, TileType};
use crate::particle::ParticleKind;
use crate::sprites::tex;
use crate::state::{BuildingKind, GamePhase, GameState};
use crate::unit::{Facing, Faction, UnitAnim};
use rwire_canvas::CanvasBuffer;

const CW: u16 = 960;
const CH: u16 = 640;
const TS: f32 = TILE_SIZE;

pub fn render_frame(state: &GameState, buf: &mut CanvasBuffer) {
    buf.set_fill_rgb(26, 26, 38);
    buf.fill_rect(0, 0, CW, CH);

    let cx = state.camera_x;
    let cy = state.camera_y;
    let zoom = state.camera_zoom;
    buf.save();
    // Match original: offset = round(canvas_half - zoom * camera_pos)
    // This snaps to integer pixels for pixel-perfect tile alignment
    let offset_x = (CW as f32 / 2.0 - zoom * cx).round() as i16;
    let offset_y = (CH as f32 / 2.0 - zoom * cy).round() as i16;
    buf.translate(offset_x, offset_y);
    buf.scale_uniform(zoom);

    // Water rendered first as base layer
    render_water(state, buf, cx, cy);
    // Land tiles on top (autotiled edges blend over water)
    render_terrain(state, buf, cx, cy);
    // Elevated terrain layer (on top of flat ground)
    render_elevation(state, buf, cx, cy);
    // Subtle global darken for moody atmosphere
    buf.set_fill_rgba(0, 5, 10, 20);
    buf.fill_rect(-1000, -1000, 12000, 12000);
    render_zones(state, buf);
    render_aim_cone(state, buf);
    render_water_rocks(state, buf, cx, cy);
    render_rocks(state, buf, cx, cy);
    render_bushes(state, buf, cx, cy);
    render_foreground(state, buf, cx, cy);
    render_hp_bars(state, buf);
    render_order_labels(state, buf);
    render_fog(state, buf, cx, cy);

    // Black borders beyond grid edges
    let world = GRID_SIZE as f32 * TS;
    let margin = 1000i16;
    buf.set_fill_rgb(15, 15, 25);
    buf.fill_rect(-margin, -margin, margin as u16, world as u16 + margin as u16 * 2);
    buf.fill_rect(world as i16, -margin, margin as u16, world as u16 + margin as u16 * 2);
    buf.fill_rect(0, -margin, world as u16, margin as u16);
    buf.fill_rect(0, world as i16, world as u16, margin as u16);

    buf.restore();

    render_minimap(state, buf);
    render_hud(state, buf);
    render_victory_progress(state, buf);

    match &state.phase {
        GamePhase::Menu => render_menu(buf),
        GamePhase::Victory(f) => render_result(buf, *f),
        GamePhase::Dead => render_death(buf),
        GamePhase::Playing => {}
    }
}

fn vis(cx: f32, cy: f32) -> (usize, usize, usize, usize) {
    // Camera is center point; compute visible rect edges
    // Use a generous margin to account for zoom levels
    let half_w = CW as f32; // generous: 2x viewport for zoom margin
    let half_h = CH as f32;
    let stx = (((cx - half_w) / TS).floor() as i32).max(0) as usize;
    let sty = (((cy - half_h) / TS).floor() as i32).max(0) as usize;
    let etx = (((cx + half_w) / TS).ceil() as i32 + 1).min(GRID_SIZE as i32) as usize;
    let ety = (((cy + half_h) / TS).ceil() as i32 + 1).min(GRID_SIZE as i32) as usize;
    (stx, sty, etx, ety)
}

fn render_terrain(state: &GameState, buf: &mut CanvasBuffer, cx: f32, cy: f32) {
    let (stx, sty, etx, ety) = vis(cx, cy);
    let ts = TS as u16;

    for ty in sty..ety {
        for tx in stx..etx {
            let tile = state.grid.get(tx, ty);
            if tile == TileType::Water { continue; }

            let dx = (tx as f32 * TS) as i16;
            let dy = (ty as f32 * TS) as i16;

            // Draw 1px larger to prevent subpixel seams
            let draw_ts = ts + 1;

            // Use tilemap1 consistently (original uses one tilemap, flips for variety)
            match tile {
                TileType::Grass | TileType::Forest => {
                    let (sx, sy) = autotile::flat_ground_src(&state.grid, tx, ty);
                    // Only flip CENTER tiles (col=1,row=1 = 64,64) like the original
                    let is_center = sx == 64 && sy == 64;
                    let flip = is_center && (tx as u32).wrapping_mul(48271).wrapping_add((ty as u32).wrapping_mul(16807)) & 1 == 0;
                    if flip {
                        buf.save();
                        buf.translate(dx + draw_ts as i16, dy);
                        buf.scale(-256, 256); // flip X
                        buf.draw_image(tex::TILEMAP1, sx, sy, 64, 64, 0, 0, draw_ts, draw_ts);
                        buf.restore();
                    } else {
                        buf.draw_image(tex::TILEMAP1, sx, sy, 64, 64, dx, dy, draw_ts, draw_ts);
                    }
                    if tile == TileType::Forest {
                        buf.set_fill_rgba(0, 10, 0, 35);
                        buf.fill_rect(dx, dy, draw_ts, draw_ts);
                    }
                }
                TileType::Rock => {
                    // Draw grass underneath, then rock decoration
                    let (sx, sy) = autotile::flat_ground_src(&state.grid, tx, ty);
                    buf.draw_image(tex::TILEMAP1, sx, sy, 64, 64, dx, dy, draw_ts, draw_ts);
                }
                _ => {}
            }
        }
    }
}

fn render_elevation(state: &GameState, buf: &mut CanvasBuffer, cx: f32, cy: f32) {
    let (stx, sty, etx, ety) = vis(cx, cy);
    let ts = TS as u16 + 1;

    for ty in sty..ety {
        for tx in stx..etx {
            if state.grid.elev(tx, ty) == 0 { continue; }

            let dx = (tx as f32 * TS) as i16;
            let dy = (ty as f32 * TS) as i16;

            // Shadow under elevated tile (drawn on the tile below)
            if ty + 1 < GRID_SIZE && state.grid.elev(tx, ty + 1) == 0 {
                buf.set_alpha(130);
                buf.draw_image(tex::SHADOW, 0, 0, 192, 192,
                    dx - 64, dy + TS as i16 / 2 - 32, 192, 192);
                buf.set_alpha(255);
            }

            // Elevated surface tile (autotiled) — use tilemap1 (same as original)
            let (sx, sy) = autotile::elevated_src(&state.grid, tx, ty);
            buf.draw_image(tex::TILEMAP1, sx, sy, 64, 64, dx, dy, ts, ts);

            // Cliff face on tile below (if that tile is not elevated)
            if ty + 1 < GRID_SIZE && state.grid.elev(tx, ty + 1) == 0 {
                let (csx, csy) = autotile::cliff_src(&state.grid, tx, ty + 1);
                let cdx = (tx as f32 * TS) as i16;
                let cdy = ((ty + 1) as f32 * TS) as i16;
                buf.draw_image(tex::TILEMAP1, csx, csy, 64, 64, cdx, cdy, ts, ts);
            }
        }
    }
}

fn render_water(state: &GameState, buf: &mut CanvasBuffer, cx: f32, cy: f32) {
    let (stx, sty, etx, ety) = vis(cx, cy);
    let ts = TS as u16 + 1;

    for ty in sty..ety {
        for tx in stx..etx {
            if state.grid.get(tx, ty) != TileType::Water { continue; }
            let dx = (tx as f32 * TS) as i16;
            let dy = (ty as f32 * TS) as i16;
            buf.draw_image(tex::WATER, 0, 0, 64, 64, dx, dy, ts, ts);
        }
    }

    // Foam on land tiles adjacent to water (8 FPS with spatial decorrelation)
    // Original: foam_fps=8, frame offset = (gx*7 + gy*13) % 16
    let global_frame = state.tick * 20 / 50; // ~8fps at 20 tick/s * (8/20)

    for ty in sty..ety {
        for tx in stx..etx {
            let tile = state.grid.get(tx, ty);
            if tile == TileType::Water { continue; }
            let has_water = [(0i32, -1), (0, 1), (-1, 0), (1, 0)].iter().any(|&(ddx, ddy)| {
                let nx = tx as i32 + ddx;
                let ny = ty as i32 + ddy;
                if nx < 0 || ny < 0 { return false; }
                state.grid.get(nx as usize, ny as usize) == TileType::Water
            });
            if !has_water { continue; }

            // Per-tile frame offset for decorrelation (matching original)
            let tile_offset = (tx.wrapping_mul(7).wrapping_add(ty.wrapping_mul(13))) as u32 % 16;
            let frame = (global_frame + tile_offset) % 16;
            let foam_sx = (frame as u16) * 192;

            // Centered on tile (matching original: tile_center - foam_size/2)
            let dx = (tx as f32 * TS + TS / 2.0 - 96.0) as i16;
            let dy = (ty as f32 * TS + TS / 2.0 - 96.0) as i16;
            buf.draw_image(tex::FOAM, foam_sx, 0, 192, 192, dx, dy, 192, 192);
        }
    }
}

fn render_rocks(state: &GameState, buf: &mut CanvasBuffer, cx: f32, cy: f32) {
    let (stx, sty, etx, ety) = vis(cx, cy);
    for ty in sty..ety {
        for tx in stx..etx {
            let tile = state.grid.get(tx, ty);
            let dx = (tx as f32 * TS) as i16;
            let dy = (ty as f32 * TS) as i16;

            if tile == TileType::Rock {
                // Full rock decoration on rock tiles
                let v = ((tx * 7 + ty * 13) % 4) as u8;
                buf.draw_image(tex::ROCK1 + v, 0, 0, 64, 64, dx, dy, 64, 64);
            } else if tile == TileType::Grass {
                // Scattered small pebble decorations on grass (like original)
                let hash = (tx.wrapping_mul(23) ^ ty.wrapping_mul(59)) % 12;
                if hash == 0 {
                    let v = ((tx * 3 + ty * 7) % 4) as u8;
                    // Draw smaller (32x32) for subtle pebbles
                    let ox = ((tx * 11 + ty * 17) % 32) as i16;
                    let oy = ((tx * 19 + ty * 7) % 32) as i16;
                    buf.set_alpha(200);
                    buf.draw_image(tex::ROCK1 + v, 0, 0, 64, 64, dx + ox, dy + oy, 32, 32);
                    buf.set_alpha(255);
                }
            }
        }
    }
}

fn render_bushes(state: &GameState, buf: &mut CanvasBuffer, cx: f32, cy: f32) {
    let (stx, sty, etx, ety) = vis(cx, cy);
    let bf = ((state.tick / 2) % 8) as u16;
    for ty in sty..ety {
        for tx in stx..etx {
            // Render bushes from decoration layer
            if state.grid.decoration(tx, ty) == Some(Decoration::Bush) {
                let v = ((tx * 3 + ty * 11) % 4) as u8;
                let sx = bf * 128;
                let dx = (tx as f32 * TS) as i16 - 16;
                let dy = (ty as f32 * TS) as i16 - 16;
                buf.draw_image(tex::BUSH1 + v, sx, 0, 128, 128, dx, dy, 80, 80);
            }
        }
    }
}

fn render_water_rocks(state: &GameState, buf: &mut CanvasBuffer, cx: f32, cy: f32) {
    let (stx, sty, etx, ety) = vis(cx, cy);
    let _ = state.tick; // tick available for future animation
    for ty in sty..ety {
        for tx in stx..etx {
            if state.grid.decoration(tx, ty) != Some(Decoration::WaterRock) { continue; }
            let v = ((tx * 7 + ty * 3) % 4) as u8;
            let dx = (tx as f32 * TS) as i16;
            let dy = (ty as f32 * TS) as i16;
            // Render small rock in water (reuse rock textures at smaller size)
            buf.set_alpha(200);
            buf.draw_image(tex::ROCK1 + v, 0, 0, 64, 64, dx + 8, dy + 8, 48, 48);
            buf.set_alpha(255);
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

fn render_foreground(state: &GameState, buf: &mut CanvasBuffer, cx: f32, cy: f32) {
    enum D { Unit(usize), Tree(usize, usize), Building(usize), Projectile(usize), Particle(usize) }
    let mut items: Vec<(f32, D)> = Vec::new();

    // Projectiles
    for (i, p) in state.projectiles.iter().enumerate() {
        let cur_y = p.start_y + (p.target_y - p.start_y) * p.progress;
        items.push((cur_y, D::Projectile(i)));
    }

    // Particles
    for (i, p) in state.particles.iter().enumerate() {
        items.push((p.y + TS * 0.5, D::Particle(i)));
    }

    for (i, u) in state.units.iter().enumerate() {
        if !u.alive { continue; }
        if u.x >= cx - 200.0 && u.x <= cx + CW as f32 + 200.0
            && u.y >= cy - 200.0 && u.y <= cy + CH as f32 + 200.0 {
            items.push((u.y + TS * 0.5, D::Unit(i)));
        }
    }

    let (stx, sty, etx, ety) = vis(cx, cy);
    for ty in sty..ety {
        for tx in stx..etx {
            if state.grid.get(tx, ty) != TileType::Forest { continue; }
            // Sparser trees — only ~1 in 6 forest tiles get a tree (was 1 in 4)
            if (tx.wrapping_mul(17) ^ ty.wrapping_mul(53)) % 6 != 0 { continue; }
            items.push(((ty as f32 + 1.0) * TS, D::Tree(tx, ty)));
        }
    }

    // Buildings
    for (i, b) in state.buildings.iter().enumerate() {
        if b.x >= cx - 200.0 && b.x <= cx + CW as f32 + 200.0
            && b.y >= cy - 300.0 && b.y <= cy + CH as f32 + 200.0 {
            items.push((b.y + TS, D::Building(i)));
        }
    }

    items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    for (_, item) in items {
        match item {
            D::Unit(i) => draw_unit(state, buf, i),
            D::Tree(tx, ty) => draw_tree(state, buf, tx, ty),
            D::Building(i) => draw_building(state, buf, i),
            D::Projectile(i) => draw_projectile(state, buf, i),
            D::Particle(i) => draw_particle(state, buf, i),
        }
    }
}

fn draw_unit(state: &GameState, buf: &mut CanvasBuffer, idx: usize) {
    let u = &state.units[idx];

    // Skip fully faded dead units
    if !u.alive && u.death_fade >= crate::unit::DEATH_FADE_DURATION { return; }

    // Check visibility for enemy units (fog of war)
    if u.faction == Faction::Red {
        let visible = state.units.iter()
            .filter(|f| f.alive && f.faction == Faction::Blue)
            .any(|f| {
                let d = ((f.x - u.x).powi(2) + (f.y - u.y).powi(2)).sqrt() / TS;
                d < 15.0
            });
        if !visible { return; }
    }

    // Death fade opacity
    if !u.alive {
        let fade = 1.0 - (u.death_fade / crate::unit::DEATH_FADE_DURATION);
        buf.set_alpha((fade * 255.0) as u8);
    }

    // Hit flash — 30% opacity on alternating frames (matching original)
    if u.hit_flash > 0.0 {
        let flash_frame = (u.hit_flash * 30.0) as i32;
        if flash_frame % 2 == 0 {
            buf.set_alpha(77); // 0.3 * 255
        }
    }

    let a = match u.anim {
        UnitAnim::Idle => crate::sprites::unit_idle(u.faction, u.kind),
        UnitAnim::Run => crate::sprites::unit_run(u.faction, u.kind),
        UnitAnim::Attack => crate::sprites::unit_attack(u.faction, u.kind),
    };

    let fw = a.frame_w;
    let fh = a.frame_h;
    let frame = u.anim_frame % a.frame_count;
    let sx = frame * fw;
    // Match original: sprite centered on (unit.x, unit.y)
    let dx = u.x as i16 - fw as i16 / 2;
    let dy = u.y as i16 - fh as i16 / 2;

    if u.facing == Facing::Left {
        buf.save();
        buf.translate(u.x as i16, u.y as i16);
        buf.scale(-256, 256); // flip X
        buf.draw_image(a.texture, sx, 0, fw, fh, -(fw as i16) / 2, -(fh as i16) / 2, fw, fh);
        buf.restore();
    } else {
        buf.draw_image(a.texture, sx, 0, fw, fh, dx, dy, fw, fh);
    }

    // Reset alpha after death/hit effects
    buf.set_alpha(255);

    // Player highlight
    if u.id == state.player_id && u.alive {
        buf.set_stroke_rgba(255, 215, 0, 200);
        buf.set_line_width(8);
        buf.begin_path();
        buf.arc_full(u.x as i16, u.y as i16 + 12, 18);
        buf.stroke();
    }
}

fn draw_tree(state: &GameState, buf: &mut CanvasBuffer, tx: usize, ty: usize) {
    let v = ((tx * 7 + ty * 3) % 4) as u8;
    let (fw, fh): (u16, u16) = if v < 2 { (192, 256) } else { (192, 192) };
    let tf = {
        let phase = (tx * 17 + ty * 31) as u32;
        ((state.tick + phase) % 24) / 3
    } as u16 % 8;
    let sx = tf * fw;
    let dw = fw * 3 / 2;
    let dh = fh * 3 / 2;
    let dx = (tx as f32 * TS) as i16 + 32 - dw as i16 / 2;
    let dy = ((ty + 1) as f32 * TS) as i16 - dh as i16;

    // Fade trees near player
    let player = state.units.iter().find(|u| u.id == state.player_id);
    if let Some(p) = player {
        let dist = ((p.x - tx as f32 * TS - 32.0).powi(2) + (p.y - ty as f32 * TS - 32.0).powi(2)).sqrt() / TS;
        if dist < 2.5 {
            let alpha = if dist < 1.0 { 77 } else { (((dist - 1.0) / 1.5) * 255.0) as u8 };
            buf.set_alpha(alpha);
        }
    }

    buf.draw_image(tex::TREE1 + v, sx, 0, fw, fh, dx, dy, dw, dh);
    buf.set_alpha(255);
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
    buf.draw_image(tex::ARROW, 0, 0, 64, 64, -16, -16, 32, 32);
    buf.restore();
}

fn draw_particle(state: &GameState, buf: &mut CanvasBuffer, idx: usize) {
    let p = &state.particles[idx];
    let (tex_id, frame_size) = match p.kind {
        ParticleKind::Dust => (tex::DUST, 64u16),
        ParticleKind::ExplosionSmall => (tex::DUST, 64), // reuse dust for now
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
        (Faction::Blue, BuildingKind::Monastery) => (tex::BARRACKS_BLUE, 192, 256), // fallback
        (Faction::Red, BuildingKind::Monastery) => (tex::BARRACKS_RED, 192, 256),
    };
    // Draw building scaled, bottom-aligned
    let scale = 3u16;
    let dw = sw * scale / 2;
    let dh = sh * scale / 2;
    let dx = b.x as i16 - dw as i16 / 2;
    let dy = b.y as i16 - dh as i16 + 32;
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
        // Tower is 128×256 source, draw at 2×4 tiles, bottom-centered on zone
        let tw: u16 = 128;
        let th: u16 = 256;
        let tdx = zcx - tw as i16 / 2;
        let tdy = zcy - th as i16 + 48;
        // Pulse opacity when capturing
        let tower_alpha = if zone.owner.is_none() && zone.progress.abs() > 0.05 {
            let pulse = ((state.tick as f32 * 0.15).sin() * 0.2 + 0.8) * 255.0;
            pulse as u8
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
    let (stx, sty, etx, ety) = vis(cx, cy);

    // Collect all friendly unit positions for multi-source visibility
    let friendly_positions: Vec<(f32, f32)> = state.units.iter()
        .filter(|u| u.alive && u.faction == Faction::Blue)
        .map(|u| (u.x, u.y))
        .collect();

    let fov = 15.0f32; // tiles — large visible area like the original
    let soft_edge = 4.0f32; // tiles of soft gradient

    // Per-tile fog (full tile steps for performance, subtler alpha)
    for ty in sty..ety {
        for tx in stx..etx {
            let wx = tx as f32 * TS + TS / 2.0;
            let wy = ty as f32 * TS + TS / 2.0;

            // Find minimum distance to any friendly unit
            let min_dist = friendly_positions.iter()
                .map(|&(px, py)| ((wx - px).powi(2) + (wy - py).powi(2)).sqrt() / TS)
                .fold(f32::MAX, f32::min);

            if min_dist > fov {
                // Fully fogged
                buf.set_fill_rgba(10, 10, 18, 180);
                buf.fill_rect(
                    (tx as f32 * TS) as i16, (ty as f32 * TS) as i16,
                    TS as u16 + 1, TS as u16 + 1,
                );
            } else if min_dist > fov - soft_edge {
                // Soft gradient
                let t = (min_dist - (fov - soft_edge)) / soft_edge;
                let alpha = (t * t * 160.0) as u8; // quadratic falloff for softer edge
                if alpha > 3 {
                    buf.set_fill_rgba(10, 10, 18, alpha);
                    buf.fill_rect(
                        (tx as f32 * TS) as i16, (ty as f32 * TS) as i16,
                        TS as u16 + 1, TS as u16 + 1,
                    );
                }
            }
            // Inside FOV — no fog at all (bright like original)
        }
    }
}

fn render_minimap(state: &GameState, buf: &mut CanvasBuffer) {
    let msize: u16 = 120;
    let mx: i16 = 8;
    let my: i16 = CH as i16 - msize as i16 - 8;
    let scale = msize as f32 / GRID_SIZE as f32;

    // Background
    buf.set_fill_rgba(0, 0, 0, 180);
    buf.fill_rect(mx - 2, my - 2, msize + 4, msize + 4);

    // Terrain (simplified: 1px per tile)
    for ty in (0..GRID_SIZE).step_by(2) {
        for tx in (0..GRID_SIZE).step_by(2) {
            let t = state.grid.get(tx, ty);
            let (r, g, b) = match t {
                TileType::Grass => (86, 125, 70),
                TileType::Water => (40, 80, 150),
                TileType::Forest => (50, 95, 45),
                TileType::Rock => (100, 95, 85),
            };
            buf.set_fill_rgb(r, g, b);
            let px = mx + (tx as f32 * scale) as i16;
            let py = my + (ty as f32 * scale) as i16;
            let ps = (scale * 2.0).ceil() as u16;
            buf.fill_rect(px, py, ps, ps);
        }
    }

    // Zones
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

    // Units
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
    let vx = mx + (state.camera_x / TS * scale) as i16;
    let vy = my + (state.camera_y / TS * scale) as i16;
    let vw = (CW as f32 / TS * scale) as u16;
    let vh = (CH as f32 / TS * scale) as u16;
    buf.stroke_rect(vx, vy, vw, vh);
}

fn render_hud(state: &GameState, buf: &mut CanvasBuffer) {
    // Zone pips
    let ps: i16 = 24;
    let sx: i16 = CW as i16 - 20 - (state.zones.len() as i16 - 1) * ps;
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
        buf.fill_text(140, CH as i16 - 24, &format!("HP: {}/{}", p.hp, p.kind.max_hp()));
    }

    // Unit counts
    let bc = state.units.iter().filter(|u| u.alive && u.faction == Faction::Blue).count();
    let rc = state.units.iter().filter(|u| u.alive && u.faction == Faction::Red).count();
    buf.set_fill_rgb(200, 200, 200);
    buf.set_font(1);
    buf.set_text_align(2);
    buf.fill_text(CW as i16 - 12, CH as i16 - 12, &format!("Blue: {}  Red: {}", bc, rc));

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
    let bx = CW as i16 / 2 - bw as i16 / 2;
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
    buf.fill_text(CW as i16 / 2, by + bh as i16 / 2, &label);
}

fn render_menu(buf: &mut CanvasBuffer) {
    buf.set_fill_rgba(0, 0, 0, 190);
    buf.fill_rect(0, 0, CW, CH);

    buf.set_fill_rgba(255, 215, 0, 255);
    buf.set_font(2);
    buf.set_text_align(1);
    buf.set_text_baseline(1);
    buf.fill_text(CW as i16 / 2, CH as i16 / 2 - 60, "THE BATTLEFIELD");

    // Play button
    buf.set_fill_rgba(80, 180, 80, 255);
    buf.round_rect(CW as i16 / 2 - 60, CH as i16 / 2 - 15, 120, 40, 8);
    buf.set_fill_rgb(255, 255, 255);
    buf.set_font(3);
    buf.fill_text(CW as i16 / 2, CH as i16 / 2 + 5, "PLAY");

    buf.set_fill_rgba(200, 200, 200, 150);
    buf.set_font(4);
    buf.fill_text(CW as i16 / 2, CH as i16 / 2 + 60, "WASD: Move  Space: Attack  H/G/R/F: Orders");
    buf.fill_text(CW as i16 / 2, CH as i16 / 2 + 80, "Press SPACE or click to start");
}

fn render_death(buf: &mut CanvasBuffer) {
    buf.set_fill_rgba(80, 0, 0, 150);
    buf.fill_rect(0, 0, CW, CH);
    buf.set_fill_rgba(204, 34, 34, 255);
    buf.set_font(5);
    buf.set_text_align(1);
    buf.set_text_baseline(1);
    buf.fill_text(CW as i16 / 2, CH as i16 / 2 - 20, "YOU DIED");
    buf.set_fill_rgba(200, 200, 200, 200);
    buf.set_font(3);
    buf.fill_text(CW as i16 / 2, CH as i16 / 2 + 30, "Press SPACE to retry");
}

fn render_result(buf: &mut CanvasBuffer, winner: Faction) {
    let (overlay_r, overlay_g, overlay_b) = match winner {
        Faction::Blue => (0, 30, 60),
        Faction::Red => (40, 0, 0),
    };
    buf.set_fill_rgba(overlay_r, overlay_g, overlay_b, 150);
    buf.fill_rect(0, 0, CW, CH);

    let (text, r, g, b) = match winner {
        Faction::Blue => ("VICTORY!", 78, 168, 255),
        Faction::Red => ("DEFEAT", 255, 85, 85),
    };
    buf.set_fill_rgb(r, g, b);
    buf.set_font(5);
    buf.set_text_align(1);
    buf.set_text_baseline(1);
    buf.fill_text(CW as i16 / 2, CH as i16 / 2 - 20, text);
    buf.set_fill_rgba(200, 200, 200, 200);
    buf.set_font(3);
    buf.fill_text(CW as i16 / 2, CH as i16 / 2 + 30, "Press SPACE to play again");
}
