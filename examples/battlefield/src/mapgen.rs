//! Procedural map generation — ported from the original game.
//!
//! Uses simplex noise for terrain heightmap, cellular automata for
//! forest clustering, and BSP for strategic zone placement.

use crate::grid::{Decoration, Grid, TileType, GRID_SIZE, BORDER_SIZE, PLAYABLE_SIZE};

// ============================================================================
// Simplex noise (ported from the-battlefield/src/mapgen/simplex.rs)
// ============================================================================

const F2: f64 = 0.3660254037844386;
const G2: f64 = 0.21132486540518713;

const GRAD2: [(f64, f64); 12] = [
    (1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0),
    (1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0),
    (1.0, 0.5), (-1.0, 0.5), (0.5, 1.0), (-0.5, 1.0),
];

struct Simplex { perm: [u8; 512] }

impl Simplex {
    fn new(seed: u64) -> Self {
        use rand::SeedableRng;
        use rand::seq::SliceRandom;
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
        let mut perm_base: Vec<u8> = (0..=255).collect();
        perm_base.shuffle(&mut rng);
        let mut perm = [0u8; 512];
        for i in 0..512 { perm[i] = perm_base[i & 255]; }
        Self { perm }
    }

    fn get(&self, x: f64, y: f64) -> f64 {
        let s = (x + y) * F2;
        let i = (x + s).floor();
        let j = (y + s).floor();
        let t = (i + j) * G2;
        let x0 = x - (i - t);
        let y0 = y - (j - t);
        let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };
        let x1 = x0 - i1 as f64 + G2;
        let y1 = y0 - j1 as f64 + G2;
        let x2 = x0 - 1.0 + 2.0 * G2;
        let y2 = y0 - 1.0 + 2.0 * G2;
        let ii = (i as i32 & 255) as usize;
        let jj = (j as i32 & 255) as usize;
        let mut n = 0.0;
        let t0 = 0.5 - x0 * x0 - y0 * y0;
        if t0 > 0.0 {
            let t0 = t0 * t0;
            let gi = self.perm[ii + self.perm[jj] as usize] as usize % 12;
            n += t0 * t0 * (GRAD2[gi].0 * x0 + GRAD2[gi].1 * y0);
        }
        let t1 = 0.5 - x1 * x1 - y1 * y1;
        if t1 > 0.0 {
            let t1 = t1 * t1;
            let gi = self.perm[ii + i1 + self.perm[jj + j1] as usize] as usize % 12;
            n += t1 * t1 * (GRAD2[gi].0 * x1 + GRAD2[gi].1 * y1);
        }
        let t2 = 0.5 - x2 * x2 - y2 * y2;
        if t2 > 0.0 {
            let t2 = t2 * t2;
            let gi = self.perm[ii + 1 + self.perm[jj + 1] as usize] as usize % 12;
            n += t2 * t2 * (GRAD2[gi].0 * x2 + GRAD2[gi].1 * y2);
        }
        70.0 * n
    }

    fn octave(&self, x: f64, y: f64, octaves: u32, persistence: f64) -> f64 {
        let mut total = 0.0;
        let mut freq = 1.0;
        let mut amp = 1.0;
        let mut max_val = 0.0;
        for _ in 0..octaves {
            total += self.get(x * freq, y * freq) * amp;
            max_val += amp;
            amp *= persistence;
            freq *= 2.0;
        }
        total / max_val
    }
}

// ============================================================================
// xorshift32 RNG
// ============================================================================

struct Rng { state: u32 }

impl Rng {
    fn new(seed: u32) -> Self { Self { state: if seed == 0 { 1 } else { seed } } }
    fn next(&mut self) -> u32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        self.state
    }
    fn f32(&mut self) -> f32 { (self.next() & 0x00FF_FFFF) as f32 / 16_777_216.0 }
    fn chance(&mut self, p: f32) -> bool { self.f32() < p }
}

// ============================================================================
// BSP layout
// ============================================================================

#[derive(Clone, Copy)]
struct Rect { x: u32, y: u32, w: u32, h: u32 }

impl Rect {
    fn center(&self) -> (u32, u32) { (self.x + self.w / 2, self.y + self.h / 2) }
}

fn bsp_split(rng: &mut Rng, rect: Rect, depth: u32, max_depth: u32, min_size: u32) -> Vec<Rect> {
    if depth >= max_depth || (rect.w < min_size * 2 && rect.h < min_size * 2) {
        return vec![rect];
    }
    let split_h = if rect.w > rect.h + 4 { false }
        else if rect.h > rect.w + 4 { true }
        else { rng.chance(0.5) };

    if split_h {
        if rect.h < min_size * 2 { return vec![rect]; }
        let range = rect.h - 2 * min_size;
        let off = min_size + (rng.next() % (range + 1));
        let top = Rect { x: rect.x, y: rect.y, w: rect.w, h: off };
        let bot = Rect { x: rect.x, y: rect.y + off, w: rect.w, h: rect.h - off };
        let mut l = bsp_split(rng, top, depth + 1, max_depth, min_size);
        l.extend(bsp_split(rng, bot, depth + 1, max_depth, min_size));
        l
    } else {
        if rect.w < min_size * 2 { return vec![rect]; }
        let range = rect.w - 2 * min_size;
        let off = min_size + (rng.next() % (range + 1));
        let left = Rect { x: rect.x, y: rect.y, w: off, h: rect.h };
        let right = Rect { x: rect.x + off, y: rect.y, w: rect.w - off, h: rect.h };
        let mut l = bsp_split(rng, left, depth + 1, max_depth, min_size);
        l.extend(bsp_split(rng, right, depth + 1, max_depth, min_size));
        l
    }
}

// ============================================================================
// Cellular automata
// ============================================================================

fn seed_from_noise(noise: &Simplex, w: u32, h: u32, offset: f64, freq: f64, threshold: f64) -> Vec<bool> {
    let mut grid = vec![false; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let val = noise.octave(x as f64 * freq + offset, y as f64 * freq + offset, 3, 0.5);
            grid[(y * w + x) as usize] = (val + 1.0) * 0.5 < threshold;
        }
    }
    grid
}

fn count_neighbors(grid: &[bool], w: u32, h: u32, x: u32, y: u32) -> u32 {
    let mut count = 0;
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            if dx == 0 && dy == 0 { continue; }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx >= 0 && ny >= 0 && (nx as u32) < w && (ny as u32) < h {
                if grid[(ny as u32 * w + nx as u32) as usize] { count += 1; }
            } else {
                count += 1; // out of bounds counts as alive
            }
        }
    }
    count
}

fn run_cellular_automaton(initial: &[bool], w: u32, h: u32, iterations: u32, birth: u32, death: u32) -> Vec<bool> {
    let size = (w * h) as usize;
    let mut current = initial.to_vec();
    let mut next = vec![false; size];
    for _ in 0..iterations {
        for y in 0..h {
            for x in 0..w {
                let n = count_neighbors(&current, w, h, x, y);
                let i = (y * w + x) as usize;
                next[i] = if current[i] { n >= death } else { n >= birth };
            }
        }
        std::mem::swap(&mut current, &mut next);
    }
    current
}

// ============================================================================
// Map layout result
// ============================================================================

pub struct MapLayout {
    pub blue_base: (u32, u32),
    pub red_base: (u32, u32),
    pub zone_centers: Vec<(u32, u32)>,
}

// ============================================================================
// Main generation function
// ============================================================================

pub fn generate_battlefield(seed: u32) -> (Grid, MapLayout) {
    let mut rng = Rng::new(seed);
    let w = GRID_SIZE as u32;
    let h = GRID_SIZE as u32;
    let noise = Simplex::new(seed as u64);

    let mut tiles = vec![TileType::Grass; (w * h) as usize];
    let mut elevation = vec![0u8; (w * h) as usize];

    let border = BORDER_SIZE as u32;

    // Phase A: Simplex noise heightmap
    let elev_scale = 0.04;
    let water_threshold = -0.3;
    let hill_threshold = 0.45;

    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;

            // Border region: organic fill with noise-driven forests/rocks/water
            let in_border = x < border || y < border || x >= w - border || y >= h - border;
            if in_border {
                let val = noise.octave(x as f64 * 0.06, y as f64 * 0.06, 3, 0.5);
                if val < -0.2 {
                    tiles[i] = TileType::Water;
                } else if val < 0.1 {
                    tiles[i] = TileType::Forest;
                } else if val > 0.4 {
                    tiles[i] = TileType::Rock;
                    elevation[i] = 2;
                } else {
                    tiles[i] = TileType::Forest;
                }
                continue;
            }

            let val = noise.octave(x as f64 * elev_scale, y as f64 * elev_scale, 4, 0.5);

            // Edge bias: more water/forest near border transition
            let dx = (x as f64 - border as f64).min((w - 1 - border - x) as f64);
            let dy = (y as f64 - border as f64).min((h - 1 - border - y) as f64);
            let edge_dist = dx.min(dy) / 8.0; // normalize over 8 tiles
            let edge_bias = if edge_dist < 1.0 {
                let t = 1.0 - edge_dist.max(0.0);
                -t * t * 0.3 // push toward water threshold near edges
            } else {
                0.0
            };

            let effective = val + edge_bias;

            if effective < water_threshold {
                tiles[i] = TileType::Water;
            } else if effective > hill_threshold {
                elevation[i] = 2; // elevation 2 = impassable (matching original)
            }
        }
    }

    // Phase B: Cellular automata for forests
    let tree_seed = run_cellular_automaton(
        &seed_from_noise(&noise, w, h, 100.0, 0.07, 0.3),
        w, h, 5, 4, 2,
    );
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if tree_seed[i] && tiles[i] == TileType::Grass && elevation[i] == 0 {
                tiles[i] = TileType::Forest;
            }
        }
    }

    // Rocks: sparse random scatter
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if rng.chance(0.03) && tiles[i] == TileType::Grass && elevation[i] == 0 {
                tiles[i] = TileType::Rock;
            }
        }
    }

    // Phase C: BSP layout for zones and bases
    let playable_start = border;
    let playable_size = PLAYABLE_SIZE as u32;
    let playable_rect = Rect { x: playable_start, y: playable_start, w: playable_size, h: playable_size };
    let mut bsp_rng = Rng::new(seed.wrapping_add(0xBEEF));
    let leaves = bsp_split(&mut bsp_rng, playable_rect, 0, 4, 20);

    // Sort leaves by distance to top-left to assign bases
    let top_left = (border as f32, border as f32);
    let mut sorted = leaves.clone();
    sorted.sort_by(|a, b| {
        let (ax, ay) = a.center();
        let (bx, by) = b.center();
        let da = (ax as f32 - top_left.0).powi(2) + (ay as f32 - top_left.1).powi(2);
        let db = (bx as f32 - top_left.0).powi(2) + (by as f32 - top_left.1).powi(2);
        da.partial_cmp(&db).unwrap()
    });

    let blue_base = sorted[0].center();
    let red_base = sorted[sorted.len() - 1].center();

    // 5 zones: 3 diagonal between bases + 2 flanks
    let (bx, by) = (blue_base.0 as f32, blue_base.1 as f32);
    let (rx, ry) = (red_base.0 as f32, red_base.1 as f32);
    let ddx = rx - bx;
    let ddy = ry - by;
    let diag: Vec<(u32, u32)> = [0.25f32, 0.50, 0.75].iter()
        .map(|&t| ((bx + ddx * t) as u32, (by + ddy * t) as u32))
        .collect();
    let perp_x = -ddy * 0.25;
    let perp_y = ddx * 0.25;
    let mid_x = (bx + rx) * 0.5;
    let mid_y = (by + ry) * 0.5;
    let flank1 = ((mid_x + perp_x) as u32, (mid_y + perp_y) as u32);
    let flank2 = ((mid_x - perp_x) as u32, (mid_y - perp_y) as u32);
    let mut zone_centers = diag;
    zone_centers.push(flank1);
    zone_centers.push(flank2);

    // Clear areas around bases and zones (before decoration pass)
    let mut decos_dummy: Vec<Option<Decoration>> = vec![None; (w * h) as usize];
    clear_rect(&mut tiles, &mut elevation, &mut decos_dummy, blue_base.0.saturating_sub(7), blue_base.1.saturating_sub(7), 14, 14, w);
    clear_rect(&mut tiles, &mut elevation, &mut decos_dummy, red_base.0.saturating_sub(7), red_base.1.saturating_sub(7), 14, 14, w);
    for &(cx, cy) in &zone_centers {
        clear_circle(&mut tiles, &mut elevation, &mut decos_dummy, cx, cy, 6, w, h);
    }

    // Generate decorations AFTER clearing (so cleared areas have no decorations)
    let mut decorations = vec![None; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            match tiles[i] {
                TileType::Grass if elevation[i] == 0 && rng.chance(0.04) => {
                    decorations[i] = Some(Decoration::Bush);
                }
                TileType::Water if rng.chance(0.12) => {
                    decorations[i] = Some(Decoration::WaterRock);
                }
                _ => {}
            }
        }
    }

    let building_occupied = vec![false; (w * h) as usize];
    let grid = Grid { tiles, elevation, decorations, building_occupied, width: w as usize, height: h as usize };
    let layout = MapLayout { blue_base, red_base, zone_centers };
    (grid, layout)
}

#[allow(clippy::too_many_arguments)]
fn clear_rect(tiles: &mut [TileType], elev: &mut [u8], decos: &mut [Option<Decoration>], x: u32, y: u32, rw: u32, rh: u32, w: u32) {
    for dy in 0..rh {
        for dx in 0..rw {
            let nx = x + dx;
            let ny = y + dy;
            let i = (ny * w + nx) as usize;
            if i < tiles.len() {
                tiles[i] = TileType::Grass;
                elev[i] = 0;
                decos[i] = None;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn clear_circle(tiles: &mut [TileType], elev: &mut [u8], decos: &mut [Option<Decoration>], cx: u32, cy: u32, radius: i32, w: u32, h: u32) {
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy > radius * radius { continue; }
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if nx >= 0 && ny >= 0 && (nx as u32) < w && (ny as u32) < h {
                let i = (ny as u32 * w + nx as u32) as usize;
                tiles[i] = TileType::Grass;
                elev[i] = 0;
                decos[i] = None;
            }
        }
    }
}
