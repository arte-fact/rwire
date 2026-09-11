use std::cell::RefCell;

use rand::prelude::*;
use rand::rngs::SmallRng;

thread_local! {
    /// One cheap generator per thread: the battles draw a duel per man met,
    /// and the cryptographic `thread_rng` was the tax on every blow.
    static RNG: RefCell<SmallRng> = RefCell::new(SmallRng::from_entropy());
}

/// Random integer in `from..to` (exclusive upper bound). Returns 0 when the
/// range is empty or inverted, mirroring the original BASIC `INT(RND*n)` use.
pub fn random(from: i32, to: i32) -> i32 {
    if to <= from {
        return 0;
    }
    RNG.with(|r| r.borrow_mut().gen_range(from..to))
}

/// The items in a fresh random order.
pub fn shuffled<T, const N: usize>(mut items: [T; N]) -> [T; N] {
    RNG.with(|r| items.shuffle(&mut *r.borrow_mut()));
    items
}

#[cfg(test)]
mod tests {
    use super::{random, shuffled};

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
}
