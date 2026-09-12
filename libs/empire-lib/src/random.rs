//! The dice of the game: one xoshiro128++ generator per thread, written
//! out in full so the GPU engine (`libs/empire-gpu`) can roll the very
//! same dice in the very same order — every operation here is on 32-bit
//! words, as WGSL has nothing wider. Seeded from entropy unless [`seed`]
//! is called, so a game replays only when asked to.

use std::cell::Cell;
use std::hash::{BuildHasher, RandomState};

/// The state of xoshiro128++ (Blackman & Vigna): four words, never all
/// zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rng {
    s: [u32; 4],
}

impl Rng {
    /// The generator seeded from `seed`, its words drawn by [`mix`] one
    /// after the other, as splitmix seeds a xoshiro.
    pub fn seeded(seed: u32) -> Rng {
        let mut s = [0; 4];
        for (i, w) in s.iter_mut().enumerate() {
            *w = mix(seed.wrapping_add((i as u32 + 1).wrapping_mul(GOLDEN)));
        }
        Rng { s }
    }

    /// A generator on the given words (see [`words`](Rng::words)).
    pub fn from_words(s: [u32; 4]) -> Rng {
        Rng { s }
    }

    /// The four words of the state, to hand the very same dice to the GPU.
    pub fn words(&self) -> [u32; 4] {
        self.s
    }

    /// The next word.
    pub fn word(&mut self) -> u32 {
        let s = &mut self.s;
        let result = s[0].wrapping_add(s[3]).rotate_left(7).wrapping_add(s[0]);
        let t = s[1] << 9;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(11);
        result
    }

    /// Random integer in `from..to` (exclusive upper bound); 0 when the
    /// range is empty or inverted, mirroring the original BASIC
    /// `INT(RND*n)` use. The remainder of a word: the bias on a range of a
    /// few thousand is one part in a million, the GPU agrees to the bit.
    pub fn random(&mut self, from: i32, to: i32) -> i32 {
        if to <= from {
            return 0;
        }
        from.wrapping_add((self.word() % to.wrapping_sub(from) as u32) as i32)
    }
}

/// The golden ratio's fraction, the increment of a splitmix.
const GOLDEN: u32 = 0x9E37_79B9;

/// A 32-bit mixer (MurmurHash3's finalizer): every bit of `z` moves
/// every bit of the result.
pub fn mix(z: u32) -> u32 {
    let z = (z ^ (z >> 16)).wrapping_mul(0x85EB_CA6B);
    let z = (z ^ (z >> 13)).wrapping_mul(0xC2B2_AE35);
    z ^ (z >> 16)
}

thread_local! {
    /// One cheap generator per thread: the battles draw a duel per man met.
    static RNG: Cell<Rng> = Cell::new(Rng::seeded(entropy()));
}

fn entropy() -> u32 {
    RandomState::new().hash_one(0u8) as u32
}

/// Seed the thread's dice: the game replays from here.
pub fn seed(seed: u32) {
    RNG.set(Rng::seeded(seed));
}

/// The thread's dice as they stand.
pub fn snapshot() -> Rng {
    RNG.get()
}

/// Put the thread's dice back as they were.
pub fn restore(rng: Rng) {
    RNG.set(rng);
}

fn roll<T>(f: impl FnOnce(&mut Rng) -> T) -> T {
    RNG.with(|r| {
        let mut rng = r.get();
        let out = f(&mut rng);
        r.set(rng);
        out
    })
}

/// Random integer in `from..to` (exclusive upper bound), see [`Rng::random`].
pub fn random(from: i32, to: i32) -> i32 {
    roll(|r| r.random(from, to))
}

/// A lot drawn once, for what is settled by it later (see
/// [`crate::front::Line`]).
pub fn lot() -> u32 {
    roll(|r| r.word())
}

/// The items put in a fresh random order: a Fisher–Yates shuffle on
/// [`random`], from the last item down.
pub fn shuffle<T>(items: &mut [T]) {
    for i in (1..items.len()).rev() {
        let j = random(0, i as i32 + 1) as usize;
        items.swap(i, j);
    }
}

/// The items in a fresh random order, see [`shuffle`].
pub fn shuffled<T, const N: usize>(mut items: [T; N]) -> [T; N] {
    shuffle(&mut items);
    items
}

#[cfg(test)]
mod tests {
    use super::{random, seed, shuffled, Rng};

    #[test]
    fn empty_and_inverted_ranges_yield_zero() {
        assert_eq!(random(0, 0), 0);
        assert_eq!(random(5, 5), 0);
        assert_eq!(random(5, 1), 0);
    }

    #[test]
    fn shuffling_keeps_every_item() {
        let mut seen = [false; 6];
        for _ in 0..200 {
            let order = shuffled([0, 1, 2, 3, 4, 5]);
            let mut sorted = order;
            sorted.sort();
            assert_eq!(sorted, [0, 1, 2, 3, 4, 5]);
            seen[order[0]] = true;
        }
        assert!(seen.iter().all(|&s| s), "every item leads at least once");
    }

    #[test]
    fn stays_within_bounds() {
        for _ in 0..1000 {
            let n = random(3, 7);
            assert!((3..7).contains(&n));
        }
    }

    #[test]
    fn a_seed_replays_the_dice() {
        seed(7);
        let first: Vec<i32> = (0..20).map(|_| random(0, 1000)).collect();
        seed(7);
        let again: Vec<i32> = (0..20).map(|_| random(0, 1000)).collect();
        assert_eq!(first, again);
        seed(8);
        let other: Vec<i32> = (0..20).map(|_| random(0, 1000)).collect();
        assert_ne!(first, other);
    }

    #[test]
    fn xoshiro_reference_words() {
        // The reference implementation on the state {1, 2, 3, 4}.
        let mut r = Rng { s: [1, 2, 3, 4] };
        assert_eq!(r.word(), 641);
        assert_eq!(r.word(), 1573767);
        assert_eq!(r.word(), 3222811527);
    }

    #[test]
    fn the_dice_are_fair_enough() {
        seed(1);
        let mut counts = [0; 6];
        for _ in 0..60_000 {
            counts[random(0, 6) as usize] += 1;
        }
        assert!(
            counts.iter().all(|&c| (9_500..10_500).contains(&c)),
            "{counts:?}"
        );
    }
}
