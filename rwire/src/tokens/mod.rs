//! Design tokens for rwire styling system.
//!
//! Tokens are organized in two tiers:
//! - `primitives`: Raw values (colors, spacing, radius, typography, shadows)
//! - `css`: CSS custom property generation for the capsule
//!
//! # Philosophy
//!
//! Tokens are Rust constants resolved at compile time. The capsule includes
//! CSS custom properties once, and components reference them via `var(--rw-*)`.
//! This provides:
//! - Zero runtime cost (no JS token resolution)
//! - Minimal bandwidth (tokens defined once, referenced by class names)
//! - Full theming via CSS variable overrides
//!
//! # Example
//!
//! ```ignore
//! use rwire::tokens::{color, space};
//!
//! // Use constants directly for build-time values
//! let padding = space::_4; // "1rem"
//!
//! // Or reference via CSS variables for themeable values
//! // background: var(--rw-accent-9);
//! ```

pub mod css;
pub mod primitives;

pub use primitives::*;
