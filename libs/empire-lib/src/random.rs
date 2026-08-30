use rand::prelude::*;

/// Random integer in `from..to` (exclusive upper bound). Returns 0 when the
/// range is empty or inverted, mirroring the original BASIC `INT(RND*n)` use.
pub fn random(from: i32, to: i32) -> i32 {
    if to <= from {
        return 0;
    }
    rand::thread_rng().gen_range(from..to)
}

#[cfg(test)]
mod tests {
    use super::random;

    #[test]
    fn empty_and_inverted_ranges_yield_zero() {
        assert_eq!(random(0, 0), 0);
        assert_eq!(random(5, 5), 0);
        assert_eq!(random(5, 1), 0);
    }

    #[test]
    fn stays_within_bounds() {
        for _ in 0..1000 {
            let n = random(3, 7);
            assert!((3..7).contains(&n));
        }
    }
}
