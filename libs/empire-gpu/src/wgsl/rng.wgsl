// The dice (random.rs) of the lane's table: xoshiro128++ on four private
// words, `random` on the remainder of a word, `mix` for the lots — and
// the wide products the CPU takes in 64 bits.

var<private> rng: array<u32, 4>;

fn rotl(x: u32, r: u32) -> u32 {
    return (x << r) | (x >> (32u - r));
}

fn next_word() -> u32 {
    let s0 = rng[0];
    let s1 = rng[1];
    let s2 = rng[2];
    let s3 = rng[3];
    let result = rotl(s0 + s3, 7u) + s0;
    let t = s1 << 9u;
    let n2 = s2 ^ s0;
    let n3 = s3 ^ s1;
    let n1 = s1 ^ n2;
    let n0 = s0 ^ n3;
    rng[0] = n0;
    rng[1] = n1;
    rng[2] = n2 ^ t;
    rng[3] = rotl(n3, 11u);
    return result;
}

// Random integer in `low..to`; 0 without a roll when the range is empty.
fn random(low: i32, to: i32) -> i32 {
    if (to <= low) {
        return 0;
    }
    return low + i32(next_word() % u32(to - low));
}

fn mix(z0: u32) -> u32 {
    var z = (z0 ^ (z0 >> 16u)) * 0x85EBCA6Bu;
    z = (z ^ (z >> 13u)) * 0xC2B2AE35u;
    return z ^ (z >> 16u);
}

// The 64-bit product of two words as (low, high).
fn mul_wide(a: u32, b: u32) -> vec2<u32> {
    let a0 = a & 0xFFFFu;
    let a1 = a >> 16u;
    let b0 = b & 0xFFFFu;
    let b1 = b >> 16u;
    let p00 = a0 * b0;
    let p01 = a0 * b1;
    let p10 = a1 * b0;
    let p11 = a1 * b1;
    let mid = (p00 >> 16u) + (p01 & 0xFFFFu) + (p10 & 0xFFFFu);
    let lo = (p00 & 0xFFFFu) | (mid << 16u);
    let hi = p11 + (p01 >> 16u) + (p10 >> 16u) + (mid >> 16u);
    return vec2<u32>(lo, hi);
}

// The low word of a 64-bit dividend over a word: long division, a bit at
// a time, once the high word's remainder is taken.
fn div_wide(lo: u32, hi: u32, d: u32) -> u32 {
    var r = hi % d;
    var q = 0u;
    for (var i = 31; i >= 0; i--) {
        let bit = (lo >> u32(i)) & 1u;
        let carry = r >> 31u;
        r = (r << 1u) | bit;
        if (carry != 0u || r >= d) {
            r = r - d;
            q = q | (1u << u32(i));
        }
    }
    return q;
}

fn magnitude(a: i32) -> u32 {
    if (a < 0) {
        return u32(-a);
    }
    return u32(a);
}

// `a as i64 * b as i64 / c as i64`, truncated towards zero, as the CPU
// takes it when the product may not fit a word.
fn mul_div(a: i32, b: i32, c: i32) -> i32 {
    let negative = ((a < 0) != (b < 0)) != (c < 0);
    let p = mul_wide(magnitude(a), magnitude(b));
    let q = i32(div_wide(p.x, p.y, magnitude(c)));
    if (negative) {
        return -q;
    }
    return q;
}
