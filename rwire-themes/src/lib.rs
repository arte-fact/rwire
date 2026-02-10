//! Predefined theme styles and color palettes for rwire.
//!
//! This crate provides ready-to-use style presets and named color palettes
//! that complement the core rwire theming system. The core library ships with
//! only the Soft style and a default blue palette — this crate adds variety.
//!
//! # Styles
//!
//! ```ignore
//! use rwire::theme::Theme;
//! use rwire_themes::styles;
//!
//! let theme = Theme::dark().style(styles::solid());
//! ```
//!
//! # Palettes
//!
//! ```ignore
//! use rwire::theme::Theme;
//! use rwire_themes::palettes;
//!
//! let theme = Theme::dark().palette(palettes::nord());
//! ```

pub mod palettes;
pub mod styles;

// Re-export enums for convenience
pub use palettes::Palette;
pub use styles::Style;
