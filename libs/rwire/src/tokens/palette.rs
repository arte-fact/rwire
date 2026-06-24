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
//! // Create a custom palette from seed colors
//! let palette = ColorPalette::default()
//!     .with_accent(ColorScale::from_color("#5E81AC"))
//!     .with_neutral(ColorScale::from_color("#2E3440"));
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

    /// Generate a 12-step color scale from a single seed color.
    ///
    /// The seed is used as step 9 (primary solid background).
    /// Steps 1-8 and 10-12 are derived automatically using the Radix
    /// 12-step methodology with oklch lightness/chroma ramps.
    ///
    /// Accepts `#RRGGBB`, `#RGB`, or `oklch(L C H)` format.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Generate a full blue scale from one color
    /// let scale = ColorScale::from_color("#5E81AC");
    ///
    /// // Generate from oklch
    /// let scale = ColorScale::from_color("oklch(0.55 0.18 250)");
    /// ```
    pub fn from_color(css_color: &str) -> Self {
        let (l, c, h) = parse_to_oklch(css_color);
        Self::from_oklch_seed(l, c, h)
    }

    /// Generate a 12-step scale from oklch seed parameters.
    fn from_oklch_seed(seed_l: f32, seed_c: f32, hue: f32) -> Self {
        // Lightness ramp: backgrounds light → dark text
        let lightness = [
            0.985,
            0.965,
            0.93,
            0.88,
            0.82,
            0.76,
            0.68,
            0.60,
            seed_l,
            (seed_l - 0.05).max(0.1),
            (seed_l - 0.10).max(0.1),
            (seed_l - 0.25).max(0.1),
        ];
        // Chroma ramp: very subtle → peak at 9-10 → moderate for text
        let chroma_ratios = [
            0.06, 0.11, 0.22, 0.39, 0.56, 0.67, 0.78, 0.89, 1.0, 1.06, 0.89, 0.67,
        ];
        let mut steps = [(0.0f32, 0.0f32, 0.0f32); 12];
        for i in 0..12 {
            steps[i] = (lightness[i], seed_c * chroma_ratios[i], hue);
        }
        Self::from_oklch(steps)
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
                color::NEUTRAL_1,
                color::NEUTRAL_2,
                color::NEUTRAL_3,
                color::NEUTRAL_4,
                color::NEUTRAL_5,
                color::NEUTRAL_6,
                color::NEUTRAL_7,
                color::NEUTRAL_8,
                color::NEUTRAL_9,
                color::NEUTRAL_10,
                color::NEUTRAL_11,
                color::NEUTRAL_12,
            ]),
            accent: ColorScale::new([
                color::BLUE_1,
                color::BLUE_2,
                color::BLUE_3,
                color::BLUE_4,
                color::BLUE_5,
                color::BLUE_6,
                color::BLUE_7,
                color::BLUE_8,
                color::BLUE_9,
                color::BLUE_10,
                color::BLUE_11,
                color::BLUE_12,
            ]),
            red: ColorScale::new([
                color::RED_1,
                color::RED_2,
                color::RED_3,
                color::RED_4,
                color::RED_5,
                color::RED_6,
                color::RED_7,
                color::RED_8,
                color::RED_9,
                color::RED_10,
                color::RED_11,
                color::RED_12,
            ]),
            green: ColorScale::new([
                color::GREEN_1,
                color::GREEN_2,
                color::GREEN_3,
                color::GREEN_4,
                color::GREEN_5,
                color::GREEN_6,
                color::GREEN_7,
                color::GREEN_8,
                color::GREEN_9,
                color::GREEN_10,
                color::GREEN_11,
                color::GREEN_12,
            ]),
            amber: ColorScale::new([
                color::AMBER_1,
                color::AMBER_2,
                color::AMBER_3,
                color::AMBER_4,
                color::AMBER_5,
                color::AMBER_6,
                color::AMBER_7,
                color::AMBER_8,
                color::AMBER_9,
                color::AMBER_10,
                color::AMBER_11,
                color::AMBER_12,
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
}

// ============================================================================
// Color Parsing: hex → oklch
// ============================================================================

/// Parse a CSS color string to oklch (lightness, chroma, hue).
///
/// Supported formats:
/// - `#RRGGBB` (e.g., `#5E81AC`)
/// - `#RGB` (e.g., `#FFF`)
/// - `oklch(L C H)` (e.g., `oklch(0.55 0.18 250)`)
fn parse_to_oklch(css_color: &str) -> (f32, f32, f32) {
    let s = css_color.trim();

    // oklch(L C H) → parse directly
    if let Some(inner) = s.strip_prefix("oklch(").and_then(|r| r.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split_whitespace().collect();
        if parts.len() == 3 {
            let l: f32 = parts[0].parse().unwrap_or(0.5);
            let c: f32 = parts[1].parse().unwrap_or(0.0);
            let h: f32 = parts[2].parse().unwrap_or(0.0);
            return (l, c, h);
        }
    }

    // #RRGGBB or #RGB → sRGB → linear sRGB → XYZ → Oklab → Oklch
    if let Some(hex) = s.strip_prefix('#') {
        let (r, g, b) = if hex.len() == 6 {
            (
                u8::from_str_radix(&hex[0..2], 16).unwrap_or(0),
                u8::from_str_radix(&hex[2..4], 16).unwrap_or(0),
                u8::from_str_radix(&hex[4..6], 16).unwrap_or(0),
            )
        } else if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[1..2], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[2..3], 16).unwrap_or(0);
            (r * 17, g * 17, b * 17)
        } else {
            return (0.5, 0.0, 0.0); // fallback
        };
        return srgb_to_oklch(r, g, b);
    }

    (0.5, 0.0, 0.0) // fallback for unsupported formats
}

/// Convert sRGB (u8) to Oklch (lightness, chroma, hue).
fn srgb_to_oklch(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    // sRGB gamma decode → linear sRGB
    let lin = |v: u8| -> f32 {
        let f = v as f32 / 255.0;
        if f <= 0.04045 {
            f / 12.92
        } else {
            ((f + 0.055) / 1.055).powf(2.4)
        }
    };
    let lr = lin(r);
    let lg = lin(g);
    let lb = lin(b);

    // Linear sRGB → XYZ (D65)
    let x = 0.412_456_4 * lr + 0.357_576_1 * lg + 0.180_437_5 * lb;
    let y = 0.212_672_9 * lr + 0.715_152_2 * lg + 0.072_175_0 * lb;
    let z = 0.019_333_9 * lr + 0.119_192 * lg + 0.950_304_1 * lb;

    // XYZ → LMS (Oklab transform)
    let l = 0.818_933_f32 * x + 0.361_866_7 * y - 0.128_859_7 * z;
    let m = 0.032_984_54_f32 * x + 0.929_311_9 * y + 0.036_145_64 * z;
    let s = 0.048_200_3_f32 * x + 0.264_366_27 * y + 0.633_851_7 * z;

    // Cube root
    let l_ = l.max(0.0).cbrt();
    let m_ = m.max(0.0).cbrt();
    let s_ = s.max(0.0).cbrt();

    // LMS → Oklab
    let ok_l = 0.210_454_26_f32 * l_ + 0.793_617_8 * m_ - 0.004_072_047 * s_;
    let ok_a = 1.977_998_5_f32 * l_ - 2.428_592_2 * m_ + 0.450_593_7 * s_;
    let ok_b = 0.025_904_037_f32 * l_ + 0.782_771_77 * m_ - 0.808_675_77 * s_;

    // Oklab → Oklch
    let c = (ok_a * ok_a + ok_b * ok_b).sqrt();
    let h = ok_b.atan2(ok_a).to_degrees();
    let h = if h < 0.0 { h + 360.0 } else { h };

    // Round to reasonable precision
    (
        (ok_l * 1000.0).round() / 1000.0,
        (c * 1000.0).round() / 1000.0,
        (h * 10.0).round() / 10.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_scale_from_hex() {
        let scale = ColorScale::from_hex([
            "#000", "#111", "#222", "#333", "#444", "#555", "#666", "#777", "#888", "#999", "#AAA",
            "#BBB",
        ]);
        assert_eq!(scale.step(0), "#000");
        assert_eq!(scale.step(11), "#BBB");
    }

    #[test]
    fn test_color_scale_from_oklch() {
        let scale = ColorScale::from_oklch([
            (0.98, 0.0, 0.0),
            (0.90, 0.0, 0.0),
            (0.80, 0.0, 0.0),
            (0.70, 0.0, 0.0),
            (0.60, 0.0, 0.0),
            (0.50, 0.0, 0.0),
            (0.40, 0.0, 0.0),
            (0.30, 0.0, 0.0),
            (0.25, 0.0, 0.0),
            (0.20, 0.0, 0.0),
            (0.15, 0.0, 0.0),
            (0.10, 0.0, 0.0),
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
    fn test_palette_builder() {
        let custom_neutral = ColorScale::from_hex([
            "#FFF", "#EEE", "#DDD", "#CCC", "#BBB", "#AAA", "#999", "#888", "#777", "#666", "#555",
            "#444",
        ]);

        let palette = ColorPalette::default().with_neutral(custom_neutral);

        assert_eq!(palette.neutral.step(0), "#FFF");
        // Others unchanged
        assert!(palette.accent.step(0).contains("oklch"));
    }

    #[test]
    fn test_parse_to_oklch_hex() {
        // Pure white
        let (l, c, _h) = parse_to_oklch("#FFFFFF");
        assert!((l - 1.0).abs() < 0.01, "White L should be ~1.0, got {}", l);
        assert!(c < 0.01, "White C should be ~0, got {}", c);

        // Pure black
        let (l, c, _h) = parse_to_oklch("#000000");
        assert!(l < 0.01, "Black L should be ~0, got {}", l);
        assert!(c < 0.01, "Black C should be ~0, got {}", c);

        // Nord blue (#5E81AC) should give reasonable oklch
        let (l, c, h) = parse_to_oklch("#5E81AC");
        assert!(l > 0.4 && l < 0.7, "Nord blue L out of range: {}", l);
        assert!(c > 0.01, "Nord blue should have some chroma: {}", c);
        assert!(
            h > 200.0 && h < 300.0,
            "Nord blue hue should be blue-ish: {}",
            h
        );
    }

    #[test]
    fn test_parse_to_oklch_short_hex() {
        let (l, _, _) = parse_to_oklch("#FFF");
        assert!(
            (l - 1.0).abs() < 0.01,
            "White #FFF L should be ~1.0, got {}",
            l
        );
    }

    #[test]
    fn test_parse_to_oklch_passthrough() {
        let (l, c, h) = parse_to_oklch("oklch(0.55 0.18 250)");
        assert!((l - 0.55).abs() < 0.001);
        assert!((c - 0.18).abs() < 0.001);
        assert!((h - 250.0).abs() < 0.001);
    }

    #[test]
    fn test_from_color_hex() {
        let scale = ColorScale::from_color("#5E81AC");
        // All 12 steps should be oklch
        for i in 0..12 {
            assert!(
                scale.step(i).starts_with("oklch("),
                "Step {} should be oklch: {}",
                i,
                scale.step(i)
            );
        }
        // Lightness should decrease from step 1 to step 12
        let step1 = scale.step(0);
        let step12 = scale.step(11);
        assert!(step1 != step12, "First and last steps should differ");
    }

    #[test]
    fn test_from_color_oklch() {
        let scale = ColorScale::from_color("oklch(0.55 0.18 250)");
        for i in 0..12 {
            assert!(
                scale.step(i).starts_with("oklch("),
                "Step {} should be oklch",
                i
            );
        }
    }
}
