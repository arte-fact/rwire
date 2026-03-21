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
    pub death_fade: f32,    // 0.0 = alive, counts up to DEATH_FADE_DURATION then removed
    pub hit_flash: f32,     // counts down from 0.3 on hit
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
        }
    }

    /// Update animation state based on movement.
    pub fn update_anim(&mut self, dt: f32) {
        let dx = self.x - self.prev_x;
        let dy = self.y - self.prev_y;
        let moved = (dx * dx + dy * dy).sqrt() > 0.5;

        let new_anim = if self.cooldown > self.kind.attack_cooldown() - 0.3 {
            UnitAnim::Attack
        } else if moved {
            UnitAnim::Run
        } else {
            UnitAnim::Idle
        };

        if new_anim != self.anim {
            self.anim = new_anim;
            self.anim_frame = 0;
            self.anim_timer = 0.0;
        }

        self.anim_timer += dt;
        let fps = 10.0;
        if self.anim_timer >= 1.0 / fps {
            self.anim_timer -= 1.0 / fps;
            let max_frames = self.anim_frame_count();
            self.anim_frame = (self.anim_frame + 1) % max_frames;
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

    pub fn move_toward(&mut self, tx: f32, ty: f32, dt: f32, grid: &Grid) {
        let dx = tx - self.x;
        let dy = ty - self.y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < 1.0 { return; }

        let speed = self.kind.speed() * TILE_SIZE * dt;
        let nx = self.x + dx / dist * speed;
        let ny = self.y + dy / dist * speed;

        if grid.passable_at(nx, self.y) {
            self.x = nx;
        }
        if grid.passable_at(self.x, ny) {
            self.y = ny;
        }

        if dx > 0.0 { self.facing = Facing::Right; }
        else if dx < 0.0 { self.facing = Facing::Left; }
    }
}
