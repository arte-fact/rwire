//! Flow field pathfinding — Dijkstra-based direction grid.
//!
//! Computes a grid of direction vectors pointing toward a goal,
//! enabling efficient O(1) per-unit navigation.

use crate::grid::{Grid, TileType, GRID_SIZE, TILE_SIZE};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// 8-directional neighbors: (dx, dy, cost).
/// Cardinal = 2, diagonal = 3.
const DIRS: [(i32, i32, u32); 8] = [
    (0, -1, 2), (1, 0, 2), (0, 1, 2), (-1, 0, 2),
    (1, -1, 3), (1, 1, 3), (-1, 1, 3), (-1, -1, 3),
];

pub struct FlowField {
    directions: Vec<(i8, i8)>,
    width: usize,
    height: usize,
}

impl FlowField {
    /// Generate a flow field from goal position using Dijkstra.
    pub fn generate(grid: &Grid, goal_x: usize, goal_y: usize) -> Self {
        let w = grid.width;
        let h = grid.height;
        let size = w * h;
        let idx = |x: usize, y: usize| y * w + x;

        let mut integration = vec![u32::MAX; size];
        let mut directions = vec![(0i8, 0i8); size];

        if goal_x >= w || goal_y >= h || !grid.get(goal_x, goal_y).passable() {
            return Self { directions, width: w, height: h };
        }

        // Dijkstra from goal outward
        integration[idx(goal_x, goal_y)] = 0;
        let mut heap: BinaryHeap<Reverse<(u32, usize, usize)>> = BinaryHeap::new();
        heap.push(Reverse((0, goal_x, goal_y)));

        while let Some(Reverse((cost, x, y))) = heap.pop() {
            if cost > integration[idx(x, y)] { continue; }

            for &(dx, dy, dir_cost) in &DIRS {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h { continue; }
                let nx = nx as usize;
                let ny = ny as usize;
                if !grid.get(nx, ny).passable() { continue; }
                // Diagonal corner-cutting check
                if dx != 0 && dy != 0
                    && (!grid.get(x, ny).passable() || !grid.get(nx, y).passable())
                {
                    continue;
                }
                let tile_cost = match grid.get(nx, ny) {
                    TileType::Forest => 2,
                    _ => 1,
                };
                let new_cost = cost + tile_cost * dir_cost;
                let ni = idx(nx, ny);
                if new_cost < integration[ni] {
                    integration[ni] = new_cost;
                    heap.push(Reverse((new_cost, nx, ny)));
                }
            }
        }

        // Build direction field: each cell points toward lowest-cost neighbor
        for y in 0..h {
            for x in 0..w {
                let ci = idx(x, y);
                if integration[ci] == u32::MAX || (x == goal_x && y == goal_y) { continue; }
                let mut best_cost = integration[ci];
                let mut best_dir = (0i8, 0i8);
                for &(dx, dy, _) in &DIRS {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h { continue; }
                    let ni = idx(nx as usize, ny as usize);
                    if integration[ni] < best_cost {
                        best_cost = integration[ni];
                        best_dir = (dx as i8, dy as i8);
                    }
                }
                directions[ci] = best_dir;
            }
        }

        Self { directions, width: w, height: h }
    }

    /// O(1) direction lookup. Returns (0,0) if unreachable or at goal.
    pub fn direction_at(&self, gx: usize, gy: usize) -> (i8, i8) {
        if gx >= self.width || gy >= self.height { return (0, 0); }
        self.directions[gy * self.width + gx]
    }

    /// Get the world-space movement direction for a unit at (wx, wy).
    pub fn world_direction(&self, wx: f32, wy: f32) -> (f32, f32) {
        let gx = (wx / TILE_SIZE) as usize;
        let gy = (wy / TILE_SIZE) as usize;
        let (dx, dy) = self.direction_at(gx, gy);
        if dx == 0 && dy == 0 { return (0.0, 0.0); }
        let len = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();
        (dx as f32 / len, dy as f32 / len)
    }
}

/// Cached flow field state per faction.
pub struct FactionFlow {
    pub field: Option<FlowField>,
    goal: (usize, usize),
}

impl FactionFlow {
    pub fn new() -> Self {
        Self { field: None, goal: (0, 0) }
    }

    /// Update the flow field if the goal has changed significantly.
    pub fn update_if_needed(&mut self, grid: &Grid, goal_wx: f32, goal_wy: f32) {
        let gx = (goal_wx / TILE_SIZE) as usize;
        let gy = (goal_wy / TILE_SIZE) as usize;
        let gx = gx.min(GRID_SIZE - 1);
        let gy = gy.min(GRID_SIZE - 1);

        if self.field.is_none() || self.goal != (gx, gy) {
            self.field = Some(FlowField::generate(grid, gx, gy));
            self.goal = (gx, gy);
        }
    }
}
