//! Primitive design tokens — raw values without semantic meaning.
//!
//! These tokens define the fundamental design palette:
//! - Colors: 12-step scales for neutral, blue, red, green, amber
//! - Spacing: 4px grid from 0 to 96px
//! - Border radius: from none to full (pill)
//! - Typography: sizes, weights, line heights
//! - Shadows: subtle to prominent elevation
//! - Transitions: timing for animations
//!
//! # Color Scale Philosophy
//!
//! Each color scale has 12 steps following Radix Colors methodology:
//! - Steps 1-4: Backgrounds (app, subtle, muted, emphasis)
//! - Steps 5-6: Interactive backgrounds (hover, active)
//! - Steps 7-8: Borders (subtle, default, interactive)
//! - Steps 9-10: Solid backgrounds (primary, hover)
//! - Steps 11-12: Text (low contrast, high contrast)
//!
//! Steps 11-12 guarantee 4.5:1 contrast ratio against steps 1-2.

/// 12-step color scales using Oklch color space.
///
/// Oklch provides perceptual uniformity and wide gamut support.
/// Format: `oklch(lightness chroma hue)`
pub mod color {
    // ========================================================================
    // Neutral scale (gray)
    // ========================================================================

    /// Neutral step 1: App background (lightest)
    pub const NEUTRAL_1: &str = "oklch(0.985 0 0)";
    /// Neutral step 2: Subtle background
    pub const NEUTRAL_2: &str = "oklch(0.97 0 0)";
    /// Neutral step 3: Muted background
    pub const NEUTRAL_3: &str = "oklch(0.94 0 0)";
    /// Neutral step 4: Emphasis background
    pub const NEUTRAL_4: &str = "oklch(0.91 0 0)";
    /// Neutral step 5: Hover background
    pub const NEUTRAL_5: &str = "oklch(0.87 0 0)";
    /// Neutral step 6: Active background
    pub const NEUTRAL_6: &str = "oklch(0.83 0 0)";
    /// Neutral step 7: Subtle border
    pub const NEUTRAL_7: &str = "oklch(0.77 0 0)";
    /// Neutral step 8: Default border (interactive minimum)
    pub const NEUTRAL_8: &str = "oklch(0.70 0 0)";
    /// Neutral step 9: Muted text
    pub const NEUTRAL_9: &str = "oklch(0.55 0 0)";
    /// Neutral step 10: Secondary text
    pub const NEUTRAL_10: &str = "oklch(0.50 0 0)";
    /// Neutral step 11: Low contrast text
    pub const NEUTRAL_11: &str = "oklch(0.40 0 0)";
    /// Neutral step 12: High contrast text (darkest)
    pub const NEUTRAL_12: &str = "oklch(0.25 0 0)";

    // ========================================================================
    // Blue scale (default accent)
    // ========================================================================

    pub const BLUE_1: &str = "oklch(0.985 0.01 250)";
    pub const BLUE_2: &str = "oklch(0.965 0.02 250)";
    pub const BLUE_3: &str = "oklch(0.93 0.04 250)";
    pub const BLUE_4: &str = "oklch(0.88 0.07 250)";
    pub const BLUE_5: &str = "oklch(0.82 0.10 250)";
    pub const BLUE_6: &str = "oklch(0.76 0.12 250)";
    pub const BLUE_7: &str = "oklch(0.68 0.14 250)";
    pub const BLUE_8: &str = "oklch(0.60 0.16 250)";
    /// Blue step 9: Primary solid background
    pub const BLUE_9: &str = "oklch(0.55 0.18 250)";
    /// Blue step 10: Hovered solid background
    pub const BLUE_10: &str = "oklch(0.50 0.19 250)";
    /// Blue step 11: Low contrast text on light bg
    pub const BLUE_11: &str = "oklch(0.45 0.16 250)";
    /// Blue step 12: High contrast text on light bg
    pub const BLUE_12: &str = "oklch(0.30 0.12 250)";

    // ========================================================================
    // Red scale (destructive/error)
    // ========================================================================

    pub const RED_1: &str = "oklch(0.985 0.015 25)";
    pub const RED_2: &str = "oklch(0.965 0.025 25)";
    pub const RED_3: &str = "oklch(0.93 0.05 25)";
    pub const RED_4: &str = "oklch(0.88 0.09 25)";
    pub const RED_5: &str = "oklch(0.82 0.12 25)";
    pub const RED_6: &str = "oklch(0.76 0.14 25)";
    pub const RED_7: &str = "oklch(0.68 0.16 25)";
    pub const RED_8: &str = "oklch(0.60 0.18 25)";
    pub const RED_9: &str = "oklch(0.55 0.20 25)";
    pub const RED_10: &str = "oklch(0.50 0.21 25)";
    pub const RED_11: &str = "oklch(0.45 0.18 25)";
    pub const RED_12: &str = "oklch(0.30 0.14 25)";

    // ========================================================================
    // Green scale (success)
    // ========================================================================

    pub const GREEN_1: &str = "oklch(0.985 0.015 145)";
    pub const GREEN_2: &str = "oklch(0.965 0.025 145)";
    pub const GREEN_3: &str = "oklch(0.93 0.05 145)";
    pub const GREEN_4: &str = "oklch(0.88 0.08 145)";
    pub const GREEN_5: &str = "oklch(0.82 0.11 145)";
    pub const GREEN_6: &str = "oklch(0.76 0.13 145)";
    pub const GREEN_7: &str = "oklch(0.68 0.15 145)";
    pub const GREEN_8: &str = "oklch(0.60 0.16 145)";
    pub const GREEN_9: &str = "oklch(0.55 0.17 145)";
    pub const GREEN_10: &str = "oklch(0.50 0.18 145)";
    pub const GREEN_11: &str = "oklch(0.45 0.15 145)";
    pub const GREEN_12: &str = "oklch(0.30 0.12 145)";

    // ========================================================================
    // Amber scale (warning)
    // ========================================================================

    pub const AMBER_1: &str = "oklch(0.985 0.015 85)";
    pub const AMBER_2: &str = "oklch(0.965 0.03 85)";
    pub const AMBER_3: &str = "oklch(0.93 0.06 85)";
    pub const AMBER_4: &str = "oklch(0.88 0.10 85)";
    pub const AMBER_5: &str = "oklch(0.82 0.13 85)";
    pub const AMBER_6: &str = "oklch(0.76 0.15 85)";
    pub const AMBER_7: &str = "oklch(0.70 0.16 85)";
    pub const AMBER_8: &str = "oklch(0.64 0.17 85)";
    /// Amber step 9: Brighter for visibility
    pub const AMBER_9: &str = "oklch(0.75 0.18 85)";
    pub const AMBER_10: &str = "oklch(0.70 0.19 85)";
    pub const AMBER_11: &str = "oklch(0.50 0.14 85)";
    pub const AMBER_12: &str = "oklch(0.35 0.10 85)";

    // ========================================================================
    // Special colors
    // ========================================================================

    /// Pure white for contrast text on dark/accent backgrounds
    pub const WHITE: &str = "oklch(1 0 0)";
    /// Pure black for maximum contrast
    pub const BLACK: &str = "oklch(0 0 0)";
}

/// Spacing scale using rem units (base 16px).
///
/// Follows a 4px grid for consistency:
/// - _1 = 4px, _2 = 8px, _3 = 12px, _4 = 16px, etc.
pub mod space {
    pub const _0: &str = "0";
    /// 4px
    pub const _1: &str = "0.25rem";
    /// 8px
    pub const _2: &str = "0.5rem";
    /// 12px
    pub const _3: &str = "0.75rem";
    /// 16px (base)
    pub const _4: &str = "1rem";
    /// 20px
    pub const _5: &str = "1.25rem";
    /// 24px
    pub const _6: &str = "1.5rem";
    /// 32px
    pub const _8: &str = "2rem";
    /// 40px
    pub const _10: &str = "2.5rem";
    /// 48px
    pub const _12: &str = "3rem";
    /// 64px
    pub const _16: &str = "4rem";
    /// 80px
    pub const _20: &str = "5rem";
    /// 96px
    pub const _24: &str = "6rem";
}

/// Border radius scale.
pub mod radius {
    /// No rounding
    pub const NONE: &str = "0";
    /// 2px - subtle
    pub const SM: &str = "0.125rem";
    /// 6px - default for inputs/buttons
    pub const MD: &str = "0.375rem";
    /// 8px - cards
    pub const LG: &str = "0.5rem";
    /// 12px - prominent
    pub const XL: &str = "0.75rem";
    /// 16px - very rounded
    pub const _2XL: &str = "1rem";
    /// Pill shape
    pub const FULL: &str = "9999px";
}

/// Font size scale.
pub mod font_size {
    /// 12px
    pub const XS: &str = "0.75rem";
    /// 14px
    pub const SM: &str = "0.875rem";
    /// 16px (base)
    pub const BASE: &str = "1rem";
    /// 18px
    pub const LG: &str = "1.125rem";
    /// 20px
    pub const XL: &str = "1.25rem";
    /// 24px
    pub const _2XL: &str = "1.5rem";
    /// 30px
    pub const _3XL: &str = "1.875rem";
    /// 36px
    pub const _4XL: &str = "2.25rem";
}

/// Font weight scale.
pub mod font_weight {
    pub const NORMAL: &str = "400";
    pub const MEDIUM: &str = "500";
    pub const SEMIBOLD: &str = "600";
    pub const BOLD: &str = "700";
}

/// Line height scale.
pub mod line_height {
    /// 1.25 - compact
    pub const TIGHT: &str = "1.25";
    /// 1.375 - slightly compact
    pub const SNUG: &str = "1.375";
    /// 1.5 - default
    pub const NORMAL: &str = "1.5";
    /// 1.625 - slightly loose
    pub const RELAXED: &str = "1.625";
    /// 2.0 - spacious
    pub const LOOSE: &str = "2";
}

/// Shadow definitions for elevation.
pub mod shadow {
    /// Subtle shadow
    pub const SM: &str = "0 1px 2px 0 rgb(0 0 0 / 0.05)";
    /// Medium shadow
    pub const MD: &str = "0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)";
    /// Large shadow
    pub const LG: &str = "0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)";
    /// Extra large shadow
    pub const XL: &str = "0 20px 25px -5px rgb(0 0 0 / 0.1), 0 8px 10px -6px rgb(0 0 0 / 0.1)";
}

/// Transition timing.
pub mod transition {
    /// 150ms - quick interactions
    pub const FAST: &str = "150ms";
    /// 200ms - standard interactions
    pub const NORMAL: &str = "200ms";
    /// 300ms - deliberate transitions
    pub const SLOW: &str = "300ms";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_scales_have_12_steps() {
        // Verify each color scale has all 12 steps defined
        let _neutrals = [
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
        ];

        let _blues = [
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
        ];

        // All should be valid Oklch strings
        assert!(color::NEUTRAL_1.starts_with("oklch("));
        assert!(color::BLUE_9.starts_with("oklch("));
    }

    #[test]
    fn test_spacing_scale() {
        assert_eq!(space::_0, "0");
        assert_eq!(space::_4, "1rem");
        assert!(space::_24.ends_with("rem"));
    }

    #[test]
    fn test_radius_scale() {
        assert_eq!(radius::NONE, "0");
        assert_eq!(radius::FULL, "9999px");
    }
}
