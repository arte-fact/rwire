//! Configurable color palettes for theming.
//!
//! Color palettes allow customizing the entire color system at runtime.
//! Each palette contains 12-step color scales following the Radix methodology:
//!
//! - Steps 1-4: Backgrounds (app, subtle, muted, emphasis)
//! - Steps 5-6: Interactive backgrounds (hover, active)
//! - Steps 7-8: Borders (subtle, default)
//! - Steps 9-10: Solid backgrounds (primary, hover)
//! - Steps 11-12: Text (low contrast, high contrast)
//!
//! # Example
//!
//! ```ignore
//! use rwire::tokens::palette::{ColorPalette, ColorScale};
//!
//! // Use the Nord preset
//! let palette = ColorPalette::nord();
//!
//! // Or create a custom palette
//! let palette = ColorPalette::default()
//!     .with_neutral(ColorScale::from_hex([
//!         "#ECEFF4", "#E5E9F0", "#D8DEE9", // ... 12 steps
//!     ]));
//! ```

use super::primitives::color;

/// A 12-step color scale.
///
/// Each step serves a specific UI purpose following Radix Colors methodology.
#[derive(Clone, Debug)]
pub struct ColorScale {
    /// The 12 color steps (can be any valid CSS color value)
    steps: [String; 12],
}

impl ColorScale {
    /// Create a color scale from 12 color strings.
    ///
    /// Colors can be any valid CSS format: hex, rgb, oklch, etc.
    pub fn new(steps: [impl Into<String>; 12]) -> Self {
        Self {
            steps: steps.map(|s| s.into()),
        }
    }

    /// Create a color scale from hex colors.
    pub fn from_hex(steps: [&str; 12]) -> Self {
        Self::new(steps.map(|s| s.to_string()))
    }

    /// Create a color scale from Oklch values.
    ///
    /// Format: `(lightness, chroma, hue)` where:
    /// - lightness: 0.0 to 1.0
    /// - chroma: 0.0 to ~0.4 (color intensity)
    /// - hue: 0 to 360 (color angle)
    pub fn from_oklch(steps: [(f32, f32, f32); 12]) -> Self {
        Self {
            steps: steps.map(|(l, c, h)| format!("oklch({} {} {})", l, c, h)),
        }
    }

    /// Get a step by index (0-11).
    pub fn step(&self, index: usize) -> &str {
        &self.steps[index.min(11)]
    }

    /// Get all steps as a slice.
    pub fn steps(&self) -> &[String; 12] {
        &self.steps
    }
}

impl Default for ColorScale {
    fn default() -> Self {
        Self::new([
            color::NEUTRAL_1.to_string(),
            color::NEUTRAL_2.to_string(),
            color::NEUTRAL_3.to_string(),
            color::NEUTRAL_4.to_string(),
            color::NEUTRAL_5.to_string(),
            color::NEUTRAL_6.to_string(),
            color::NEUTRAL_7.to_string(),
            color::NEUTRAL_8.to_string(),
            color::NEUTRAL_9.to_string(),
            color::NEUTRAL_10.to_string(),
            color::NEUTRAL_11.to_string(),
            color::NEUTRAL_12.to_string(),
        ])
    }
}

/// Complete color palette with all scales.
#[derive(Clone, Debug)]
pub struct ColorPalette {
    /// Neutral/gray scale for backgrounds, borders, text
    pub neutral: ColorScale,
    /// Primary accent color (default: blue)
    pub accent: ColorScale,
    /// Destructive/error color
    pub red: ColorScale,
    /// Success color
    pub green: ColorScale,
    /// Warning color
    pub amber: ColorScale,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            neutral: ColorScale::new([
                color::NEUTRAL_1, color::NEUTRAL_2, color::NEUTRAL_3, color::NEUTRAL_4,
                color::NEUTRAL_5, color::NEUTRAL_6, color::NEUTRAL_7, color::NEUTRAL_8,
                color::NEUTRAL_9, color::NEUTRAL_10, color::NEUTRAL_11, color::NEUTRAL_12,
            ]),
            accent: ColorScale::new([
                color::BLUE_1, color::BLUE_2, color::BLUE_3, color::BLUE_4,
                color::BLUE_5, color::BLUE_6, color::BLUE_7, color::BLUE_8,
                color::BLUE_9, color::BLUE_10, color::BLUE_11, color::BLUE_12,
            ]),
            red: ColorScale::new([
                color::RED_1, color::RED_2, color::RED_3, color::RED_4,
                color::RED_5, color::RED_6, color::RED_7, color::RED_8,
                color::RED_9, color::RED_10, color::RED_11, color::RED_12,
            ]),
            green: ColorScale::new([
                color::GREEN_1, color::GREEN_2, color::GREEN_3, color::GREEN_4,
                color::GREEN_5, color::GREEN_6, color::GREEN_7, color::GREEN_8,
                color::GREEN_9, color::GREEN_10, color::GREEN_11, color::GREEN_12,
            ]),
            amber: ColorScale::new([
                color::AMBER_1, color::AMBER_2, color::AMBER_3, color::AMBER_4,
                color::AMBER_5, color::AMBER_6, color::AMBER_7, color::AMBER_8,
                color::AMBER_9, color::AMBER_10, color::AMBER_11, color::AMBER_12,
            ]),
        }
    }
}

impl ColorPalette {
    /// Create a new palette with default colors.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the neutral scale.
    pub fn with_neutral(mut self, scale: ColorScale) -> Self {
        self.neutral = scale;
        self
    }

    /// Set the accent scale.
    pub fn with_accent(mut self, scale: ColorScale) -> Self {
        self.accent = scale;
        self
    }

    /// Set the red/error scale.
    pub fn with_red(mut self, scale: ColorScale) -> Self {
        self.red = scale;
        self
    }

    /// Set the green/success scale.
    pub fn with_green(mut self, scale: ColorScale) -> Self {
        self.green = scale;
        self
    }

    /// Set the amber/warning scale.
    pub fn with_amber(mut self, scale: ColorScale) -> Self {
        self.amber = scale;
        self
    }

    // ========================================================================
    // Preset Palettes
    // ========================================================================

    /// Nord color palette.
    ///
    /// A arctic, north-bluish color palette with:
    /// - Polar Night (dark backgrounds)
    /// - Snow Storm (light backgrounds)
    /// - Frost (accent blues)
    /// - Aurora (semantic colors)
    ///
    /// See: https://www.nordtheme.com/
    pub fn nord() -> Self {
        Self {
            // Nord Polar Night + Snow Storm for neutral scale
            // Arranged light-to-dark for light theme compatibility
            neutral: ColorScale::from_hex([
                "#ECEFF4", // Snow Storm 3 (lightest)
                "#E5E9F0", // Snow Storm 2
                "#D8DEE9", // Snow Storm 1
                "#C8CED8", // Interpolated
                "#B8BFC9", // Interpolated
                "#A8B0BA", // Interpolated
                "#6D7A8A", // Interpolated
                "#4C566A", // Polar Night 4
                "#434C5E", // Polar Night 3
                "#3B4252", // Polar Night 2
                "#2E3440", // Polar Night 1
                "#242933", // Darker (for high contrast)
            ]),
            // Nord Frost for accent (blue-cyan tones)
            accent: ColorScale::from_hex([
                "#F0F4F8", // Very light frost
                "#E3EDF5", // Light frost
                "#C9DDE9", // Lighter frost
                "#A3C9DC", // Light frost
                "#8FBCBB", // Frost 1 (teal)
                "#88C0D0", // Frost 2 (light blue)
                "#81A1C1", // Frost 3 (blue)
                "#6E94B4", // Interpolated
                "#5E81AC", // Frost 4 (dark blue) - PRIMARY
                "#5476A0", // Slightly darker
                "#4C6B94", // Darker for text
                "#3D5A80", // Darkest for high contrast text
            ]),
            // Nord Aurora Red
            red: ColorScale::from_hex([
                "#FDF2F2", // Lightest red bg
                "#FAE5E5", // Light red bg
                "#F5CCCC", // Lighter
                "#EBADAD", // Light
                "#D98E8E", // Medium light
                "#C87878", // Medium
                "#BF616A", // Aurora Red - PRIMARY
                "#B55760", // Darker
                "#A84D56", // Darker
                "#9A444C", // Dark
                "#8C3B42", // Text
                "#7D3238", // High contrast text
            ]),
            // Nord Aurora Green
            green: ColorScale::from_hex([
                "#F4F8F4", // Lightest green bg
                "#E8F0E8", // Light green bg
                "#D4E4D4", // Lighter
                "#BCD6BC", // Light
                "#A8C8A8", // Medium light
                "#96BA96", // Medium
                "#A3BE8C", // Aurora Green - PRIMARY
                "#94AE7E", // Darker
                "#859E70", // Darker
                "#768E62", // Dark
                "#677E54", // Text
                "#586E46", // High contrast text
            ]),
            // Nord Aurora Yellow/Orange for warning
            amber: ColorScale::from_hex([
                "#FFFBF0", // Lightest amber bg
                "#FFF5DC", // Light amber bg
                "#FFECC4", // Lighter
                "#FFE0A8", // Light
                "#F5D08C", // Medium light
                "#EBCB8B", // Aurora Yellow - PRIMARY
                "#D9B870", // Darker
                "#C7A55A", // Darker
                "#D08770", // Aurora Orange
                "#C47A64", // Darker orange
                "#A66A50", // Text
                "#8A5A40", // High contrast text
            ]),
        }
    }

    /// Generate CSS custom properties for this palette.
    ///
    /// Returns CSS defining all color variables.
    pub fn to_css(&self) -> String {
        let mut css = String::with_capacity(2048);

        // Neutral scale
        for (i, color) in self.neutral.steps().iter().enumerate() {
            css.push_str(&format!("--rw-neutral-{}:{};\n", i + 1, color));
        }

        // Accent scale (also output as "blue" for compatibility)
        for (i, color) in self.accent.steps().iter().enumerate() {
            css.push_str(&format!("--rw-blue-{}:{};\n", i + 1, color));
        }

        // Red scale
        for (i, color) in self.red.steps().iter().enumerate() {
            css.push_str(&format!("--rw-red-{}:{};\n", i + 1, color));
        }

        // Green scale
        for (i, color) in self.green.steps().iter().enumerate() {
            css.push_str(&format!("--rw-green-{}:{};\n", i + 1, color));
        }

        // Amber scale
        for (i, color) in self.amber.steps().iter().enumerate() {
            css.push_str(&format!("--rw-amber-{}:{};\n", i + 1, color));
        }

        css
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_scale_from_hex() {
        let scale = ColorScale::from_hex([
            "#000", "#111", "#222", "#333", "#444", "#555",
            "#666", "#777", "#888", "#999", "#AAA", "#BBB",
        ]);
        assert_eq!(scale.step(0), "#000");
        assert_eq!(scale.step(11), "#BBB");
    }

    #[test]
    fn test_color_scale_from_oklch() {
        let scale = ColorScale::from_oklch([
            (0.98, 0.0, 0.0), (0.90, 0.0, 0.0), (0.80, 0.0, 0.0), (0.70, 0.0, 0.0),
            (0.60, 0.0, 0.0), (0.50, 0.0, 0.0), (0.40, 0.0, 0.0), (0.30, 0.0, 0.0),
            (0.25, 0.0, 0.0), (0.20, 0.0, 0.0), (0.15, 0.0, 0.0), (0.10, 0.0, 0.0),
        ]);
        assert!(scale.step(0).starts_with("oklch("));
    }

    #[test]
    fn test_default_palette() {
        let palette = ColorPalette::default();
        assert!(palette.neutral.step(0).contains("oklch"));
        assert!(palette.accent.step(8).contains("oklch")); // blue-9
    }

    #[test]
    fn test_nord_palette() {
        let palette = ColorPalette::nord();
        assert_eq!(palette.neutral.step(0), "#ECEFF4"); // Snow Storm
        assert_eq!(palette.neutral.step(10), "#2E3440"); // Polar Night
        assert_eq!(palette.accent.step(8), "#5E81AC"); // Frost 4
    }

    #[test]
    fn test_palette_to_css() {
        let palette = ColorPalette::nord();
        let css = palette.to_css();

        assert!(css.contains("--rw-neutral-1:#ECEFF4"));
        assert!(css.contains("--rw-blue-9:#5E81AC"));
        assert!(css.contains("--rw-red-7:#BF616A"));
    }

    #[test]
    fn test_palette_builder() {
        let custom_neutral = ColorScale::from_hex([
            "#FFF", "#EEE", "#DDD", "#CCC", "#BBB", "#AAA",
            "#999", "#888", "#777", "#666", "#555", "#444",
        ]);

        let palette = ColorPalette::default()
            .with_neutral(custom_neutral);

        assert_eq!(palette.neutral.step(0), "#FFF");
        // Others unchanged
        assert!(palette.accent.step(0).contains("oklch"));
    }
}
