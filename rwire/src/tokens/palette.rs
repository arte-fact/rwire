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
            0.985, 0.965, 0.93, 0.88, 0.82, 0.76, 0.68, 0.60,
            seed_l,
            (seed_l - 0.05).max(0.1),
            (seed_l - 0.10).max(0.1),
            (seed_l - 0.25).max(0.1),
        ];
        // Chroma ramp: very subtle → peak at 9-10 → moderate for text
        let chroma_ratios = [
            0.06, 0.11, 0.22, 0.39, 0.56, 0.67, 0.78, 0.89,
            1.0, 1.06, 0.89, 0.67,
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

    /// Catppuccin Mocha color palette.
    ///
    /// Warm pastel colors with a cozy, modern feel.
    /// Accent: Mauve, Neutral: warm gray.
    pub fn catppuccin() -> Self {
        Self {
            neutral: ColorScale::from_hex([
                "#EFF1F5", "#E6E9EF", "#DCE0E8", "#CCD0DA",
                "#BCC0CC", "#ACB0BE", "#7C7F93", "#6C6F85",
                "#585B70", "#45475A", "#313244", "#1E1E2E",
            ]),
            accent: ColorScale::from_hex([
                "#F5F0FA", "#EDE4F5", "#DCC8EB", "#CBADE1",
                "#BA91D7", "#AD7FD0", "#CBA6F7", "#B98EEF",
                "#B07BE8", "#A76DDE", "#9D5FD4", "#8839EF",
            ]),
            red: ColorScale::from_hex([
                "#FDF0F0", "#FAE0E0", "#F5C2C2", "#F0A4A4",
                "#EB8686", "#E56868", "#F38BA8", "#ED6D8D",
                "#E64F73", "#D94258", "#CC3548", "#BF2838",
            ]),
            green: ColorScale::from_hex([
                "#F0F8F0", "#E0F0E0", "#C2E0C2", "#A4D0A4",
                "#86C086", "#68B068", "#A6E3A1", "#8ED98A",
                "#76CF73", "#5EC55C", "#46BB45", "#2EB12E",
            ]),
            amber: ColorScale::from_hex([
                "#FFF8F0", "#FFF0DC", "#FFE4C0", "#FFD8A0",
                "#FFCC80", "#FFC060", "#F9E2AF", "#F5D48A",
                "#F1C665", "#EDB840", "#E9AA1B", "#E59C00",
            ]),
        }
    }

    /// Dracula color palette.
    ///
    /// Vivid dark theme with bold, saturated colors.
    /// Accent: Purple, Neutral: cool dark.
    pub fn dracula() -> Self {
        Self {
            neutral: ColorScale::from_hex([
                "#F8F8F2", "#ECEEF0", "#D6D9E0", "#C0C4D0",
                "#A0A4B8", "#808498", "#6272A4", "#555A70",
                "#44475A", "#383A4A", "#2C2E3A", "#282A36",
            ]),
            accent: ColorScale::from_hex([
                "#F8F0FF", "#F0E0FF", "#E0C4FF", "#D0A8FF",
                "#C08CFF", "#B580F0", "#BD93F9", "#AB80F0",
                "#9A6DE7", "#895ADE", "#7847D5", "#6734CC",
            ]),
            red: ColorScale::from_hex([
                "#FFF0F0", "#FFE0E0", "#FFC2C2", "#FFA4A4",
                "#FF8686", "#FF7979", "#FF5555", "#EE4444",
                "#DD3333", "#CC2222", "#BB1111", "#AA0000",
            ]),
            green: ColorScale::from_hex([
                "#F0FFF0", "#E0FFE0", "#C2FFC2", "#A4FFA4",
                "#86FF86", "#68FF68", "#50FA7B", "#40E868",
                "#30D655", "#20C442", "#10B22F", "#00A01C",
            ]),
            amber: ColorScale::from_hex([
                "#FFFFF0", "#FFFFE0", "#FFFFC2", "#FFFFA4",
                "#FFFF86", "#FFFF68", "#F1FA8C", "#E0E878",
                "#CFD664", "#BEC450", "#ADB23C", "#9CA028",
            ]),
        }
    }

    /// Solarized color palette.
    ///
    /// Precision-crafted with controlled contrast.
    /// Accent: Cyan, Neutral: warm ochre.
    /// See: https://ethanschoonover.com/solarized/
    pub fn solarized() -> Self {
        Self {
            neutral: ColorScale::from_hex([
                "#FDF6E3", "#EEE8D5", "#DDD6C1", "#CCc4AD",
                "#B8B09A", "#A09A86", "#839496", "#657B83",
                "#586E75", "#475B62", "#073642", "#002B36",
            ]),
            accent: ColorScale::from_hex([
                "#F0FAFA", "#E0F5F5", "#C2EAEA", "#A4DFDF",
                "#86D4D4", "#6CCAC8", "#2AA198", "#249188",
                "#1E8178", "#187168", "#126158", "#0C5148",
            ]),
            red: ColorScale::from_hex([
                "#FDF2F0", "#FAE4E0", "#F5C8C2", "#F0ACA4",
                "#EB9086", "#E57468", "#DC322F", "#CC2825",
                "#BC1E1B", "#AC1411", "#9C0A07", "#8C0000",
            ]),
            green: ColorScale::from_hex([
                "#F4FAF0", "#E6F5E0", "#CCE8C2", "#B2DBA4",
                "#98CE86", "#7EC168", "#859900", "#768A00",
                "#677B00", "#586C00", "#495D00", "#3A4E00",
            ]),
            amber: ColorScale::from_hex([
                "#FFF8F0", "#FFF0DC", "#FFE4C0", "#FFD8A0",
                "#FFCC80", "#FFC060", "#B58900", "#A67C00",
                "#976F00", "#886200", "#795500", "#6A4800",
            ]),
        }
    }

    /// Gruvbox color palette.
    ///
    /// Retro warm colors with a nostalgic feel.
    /// Accent: Orange, Neutral: warm beige.
    /// See: https://github.com/morhetz/gruvbox
    pub fn gruvbox() -> Self {
        Self {
            neutral: ColorScale::from_hex([
                "#FBF1C7", "#EBDBB2", "#D5C4A1", "#BDAE93",
                "#A89984", "#928374", "#7C6F64", "#665C54",
                "#504945", "#3C3836", "#282828", "#1D2021",
            ]),
            accent: ColorScale::from_hex([
                "#FFF0E0", "#FFE0C0", "#FFC490", "#FFA860",
                "#FF8C40", "#F07830", "#D65D0E", "#C45208",
                "#B24702", "#A03C00", "#8E3100", "#7C2600",
            ]),
            red: ColorScale::from_hex([
                "#FDF0F0", "#FAE0E0", "#F5C4C4", "#F0A8A8",
                "#EB8C8C", "#E07070", "#CC241D", "#BB1A14",
                "#AA100B", "#990602", "#880000", "#770000",
            ]),
            green: ColorScale::from_hex([
                "#F4FAF0", "#E8F5E0", "#D0E8C4", "#B8DBA8",
                "#A0CE8C", "#88C170", "#98971A", "#8A8A10",
                "#7C7D06", "#6E7000", "#606300", "#525600",
            ]),
            amber: ColorScale::from_hex([
                "#FFF8E0", "#FFF0C0", "#FFE490", "#FFD860",
                "#FFCC40", "#F0C030", "#D79921", "#C88C18",
                "#B97F0F", "#AA7206", "#9B6500", "#8C5800",
            ]),
        }
    }

    /// Tokyo Night color palette.
    ///
    /// Modern neon-accented dark theme.
    /// Accent: Blue-purple, Neutral: cool slate.
    pub fn tokyo_night() -> Self {
        Self {
            neutral: ColorScale::from_hex([
                "#D5D6DB", "#C0C1C8", "#A9B1D6", "#9AA5CE",
                "#787C99", "#565F89", "#414868", "#3B4261",
                "#343B59", "#2C3052", "#24283B", "#1A1B26",
            ]),
            accent: ColorScale::from_hex([
                "#F0F4FF", "#E0E8FF", "#C4D4FF", "#A8C0FF",
                "#8CACFF", "#7EA0FF", "#7AA2F7", "#6890EE",
                "#567EE5", "#446CDC", "#325AD3", "#2048CA",
            ]),
            red: ColorScale::from_hex([
                "#FFF0F0", "#FFE0E0", "#FFC4C4", "#FFA8A8",
                "#FF8C8C", "#FF7070", "#F7768E", "#EE6478",
                "#E55262", "#DC404C", "#D32E36", "#CA1C20",
            ]),
            green: ColorScale::from_hex([
                "#F0FFF4", "#E0FFE8", "#C4FFCC", "#A8FFB0",
                "#8CFF94", "#70FF78", "#9ECE6A", "#8CBE5A",
                "#7AAE4A", "#689E3A", "#568E2A", "#447E1A",
            ]),
            amber: ColorScale::from_hex([
                "#FFFFF0", "#FFFFE0", "#FFFFC4", "#FFFFA8",
                "#FFFF8C", "#FFEE70", "#E0AF68", "#D09E58",
                "#C08D48", "#B07C38", "#A06B28", "#905A18",
            ]),
        }
    }

    /// Rose Pine color palette.
    ///
    /// Elegant pastel theme with muted tones.
    /// Accent: Rose, Neutral: muted purple.
    /// See: https://rosepinetheme.com/
    pub fn rose_pine() -> Self {
        Self {
            neutral: ColorScale::from_hex([
                "#FAF4ED", "#F2E9E1", "#E0D5CB", "#CECACD",
                "#908CAA", "#6E6A86", "#56526E", "#44415A",
                "#393552", "#2A273F", "#1F1D2E", "#191724",
            ]),
            accent: ColorScale::from_hex([
                "#FFF0F4", "#FFE0E8", "#FFC4CC", "#FFA8B0",
                "#FF8C94", "#F57E84", "#EB6F92", "#E15D80",
                "#D74B6E", "#CD395C", "#C3274A", "#B91538",
            ]),
            red: ColorScale::from_hex([
                "#FFF0F0", "#FFE0E0", "#FFC4C4", "#FFA8A8",
                "#FF8C8C", "#F07070", "#EB6F92", "#E05D7E",
                "#D54B6A", "#CA3956", "#BF2742", "#B4152E",
            ]),
            green: ColorScale::from_hex([
                "#F0FAF4", "#E0F5E8", "#C4EACC", "#A8DFB0",
                "#8CD494", "#70C978", "#9CCFD8", "#88BFC8",
                "#74AFB8", "#609FA8", "#4C8F98", "#387F88",
            ]),
            amber: ColorScale::from_hex([
                "#FFF8F0", "#FFF0E0", "#FFE4C0", "#FFD8A0",
                "#FFCC80", "#F0C060", "#F6C177", "#EAB567",
                "#DEA957", "#D29D47", "#C69137", "#BA8527",
            ]),
        }
    }

    /// One Dark color palette.
    ///
    /// Classic VS Code-inspired dark theme.
    /// Accent: Blue, Neutral: neutral dark.
    pub fn one_dark() -> Self {
        Self {
            neutral: ColorScale::from_hex([
                "#F0F2F4", "#E0E4E8", "#C8CCD2", "#B0B4BC",
                "#979DA6", "#7F8590", "#5C6370", "#4B5263",
                "#3E4452", "#353B45", "#282C34", "#21252B",
            ]),
            accent: ColorScale::from_hex([
                "#F0F8FF", "#E0F0FF", "#C0DCFF", "#A0C8FF",
                "#80B4FF", "#70A8FF", "#61AFEF", "#529EE6",
                "#438DDD", "#347CD4", "#256BCB", "#165AC2",
            ]),
            red: ColorScale::from_hex([
                "#FFF0F0", "#FFE0E0", "#FFC2C2", "#FFA4A4",
                "#FF8686", "#F07070", "#E06C75", "#D45A62",
                "#C8484F", "#BC363C", "#B02429", "#A41216",
            ]),
            green: ColorScale::from_hex([
                "#F0FAF0", "#E0F5E0", "#C2E8C4", "#A4DBA8",
                "#86CE8C", "#68C170", "#98C379", "#88B56A",
                "#78A75B", "#68994C", "#588B3D", "#487D2E",
            ]),
            amber: ColorScale::from_hex([
                "#FFF8E8", "#FFF0D0", "#FFE4B0", "#FFD890",
                "#FFCC70", "#F0C060", "#E5C07B", "#D8B46C",
                "#CBA85D", "#BE9C4E", "#B1903F", "#A48430",
            ]),
        }
    }
}

// ============================================================================
// PalettePreset enum
// ============================================================================

/// Named palette presets for easy selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PalettePreset {
    /// Default oklch-based palette
    #[default]
    Default,
    /// Nord - Arctic, north-bluish
    Nord,
    /// Catppuccin Mocha - Warm pastel
    Catppuccin,
    /// Dracula - Vivid dark
    Dracula,
    /// Solarized - Precision contrast
    Solarized,
    /// Gruvbox - Retro warm
    Gruvbox,
    /// Tokyo Night - Modern neon
    TokyoNight,
    /// Rose Pine - Elegant pastel
    RosePine,
    /// One Dark - VS Code classic
    OneDark,
}

impl PalettePreset {
    /// All available presets in display order.
    pub const ALL: &[PalettePreset] = &[
        PalettePreset::Default,
        PalettePreset::Nord,
        PalettePreset::Catppuccin,
        PalettePreset::Dracula,
        PalettePreset::Solarized,
        PalettePreset::Gruvbox,
        PalettePreset::TokyoNight,
        PalettePreset::RosePine,
        PalettePreset::OneDark,
    ];

    /// Get the data-attribute value for this preset.
    pub fn as_str(&self) -> &'static str {
        match self {
            PalettePreset::Default => "default",
            PalettePreset::Nord => "nord",
            PalettePreset::Catppuccin => "catppuccin",
            PalettePreset::Dracula => "dracula",
            PalettePreset::Solarized => "solarized",
            PalettePreset::Gruvbox => "gruvbox",
            PalettePreset::TokyoNight => "tokyo-night",
            PalettePreset::RosePine => "rose-pine",
            PalettePreset::OneDark => "one-dark",
        }
    }

    /// Get the human-readable display label.
    pub fn label(&self) -> &'static str {
        match self {
            PalettePreset::Default => "Default",
            PalettePreset::Nord => "Nord",
            PalettePreset::Catppuccin => "Catppuccin",
            PalettePreset::Dracula => "Dracula",
            PalettePreset::Solarized => "Solarized",
            PalettePreset::Gruvbox => "Gruvbox",
            PalettePreset::TokyoNight => "Tokyo Night",
            PalettePreset::RosePine => "Rose Pine",
            PalettePreset::OneDark => "One Dark",
        }
    }

    /// Get the ColorPalette for this preset.
    pub fn palette(&self) -> ColorPalette {
        match self {
            PalettePreset::Default => ColorPalette::default(),
            PalettePreset::Nord => ColorPalette::nord(),
            PalettePreset::Catppuccin => ColorPalette::catppuccin(),
            PalettePreset::Dracula => ColorPalette::dracula(),
            PalettePreset::Solarized => ColorPalette::solarized(),
            PalettePreset::Gruvbox => ColorPalette::gruvbox(),
            PalettePreset::TokyoNight => ColorPalette::tokyo_night(),
            PalettePreset::RosePine => ColorPalette::rose_pine(),
            PalettePreset::OneDark => ColorPalette::one_dark(),
        }
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
        if f <= 0.04045 { f / 12.92 } else { ((f + 0.055) / 1.055).powf(2.4) }
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
        assert!(h > 200.0 && h < 300.0, "Nord blue hue should be blue-ish: {}", h);
    }

    #[test]
    fn test_parse_to_oklch_short_hex() {
        let (l, _, _) = parse_to_oklch("#FFF");
        assert!((l - 1.0).abs() < 0.01, "White #FFF L should be ~1.0, got {}", l);
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
            assert!(scale.step(i).starts_with("oklch("),
                "Step {} should be oklch: {}", i, scale.step(i));
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
            assert!(scale.step(i).starts_with("oklch("),
                "Step {} should be oklch", i);
        }
    }

    #[test]
    fn test_catppuccin_palette() {
        let palette = ColorPalette::catppuccin();
        assert_eq!(palette.neutral.steps().len(), 12);
        assert_eq!(palette.accent.steps().len(), 12);
        assert_eq!(palette.neutral.step(11), "#1E1E2E"); // Mocha base
    }

    #[test]
    fn test_dracula_palette() {
        let palette = ColorPalette::dracula();
        assert_eq!(palette.neutral.step(11), "#282A36"); // Dracula background
        assert_eq!(palette.accent.step(6), "#BD93F9"); // Dracula purple
    }

    #[test]
    fn test_solarized_palette() {
        let palette = ColorPalette::solarized();
        assert_eq!(palette.neutral.step(0), "#FDF6E3"); // Solarized light base
        assert_eq!(palette.neutral.step(11), "#002B36"); // Solarized dark base
    }

    #[test]
    fn test_gruvbox_palette() {
        let palette = ColorPalette::gruvbox();
        assert_eq!(palette.neutral.step(0), "#FBF1C7"); // Gruvbox light
        assert_eq!(palette.neutral.step(11), "#1D2021"); // Gruvbox dark
    }

    #[test]
    fn test_tokyo_night_palette() {
        let palette = ColorPalette::tokyo_night();
        assert_eq!(palette.neutral.step(11), "#1A1B26"); // Tokyo Night bg
        assert_eq!(palette.accent.step(6), "#7AA2F7"); // Tokyo Night blue
    }

    #[test]
    fn test_rose_pine_palette() {
        let palette = ColorPalette::rose_pine();
        assert_eq!(palette.neutral.step(11), "#191724"); // Rose Pine base
        assert_eq!(palette.accent.step(6), "#EB6F92"); // Rose Pine rose
    }

    #[test]
    fn test_one_dark_palette() {
        let palette = ColorPalette::one_dark();
        assert_eq!(palette.neutral.step(10), "#282C34"); // One Dark bg
        assert_eq!(palette.accent.step(6), "#61AFEF"); // One Dark blue
    }

    #[test]
    fn test_palette_preset_all() {
        assert_eq!(PalettePreset::ALL.len(), 9);
        // All presets should produce valid palettes with 12 steps each
        for preset in PalettePreset::ALL {
            let p = preset.palette();
            assert_eq!(p.neutral.steps().len(), 12, "{:?} neutral", preset);
            assert_eq!(p.accent.steps().len(), 12, "{:?} accent", preset);
            assert_eq!(p.red.steps().len(), 12, "{:?} red", preset);
            assert_eq!(p.green.steps().len(), 12, "{:?} green", preset);
            assert_eq!(p.amber.steps().len(), 12, "{:?} amber", preset);
        }
    }

    #[test]
    fn test_palette_preset_labels() {
        assert_eq!(PalettePreset::Default.label(), "Default");
        assert_eq!(PalettePreset::TokyoNight.label(), "Tokyo Night");
        assert_eq!(PalettePreset::RosePine.label(), "Rose Pine");
    }

    #[test]
    fn test_palette_preset_as_str() {
        assert_eq!(PalettePreset::Default.as_str(), "default");
        assert_eq!(PalettePreset::TokyoNight.as_str(), "tokyo-night");
        assert_eq!(PalettePreset::RosePine.as_str(), "rose-pine");
    }
}
