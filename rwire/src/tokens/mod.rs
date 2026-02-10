//! Design tokens for rwire styling system.
//!
//! Tokens are organized in tiers:
//! - `primitives`: Raw values (colors, spacing, radius, typography, shadows)
//! - `palette`: Configurable color palettes for theming (Nord, custom, etc.)
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
//! # Color Palettes
//!
//! ```ignore
//! use rwire::tokens::palette::{ColorPalette, ColorScale};
//!
//! // Use the default Oklch-based palette
//! let palette = ColorPalette::default();
//!
//! // Or build a custom palette from seed colors
//! let palette = ColorPalette::default()
//!     .with_accent(ColorScale::from_color("#5E81AC"));
//! ```
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
pub mod palette;
pub mod primitives;

pub use palette::{ColorPalette, ColorScale};
pub use primitives::*;
