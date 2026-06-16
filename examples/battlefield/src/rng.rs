//! Deterministic random number generation using ChaCha.

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// Create a seeded RNG for the game session.
pub fn game_rng(seed: u64) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(seed)
}

/// Deterministic per-tile random values.
/// Same (seed, tx, ty) always produces the same result.
pub struct TileRng {
    seed: u64,
}

impl TileRng {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    /// Get a ChaCha RNG seeded for a specific tile position.
    fn rng_for(&self, tx: usize, ty: usize) -> ChaCha8Rng {
        let tile_seed = self.seed
            ^ (tx as u64).wrapping_mul(6364136223846793005)
            ^ (ty as u64).wrapping_mul(1442695040888963407);
        ChaCha8Rng::seed_from_u64(tile_seed)
    }

    /// Get a variant index (0..n) for a tile.
    pub fn variant(&self, tx: usize, ty: usize, n: u32) -> u8 {
        self.rng_for(tx, ty).random_range(0..n) as u8
    }

    /// Get whether a tile should be flipped.
    pub fn flip(&self, tx: usize, ty: usize) -> bool {
        self.rng_for(tx, ty).random_bool(0.5)
    }

    /// Get a phase offset for animation decorrelation (0..max).
    pub fn phase_offset(&self, tx: usize, ty: usize, max: u32) -> u32 {
        self.rng_for(tx, ty).random_range(0..max)
    }
}
