//! Server-side rendering: GameState → CanvasBuffer draw commands.

use crate::autotile;
use crate::grid::{GRID_SIZE, TILE_SIZE, TileType};
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
    buf.save();
    buf.translate(-cx as i16, -cy as i16);
    buf.scale_uniform(state.camera_zoom);

    // Water rendered first as base layer
    render_water(state, buf, cx, cy);
    // Land tiles on top (autotiled edges blend over water)
    render_terrain(state, buf, cx, cy);
    // Global darken for moody atmosphere
    buf.set_fill_rgba(0, 5, 10, 35);
    buf.fill_rect(-1000, -1000, 12000, 12000);
    render_zones(state, buf);
    render_aim_cone(state, buf);
    render_rocks(state, buf, cx, cy);
    render_bushes(state, buf, cx, cy);
    render_foreground(state, buf, cx, cy);
    render_hp_bars(state, buf);
    render_fog(state, buf, cx, cy);

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
    let stx = ((cx / TS) as i32).max(0) as usize;
    let sty = ((cy / TS) as i32).max(0) as usize;
    let etx = (((cx + CW as f32) / TS) as usize + 2).min(GRID_SIZE);
    let ety = (((cy + CH as f32) / TS) as usize + 2).min(GRID_SIZE);
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

            // Alternate between tilemap1 and tilemap2 for variety
            let tilemap = if (tx + ty) % 3 == 0 { tex::TILEMAP2 } else { tex::TILEMAP1 };

            match tile {
                TileType::Grass | TileType::Forest => {
                    let (sx, sy) = autotile::flat_ground_src(&state.grid, tx, ty);
                    buf.draw_image(tilemap, sx, sy, 64, 64, dx, dy, draw_ts, draw_ts);
                    if tile == TileType::Forest {
                        buf.set_fill_rgba(0, 15, 0, 50);
                        buf.fill_rect(dx, dy, draw_ts, draw_ts);
                    }
                }
                TileType::Road => {
                    let (sx, sy) = autotile::flat_ground_src(&state.grid, tx, ty);
                    buf.draw_image(tilemap, sx, sy, 64, 64, dx, dy, draw_ts, draw_ts);
                    buf.set_fill_rgba(196, 162, 101, 80);
                    buf.fill_rect(dx, dy, draw_ts, draw_ts);
                }
                TileType::Rock => {
                    let (sx, sy) = autotile::flat_ground_src(&state.grid, tx, ty);
                    buf.draw_image(tilemap, sx, sy, 64, 64, dx, dy, draw_ts, draw_ts);
                    // Slight darkening for rocky areas
                    buf.set_fill_rgba(30, 25, 20, 40);
                    buf.fill_rect(dx, dy, draw_ts, draw_ts);
                }
                _ => {}
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

    // Foam on land tiles adjacent to water
    let foam_frame = ((state.tick / 3) % 16) as u16;
    let foam_sx = foam_frame * 192;

    for ty in sty..ety {
        for tx in stx..etx {
            // Draw foam on LAND tiles that border water
            let tile = state.grid.get(tx, ty);
            if tile == TileType::Water { continue; }
            let has_water = [(0i32, -1), (0, 1), (-1, 0), (1, 0)].iter().any(|&(ddx, ddy)| {
                let nx = tx as i32 + ddx;
                let ny = ty as i32 + ddy;
                if nx < 0 || ny < 0 { return false; }
                state.grid.get(nx as usize, ny as usize) == TileType::Water
            });
            if !has_water { continue; }
            let dx = (tx as f32 * TS) as i16 - 64;
            let dy = (ty as f32 * TS) as i16 - 64;
            buf.set_alpha(180);
            buf.draw_image(tex::FOAM, foam_sx, 0, 192, 192, dx, dy, 192, 192);
            buf.set_alpha(255);
        }
    }
}

fn render_rocks(state: &GameState, buf: &mut CanvasBuffer, cx: f32, cy: f32) {
    let (stx, sty, etx, ety) = vis(cx, cy);
    for ty in sty..ety {
        for tx in stx..etx {
            if state.grid.get(tx, ty) != TileType::Rock { continue; }
            let v = ((tx * 7 + ty * 13) % 4) as u8;
            let dx = (tx as f32 * TS) as i16;
            let dy = (ty as f32 * TS) as i16;
            buf.draw_image(tex::ROCK1 + v, 0, 0, 64, 64, dx, dy, 64, 64);
        }
    }
}

fn render_bushes(state: &GameState, buf: &mut CanvasBuffer, cx: f32, cy: f32) {
    let (stx, sty, etx, ety) = vis(cx, cy);
    let bf = ((state.tick / 2) % 8) as u16;
    for ty in sty..ety {
        for tx in stx..etx {
            if state.grid.get(tx, ty) != TileType::Forest { continue; }
            if (tx.wrapping_mul(31) ^ ty.wrapping_mul(97)) % 5 != 0 { continue; }
            let v = ((tx * 3 + ty * 11) % 4) as u8;
            let sx = bf * 128;
            let dx = (tx as f32 * TS) as i16 - 32;
            let dy = (ty as f32 * TS) as i16 - 32;
            buf.draw_image(tex::BUSH1 + v, sx, 0, 128, 128, dx, dy, 128, 128);
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
    let radius: u16 = 40;

    // Aim direction
    let aim_angle = state.aim_y.atan2(state.aim_x);
    let half_cone = std::f32::consts::FRAC_PI_3; // 60 degrees

    // Yellow semi-transparent wedge
    buf.set_fill_rgba(255, 255, 100, 30);
    buf.begin_path();
    buf.move_to(px, py);
    buf.arc(px, py, radius, aim_angle - half_cone, aim_angle + half_cone);
    buf.close_path();
    buf.fill();

    buf.set_stroke_rgba(255, 255, 100, 90);
    buf.set_line_width(4);
    buf.begin_path();
    buf.move_to(px, py);
    buf.arc(px, py, radius, aim_angle - half_cone, aim_angle + half_cone);
    buf.close_path();
    buf.stroke();

    // Position indicator circle
    buf.set_fill_rgba(255, 215, 0, 60);
    buf.begin_path();
    buf.arc_full(px, py + 12, 24);
    buf.fill();
}

fn render_foreground(state: &GameState, buf: &mut CanvasBuffer, cx: f32, cy: f32) {
    enum D { Unit(usize), Tree(usize, usize), Building(usize) }
    let mut items: Vec<(f32, D)> = Vec::new();

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
            if (tx.wrapping_mul(17) ^ ty.wrapping_mul(53)) % 4 != 0 { continue; }
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
        }
    }
}

fn draw_unit(state: &GameState, buf: &mut CanvasBuffer, idx: usize) {
    let u = &state.units[idx];

    // Check visibility for enemy units (fog of war)
    if u.faction == Faction::Red {
        let visible = state.units.iter()
            .filter(|f| f.alive && f.faction == Faction::Blue)
            .any(|f| {
                let d = ((f.x - u.x).powi(2) + (f.y - u.y).powi(2)).sqrt() / TS;
                d < 12.0
            });
        if !visible { return; }
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
    let dx = u.x as i16 - fw as i16 / 2;
    let dy = u.y as i16 - fh as i16 + 32;

    if u.facing == Facing::Left {
        buf.save();
        buf.translate(u.x as i16, u.y as i16 - fh as i16 / 2 + 32);
        buf.scale(-256, 256);
        buf.draw_image(a.texture, sx, 0, fw, fh, -(fw as i16) / 2, -(fh as i16) / 2, fw, fh);
        buf.restore();
    } else {
        buf.draw_image(a.texture, sx, 0, fw, fh, dx, dy, fw, fh);
    }

    // Player highlight
    if u.id == state.player_id {
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

        let (zr, zg, zb, za) = match zone.owner {
            Some(Faction::Blue) => (60, 120, 255, 40),
            Some(Faction::Red) => (255, 60, 60, 40),
            None => (200, 200, 200, 20),
        };
        buf.set_fill_rgba(zr, zg, zb, za);
        buf.begin_path().arc_full(zcx, zcy, r).fill();

        let (sr, sg, sb) = match zone.owner {
            Some(Faction::Blue) => (60, 120, 255),
            Some(Faction::Red) => (255, 60, 60),
            None => (180, 180, 180),
        };
        buf.set_stroke_rgba(sr, sg, sb, 160);
        buf.set_line_width(8);
        buf.set_line_dash(&[8, 4]);
        buf.begin_path().arc_full(zcx, zcy, r).stroke();
        buf.clear_line_dash();

        // Zone label
        buf.set_fill_rgba(255, 255, 255, 180);
        buf.set_font(1);
        buf.set_text_align(1);
        buf.set_text_baseline(2);
        let label = match zone.owner {
            Some(Faction::Blue) => "BLUE",
            Some(Faction::Red) => "RED",
            None => "NEUTRAL",
        };
        buf.fill_text(zcx, zcy - r as i16 - 22, label);

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

        let bx = u.x as i16 - bw as i16 / 2;
        let by = u.y as i16 - 28;
        let ratio = u.hp as f32 / u.kind.max_hp() as f32;

        // Background
        buf.set_alpha(200);
        buf.set_fill_rgb(51, 51, 51);
        buf.fill_rect(bx, by, bw, bh);

        // Health fill
        let (cr, cg, cb) = if ratio > 0.5 {
            (51, 204, 51)
        } else if ratio > 0.25 {
            (230, 179, 26)
        } else {
            (230, 51, 26)
        };
        buf.set_alpha(230);
        buf.set_fill_rgb(cr, cg, cb);
        buf.fill_rect(bx, by, (ratio * bw as f32) as u16, bh);
        buf.set_alpha(255);
    }
}

fn render_fog(state: &GameState, buf: &mut CanvasBuffer, cx: f32, cy: f32) {
    let (stx, sty, etx, ety) = vis(cx, cy);

    // Collect all friendly unit positions for multi-source visibility
    let friendly_positions: Vec<(f32, f32)> = state.units.iter()
        .filter(|u| u.alive && u.faction == Faction::Blue)
        .map(|u| (u.x, u.y))
        .collect();

    let fov = 12.0f32; // tiles
    let soft_edge = 3.0f32; // tiles of gradient

    // Use half-tile steps for smoother fog
    let step = TS / 2.0;
    let half = step as u16 + 1;

    let start_wx = (stx as f32 * TS) as i32;
    let start_wy = (sty as f32 * TS) as i32;
    let end_wx = (etx as f32 * TS) as i32;
    let end_wy = (ety as f32 * TS) as i32;

    let mut wy = start_wy as f32;
    while wy < end_wy as f32 {
        let mut wx = start_wx as f32;
        while wx < end_wx as f32 {
            let cx_pos = wx + step / 2.0;
            let cy_pos = wy + step / 2.0;

            // Find minimum distance to any friendly unit
            let min_dist = friendly_positions.iter()
                .map(|&(px, py)| ((cx_pos - px).powi(2) + (cy_pos - py).powi(2)).sqrt() / TS)
                .fold(f32::MAX, f32::min);

            if min_dist > fov - soft_edge {
                let t = ((min_dist - (fov - soft_edge)) / soft_edge).min(1.0);
                let alpha = (t * 180.0) as u8;
                if alpha > 5 {
                    buf.set_fill_rgba(12, 12, 22, alpha);
                    buf.fill_rect(wx as i16, wy as i16, half, half);
                }
            }

            wx += step;
        }
        wy += step;
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
                TileType::Road => (160, 145, 105),
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

    // Check if either faction holds all zones
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

    // Simulate victory timer (ticks toward 120s hold)
    let hold_progress = (state.tick as f32 * 0.05 / 120.0).min(1.0); // rough approximation
    let fill_w = (hold_progress * (bw - 4) as f32) as u16;

    let (fr, fg, fb) = if all_blue { (70, 130, 230) } else { (220, 60, 60) };
    buf.set_fill_rgba(fr, fg, fb, 230);
    buf.round_rect(bx + 2, by + 2, fill_w, bh - 4, 4);

    // Label
    buf.set_fill_rgb(255, 255, 255);
    buf.set_font(0);
    buf.set_text_align(1);
    buf.set_text_baseline(1);
    let label = if all_blue { "Holding all zones..." } else { "Enemy holds all zones!" };
    buf.fill_text(CW as i16 / 2, by + bh as i16 / 2, label);
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
