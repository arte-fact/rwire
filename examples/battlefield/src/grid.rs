//! Tile grid and terrain system.

pub const GRID_SIZE: usize = 120;
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
    pub elevation: Vec<u8>, // 0 = flat, 1 = elevated
    pub width: usize,
    pub height: usize,
}

impl Grid {
    pub fn new() -> Self {
        let w = GRID_SIZE;
        let h = GRID_SIZE;
        let mut tiles = vec![TileType::Grass; w * h];

        let seed = 42u32;

        // Multi-octave noise for organic terrain
        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;

                // Water border
                if x < 3 || y < 3 || x >= w - 3 || y >= h - 3 {
                    tiles[idx] = TileType::Water;
                    continue;
                }

                // Blend two noise scales for variety
                let n1 = noise(x as u32, y as u32, seed);
                let n2 = noise(x as u32 / 2, y as u32 / 2, seed.wrapping_add(1000));
                let n = n1 * 0.6 + n2 * 0.4;

                // Distance from center for radial falloff
                let cx = w as f32 / 2.0;
                let cy = h as f32 / 2.0;
                let dist = (((x as f32 - cx) / cx).powi(2) + ((y as f32 - cy) / cy).powi(2)).sqrt();

                // Water bodies (more likely near edges, less at center)
                let water_threshold = 0.12 + dist * 0.08;
                if n < water_threshold && x > 8 && x < w - 8 && y > 8 && y < h - 8 {
                    tiles[idx] = TileType::Water;
                }
                // Forest clusters
                else if n > 0.55 && n < 0.72 {
                    tiles[idx] = TileType::Forest;
                }
                // Dense forest
                else if n > 0.72 && n < 0.78 && n1 > 0.4 {
                    tiles[idx] = TileType::Forest;
                }
                // Rocks (sparse)
                else if n > 0.90 {
                    tiles[idx] = TileType::Rock;
                }
            }
        }

        // Roads: diagonal paths connecting bases through center
        let mid = w / 2;
        for i in 0..w {
            // Main diagonal road (blue base → red base)
            let ry = i;
            let rx = i;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nx = (rx as i32 + dx).clamp(0, w as i32 - 1) as usize;
                    let ny = (ry as i32 + dy).clamp(0, h as i32 - 1) as usize;
                    if tiles[ny * w + nx] != TileType::Water {
                        tiles[ny * w + nx] = TileType::Road;
                    }
                }
            }
            // Cross road through center
            if i > mid - 2 && i < mid + 2 {
                for j in 0..w {
                    if tiles[i * w + j] != TileType::Water { tiles[i * w + j] = TileType::Road; }
                    if tiles[j * w + i] != TileType::Water { tiles[j * w + i] = TileType::Road; }
                }
            }
        }

        // Clear spawn areas (10×10 around each base)
        for dy in 0..12 {
            for dx in 0..12 {
                let bi = (6 + dy) * w + (6 + dx);
                if bi < tiles.len() { tiles[bi] = TileType::Grass; }
                let ri = (h - 18 + dy) * w + (w - 18 + dx);
                if ri < tiles.len() { tiles[ri] = TileType::Grass; }
            }
        }

        // Clear zone areas (radius 6 around each zone center)
        let zone_centers = [
            (mid, mid),
            (mid - 20, mid - 10),
            (mid + 20, mid + 10),
            (mid - 10, mid + 15),
            (mid + 10, mid - 15),
        ];
        for &(zx, zy) in &zone_centers {
            for dy in -6i32..=6 {
                for dx in -6i32..=6 {
                    if dx * dx + dy * dy > 36 { continue; }
                    let nx = (zx as i32 + dx).clamp(0, w as i32 - 1) as usize;
                    let ny = (zy as i32 + dy).clamp(0, h as i32 - 1) as usize;
                    tiles[ny * w + nx] = TileType::Grass;
                }
            }
        }

        // Generate elevation: rock tiles become elevated terrain
        let mut elevation = vec![0u8; w * h];
        for y in 0..h {
            for x in 0..w {
                if tiles[y * w + x] == TileType::Rock {
                    elevation[y * w + x] = 1;
                    // Make rock tiles into elevated grass
                    tiles[y * w + x] = TileType::Grass;
                }
            }
        }

        Self { tiles, elevation, width: w, height: h }
    }

    pub fn get(&self, x: usize, y: usize) -> TileType {
        if x >= self.width || y >= self.height {
            return TileType::Water;
        }
        self.tiles[y * self.width + x]
    }

    pub fn elev(&self, x: usize, y: usize) -> u8 {
        if x >= self.width || y >= self.height { return 0; }
        self.elevation[y * self.width + x]
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
