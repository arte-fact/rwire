//! Auto-tiling: pick the correct tilemap sub-tile based on neighbors.
//!
//! Uses a 4-bit cardinal bitmask (N=1, E=2, S=4, W=8) to select
//! from a 4×4 tile pattern in the tilemap sprite sheet.

use crate::grid::{Grid, TileType, GRID_SIZE};

const N: u8 = 1;
const E: u8 = 2;
const S: u8 = 4;
const W: u8 = 8;

/// Cardinal bitmask → tilemap (col, row) for flat ground tiles.
/// Cols 0-3, rows 0-3 of the tilemap texture.
const FLAT_GROUND: [(u16, u16); 16] = [
    (3, 3), //  0: isolated
    (3, 2), //  1: N only
    (0, 3), //  2: E only
    (0, 2), //  3: N+E
    (3, 0), //  4: S only
    (3, 1), //  5: N+S
    (0, 0), //  6: E+S
    (0, 1), //  7: N+E+S
    (2, 3), //  8: W only
    (2, 2), //  9: N+W
    (1, 3), // 10: E+W
    (1, 2), // 11: N+E+W
    (2, 0), // 12: S+W
    (2, 1), // 13: N+S+W
    (1, 0), // 14: E+S+W
    (1, 1), // 15: all → center fill
];

fn cardinal_mask(grid: &Grid, x: usize, y: usize, is_same: impl Fn(usize, usize) -> bool) -> u8 {
    let mut mask = 0u8;
    if y == 0 || is_same(x, y - 1) { mask |= N; }
    if x + 1 >= GRID_SIZE || is_same(x + 1, y) { mask |= E; }
    if y + 1 >= GRID_SIZE || is_same(x, y + 1) { mask |= S; }
    if x == 0 || is_same(x - 1, y) { mask |= W; }
    mask
}

/// Get the tilemap source rect (sx, sy) in pixels for a ground tile.
/// Returns (sx, sy) where each tile is 64×64 in the tilemap texture.
pub fn flat_ground_src(grid: &Grid, x: usize, y: usize) -> (u16, u16) {
    let mask = cardinal_mask(grid, x, y, |nx, ny| {
        let t = grid.get(nx, ny);
        t != TileType::Water && t != TileType::Rock
    });
    let (col, row) = FLAT_GROUND[mask as usize];
    (col * 64, row * 64)
}

/// Get the tilemap source rect for a road tile (uses tilemap2).
pub fn road_src(grid: &Grid, x: usize, y: usize) -> (u16, u16) {
    let mask = cardinal_mask(grid, x, y, |nx, ny| {
        grid.get(nx, ny) == TileType::Road
    });
    let (col, row) = FLAT_GROUND[mask as usize];
    (col * 64, row * 64)
}
