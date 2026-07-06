//! Numeric conversions with no lossless or checked `std` equivalent.
//!
//! `clippy::pedantic` flags every `as` cast between floats and integers. Almost
//! all casts in this crate are expressed instead with `From`/`TryFrom`; the
//! handful that *cannot* be are isolated here so the rest of the code stays
//! cast-free:
//!
//! - **byte/size → `f64`**: byte counts legitimately exceed `u32` (models are
//!   gigabytes), so the `u32::try_from` trick would corrupt them. `f64` holds
//!   integers exactly up to 2^52 (≈ 4 PiB), so the conversion is lossless in
//!   practice — but `u64` has no `From<u64> for f64`.
//! - **`f64` → integer**: `std` has no checked float-to-int conversion (only the
//!   `unsafe` `to_int_unchecked`), so a bounds-checked saturating helper must use
//!   `as` internally.
//!
//! These are the only sanctioned `as` casts in the crate; the lint is allowed on
//! this module alone, with each conversion bounds-aware where applicable.

// Rust guideline compliant 2026-02-21
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

/// Convert a byte or mebibyte count to `f64` (exact up to 2^52 ≈ 4 PiB).
#[inline]
pub const fn u64_f64(x: u64) -> f64 {
    x as f64
}

/// Convert a byte or mebibyte count to `f32` (for coarse on-screen meters).
#[inline]
pub const fn u64_f32(x: u64) -> f32 {
    x as f32
}

/// Convert a count to `f32` for coarse on-screen meters (precision loss beyond
/// 2^24 is irrelevant for the displayed magnitudes: cores, percent, MHz).
#[inline]
pub const fn u32_f32(x: u32) -> f32 {
    x as f32
}

/// Narrow an `f64` metric to `f32` for compact storage/display.
#[inline]
pub const fn f64_f32(x: f64) -> f32 {
    x as f32
}

/// Saturating, truncating `f64` → `u32` for a non-negative computed count.
///
/// Values at or below zero map to `0`; values at or above `u32::MAX` saturate.
#[inline]
pub fn f64_u32(x: f64) -> u32 {
    if x <= 0.0 {
        0
    } else if x >= f64::from(u32::MAX) {
        u32::MAX
    } else {
        x as u32
    }
}

/// Saturating, truncating `f32` → `u32` for a non-negative computed count.
#[inline]
pub fn f32_u32(x: f32) -> u32 {
    f64_u32(f64::from(x))
}
