//! Game state and tick logic.

use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::flowfield::FactionFlow;
use crate::grid::{Grid, GRID_SIZE, TILE_SIZE};
use crate::mapgen;
use crate::particle::{Particle, ParticleKind};
use crate::rng::{self, TileRng};
use crate::unit::{Unit, UnitKind, Faction, OrderKind};
use crate::combat;
use rwire_canvas::InputState;

const REINFORCE_INTERVAL: f32 = 20.0;
const MAX_UNITS: usize = 35;
const AI_VISION: f32 = 10.0; // tiles
const VICTORY_HOLD_TIME: f32 = 120.0; // seconds to hold all zones for victory
const ORDER_RADIUS: f32 = 7.0; // tiles
const MONK_SAFE_DIST: f32 = 3.0; // tiles
const ARROW_SPEED: f32 = 600.0; // pixels per second
const ARC_BASE: f32 = 30.0;
const ARC_DIST_FACTOR: f32 = 0.25;
const MELEE_RANGE: f32 = 96.0; // TILE_SIZE * 1.5
const KNOCKBACK_DIST: f32 = 32.0; // TILE_SIZE * 0.5
const UNIT_RADIUS: f32 = 28.0;
const ATTACK_CONE_HALF: f32 = std::f32::consts::FRAC_PI_3; // 60 degrees
const CAMERA_LERP: f32 = 5.0; // camera smoothing factor (matching original)
const MAX_CAPTURE_MULTIPLIER: f32 = 3.0; // max capture rate = 3× base (matching original)

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
    pub name: &'static str,
}

pub struct Building {
    pub x: f32,
    pub y: f32,
    pub grid_x: usize,
    pub grid_y: usize,
    pub faction: Faction,
    pub kind: BuildingKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
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
    pub progress: f32,
    pub duration: f32,
    pub arc_height: f32,
    pub damage: i32,     // damage applied on impact
    pub attacker_faction: Faction,
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
    pub particles: Vec<Particle>,
    pub aim_x: f32,
    pub aim_y: f32,
    pub blue_base: (f32, f32),
    pub red_base: (f32, f32),
    pub victory_hold_timer: f32,
    pub blue_flow: FactionFlow,
    pub red_flow: FactionFlow,
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub elapsed: f32,
    pub tile_rng: TileRng,
    pub rng: ChaCha8Rng,
    /// Tree sprite IDs and their world-space tile centers, for per-frame alpha updates.
    pub tree_sprites: Vec<(u16, f32, f32)>, // (sprite_id, world_cx, world_cy)
}

impl GameState {
    pub fn new() -> Self {
        let seed = 42u32; // deterministic for now
        let (mut grid, layout) = mapgen::generate_battlefield(seed);

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
            let mut u = Unit::new(next_id, kind, Faction::Blue, x * ts, y * ts);
            u.cooldown = (next_id as f32 * 0.05) % 0.3; // stagger initial cooldowns
            units.push(u);
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
            let mut u = Unit::new(next_id, kind, Faction::Red, x * ts, y * ts);
            u.cooldown = (next_id as f32 * 0.05) % 0.3; // stagger initial cooldowns
            units.push(u);
            next_id += 1;
        }

        // Zones from BSP layout
        let zone_names = ["Zone A", "Zone B", "Zone C", "Zone D", "Zone E"];
        let zones: Vec<ZoneState> = layout.zone_centers.iter().enumerate().map(|(i, &(zx, zy))| {
            let radius = 4.0; // uniform 4-tile radius (matching original ZONE_RADIUS)
            ZoneState {
                cx: zx as f32 * ts,
                cy: zy as f32 * ts,
                radius,
                progress: 0.0,
                owner: None,
                name: zone_names.get(i).unwrap_or(&"Zone"),
            }
        }).collect();

        // Buildings at BSP-derived base positions
        let buildings = vec![
            Building { x: (bcx as f32 - 4.0) * ts, y: bcy as f32 * ts, grid_x: bcx as usize - 4, grid_y: bcy as usize, faction: Faction::Blue, kind: BuildingKind::Barracks },
            Building { x: (bcx as f32 + 4.0) * ts, y: bcy as f32 * ts, grid_x: bcx as usize + 4, grid_y: bcy as usize, faction: Faction::Blue, kind: BuildingKind::Archery },
            Building { x: bcx as f32 * ts, y: (bcy as f32 - 3.0) * ts, grid_x: bcx as usize, grid_y: bcy as usize - 3, faction: Faction::Blue, kind: BuildingKind::Monastery },
            Building { x: (rcx as f32 + 4.0) * ts, y: rcy as f32 * ts, grid_x: rcx as usize + 4, grid_y: rcy as usize, faction: Faction::Red, kind: BuildingKind::Barracks },
            Building { x: (rcx as f32 - 4.0) * ts, y: rcy as f32 * ts, grid_x: rcx as usize - 4, grid_y: rcy as usize, faction: Faction::Red, kind: BuildingKind::Archery },
            Building { x: rcx as f32 * ts, y: (rcy as f32 + 3.0) * ts, grid_x: rcx as usize, grid_y: rcy as usize + 3, faction: Faction::Red, kind: BuildingKind::Monastery },
        ];

        // Mark building footprints on grid (3x3 tiles each, matching original)
        for b in &buildings {
            grid.mark_building(b.grid_x, b.grid_y, 1, 1);
        }

        Self {
            grid,
            units,
            zones,
            buildings,
            projectiles: Vec::new(),
            phase: GamePhase::Menu,
            player_id,
            camera_x: px,
            camera_y: py,
            camera_zoom: 0.0, // 0 = needs ideal zoom calculation on first viewport report
            next_id,
            reinforce_timer: REINFORCE_INTERVAL,
            tick: 0,
            aim_x: 1.0,
            aim_y: 0.0,
            particles: Vec::new(),
            blue_base: (bcx as f32 * ts, bcy as f32 * ts),
            red_base: (rcx as f32 * ts, rcy as f32 * ts),
            victory_hold_timer: 0.0,
            blue_flow: FactionFlow::new(),
            red_flow: FactionFlow::new(),
            viewport_w: 960.0,
            viewport_h: 640.0,
            elapsed: 0.0,
            tile_rng: TileRng::new(seed as u64),
            rng: rng::game_rng(seed as u64),
            tree_sprites: Vec::new(),
        }
    }

    pub fn tick(&mut self, input: &InputState, dt: f32) {
        // Update viewport from client
        if input.viewport_w > 0 && input.viewport_h > 0 {
            self.viewport_w = input.viewport_w as f32;
            self.viewport_h = input.viewport_h as f32;
            // Compute ideal zoom on first viewport report
            if self.camera_zoom < 0.01 {
                let short = self.viewport_w.min(self.viewport_h);
                let target_tiles = if self.viewport_h > self.viewport_w { 10.0 } else { 15.0 };
                let raw = short / (target_tiles * TILE_SIZE);
                self.camera_zoom = ((raw * 64.0).round() / 64.0).clamp(0.5, 4.0);
            }
        }
        // Fallback if no viewport yet
        if self.camera_zoom < 0.01 {
            self.camera_zoom = 0.85;
        }

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
        self.elapsed += dt;

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

        // Update flow fields for AI pathfinding — target best zone, fall back to enemy base
        let blue_target = self.best_target_zone(Faction::Blue)
            .unwrap_or(self.red_base);
        let red_target = self.best_target_zone(Faction::Red)
            .unwrap_or(self.blue_base);
        self.blue_flow.update_if_needed(&self.grid, blue_target.0, blue_target.1);
        self.red_flow.update_if_needed(&self.grid, red_target.0, red_target.1);

        // AI
        self.tick_ai(dt);

        // Update particles
        for p in &mut self.particles { p.update(dt); }
        self.particles.retain(|p| !p.finished);

        // Collision resolution — hard body push-apart with wall-sliding
        self.resolve_collisions();

        // Update projectiles — apply damage on impact
        let mut impacts = Vec::new();
        self.projectiles.retain_mut(|p| {
            if p.duration > 0.0 {
                p.progress += dt / p.duration;
            }
            if p.progress >= 1.0 {
                // Projectile landed — find nearest enemy at impact point for damage
                impacts.push((p.target_x, p.target_y, p.damage, p.attacker_faction));
                false
            } else {
                true
            }
        });
        // Apply deferred projectile damage
        let hit_radius = TILE_SIZE * 0.75;
        for (ix, iy, dmg, faction) in impacts {
            // Find nearest enemy near impact point
            if let Some(ti) = self.units.iter().position(|u| {
                u.alive && u.faction != faction
                    && ((u.x - ix).powi(2) + (u.y - iy).powi(2)).sqrt() <= hit_radius
            }) {
                self.units[ti].hp -= dmg;
                self.units[ti].hit_flash = 0.15;
                if self.units[ti].hp <= 0 {
                    self.units[ti].alive = false;
                    self.units[ti].death_fade = 0.01;
                    self.particles.push(Particle::new(self.units[ti].x, self.units[ti].y, ParticleKind::ExplosionLarge));
                }
                self.particles.push(Particle::new(ix, iy, ParticleKind::Dust));
            }
        }

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

        // Movement (circle collision, speed factor from terrain)
        // Player facing is controlled by aim direction, NOT movement (matching original)
        let (dx, dy) = input.move_dir();
        if dx != 0 || dy != 0 {
            let len = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();
            self.units[pidx].move_dir_opts(dx as f32 / len, dy as f32 / len, dt, &self.grid, false);
        }
        // Player facing follows aim direction
        if self.aim_x > 0.01 {
            self.units[pidx].facing = crate::unit::Facing::Right;
        } else if self.aim_x < -0.01 {
            self.units[pidx].facing = crate::unit::Facing::Left;
        }

        // Camera: smooth lerp following player (matching original: dt * 5.0)
        let target_x = self.units[pidx].x;
        let target_y = self.units[pidx].y;
        let lerp = (CAMERA_LERP * dt).min(1.0);
        self.camera_x += (target_x - self.camera_x) * lerp;
        self.camera_y += (target_y - self.camera_y) * lerp;
        let world_max = GRID_SIZE as f32 * TILE_SIZE;
        let half_vw = (self.viewport_w / 2.0) / self.camera_zoom;
        let half_vh = (self.viewport_h / 2.0) / self.camera_zoom;
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
            // Pre-roll 85% chance per unit (avoid borrow conflict with self.units)
            let rolls: Vec<bool> = (0..self.units.len())
                .map(|_| self.rng.random_ratio(85, 100))
                .collect();
            for (i, u) in self.units.iter_mut().enumerate() {
                if u.id == self.player_id || !u.alive || u.faction != Faction::Blue { continue; }
                let dsq = (u.x - px).powi(2) + (u.y - py).powi(2);
                if dsq < radius_sq && rolls[i] {
                    u.order = Some(order_kind);
                    u.order_flash = 1.0;
                }
            }
        }

        // Attack: cone-based targeting (matching original — hits ALL enemies in 60° cone)
        if input.attacking() && self.units[pidx].can_attack() {
            let px = self.units[pidx].x;
            let py = self.units[pidx].y;
            let pfac = self.units[pidx].faction;
            let aim = self.aim_y.atan2(self.aim_x);
            let attack_range = if self.units[pidx].kind.is_ranged() {
                self.units[pidx].kind.range() * TILE_SIZE
            } else {
                MELEE_RANGE
            };

            // Find ALL enemies in cone
            let targets: Vec<usize> = self.units.iter().enumerate()
                .filter(|(i, u)| *i != pidx && u.alive && u.faction != pfac)
                .filter(|(_, u)| {
                    let dist = ((u.x - px).powi(2) + (u.y - py).powi(2)).sqrt();
                    if dist > attack_range { return false; }
                    let angle_to = (u.y - py).atan2(u.x - px);
                    let mut diff = angle_to - aim;
                    diff = (diff + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                        - std::f32::consts::PI;
                    diff.abs() <= ATTACK_CONE_HALF
                })
                .map(|(i, _)| i)
                .collect();

            if targets.is_empty() {
                // Whiff: play attack with half cooldown (matching original)
                self.units[pidx].play_attack_anim();
                self.units[pidx].cooldown = self.units[pidx].kind.attack_cooldown() * 0.5;
            } else if self.units[pidx].kind.is_ranged() {
                // Player archer: spawn projectile with deferred damage (matching original)
                // Target the nearest enemy in cone
                let nearest = targets.iter().copied().min_by(|&a, &b| {
                    let da = (self.units[a].x - px).powi(2) + (self.units[a].y - py).powi(2);
                    let db = (self.units[b].x - px).powi(2) + (self.units[b].y - py).powi(2);
                    da.partial_cmp(&db).unwrap()
                });
                if let Some(ti) = nearest {
                    let tx = self.units[ti].x;
                    let ty = self.units[ti].y;
                    let dist = ((tx - px).powi(2) + (ty - py).powi(2)).sqrt();
                    let dmg = combat::calc_damage(&self.units[pidx], &self.units[ti], &self.grid);
                    self.units[pidx].cooldown = self.units[pidx].kind.attack_cooldown();
                    self.units[pidx].play_attack_anim();
                    self.projectiles.push(Projectile {
                        start_x: px, start_y: py,
                        target_x: tx, target_y: ty,
                        progress: 0.0,
                        duration: dist / ARROW_SPEED,
                        arc_height: ARC_BASE + dist * ARC_DIST_FACTOR,
                        damage: dmg,
                        attacker_faction: self.units[pidx].faction,
                    });
                }
            } else {
                // Melee: hit all enemies in cone with knockback
                for &ti in &targets {
                    let dmg = combat::resolve_attack(pidx, ti, &mut self.units, &self.grid);
                    if dmg > 0 {
                        // Knockback on melee hit (matching original)
                        let tx = self.units[ti].x;
                        let ty = self.units[ti].y;
                        let ddx = tx - px;
                        let ddy = ty - py;
                        let dd = (ddx * ddx + ddy * ddy).sqrt();
                        if dd > 0.01 {
                            let kx = ddx / dd * KNOCKBACK_DIST;
                            let ky = ddy / dd * KNOCKBACK_DIST;
                            if self.grid.is_circle_passable(tx + kx, ty + ky, UNIT_RADIUS) {
                                self.units[ti].x = tx + kx;
                                self.units[ti].y = ty + ky;
                            }
                        }
                        // Particle effects
                        self.particles.push(Particle::new(self.units[ti].x, self.units[ti].y, ParticleKind::Dust));
                        if !self.units[ti].alive {
                            self.particles.push(Particle::new(self.units[ti].x, self.units[ti].y, ParticleKind::ExplosionLarge));
                        }
                    }
                }
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

            // Find nearest enemy (with line-of-sight check)
            let mut nearest_enemy = None;
            let mut nearest_dist = f32::MAX;
            for j in 0..unit_count {
                if i == j || !self.units[j].alive || self.units[j].faction == ufac { continue; }
                let d = ((self.units[j].x - ux).powi(2) + (self.units[j].y - uy).powi(2)).sqrt() / TILE_SIZE;
                if d < nearest_dist && d < AI_VISION
                    && self.has_line_of_sight(ux, uy, self.units[j].x, self.units[j].y)
                {
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
                // No wounded allies, no close enemies — monk follows the group
                // (don't fall through to default which would make monk charge enemies)
                // Skip to flow field march, same as default but without attacking
                if nearest_enemy.is_none() {
                    let flow = match ufac {
                        Faction::Blue => &self.blue_flow,
                        Faction::Red => &self.red_flow,
                    };
                    let (bx, by) = match ufac {
                        Faction::Blue => self.red_base,
                        Faction::Red => self.blue_base,
                    };
                    let mut moved = false;
                    if let Some(ref field) = flow.field {
                        let (fdx, fdy) = field.world_direction(ux, uy);
                        if fdx != 0.0 || fdy != 0.0 {
                            self.units[i].move_dir(fdx, fdy, dt, &self.grid);
                            moved = true;
                        }
                    }
                    if !moved {
                        self.units[i].move_toward(bx, by, dt, &self.grid);
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
                                    self.ai_attack(i, ei);
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
                                        self.ai_attack(i, ei);
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
                                    self.ai_attack(i, ei);
                                }
                            }
                            self.units[i].move_toward(bx, by, dt, &self.grid);
                            continue;
                        }
                        OrderKind::Go => {
                            // Advance toward nearest uncaptured zone, engage enemies on the way
                            if let Some(ei) = nearest_enemy {
                                if nearest_dist <= ukind.range() {
                                    self.ai_attack(i, ei);
                                    continue;
                                }
                            }
                            // Move toward best zone target
                            if let Some((zx, zy)) = self.best_target_zone(ufac) {
                                self.units[i].move_toward(zx, zy, dt, &self.grid);
                            }
                            continue;
                        }
                    }
                }
            }

            // Default behavior: attack or move toward enemy
            if let Some(ei) = nearest_enemy {
                if nearest_dist <= ukind.range() {
                    self.ai_attack(i, ei);
                } else {
                    let tx = self.units[ei].x;
                    let ty = self.units[ei].y;
                    self.units[i].move_toward(tx, ty, dt, &self.grid);
                }
            } else {
                // March toward enemy base using flow field
                let flow = match ufac {
                    Faction::Blue => &self.blue_flow,
                    Faction::Red => &self.red_flow,
                };
                let (bx, by) = match ufac {
                    Faction::Blue => self.red_base,
                    Faction::Red => self.blue_base,
                };
                let mut moved = false;
                if let Some(ref field) = flow.field {
                    let (fdx, fdy) = field.world_direction(ux, uy);
                    if fdx != 0.0 || fdy != 0.0 {
                        self.units[i].move_dir(fdx, fdy, dt, &self.grid);
                        moved = true;
                    }
                }
                // Fallback: direct movement when flow field has no direction
                if !moved {
                    self.units[i].move_toward(bx, by, dt, &self.grid);
                }
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

            let rate_per_unit = 0.125; // 1/8 = base rate per unit per second
            let max_rate = MAX_CAPTURE_MULTIPLIER * rate_per_unit; // cap at 3× base

            if blue_count > 0 && red_count > 0 {
                // Contested — no progress
            } else if blue_count > 0 {
                let rate = ((blue_count as f32).sqrt() * rate_per_unit).min(max_rate) * dt;
                zone.progress = (zone.progress + rate).min(1.0);
            } else if red_count > 0 {
                let rate = ((red_count as f32).sqrt() * rate_per_unit).min(max_rate) * dt;
                zone.progress = (zone.progress - rate).max(-1.0);
            }

            // Full control at 1.0/-1.0 (matching original)
            zone.owner = if zone.progress >= 1.0 {
                Some(Faction::Blue)
            } else if zone.progress <= -1.0 {
                Some(Faction::Red)
            } else {
                None
            };
        }
    }

    /// Hard body collision: immediate displacement with wall-sliding.
    /// Matching original game — units are pushed apart in one frame, never stack.
    fn resolve_collisions(&mut self) {
        let min_dist = UNIT_RADIUS * 2.0;

        for i in 0..self.units.len() {
            if !self.units[i].alive { continue; }
            for j in (i + 1)..self.units.len() {
                if !self.units[j].alive { continue; }
                let dx = self.units[j].x - self.units[i].x;
                let dy = self.units[j].y - self.units[i].y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < min_dist && dist > 0.001 {
                    let overlap = (min_dist - dist) / 2.0;
                    let nx = dx / dist;
                    let ny = dy / dist;

                    let strength = if self.units[i].faction == self.units[j].faction {
                        0.4
                    } else {
                        1.0
                    };
                    let push = overlap * strength;

                    // Immediate displacement with wall-sliding (split_at_mut for dual borrow)
                    let (left, right) = self.units.split_at_mut(j);
                    Self::try_push_unit(&self.grid, &mut left[i], -nx * push, -ny * push);
                    Self::try_push_unit(&self.grid, &mut right[0], nx * push, ny * push);
                }
            }
        }
    }

    /// Push a unit with wall-sliding fallback.
    fn try_push_unit(grid: &Grid, unit: &mut Unit, push_x: f32, push_y: f32) {
        let radius = UNIT_RADIUS;
        let ox = unit.x;
        let oy = unit.y;
        // Try full push
        if grid.is_circle_passable(ox + push_x, oy + push_y, radius) {
            unit.x = ox + push_x;
            unit.y = oy + push_y;
            return;
        }
        // Wall slide X only
        if push_x.abs() > 0.001 && grid.is_circle_passable(ox + push_x, oy, radius) {
            unit.x = ox + push_x;
            return;
        }
        // Wall slide Y only
        if push_y.abs() > 0.001 && grid.is_circle_passable(ox, oy + push_y, radius) {
            unit.y = oy + push_y;
        }
    }

    /// AI attack helper — handles ranged (deferred projectile) and melee (direct damage + particles).
    fn ai_attack(&mut self, attacker: usize, defender: usize) {
        let ukind = self.units[attacker].kind;
        let ufac = self.units[attacker].faction;

        if ukind.is_ranged() && self.units[attacker].can_attack() {
            let sx = self.units[attacker].x;
            let sy = self.units[attacker].y;
            let tx = self.units[defender].x;
            let ty = self.units[defender].y;
            let dist = ((tx - sx).powi(2) + (ty - sy).powi(2)).sqrt();
            let dmg = combat::calc_damage(&self.units[attacker], &self.units[defender], &self.grid);
            self.units[attacker].cooldown = self.units[attacker].kind.attack_cooldown();
            self.units[attacker].play_attack_anim();
            if tx > sx { self.units[attacker].facing = crate::unit::Facing::Right; }
            else { self.units[attacker].facing = crate::unit::Facing::Left; }
            self.projectiles.push(Projectile {
                start_x: sx, start_y: sy,
                target_x: tx, target_y: ty,
                progress: 0.0,
                duration: dist / ARROW_SPEED,
                arc_height: ARC_BASE + dist * ARC_DIST_FACTOR,
                damage: dmg,
                attacker_faction: ufac,
            });
        } else {
            let dmg = combat::resolve_attack(attacker, defender, &mut self.units, &self.grid);
            if dmg > 0 {
                let hx = self.units[defender].x;
                let hy = self.units[defender].y;
                self.particles.push(Particle::new(hx, hy, ParticleKind::Dust));
                if !self.units[defender].alive {
                    self.particles.push(Particle::new(hx, hy, ParticleKind::ExplosionLarge));
                }
            }
        }
    }

    /// Spiral search for nearest passable world position.
    fn find_nearest_passable(&self, wx: f32, wy: f32) -> Option<(f32, f32)> {
        let cx = (wx / TILE_SIZE) as i32;
        let cy = (wy / TILE_SIZE) as i32;
        // Check if current position is already passable
        if self.grid.is_circle_passable(wx, wy, UNIT_RADIUS) {
            return Some((wx, wy));
        }
        for radius in 1..16i32 {
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx.abs() != radius && dy.abs() != radius { continue; } // perimeter only
                    let nx = cx + dx;
                    let ny = cy + dy;
                    if nx < 0 || ny < 0 || nx >= GRID_SIZE as i32 || ny >= GRID_SIZE as i32 { continue; }
                    let nwx = nx as f32 * TILE_SIZE + TILE_SIZE * 0.5;
                    let nwy = ny as f32 * TILE_SIZE + TILE_SIZE * 0.5;
                    if self.grid.is_circle_passable(nwx, nwy, UNIT_RADIUS) {
                        return Some((nwx, nwy));
                    }
                }
            }
        }
        None
    }

    /// Bresenham line-of-sight raycast on the tile grid.
    /// Forests and elevated terrain (elevation >= 2) block LOS.
    fn has_line_of_sight(&self, x1: f32, y1: f32, x2: f32, y2: f32) -> bool {
        let gx1 = (x1 / TILE_SIZE) as i32;
        let gy1 = (y1 / TILE_SIZE) as i32;
        let gx2 = (x2 / TILE_SIZE) as i32;
        let gy2 = (y2 / TILE_SIZE) as i32;

        let mut cx = gx1;
        let mut cy = gy1;
        let dx = (gx2 - gx1).abs();
        let dy = -(gy2 - gy1).abs();
        let sx: i32 = if gx1 < gx2 { 1 } else { -1 };
        let sy: i32 = if gy1 < gy2 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            // Skip start and end tiles
            if (cx != gx1 || cy != gy1) && (cx != gx2 || cy != gy2) {
                if cx < 0 || cy < 0 || cx >= GRID_SIZE as i32 || cy >= GRID_SIZE as i32 {
                    return false;
                }
                let tile = self.grid.get(cx as usize, cy as usize);
                // Forest blocks LOS
                if tile == crate::grid::TileType::Forest { return false; }
                // Elevated terrain blocks LOS
                if self.grid.elev(cx as usize, cy as usize) >= 2 { return false; }
            }
            if cx == gx2 && cy == gy2 { break; }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                cx += sx;
            }
            if e2 <= dx {
                err += dx;
                cy += sy;
            }
        }
        true
    }

    /// Find the best zone target for a Go order.
    fn best_target_zone(&self, faction: Faction) -> Option<(f32, f32)> {
        let (bx, by) = match faction {
            Faction::Blue => self.blue_base,
            Faction::Red => self.red_base,
        };
        let dist_sq = |z: &ZoneState| {
            ((z.cx - bx).powi(2) + (z.cy - by).powi(2)) as i64
        };

        // Priority 1: Contested zones (nearest to own base)
        let contested = self.zones.iter()
            .filter(|z| z.owner.is_none() && z.progress.abs() > 0.1)
            .min_by_key(|z| dist_sq(z));
        if let Some(z) = contested {
            return Some((z.cx, z.cy));
        }

        // Priority 2: Nearest uncontrolled zone
        let uncontrolled = self.zones.iter()
            .filter(|z| z.owner != Some(faction))
            .min_by_key(|z| dist_sq(z));
        uncontrolled.map(|z| (z.cx, z.cy))
    }

    /// Map unit kind to the building that produces it (matching original).
    fn building_for_kind(kind: UnitKind) -> BuildingKind {
        match kind {
            UnitKind::Warrior | UnitKind::Lancer => BuildingKind::Barracks,
            UnitKind::Archer => BuildingKind::Archery,
            UnitKind::Monk => BuildingKind::Monastery,
        }
    }

    fn tick_reinforcements(&mut self, dt: f32) {
        self.reinforce_timer -= dt;
        if self.reinforce_timer > 0.0 { return; }
        self.reinforce_timer = REINFORCE_INTERVAL;

        // 7 units per wave like the original
        let wave = [
            UnitKind::Warrior, UnitKind::Warrior,
            UnitKind::Lancer, UnitKind::Lancer,
            UnitKind::Archer, UnitKind::Archer,
            UnitKind::Monk,
        ];

        for faction in [Faction::Blue, Faction::Red] {
            let alive_count = self.units.iter().filter(|u| u.alive && u.faction == faction).count();
            if alive_count >= MAX_UNITS { continue; }

            // Cap wave to available slots (matching original)
            let slots = MAX_UNITS.saturating_sub(alive_count);
            let wave_size = slots.min(wave.len());

            let base_pos = match faction {
                Faction::Blue => self.blue_base,
                Faction::Red => self.red_base,
            };

            for &kind in &wave[..wave_size] {
                let bk = Self::building_for_kind(kind);
                // Find the building of the right type for this faction
                let spawn = self.buildings.iter()
                    .find(|b| b.faction == faction && b.kind == bk)
                    .map(|b| {
                        // Spawn 3 tiles toward battlefield from building (matching original)
                        let offset = match faction {
                            Faction::Blue => (0.0, 3.0 * TILE_SIZE),
                            Faction::Red => (0.0, -3.0 * TILE_SIZE),
                        };
                        (b.x + offset.0, b.y + offset.1)
                    });
                let (sx, sy) = spawn.unwrap_or(base_pos);
                let (sx, sy) = self.find_nearest_passable(sx, sy).unwrap_or((sx, sy));
                let mut u = Unit::new(self.next_id, kind, faction, sx, sy);
                u.cooldown = (self.next_id as f32 * 0.05) % 0.3;
                self.units.push(u);
                self.next_id += 1;
            }
        }
    }
}
