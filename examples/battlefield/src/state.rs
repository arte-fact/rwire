//! Game state and tick logic.

use crate::grid::{Grid, GRID_SIZE, TILE_SIZE};
use crate::unit::{Unit, UnitKind, Faction, Facing};
use crate::combat;
use rwire_canvas::InputState;

const REINFORCE_INTERVAL: f32 = 20.0;
const MAX_UNITS: usize = 35;
const AI_VISION: f32 = 10.0; // tiles

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

pub struct GameState {
    pub grid: Grid,
    pub units: Vec<Unit>,
    pub zones: Vec<ZoneState>,
    pub buildings: Vec<Building>,
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
}

impl GameState {
    pub fn new() -> Self {
        let grid = Grid::new();
        let mut units = Vec::new();
        let mut next_id = 0u32;
        let gs = GRID_SIZE as f32;
        let ts = TILE_SIZE;

        // Blue base at top-left area
        let bx = 12.0;
        let by = 12.0;

        // Spawn player unit (blue warrior, front of formation)
        let px = (bx + 1.0) * ts;
        let py = (by + 5.0) * ts;
        units.push(Unit::new(next_id, UnitKind::Warrior, Faction::Blue, px, py));
        let player_id = next_id;
        next_id += 1;

        // Blue army: 5 warriors, 6 lancers, 5 archers, 2 monks = 18
        let blue_spawn = [
            (UnitKind::Warrior, bx + 2.0, by + 5.0),
            (UnitKind::Warrior, bx + 3.0, by + 5.0),
            (UnitKind::Warrior, bx + 1.0, by + 6.0),
            (UnitKind::Warrior, bx + 2.0, by + 6.0),
            (UnitKind::Lancer, bx, by + 4.0),
            (UnitKind::Lancer, bx + 1.0, by + 4.0),
            (UnitKind::Lancer, bx + 2.0, by + 4.0),
            (UnitKind::Lancer, bx, by + 3.0),
            (UnitKind::Lancer, bx + 1.0, by + 3.0),
            (UnitKind::Lancer, bx + 2.0, by + 3.0),
            (UnitKind::Archer, bx - 1.0, by + 5.0),
            (UnitKind::Archer, bx - 1.0, by + 6.0),
            (UnitKind::Archer, bx - 2.0, by + 5.0),
            (UnitKind::Archer, bx + 4.0, by + 5.0),
            (UnitKind::Archer, bx + 4.0, by + 6.0),
            (UnitKind::Monk, bx - 2.0, by + 4.0),
            (UnitKind::Monk, bx - 2.0, by + 6.0),
        ];
        for &(kind, x, y) in &blue_spawn {
            units.push(Unit::new(next_id, kind, Faction::Blue, x * ts, y * ts));
            next_id += 1;
        }

        // Red base at bottom-right
        let rx = gs - 12.0;
        let ry = gs - 12.0;

        let red_spawn = [
            (UnitKind::Warrior, rx - 1.0, ry - 5.0),
            (UnitKind::Warrior, rx - 2.0, ry - 5.0),
            (UnitKind::Warrior, rx - 3.0, ry - 5.0),
            (UnitKind::Warrior, rx - 1.0, ry - 6.0),
            (UnitKind::Warrior, rx - 2.0, ry - 6.0),
            (UnitKind::Lancer, rx, ry - 4.0),
            (UnitKind::Lancer, rx - 1.0, ry - 4.0),
            (UnitKind::Lancer, rx - 2.0, ry - 4.0),
            (UnitKind::Lancer, rx, ry - 3.0),
            (UnitKind::Lancer, rx - 1.0, ry - 3.0),
            (UnitKind::Lancer, rx - 2.0, ry - 3.0),
            (UnitKind::Archer, rx + 1.0, ry - 5.0),
            (UnitKind::Archer, rx + 1.0, ry - 6.0),
            (UnitKind::Archer, rx + 2.0, ry - 5.0),
            (UnitKind::Archer, rx - 4.0, ry - 5.0),
            (UnitKind::Archer, rx - 4.0, ry - 6.0),
            (UnitKind::Monk, rx + 2.0, ry - 4.0),
            (UnitKind::Monk, rx + 2.0, ry - 6.0),
        ];
        for &(kind, x, y) in &red_spawn {
            units.push(Unit::new(next_id, kind, Faction::Red, x * ts, y * ts));
            next_id += 1;
        }

        // 5 Capture zones across the map
        let mid = gs / 2.0 * ts;
        let zones = vec![
            ZoneState { cx: mid, cy: mid, radius: 4.0, progress: 0.0, owner: None },
            ZoneState { cx: mid - 16.0 * ts, cy: mid - 8.0 * ts, radius: 3.5, progress: 0.0, owner: None },
            ZoneState { cx: mid + 16.0 * ts, cy: mid + 8.0 * ts, radius: 3.5, progress: 0.0, owner: None },
            ZoneState { cx: mid - 8.0 * ts, cy: mid + 12.0 * ts, radius: 3.0, progress: 0.0, owner: None },
            ZoneState { cx: mid + 8.0 * ts, cy: mid - 12.0 * ts, radius: 3.0, progress: 0.0, owner: None },
        ];

        // Buildings at bases
        let buildings = vec![
            Building { x: (bx - 3.0) * ts, y: (by + 1.0) * ts, faction: Faction::Blue, kind: BuildingKind::Barracks },
            Building { x: (bx + 4.0) * ts, y: (by + 1.0) * ts, faction: Faction::Blue, kind: BuildingKind::Archery },
            Building { x: (bx) * ts, y: (by - 1.0) * ts, faction: Faction::Blue, kind: BuildingKind::Monastery },
            Building { x: (rx + 1.0) * ts, y: (ry - 1.0) * ts, faction: Faction::Red, kind: BuildingKind::Barracks },
            Building { x: (rx - 5.0) * ts, y: (ry - 1.0) * ts, faction: Faction::Red, kind: BuildingKind::Archery },
            Building { x: (rx - 2.0) * ts, y: (ry + 1.0) * ts, faction: Faction::Red, kind: BuildingKind::Monastery },
        ];

        Self {
            grid,
            units,
            zones,
            buildings,
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

        self.tick += 1;

        // Cooldowns and animation
        for u in &mut self.units {
            if u.cooldown > 0.0 { u.cooldown -= dt; }
            u.update_anim(dt);
        }

        // Player movement
        self.tick_player(input, dt);

        // AI
        self.tick_ai(dt);

        // Zones
        self.tick_zones(dt);

        // Reinforcements
        self.tick_reinforcements(dt);

        // Remove dead units (keep IDs stable)
        self.units.retain(|u| u.alive || u.id == self.player_id);

        // Player death check
        if !self.units.iter().any(|u| u.id == self.player_id && u.alive) {
            self.phase = GamePhase::Dead;
            return;
        }

        // Victory check
        if self.zones.iter().all(|z| z.owner == Some(Faction::Blue)) {
            self.phase = GamePhase::Victory(Faction::Blue);
        } else if self.zones.iter().all(|z| z.owner == Some(Faction::Red)) {
            self.phase = GamePhase::Victory(Faction::Red);
        }
    }

    fn tick_player(&mut self, input: &InputState, dt: f32) {
        let (dx, dy) = input.move_dir();
        if let Some(player) = self.units.iter_mut().find(|u| u.id == self.player_id) {
            if !player.alive { return; }
            let speed = player.kind.speed() * TILE_SIZE * dt;
            let nx = player.x + dx as f32 * speed;
            let ny = player.y + dy as f32 * speed;
            if self.grid.passable_at(nx, player.y) { player.x = nx; }
            if self.grid.passable_at(player.x, ny) { player.y = ny; }
            if dx > 0 { player.facing = Facing::Right; }
            else if dx < 0 { player.facing = Facing::Left; }

            // Camera follows player
            self.camera_x = player.x - 480.0;
            self.camera_y = player.y - 320.0;

            // Player attack: find nearest enemy in range
            if input.attacking() {
                let px = player.x;
                let py = player.y;
                let pfac = player.faction;
                let range = player.kind.range();
                let pidx = self.units.iter().position(|u| u.id == self.player_id).unwrap();

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

            // Monk: find wounded ally to heal
            if ukind.is_healer() {
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

            // Attack or move toward enemy
            if let Some(ei) = nearest_enemy {
                if nearest_dist <= ukind.range() {
                    combat::resolve_attack(i, ei, &mut self.units, &self.grid);
                } else {
                    let tx = self.units[ei].x;
                    let ty = self.units[ei].y;
                    self.units[i].move_toward(tx, ty, dt, &self.grid);
                }
            } else {
                // March toward enemy base
                let (bx, by) = match ufac {
                    Faction::Blue => ((GRID_SIZE as f32 - 8.0) * TILE_SIZE, (GRID_SIZE as f32 - 8.0) * TILE_SIZE),
                    Faction::Red => (8.0 * TILE_SIZE, 8.0 * TILE_SIZE),
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

    fn tick_reinforcements(&mut self, dt: f32) {
        self.reinforce_timer -= dt;
        if self.reinforce_timer > 0.0 { return; }
        self.reinforce_timer = REINFORCE_INTERVAL;

        for faction in [Faction::Blue, Faction::Red] {
            let count = self.units.iter().filter(|u| u.alive && u.faction == faction).count();
            if count >= MAX_UNITS { continue; }

            let (bx, by) = match faction {
                Faction::Blue => (6.0 * TILE_SIZE, 6.0 * TILE_SIZE),
                Faction::Red => ((GRID_SIZE as f32 - 8.0) * TILE_SIZE, (GRID_SIZE as f32 - 8.0) * TILE_SIZE),
            };

            let wave = [UnitKind::Warrior, UnitKind::Warrior, UnitKind::Archer, UnitKind::Lancer];
            for (i, &kind) in wave.iter().enumerate() {
                let ox = (i % 2) as f32 * TILE_SIZE;
                let oy = (i / 2) as f32 * TILE_SIZE;
                self.units.push(Unit::new(self.next_id, kind, faction, bx + ox, by + oy));
                self.next_id += 1;
            }
        }
    }
}
