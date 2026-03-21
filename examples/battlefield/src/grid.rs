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

pub struct Grid {
    pub tiles: Vec<TileType>,
    pub elevation: Vec<u8>, // 0 = flat, 1 = elevated
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

    pub fn tile_at_world(&self, wx: f32, wy: f32) -> TileType {
        let tx = (wx / TILE_SIZE) as usize;
        let ty = (wy / TILE_SIZE) as usize;
        self.get(tx, ty)
    }

    pub fn passable_at(&self, wx: f32, wy: f32) -> bool {
        self.tile_at_world(wx, wy).passable()
    }
}
