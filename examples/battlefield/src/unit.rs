//! Unit types, stats, and state.

use crate::grid::{Grid, TILE_SIZE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Faction {
    Blue,
    Red,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    Warrior,
    Archer,
    Lancer,
    Monk,
}

impl UnitKind {
    pub fn max_hp(self) -> i32 {
        match self { UnitKind::Warrior => 10, UnitKind::Archer => 6, UnitKind::Lancer => 10, UnitKind::Monk => 5 }
    }
    pub fn attack(self) -> i32 {
        match self { UnitKind::Warrior => 3, UnitKind::Archer => 2, UnitKind::Lancer => 4, UnitKind::Monk => 1 }
    }
    pub fn defense(self) -> i32 {
        match self { UnitKind::Warrior => 3, UnitKind::Archer => 1, UnitKind::Lancer => 1, UnitKind::Monk => 1 }
    }
    pub fn speed(self) -> f32 {
        match self { UnitKind::Warrior => 5.0, UnitKind::Archer => 4.0, UnitKind::Lancer => 4.0, UnitKind::Monk => 3.0 }
    }
    pub fn range(self) -> f32 {
        match self { UnitKind::Warrior => 1.5, UnitKind::Archer => 7.0, UnitKind::Lancer => 2.0, UnitKind::Monk => 2.0 }
    }
    pub fn attack_cooldown(self) -> f32 {
        match self { UnitKind::Warrior => 0.6, UnitKind::Archer => 0.8, UnitKind::Lancer => 0.5, UnitKind::Monk => 0.7 }
    }
    pub fn is_healer(self) -> bool {
        matches!(self, UnitKind::Monk)
    }
    pub fn is_ranged(self) -> bool {
        matches!(self, UnitKind::Archer)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Facing {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitAnim {
    Idle,
    Run,
    Attack,
}

/// Player orders that can be issued to nearby friendly units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderKind {
    Hold,    // defend current position
    Go,      // advance toward nearest zone
    Retreat, // fall back toward base
    Follow,  // stay near player
}

pub struct Unit {
    pub id: u32,
    pub kind: UnitKind,
    pub faction: Faction,
    pub x: f32,
    pub y: f32,
    pub prev_x: f32,
    pub prev_y: f32,
    pub hp: i32,
    pub facing: Facing,
    pub alive: bool,
    pub cooldown: f32,
    pub anim: UnitAnim,
    pub anim_frame: u16,
    pub anim_timer: f32,
    pub death_fade: f32,
    pub hit_flash: f32,
    pub order: Option<OrderKind>,
    pub order_flash: f32, // visual flash when order received
}

pub const DEATH_FADE_DURATION: f32 = 0.3; // matches original

impl Unit {
    pub fn new(id: u32, kind: UnitKind, faction: Faction, x: f32, y: f32) -> Self {
        Self {
            id, kind, faction, x, y,
            prev_x: x, prev_y: y,
            hp: kind.max_hp(),
            facing: if faction == Faction::Blue { Facing::Right } else { Facing::Left },
            alive: true,
            cooldown: 0.0,
            anim: UnitAnim::Idle,
            anim_frame: 0,
            anim_timer: 0.0,
            death_fade: 0.0,
            hit_flash: 0.0,
            order: None,
            order_flash: 0.0,
        }
    }

    /// Trigger attack animation (called when an attack is performed).
    pub fn play_attack_anim(&mut self) {
        if self.anim != UnitAnim::Attack {
            self.anim = UnitAnim::Attack;
            self.anim_frame = 0;
            self.anim_timer = 0.0;
        }
    }

    /// Update animation state based on movement.
    pub fn update_anim(&mut self, dt: f32) {
        let dx = self.x - self.prev_x;
        let dy = self.y - self.prev_y;
        let moved = (dx * dx + dy * dy).sqrt() > 0.5;

        // If attack animation is playing, let it finish all frames before transitioning
        if self.anim == UnitAnim::Attack {
            self.anim_timer += dt;
            let fps = 12.0;
            if self.anim_timer >= 1.0 / fps {
                self.anim_timer -= 1.0 / fps;
                let max_frames = self.anim_frame_count();
                self.anim_frame += 1;
                if self.anim_frame >= max_frames {
                    // Attack animation finished — transition to idle/run
                    self.anim_frame = 0;
                    self.anim = if moved { UnitAnim::Run } else { UnitAnim::Idle };
                    self.anim_timer = 0.0;
                }
            }
        } else {
            let new_anim = if moved { UnitAnim::Run } else { UnitAnim::Idle };
            if new_anim != self.anim {
                self.anim = new_anim;
                self.anim_frame = 0;
                self.anim_timer = 0.0;
            }
            self.anim_timer += dt;
            let fps = match self.anim {
                UnitAnim::Idle => 10.0,
                UnitAnim::Run => 12.0,
                UnitAnim::Attack => 12.0,
            };
            if self.anim_timer >= 1.0 / fps {
                self.anim_timer -= 1.0 / fps;
                let max_frames = self.anim_frame_count();
                self.anim_frame = (self.anim_frame + 1) % max_frames;
            }
        }

        self.prev_x = self.x;
        self.prev_y = self.y;
    }

    fn anim_frame_count(&self) -> u16 {
        use crate::sprites;
        let a = match self.anim {
            UnitAnim::Idle => sprites::unit_idle(self.faction, self.kind),
            UnitAnim::Run => sprites::unit_run(self.faction, self.kind),
            UnitAnim::Attack => sprites::unit_attack(self.faction, self.kind),
        };
        a.frame_count
    }

    pub fn distance_to(&self, other: &Unit) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt() / TILE_SIZE
    }

    pub fn can_attack(&self) -> bool {
        self.alive && self.cooldown <= 0.0
    }

    pub fn in_range(&self, target: &Unit) -> bool {
        self.distance_to(target) <= self.kind.range()
    }

    /// Move speed in pixels/sec (matching original formula).
    pub fn move_speed(&self) -> f32 {
        TILE_SIZE * self.kind.speed() / 0.9
    }

    pub fn move_toward(&mut self, tx: f32, ty: f32, dt: f32, grid: &Grid) {
        let dx = tx - self.x;
        let dy = ty - self.y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < 1.0 { return; }

        let dir_x = dx / dist;
        let dir_y = dy / dist;
        self.move_dir(dir_x, dir_y, dt, grid);
    }

    /// Move in a direction with split-axis circle collision (matching original).
    pub fn move_dir(&mut self, dir_x: f32, dir_y: f32, dt: f32, grid: &Grid) {
        self.move_dir_opts(dir_x, dir_y, dt, grid, true);
    }

    /// Move in a direction. If `update_facing` is false, facing is not changed
    /// (used for the player, whose facing is controlled by aim direction).
    pub fn move_dir_opts(&mut self, dir_x: f32, dir_y: f32, dt: f32, grid: &Grid, update_facing: bool) {
        let speed = self.move_speed()
            * grid.speed_factor_at(self.x, self.y).max(0.25)
            * dt;
        let vx = dir_x * speed;
        let vy = dir_y * speed;

        let radius = 28.0f32;

        // Split-axis collision
        let nx = self.x + vx;
        if grid.is_circle_passable(nx, self.y, radius) {
            self.x = nx;
        }
        let ny = self.y + vy;
        if grid.is_circle_passable(self.x, ny, radius) {
            self.y = ny;
        }

        if update_facing {
            if dir_x > 0.01 { self.facing = Facing::Right; }
            else if dir_x < -0.01 { self.facing = Facing::Left; }
        }
    }
}
