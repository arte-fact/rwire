//! Particle effects — dust clouds and explosions.

#[derive(Clone, Copy, Debug)]
pub enum ParticleKind {
    Dust,           // 8 frames, 64x64
    ExplosionLarge, // 10 frames, 192x192 (matching original)
}

impl ParticleKind {
    pub fn frame_count(self) -> u16 {
        match self {
            ParticleKind::Dust => 8,
            ParticleKind::ExplosionLarge => 10,
        }
    }

    pub fn fps(self) -> f32 {
        15.0
    }
}

pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub kind: ParticleKind,
    pub frame: u16,
    pub timer: f32,
    pub finished: bool,
}

impl Particle {
    pub fn new(x: f32, y: f32, kind: ParticleKind) -> Self {
        Self { x, y, kind, frame: 0, timer: 0.0, finished: false }
    }

    pub fn update(&mut self, dt: f32) {
        self.timer += dt;
        let frame_dur = 1.0 / self.kind.fps();
        if self.timer >= frame_dur {
            self.timer -= frame_dur;
            self.frame += 1;
            if self.frame >= self.kind.frame_count() {
                self.finished = true;
            }
        }
    }
}
