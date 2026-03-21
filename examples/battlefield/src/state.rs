//! Game state and tick logic.

use crate::grid::{Grid, GRID_SIZE, TILE_SIZE};
use crate::mapgen;
use crate::unit::{Unit, UnitKind, Faction, Facing, OrderKind};
use crate::combat;
use rwire_canvas::InputState;

const REINFORCE_INTERVAL: f32 = 20.0;
const MAX_UNITS: usize = 35;
const AI_VISION: f32 = 10.0; // tiles
const VICTORY_HOLD_TIME: f32 = 120.0; // seconds to hold all zones for victory
const ORDER_RADIUS: f32 = 7.0; // tiles — radius for order broadcasting
const MONK_SAFE_DIST: f32 = 3.0; // tiles — monks flee from enemies within this range
const ARROW_SPEED: f32 = 600.0; // pixels per second
const ARC_BASE: f32 = 30.0;
const ARC_DIST_FACTOR: f32 = 0.25;

#[derive(Debug, PartialEq, Eq)]
pub enum GamePhase {
    Menu,
    Playing,
    Dead,
    Victory(Faction),
}

pub struct ZoneState {
    pub cx: f32,
    pub cy: f32,
    pub radius: f32,
    pub progress: f32, // -1.0 (red) to +1.0 (blue)
    pub owner: Option<Faction>,
}

pub struct Building {
    pub x: f32,
    pub y: f32,
    pub faction: Faction,
    pub kind: BuildingKind,
}

#[derive(Clone, Copy)]
pub enum BuildingKind {
    Barracks,
    Archery,
    Monastery,
}

pub struct Projectile {
    pub start_x: f32,
    pub start_y: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub progress: f32,   // 0.0 to 1.0
    pub duration: f32,   // total flight time in seconds
    pub arc_height: f32, // peak height of parabolic arc
}

pub struct GameState {
    pub grid: Grid,
    pub units: Vec<Unit>,
    pub zones: Vec<ZoneState>,
    pub buildings: Vec<Building>,
    pub projectiles: Vec<Projectile>,
    pub phase: GamePhase,
    pub player_id: u32,
    pub camera_x: f32,
    pub camera_y: f32,
    pub camera_zoom: f32,
    pub next_id: u32,
    pub reinforce_timer: f32,
    pub tick: u32,
    pub aim_x: f32,
    pub aim_y: f32,
    pub blue_base: (f32, f32),
    pub red_base: (f32, f32),
    pub victory_hold_timer: f32, // counts up while holding all zones
}

impl GameState {
    pub fn new() -> Self {
        let seed = 42u32; // deterministic for now
        let (grid, layout) = mapgen::generate_battlefield(seed);

        let ts = TILE_SIZE;
        let mut units = Vec::new();
        let mut next_id = 0u32;

        // Blue base from BSP layout
        let (bcx, bcy) = layout.blue_base;
        let bx = bcx as f32;
        let by = bcy as f32 + 5.0; // spawn in front of base

        // Spawn player (blue warrior, front of formation)
        let px = (bx + 1.0) * ts;
        let py = by * ts;
        units.push(Unit::new(next_id, UnitKind::Warrior, Faction::Blue, px, py));
        let player_id = next_id;
        next_id += 1;

        // Blue army: 5 warriors, 6 lancers, 5 archers, 2 monks = 18
        let blue_spawn = [
            (UnitKind::Warrior, bx + 1.0, by + 1.0),
            (UnitKind::Warrior, bx + 1.0, by - 1.0),
            (UnitKind::Warrior, bx + 1.0, by + 2.0),
            (UnitKind::Warrior, bx + 1.0, by - 2.0),
            (UnitKind::Lancer, bx, by),
            (UnitKind::Lancer, bx, by + 1.0),
            (UnitKind::Lancer, bx, by - 1.0),
            (UnitKind::Lancer, bx, by + 2.0),
            (UnitKind::Lancer, bx, by - 2.0),
            (UnitKind::Lancer, bx, by - 3.0),
            (UnitKind::Archer, bx - 1.0, by + 1.0),
            (UnitKind::Archer, bx - 1.0, by - 1.0),
            (UnitKind::Archer, bx - 2.0, by),
            (UnitKind::Archer, bx - 2.0, by + 1.0),
            (UnitKind::Archer, bx - 2.0, by - 1.0),
            (UnitKind::Monk, bx - 2.0, by + 2.0),
            (UnitKind::Monk, bx - 2.0, by - 2.0),
        ];
        for &(kind, x, y) in &blue_spawn {
            units.push(Unit::new(next_id, kind, Faction::Blue, x * ts, y * ts));
            next_id += 1;
        }

        // Red base from BSP layout
        let (rcx, rcy) = layout.red_base;
        let rx = rcx as f32;
        let ry = rcy as f32 - 5.0; // spawn in front of base

        let red_spawn = [
            (UnitKind::Warrior, rx - 1.0, ry),
            (UnitKind::Warrior, rx - 1.0, ry - 1.0),
            (UnitKind::Warrior, rx - 1.0, ry + 1.0),
            (UnitKind::Warrior, rx - 1.0, ry - 2.0),
            (UnitKind::Warrior, rx - 1.0, ry + 2.0),
            (UnitKind::Lancer, rx, ry),
            (UnitKind::Lancer, rx, ry - 1.0),
            (UnitKind::Lancer, rx, ry + 1.0),
            (UnitKind::Lancer, rx, ry - 2.0),
            (UnitKind::Lancer, rx, ry + 2.0),
            (UnitKind::Lancer, rx, ry + 3.0),
            (UnitKind::Archer, rx + 1.0, ry - 1.0),
            (UnitKind::Archer, rx + 1.0, ry + 1.0),
            (UnitKind::Archer, rx + 2.0, ry),
            (UnitKind::Archer, rx + 2.0, ry - 1.0),
            (UnitKind::Archer, rx + 2.0, ry + 1.0),
            (UnitKind::Monk, rx + 2.0, ry - 2.0),
            (UnitKind::Monk, rx + 2.0, ry + 2.0),
        ];
        for &(kind, x, y) in &red_spawn {
            units.push(Unit::new(next_id, kind, Faction::Red, x * ts, y * ts));
            next_id += 1;
        }

        // Zones from BSP layout
        let zones: Vec<ZoneState> = layout.zone_centers.iter().enumerate().map(|(i, &(zx, zy))| {
            let radius = if i == 1 { 4.0 } else { 3.5 }; // center zone bigger
            ZoneState {
                cx: zx as f32 * ts,
                cy: zy as f32 * ts,
                radius,
                progress: 0.0,
                owner: None,
            }
        }).collect();

        // Buildings at BSP-derived base positions
        let buildings = vec![
            Building { x: (bcx as f32 - 4.0) * ts, y: bcy as f32 * ts, faction: Faction::Blue, kind: BuildingKind::Barracks },
            Building { x: (bcx as f32 + 4.0) * ts, y: bcy as f32 * ts, faction: Faction::Blue, kind: BuildingKind::Archery },
            Building { x: bcx as f32 * ts, y: (bcy as f32 - 3.0) * ts, faction: Faction::Blue, kind: BuildingKind::Monastery },
            Building { x: (rcx as f32 + 4.0) * ts, y: rcy as f32 * ts, faction: Faction::Red, kind: BuildingKind::Barracks },
            Building { x: (rcx as f32 - 4.0) * ts, y: rcy as f32 * ts, faction: Faction::Red, kind: BuildingKind::Archery },
            Building { x: rcx as f32 * ts, y: (rcy as f32 + 3.0) * ts, faction: Faction::Red, kind: BuildingKind::Monastery },
        ];

        Self {
            grid,
            units,
            zones,
            buildings,
            projectiles: Vec::new(),
            phase: GamePhase::Menu,
            player_id,
            camera_x: px - 480.0,
            camera_y: py - 320.0,
            camera_zoom: 0.85, // slightly zoomed out to see more
            next_id,
            reinforce_timer: REINFORCE_INTERVAL,
            tick: 0,
            aim_x: 1.0,
            aim_y: 0.0,
            blue_base: (bcx as f32 * ts, bcy as f32 * ts),
            red_base: (rcx as f32 * ts, rcy as f32 * ts),
            victory_hold_timer: 0.0,
        }
    }

    pub fn tick(&mut self, input: &InputState, dt: f32) {
        // Menu: wait for space/attack to start
        if self.phase == GamePhase::Menu {
            if input.attacking() || input.touching {
                self.phase = GamePhase::Playing;
            }
            return;
        }

        // Dead/Victory: space to restart
        if matches!(self.phase, GamePhase::Dead | GamePhase::Victory(_)) {
            if input.attacking() {
                let mut fresh = Self::new();
                fresh.phase = GamePhase::Playing;
                *self = fresh;
            }
            return;
        }

        // Track aim direction from movement
        let (dx, dy) = input.move_dir();
        if dx != 0 || dy != 0 {
            self.aim_x = dx as f32;
            self.aim_y = dy as f32;
        }

        // Camera zoom with +/- (snap to 1/64 increments like original)
        if input.key2(rwire_canvas::input::KEY2_ZOOM_IN) {
            self.camera_zoom = (self.camera_zoom + 0.02).min(4.0);
        }
        if input.key2(rwire_canvas::input::KEY2_ZOOM_OUT) {
            self.camera_zoom = (self.camera_zoom - 0.02).max(0.5);
        }
        self.camera_zoom = (self.camera_zoom * 64.0).round() / 64.0;

        self.tick += 1;

        // Cooldowns, animation, death fade, hit/order flash
        for u in &mut self.units {
            if u.cooldown > 0.0 { u.cooldown -= dt; }
            if u.hit_flash > 0.0 { u.hit_flash -= dt; }
            if u.order_flash > 0.0 { u.order_flash -= dt; }
            if !u.alive && u.death_fade > 0.0 {
                u.death_fade += dt;
            }
            u.update_anim(dt);
        }

        // Player movement
        self.tick_player(input, dt);

        // AI
        self.tick_ai(dt);

        // Collision resolution — push overlapping units apart
        self.resolve_collisions();

        // Update projectiles
        self.projectiles.retain_mut(|p| {
            if p.duration > 0.0 {
                p.progress += dt / p.duration;
            }
            p.progress < 1.0
        });

        // Zones
        self.tick_zones(dt);

        // Reinforcements
        self.tick_reinforcements(dt);

        // Remove dead units after fade completes
        self.units.retain(|u| {
            u.alive || u.id == self.player_id || u.death_fade < crate::unit::DEATH_FADE_DURATION
        });

        // Player death check
        if !self.units.iter().any(|u| u.id == self.player_id && u.alive) {
            self.phase = GamePhase::Dead;
            return;
        }

        // Victory check — must hold ALL zones for VICTORY_HOLD_TIME seconds
        let all_blue = self.zones.iter().all(|z| z.owner == Some(Faction::Blue));
        let all_red = self.zones.iter().all(|z| z.owner == Some(Faction::Red));
        if all_blue || all_red {
            self.victory_hold_timer += dt;
            if self.victory_hold_timer >= VICTORY_HOLD_TIME {
                self.phase = GamePhase::Victory(if all_blue { Faction::Blue } else { Faction::Red });
            }
        } else {
            self.victory_hold_timer = 0.0;
        }
    }

    fn tick_player(&mut self, input: &InputState, dt: f32) {
        let pidx = match self.units.iter().position(|u| u.id == self.player_id) {
            Some(i) => i,
            None => return,
        };
        if !self.units[pidx].alive { return; }

        // Movement
        let (dx, dy) = input.move_dir();
        let speed = self.units[pidx].kind.speed() * TILE_SIZE * dt;
        let nx = self.units[pidx].x + dx as f32 * speed;
        let ny = self.units[pidx].y + dy as f32 * speed;
        if self.grid.passable_at(nx, self.units[pidx].y) { self.units[pidx].x = nx; }
        if self.grid.passable_at(self.units[pidx].x, ny) { self.units[pidx].y = ny; }
        if dx > 0 { self.units[pidx].facing = Facing::Right; }
        else if dx < 0 { self.units[pidx].facing = Facing::Left; }

        // Camera
        self.camera_x = self.units[pidx].x;
        self.camera_y = self.units[pidx].y;
        let world_max = GRID_SIZE as f32 * TILE_SIZE;
        let half_vw = 480.0 / self.camera_zoom;
        let half_vh = 320.0 / self.camera_zoom;
        self.camera_x = self.camera_x.clamp(half_vw, world_max - half_vw);
        self.camera_y = self.camera_y.clamp(half_vh, world_max - half_vh);

        // Issue orders (H/G/R/F)
        let order = if input.key(rwire_canvas::input::KEY_ORDER_HOLD) {
            Some(OrderKind::Hold)
        } else if input.key(rwire_canvas::input::KEY_ORDER_GO) {
            Some(OrderKind::Go)
        } else if input.key(rwire_canvas::input::KEY_ORDER_RETREAT) {
            Some(OrderKind::Retreat)
        } else if input.key2(rwire_canvas::input::KEY2_ORDER_FOLLOW) {
            Some(OrderKind::Follow)
        } else {
            None
        };
        if let Some(order_kind) = order {
            let px = self.units[pidx].x;
            let py = self.units[pidx].y;
            let radius_sq = (ORDER_RADIUS * TILE_SIZE).powi(2);
            for u in &mut self.units {
                if u.id == self.player_id || !u.alive || u.faction != Faction::Blue { continue; }
                let dsq = (u.x - px).powi(2) + (u.y - py).powi(2);
                if dsq < radius_sq {
                    u.order = Some(order_kind);
                    u.order_flash = 1.0;
                }
            }
        }

        // Attack nearest enemy in range
        if input.attacking() {
            let px = self.units[pidx].x;
            let py = self.units[pidx].y;
            let pfac = self.units[pidx].faction;
            let range = self.units[pidx].kind.range();

            let target_idx = self.units.iter().enumerate()
                .filter(|(i, u)| *i != pidx && u.alive && u.faction != pfac)
                .filter(|(_, u)| {
                    let d = ((u.x - px).powi(2) + (u.y - py).powi(2)).sqrt() / TILE_SIZE;
                    d <= range
                })
                .min_by(|(_, a), (_, b)| {
                    let da = (a.x - px).powi(2) + (a.y - py).powi(2);
                    let db = (b.x - px).powi(2) + (b.y - py).powi(2);
                    da.partial_cmp(&db).unwrap()
                })
                .map(|(i, _)| i);

            if let Some(ti) = target_idx {
                combat::resolve_attack(pidx, ti, &mut self.units, &self.grid);
            }
        }
    }

    fn tick_ai(&mut self, dt: f32) {
        let unit_count = self.units.len();
        for i in 0..unit_count {
            if self.units[i].id == self.player_id || !self.units[i].alive {
                continue;
            }

            let ux = self.units[i].x;
            let uy = self.units[i].y;
            let ufac = self.units[i].faction;
            let ukind = self.units[i].kind;

            // Find nearest enemy
            let mut nearest_enemy = None;
            let mut nearest_dist = f32::MAX;
            for j in 0..unit_count {
                if i == j || !self.units[j].alive || self.units[j].faction == ufac { continue; }
                let d = ((self.units[j].x - ux).powi(2) + (self.units[j].y - uy).powi(2)).sqrt() / TILE_SIZE;
                if d < nearest_dist && d < AI_VISION {
                    nearest_dist = d;
                    nearest_enemy = Some(j);
                }
            }

            // Monk behavior: heal wounded allies, flee from enemies
            if ukind.is_healer() {
                // Flee from nearby enemies (MONK_SAFE_DIST)
                if let Some(ei) = nearest_enemy {
                    if nearest_dist < MONK_SAFE_DIST {
                        let ex = self.units[ei].x;
                        let ey = self.units[ei].y;
                        // Move away from enemy
                        let flee_x = ux + (ux - ex);
                        let flee_y = uy + (uy - ey);
                        self.units[i].move_toward(flee_x, flee_y, dt, &self.grid);
                        continue;
                    }
                }
                // Find wounded ally
                let mut nearest_wounded = None;
                let mut wound_dist = f32::MAX;
                for j in 0..unit_count {
                    if i == j || !self.units[j].alive || self.units[j].faction != ufac { continue; }
                    if self.units[j].hp >= self.units[j].kind.max_hp() { continue; }
                    let d = ((self.units[j].x - ux).powi(2) + (self.units[j].y - uy).powi(2)).sqrt() / TILE_SIZE;
                    if d < wound_dist {
                        wound_dist = d;
                        nearest_wounded = Some(j);
                    }
                }
                if let Some(wi) = nearest_wounded {
                    if wound_dist <= ukind.range() {
                        combat::resolve_heal(i, wi, &mut self.units);
                    } else {
                        let tx = self.units[wi].x;
                        let ty = self.units[wi].y;
                        self.units[i].move_toward(tx, ty, dt, &self.grid);
                    }
                    continue;
                }
            }

            // Blue units with orders — follow the order instead of default behavior
            let unit_order = self.units[i].order;
            if ufac == Faction::Blue {
                if let Some(order) = unit_order {
                    match order {
                        OrderKind::Hold => {
                            // Hold: attack enemies within range, don't move
                            if let Some(ei) = nearest_enemy {
                                if nearest_dist <= ukind.range() {
                                    combat::resolve_attack(i, ei, &mut self.units, &self.grid);
                                }
                            }
                            continue;
                        }
                        OrderKind::Follow => {
                            // Follow player, attack nearby enemies
                            if let Some(pi) = self.units.iter().position(|u| u.id == self.player_id) {
                                let px = self.units[pi].x;
                                let py = self.units[pi].y;
                                let pdist = ((ux - px).powi(2) + (uy - py).powi(2)).sqrt() / TILE_SIZE;
                                if let Some(ei) = nearest_enemy {
                                    if nearest_dist <= ukind.range() {
                                        combat::resolve_attack(i, ei, &mut self.units, &self.grid);
                                    }
                                }
                                if pdist > 3.0 {
                                    self.units[i].move_toward(px, py, dt, &self.grid);
                                }
                            }
                            continue;
                        }
                        OrderKind::Retreat => {
                            // Retreat to base, only melee self-defense
                            let (bx, by) = self.blue_base;
                            if let Some(ei) = nearest_enemy {
                                if nearest_dist <= 1.5 {
                                    combat::resolve_attack(i, ei, &mut self.units, &self.grid);
                                }
                            }
                            self.units[i].move_toward(bx, by, dt, &self.grid);
                            continue;
                        }
                        OrderKind::Go => {
                            // Advance toward nearest zone, engage enemies
                            // (falls through to default behavior below)
                        }
                    }
                }
            }

            // Default behavior: attack or move toward enemy
            if let Some(ei) = nearest_enemy {
                if nearest_dist <= ukind.range() {
                    // Spawn arrow projectile for ranged units
                    if ukind.is_ranged() && self.units[i].can_attack() {
                        let sx = self.units[i].x;
                        let sy = self.units[i].y;
                        let tx = self.units[ei].x;
                        let ty = self.units[ei].y;
                        let dist = ((tx - sx).powi(2) + (ty - sy).powi(2)).sqrt();
                        self.projectiles.push(Projectile {
                            start_x: sx,
                            start_y: sy,
                            target_x: tx,
                            target_y: ty,
                            progress: 0.0,
                            duration: dist / ARROW_SPEED,
                            arc_height: ARC_BASE + dist * ARC_DIST_FACTOR,
                        });
                    }
                    combat::resolve_attack(i, ei, &mut self.units, &self.grid);
                } else {
                    let tx = self.units[ei].x;
                    let ty = self.units[ei].y;
                    self.units[i].move_toward(tx, ty, dt, &self.grid);
                }
            } else {
                // March toward enemy base
                let (bx, by) = match ufac {
                    Faction::Blue => self.red_base,
                    Faction::Red => self.blue_base,
                };
                self.units[i].move_toward(bx, by, dt, &self.grid);
            }
        }
    }

    fn tick_zones(&mut self, dt: f32) {
        for zone in &mut self.zones {
            let mut blue_count = 0;
            let mut red_count = 0;
            for u in &self.units {
                if !u.alive { continue; }
                let dx = u.x - zone.cx;
                let dy = u.y - zone.cy;
                let d = (dx * dx + dy * dy).sqrt() / TILE_SIZE;
                if d <= zone.radius {
                    match u.faction {
                        Faction::Blue => blue_count += 1,
                        Faction::Red => red_count += 1,
                    }
                }
            }

            if blue_count > 0 && red_count > 0 {
                // Contested — no progress
            } else if blue_count > 0 {
                let rate = (blue_count as f32).sqrt() * dt * 0.125;
                zone.progress = (zone.progress + rate).min(1.0);
            } else if red_count > 0 {
                let rate = (red_count as f32).sqrt() * dt * 0.125;
                zone.progress = (zone.progress - rate).max(-1.0);
            }

            zone.owner = if zone.progress > 0.8 {
                Some(Faction::Blue)
            } else if zone.progress < -0.8 {
                Some(Faction::Red)
            } else {
                None
            };
        }
    }

    fn resolve_collisions(&mut self) {
        let unit_radius = 28.0f32; // pixels
        let min_dist = unit_radius * 2.0;

        for i in 0..self.units.len() {
            if !self.units[i].alive { continue; }
            for j in (i + 1)..self.units.len() {
                if !self.units[j].alive { continue; }
                let dx = self.units[j].x - self.units[i].x;
                let dy = self.units[j].y - self.units[i].y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < min_dist && dist > 0.1 {
                    let overlap = (min_dist - dist) * 0.5;
                    let nx = dx / dist;
                    let ny = dy / dist;
                    // Push units apart (skip player for stability)
                    if self.units[i].id != self.player_id {
                        self.units[i].x -= nx * overlap;
                        self.units[i].y -= ny * overlap;
                    }
                    if self.units[j].id != self.player_id {
                        self.units[j].x += nx * overlap;
                        self.units[j].y += ny * overlap;
                    }
                }
            }
        }
    }

    fn tick_reinforcements(&mut self, dt: f32) {
        self.reinforce_timer -= dt;
        if self.reinforce_timer > 0.0 { return; }
        self.reinforce_timer = REINFORCE_INTERVAL;

        for faction in [Faction::Blue, Faction::Red] {
            let count = self.units.iter().filter(|u| u.alive && u.faction == faction).count();
            if count >= MAX_UNITS { continue; }

            let (bx, by) = match faction {
                Faction::Blue => self.blue_base,
                Faction::Red => self.red_base,
            };

            // 7 units per wave like the original
            let wave = [
                UnitKind::Warrior, UnitKind::Warrior,
                UnitKind::Lancer, UnitKind::Lancer,
                UnitKind::Archer, UnitKind::Archer,
                UnitKind::Monk,
            ];
            for (i, &kind) in wave.iter().enumerate() {
                let ox = (i % 3) as f32 * TILE_SIZE;
                let oy = (i / 3) as f32 * TILE_SIZE;
                self.units.push(Unit::new(self.next_id, kind, faction, bx + ox, by + oy));
                self.next_id += 1;
            }
        }
    }
}
