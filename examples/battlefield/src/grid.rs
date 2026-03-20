//! Tile grid and terrain system.

pub const GRID_SIZE: usize = 80;
pub const TILE_SIZE: f32 = 64.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileType {
    Grass,
    Water,
    Forest,
    Rock,
    Road,
}

impl TileType {
    pub fn passable(self) -> bool {
        !matches!(self, TileType::Water | TileType::Rock)
    }

    pub fn movement_cost(self) -> f32 {
        match self {
            TileType::Grass => 1.0,
            TileType::Forest => 2.0,
            TileType::Road => 0.8,
            TileType::Water | TileType::Rock => f32::MAX,
        }
    }

    pub fn defense_bonus(self) -> i32 {
        match self {
            TileType::Forest => 1,
            _ => 0,
        }
    }
}

pub struct Grid {
    pub tiles: Vec<TileType>,
    pub width: usize,
    pub height: usize,
}

impl Grid {
    pub fn new() -> Self {
        let w = GRID_SIZE;
        let h = GRID_SIZE;
        let mut tiles = vec![TileType::Grass; w * h];

        // Simple procedural map: water border, forest clusters, road
        let seed = 42u32;
        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;

                // Water border (2-tile margin)
                if x < 2 || y < 2 || x >= w - 2 || y >= h - 2 {
                    tiles[idx] = TileType::Water;
                    continue;
                }

                // Deterministic pseudo-noise for terrain
                let n = noise(x as u32, y as u32, seed);

                // Water bodies
                if n < 0.15 && x > 10 && x < w - 10 && y > 10 && y < h - 10 {
                    tiles[idx] = TileType::Water;
                }
                // Forest clusters
                else if n > 0.6 && n < 0.75 {
                    tiles[idx] = TileType::Forest;
                }
                // Rocks
                else if n > 0.92 {
                    tiles[idx] = TileType::Rock;
                }

                // Central road (horizontal)
                if y >= h / 2 - 1 && y <= h / 2 + 1 && tiles[idx] != TileType::Water {
                    tiles[idx] = TileType::Road;
                }
                // Vertical road
                if x >= w / 2 - 1 && x <= w / 2 + 1 && tiles[idx] != TileType::Water {
                    tiles[idx] = TileType::Road;
                }
            }
        }

        // Ensure spawn areas are clear
        for dy in 0..8 {
            for dx in 0..8 {
                // Blue base (top-left)
                tiles[(4 + dy) * w + (4 + dx)] = TileType::Grass;
                // Red base (bottom-right)
                tiles[(h - 12 + dy) * w + (w - 12 + dx)] = TileType::Grass;
            }
        }

        Self { tiles, width: w, height: h }
    }

    pub fn get(&self, x: usize, y: usize) -> TileType {
        if x >= self.width || y >= self.height {
            return TileType::Water;
        }
        self.tiles[y * self.width + x]
    }

    pub fn tile_at_world(&self, wx: f32, wy: f32) -> TileType {
        let tx = (wx / TILE_SIZE) as usize;
        let ty = (wy / TILE_SIZE) as usize;
        self.get(tx, ty)
    }

    pub fn passable_at(&self, wx: f32, wy: f32) -> bool {
        self.tile_at_world(wx, wy).passable()
    }
}

/// Simple deterministic noise (hash-based).
fn noise(x: u32, y: u32, seed: u32) -> f32 {
    let mut h = seed;
    h ^= x.wrapping_mul(374761393);
    h ^= y.wrapping_mul(668265263);
    h = h.wrapping_mul(h ^ (h >> 15));
    h ^= h >> 13;
    h = h.wrapping_mul(h ^ (h >> 15));
    (h & 0xFFFF) as f32 / 65536.0
}
