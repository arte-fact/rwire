//! Predefined color palettes for rwire themes.
//!
//! Named palettes inspired by popular editor and terminal themes.
//!
//! # Usage
//!
//! ```ignore
//! use rwire::theme::Theme;
//! use rwire_themes::palettes;
//!
//! let theme = Theme::dark().palette(palettes::nord());
//! ```

use rwire::theme::IntoPalette;
use rwire::tokens::palette::{ColorPalette, ColorScale};

/// Nord color palette.
///
/// An arctic, north-bluish color palette with:
/// - Polar Night (dark backgrounds)
/// - Snow Storm (light backgrounds)
/// - Frost (accent blues)
/// - Aurora (semantic colors)
///
/// See: <https://www.nordtheme.com/>
pub fn nord() -> ColorPalette {
    ColorPalette {
        neutral: ColorScale::from_hex([
            "#ECEFF4", "#E5E9F0", "#D8DEE9", "#C8CED8",
            "#B8BFC9", "#A8B0BA", "#6D7A8A", "#4C566A",
            "#434C5E", "#3B4252", "#2E3440", "#242933",
        ]),
        accent: ColorScale::from_hex([
            "#F0F4F8", "#E3EDF5", "#C9DDE9", "#A3C9DC",
            "#8FBCBB", "#88C0D0", "#81A1C1", "#6E94B4",
            "#5E81AC", "#5476A0", "#4C6B94", "#3D5A80",
        ]),
        red: ColorScale::from_hex([
            "#FDF2F2", "#FAE5E5", "#F5CCCC", "#EBADAD",
            "#D98E8E", "#C87878", "#BF616A", "#B55760",
            "#A84D56", "#9A444C", "#8C3B42", "#7D3238",
        ]),
        green: ColorScale::from_hex([
            "#F4F8F4", "#E8F0E8", "#D4E4D4", "#BCD6BC",
            "#A8C8A8", "#96BA96", "#A3BE8C", "#94AE7E",
            "#859E70", "#768E62", "#677E54", "#586E46",
        ]),
        amber: ColorScale::from_hex([
            "#FFFBF0", "#FFF5DC", "#FFECC4", "#FFE0A8",
            "#F5D08C", "#EBCB8B", "#D9B870", "#C7A55A",
            "#D08770", "#C47A64", "#A66A50", "#8A5A40",
        ]),
    }
}

/// Catppuccin Mocha color palette.
///
/// Warm pastel colors with a cozy, modern feel.
/// Accent: Mauve, Neutral: warm gray.
pub fn catppuccin() -> ColorPalette {
    ColorPalette {
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
pub fn dracula() -> ColorPalette {
    ColorPalette {
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
/// See: <https://ethanschoonover.com/solarized/>
pub fn solarized() -> ColorPalette {
    ColorPalette {
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
/// See: <https://github.com/morhetz/gruvbox>
pub fn gruvbox() -> ColorPalette {
    ColorPalette {
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
pub fn tokyo_night() -> ColorPalette {
    ColorPalette {
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
/// See: <https://rosepinetheme.com/>
pub fn rose_pine() -> ColorPalette {
    ColorPalette {
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
pub fn one_dark() -> ColorPalette {
    ColorPalette {
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

/// Modern indigo palette.
///
/// High-contrast slate neutrals with a vibrant indigo accent.
/// Designed for excellent readability in both light and dark modes.
pub fn indigo() -> ColorPalette {
    ColorPalette {
        neutral: ColorScale::from_hex([
            "#F8FAFC", "#F1F5F9", "#E2E8F0", "#CBD5E1",
            "#94A3B8", "#64748B", "#475569", "#334155",
            "#1E293B", "#0F172A", "#0C1322", "#080E1A",
        ]),
        accent: ColorScale::from_hex([
            "#EEF2FF", "#E0E7FF", "#C7D2FE", "#A5B4FC",
            "#818CF8", "#6366F1", "#4F46E5", "#4338CA",
            "#3730A3", "#312E81", "#2E1065", "#1E1B4B",
        ]),
        red: ColorScale::from_hex([
            "#FEF2F2", "#FEE2E2", "#FECACA", "#FCA5A5",
            "#F87171", "#EF4444", "#DC2626", "#B91C1C",
            "#991B1B", "#7F1D1D", "#6B1515", "#570D0D",
        ]),
        green: ColorScale::from_hex([
            "#F0FDF4", "#DCFCE7", "#BBF7D0", "#86EFAC",
            "#4ADE80", "#22C55E", "#16A34A", "#15803D",
            "#166534", "#14532D", "#0F4025", "#0A2D1C",
        ]),
        amber: ColorScale::from_hex([
            "#FFFBEB", "#FEF3C7", "#FDE68A", "#FCD34D",
            "#FBBF24", "#F59E0B", "#D97706", "#B45309",
            "#92400E", "#78350F", "#613010", "#4A2510",
        ]),
    }
}

/// Convenience enum for all palette presets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Palette {
    Nord,
    Indigo,
    Catppuccin,
    Dracula,
    Solarized,
    Gruvbox,
    TokyoNight,
    RosePine,
    OneDark,
}

impl Palette {
    /// All palette variants.
    pub const ALL: &[Palette] = &[
        Palette::Nord,
        Palette::Indigo,
        Palette::Catppuccin,
        Palette::Dracula,
        Palette::Solarized,
        Palette::Gruvbox,
        Palette::TokyoNight,
        Palette::RosePine,
        Palette::OneDark,
    ];

    /// Get the human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            Palette::Nord => "Nord",
            Palette::Indigo => "Indigo",
            Palette::Catppuccin => "Catppuccin",
            Palette::Dracula => "Dracula",
            Palette::Solarized => "Solarized",
            Palette::Gruvbox => "Gruvbox",
            Palette::TokyoNight => "Tokyo Night",
            Palette::RosePine => "Rose Pine",
            Palette::OneDark => "One Dark",
        }
    }

    /// Get the identifier string.
    pub fn id(&self) -> &'static str {
        match self {
            Palette::Nord => "nord",
            Palette::Indigo => "indigo",
            Palette::Catppuccin => "catppuccin",
            Palette::Dracula => "dracula",
            Palette::Solarized => "solarized",
            Palette::Gruvbox => "gruvbox",
            Palette::TokyoNight => "tokyo-night",
            Palette::RosePine => "rose-pine",
            Palette::OneDark => "one-dark",
        }
    }
}

impl IntoPalette for Palette {
    fn into_palette(self) -> ColorPalette {
        match self {
            Palette::Nord => nord(),
            Palette::Indigo => indigo(),
            Palette::Catppuccin => catppuccin(),
            Palette::Dracula => dracula(),
            Palette::Solarized => solarized(),
            Palette::Gruvbox => gruvbox(),
            Palette::TokyoNight => tokyo_night(),
            Palette::RosePine => rose_pine(),
            Palette::OneDark => one_dark(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nord_palette() {
        let palette = nord();
        assert_eq!(palette.neutral.step(0), "#ECEFF4");
        assert_eq!(palette.neutral.step(10), "#2E3440");
        assert_eq!(palette.accent.step(8), "#5E81AC");
    }

    #[test]
    fn test_catppuccin_palette() {
        let palette = catppuccin();
        assert_eq!(palette.neutral.steps().len(), 12);
        assert_eq!(palette.neutral.step(11), "#1E1E2E");
    }

    #[test]
    fn test_dracula_palette() {
        let palette = dracula();
        assert_eq!(palette.neutral.step(11), "#282A36");
        assert_eq!(palette.accent.step(6), "#BD93F9");
    }

    #[test]
    fn test_solarized_palette() {
        let palette = solarized();
        assert_eq!(palette.neutral.step(0), "#FDF6E3");
        assert_eq!(palette.neutral.step(11), "#002B36");
    }

    #[test]
    fn test_gruvbox_palette() {
        let palette = gruvbox();
        assert_eq!(palette.neutral.step(0), "#FBF1C7");
        assert_eq!(palette.neutral.step(11), "#1D2021");
    }

    #[test]
    fn test_tokyo_night_palette() {
        let palette = tokyo_night();
        assert_eq!(palette.neutral.step(11), "#1A1B26");
        assert_eq!(palette.accent.step(6), "#7AA2F7");
    }

    #[test]
    fn test_rose_pine_palette() {
        let palette = rose_pine();
        assert_eq!(palette.neutral.step(11), "#191724");
        assert_eq!(palette.accent.step(6), "#EB6F92");
    }

    #[test]
    fn test_one_dark_palette() {
        let palette = one_dark();
        assert_eq!(palette.neutral.step(10), "#282C34");
        assert_eq!(palette.accent.step(6), "#61AFEF");
    }

    #[test]
    fn test_all_palettes_valid() {
        let all_fns: &[fn() -> ColorPalette] = &[
            nord, catppuccin, dracula, solarized, gruvbox, tokyo_night, rose_pine, one_dark,
        ];
        for palette_fn in all_fns {
            let p = palette_fn();
            assert_eq!(p.neutral.steps().len(), 12);
            assert_eq!(p.accent.steps().len(), 12);
            assert_eq!(p.red.steps().len(), 12);
            assert_eq!(p.green.steps().len(), 12);
            assert_eq!(p.amber.steps().len(), 12);
        }
    }

    #[test]
    fn test_palette_enum() {
        use rwire::theme::Theme;
        let theme = Theme::dark().palette(Palette::Nord);
        assert!(theme.palette_ref().is_some());
    }

    #[test]
    fn test_palette_labels() {
        assert_eq!(Palette::Nord.label(), "Nord");
        assert_eq!(Palette::TokyoNight.label(), "Tokyo Night");
        assert_eq!(Palette::RosePine.label(), "Rose Pine");
    }

    #[test]
    fn test_palette_ids() {
        assert_eq!(Palette::Nord.id(), "nord");
        assert_eq!(Palette::TokyoNight.id(), "tokyo-night");
        assert_eq!(Palette::RosePine.id(), "rose-pine");
    }
}
