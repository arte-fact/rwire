//! Tile grid and terrain system.

pub const GRID_SIZE: usize = 100;
pub const TILE_SIZE: f32 = 64.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileType {
    Grass,
    Water,
    Forest,
    Rock,
}

impl TileType {
    pub fn passable(self) -> bool {
        !matches!(self, TileType::Water | TileType::Rock)
    }

    pub fn defense_bonus(self) -> i32 {
        match self {
            TileType::Forest => 1,
            _ => 0,
        }
    }
}

/// Decorative elements on tiles (separate from tile type).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decoration {
    Bush,
    WaterRock,
}

pub struct Grid {
    pub tiles: Vec<TileType>,
    pub elevation: Vec<u8>,
    pub decorations: Vec<Option<Decoration>>,
    pub width: usize,
    pub height: usize,
}

impl Grid {
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

    pub fn decoration(&self, x: usize, y: usize) -> Option<Decoration> {
        if x >= self.width || y >= self.height { return None; }
        self.decorations[y * self.width + x]
    }

    pub fn tile_at_world(&self, wx: f32, wy: f32) -> TileType {
        let tx = (wx / TILE_SIZE) as usize;
        let ty = (wy / TILE_SIZE) as usize;
        self.get(tx, ty)
    }

    pub fn passable_at(&self, wx: f32, wy: f32) -> bool {
        self.tile_at_world(wx, wy).passable()
            && self.elev_at_world(wx, wy) <= 1
    }

    fn elev_at_world(&self, wx: f32, wy: f32) -> u8 {
        let tx = (wx / TILE_SIZE) as usize;
        let ty = (wy / TILE_SIZE) as usize;
        self.elev(tx, ty)
    }

    /// Circle passability check — tests 9 points around the circle perimeter.
    /// Matches original: center + 4 cardinal + 4 diagonal at unit radius.
    pub fn is_circle_passable(&self, wx: f32, wy: f32, radius: f32) -> bool {
        let d = radius * 0.707; // cos(45°)
        let points = [
            (wx, wy),
            (wx + radius, wy), (wx - radius, wy),
            (wx, wy + radius), (wx, wy - radius),
            (wx + d, wy + d), (wx + d, wy - d),
            (wx - d, wy + d), (wx - d, wy - d),
        ];
        points.iter().all(|&(px, py)| self.passable_at(px, py))
    }

    /// Speed multiplier at world position.
    pub fn speed_factor_at(&self, wx: f32, wy: f32) -> f32 {
        let tx = (wx / TILE_SIZE) as usize;
        let ty = (wy / TILE_SIZE) as usize;
        match self.get(tx, ty) {
            TileType::Forest => 0.5,
            TileType::Rock => 0.75,
            TileType::Grass => {
                if self.decoration(tx, ty) == Some(Decoration::Bush) { 0.75 } else { 1.0 }
            }
            TileType::Water => 0.0,
        }
    }
}
