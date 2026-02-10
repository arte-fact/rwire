//! Theme system for rwire.
//!
//! Theme is a framework-provided state type. Handlers mutate `&mut Theme`,
//! a built-in renderer converts the theme to CSS variables, and the synced
//! element system patches them on the client. One concept, zero new abstractions.
//!
//! CSS variables use short names (`--a` through `--Z`) to minimize wire size.
//! The Rust enum variant names (`St::BgApp`, `St::Primary`) serve as documentation.
//!
//! # Example
//!
//! ```ignore
//! use rwire::theme::Theme;
//!
//! // Light theme with custom accent
//! let theme = Theme::light().accent("#5E81AC");
//!
//! // Dark theme with default blue accent
//! let theme = Theme::dark();
//!
//! // Dark theme with Nord palette
//! let theme = Theme::dark_nord();
//! ```

use crate::builder::{el, ElementBuilder};
use crate::protocol::El;
use crate::state::{State, StorageType};
use crate::tokens::ColorPalette;

/// Theme mode (light or dark).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl ThemeMode {
    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    }

    /// Toggle between light and dark mode.
    pub fn toggle(&self) -> Self {
        match self {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        }
    }
}

/// Theme style preset that remaps semantic CSS variables.
///
/// ThemeStyle controls the *feel* of components (solid vs subtle, sharp vs soft)
/// without changing individual components.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeStyle {
    /// Solid accents, medium radius, subtle shadows (current default look).
    #[default]
    Default,
    /// Subtle tinted backgrounds, large radius, no shadows.
    Soft,
    /// Sharp corners, heavy borders, high contrast.
    Brutalist,
    /// Near-zero borders, large spacing, text hierarchy.
    Minimal,
    /// Glassmorphism: frosted surfaces, backdrop-blur, subtle borders.
    Glass,
    /// Cyberpunk/Neon: glowing borders, text-shadow, dark-first.
    Neon,
}

impl ThemeStyle {
    /// All variants including Default (for UI cycling).
    pub const ALL_WITH_DEFAULT: &[ThemeStyle] = &[
        ThemeStyle::Default,
        ThemeStyle::Soft,
        ThemeStyle::Brutalist,
        ThemeStyle::Minimal,
        ThemeStyle::Glass,
        ThemeStyle::Neon,
    ];

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeStyle::Default => "default",
            ThemeStyle::Soft => "soft",
            ThemeStyle::Brutalist => "brutalist",
            ThemeStyle::Minimal => "minimal",
            ThemeStyle::Glass => "glass",
            ThemeStyle::Neon => "neon",
        }
    }

    /// Get the human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            ThemeStyle::Default => "Default",
            ThemeStyle::Soft => "Soft",
            ThemeStyle::Brutalist => "Brutalist",
            ThemeStyle::Minimal => "Minimal",
            ThemeStyle::Glass => "Glass",
            ThemeStyle::Neon => "Neon",
        }
    }
}

/// Border radius scaling for components.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RadiusScale {
    /// Sharp corners (0)
    None,
    /// Subtle rounding (sm)
    Small,
    /// Default rounding (md)
    #[default]
    Medium,
    /// Pronounced rounding (lg)
    Large,
    /// Pill shapes (full)
    Full,
}

impl RadiusScale {
    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            RadiusScale::None => "none",
            RadiusScale::Small => "small",
            RadiusScale::Medium => "medium",
            RadiusScale::Large => "large",
            RadiusScale::Full => "full",
        }
    }
}

/// Theme configuration — a framework-provided state type.
///
/// Handlers can mutate `&mut Theme` to change the visual appearance at runtime.
/// A built-in renderer converts theme state to CSS variables that get patched
/// on the client via the synced element system.
///
/// # Example
///
/// ```ignore
/// // Simple dark theme
/// let theme = Theme::dark();
///
/// // Dark with custom accent and neutral colors
/// let theme = Theme::dark().accent("#5E81AC").neutral("#2E3440");
///
/// // Full control with palette preset
/// let theme = Theme::dark().palette(ColorPalette::nord()).style(ThemeStyle::Soft);
/// ```
#[derive(Clone, Debug, Default)]
pub struct Theme {
    /// Light or dark mode
    pub mode: ThemeMode,
    /// Border radius scale
    pub radius: RadiusScale,
    /// Visual style preset
    pub style: ThemeStyle,
    /// Custom color palette (private — use builder methods)
    palette: Option<ColorPalette>,
}

// Theme implements State manually (can't use derive macro from within rwire crate).
impl State for Theme {
    const STORAGE_TYPE: StorageType = StorageType::Memory;
}

impl Theme {
    /// Create a new theme with default settings (light mode, blue accent).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a light theme.
    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            ..Self::default()
        }
    }

    /// Create a dark theme.
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            ..Self::default()
        }
    }

    /// Create a dark theme with Nord color palette.
    pub fn dark_nord() -> Self {
        Self::dark().palette(ColorPalette::nord())
    }

    /// Create a light theme with Nord color palette.
    pub fn light_nord() -> Self {
        Self::light().palette(ColorPalette::nord())
    }

    /// Set a full color palette.
    ///
    /// Use presets like `ColorPalette::nord()` or build custom palettes.
    pub fn palette(mut self, palette: ColorPalette) -> Self {
        self.palette = Some(palette);
        self
    }

    /// Set the accent color from a CSS color string.
    ///
    /// Auto-generates a 12-step color scale from the seed color.
    /// Accepts `#RRGGBB`, `#RGB`, or `oklch(L C H)`.
    pub fn accent(mut self, color: &str) -> Self {
        use crate::tokens::ColorScale;
        let mut p = self.palette.take().unwrap_or_default();
        p = p.with_accent(ColorScale::from_color(color));
        self.palette = Some(p);
        self
    }

    /// Set the neutral color from a CSS color string.
    ///
    /// Auto-generates a 12-step color scale from the seed color.
    pub fn neutral(mut self, color: &str) -> Self {
        use crate::tokens::ColorScale;
        let mut p = self.palette.take().unwrap_or_default();
        p = p.with_neutral(ColorScale::from_color(color));
        self.palette = Some(p);
        self
    }

    /// Set the error/destructive color from a CSS color string.
    pub fn error(mut self, color: &str) -> Self {
        use crate::tokens::ColorScale;
        let mut p = self.palette.take().unwrap_or_default();
        p = p.with_red(ColorScale::from_color(color));
        self.palette = Some(p);
        self
    }

    /// Set the success color from a CSS color string.
    pub fn success(mut self, color: &str) -> Self {
        use crate::tokens::ColorScale;
        let mut p = self.palette.take().unwrap_or_default();
        p = p.with_green(ColorScale::from_color(color));
        self.palette = Some(p);
        self
    }

    /// Set the warning color from a CSS color string.
    pub fn warning(mut self, color: &str) -> Self {
        use crate::tokens::ColorScale;
        let mut p = self.palette.take().unwrap_or_default();
        p = p.with_amber(ColorScale::from_color(color));
        self.palette = Some(p);
        self
    }

    /// Set the radius scale.
    pub fn radius(mut self, radius: RadiusScale) -> Self {
        self.radius = radius;
        self
    }

    /// Set the visual style preset.
    pub fn style(mut self, style: ThemeStyle) -> Self {
        self.style = style;
        self
    }

    /// Get a reference to the palette, if set.
    pub fn palette_ref(&self) -> Option<&ColorPalette> {
        self.palette.as_ref()
    }
}

// ============================================================================
// ThemeProvider — bridge between #[theme] macro and server
// ============================================================================

/// Opaque provider type returned by the `#[theme]` proc macro.
///
/// This is the only type accepted by `ServerWithRoot::theme()`, ensuring
/// themes are always defined via `#[theme]` functions.
pub struct ThemeProvider {
    /// The function that creates the initial theme value.
    pub(crate) init_fn: fn() -> Theme,
}

impl ThemeProvider {
    /// Create a new ThemeProvider (called by the `#[theme]` macro).
    pub fn new(init_fn: fn() -> Theme) -> Self {
        Self { init_fn }
    }

    /// Call the init function to get the initial theme.
    pub fn init(&self) -> Theme {
        (self.init_fn)()
    }
}

// ============================================================================
// Built-in Theme Renderer
// ============================================================================

/// The theme renderer function used by the synced element system.
///
/// Converts `Theme` state to a `<style>` element with CSS variable declarations.
fn theme_renderer(theme: &Theme) -> ElementBuilder {
    let css = generate_theme_css(theme);
    el(El::Style).text(&css)
}

/// Create an `ElementBuilder` representing the theme synced region.
///
/// This returns a synced element that, when included in the element tree,
/// renders to a `<style>` element with CSS variable declarations.
/// On theme state changes, the synced element system patches the `<style>`
/// content, causing an instant browser restyle.
pub(crate) fn theme_synced_builder() -> ElementBuilder {
    ElementBuilder::synced::<Theme>(theme_renderer)
}

// ============================================================================
// Resolved Palette — flattens all color indirection at build time
// ============================================================================

/// A fully resolved color palette where all indirection has been eliminated.
///
/// Instead of `var(--rw-neutral-1)`, semantic variables get the actual CSS value
/// like `oklch(0.985 0 0)`. This struct is constructed once at capsule build time.
pub(crate) struct ResolvedPalette {
    pub neutral: [String; 12],
    pub accent: [String; 12],
    pub red: [String; 12],
    pub green: [String; 12],
    pub amber: [String; 12],
    pub white: String,
}

impl ResolvedPalette {
    /// Build a resolved palette from a theme.
    ///
    /// If the theme has a custom palette, its colors are used. Otherwise, the
    /// default Oklch primitives are used. The accent scale is always the palette's
    /// accent (no more `AccentColor` routing).
    pub fn new(theme: &Theme) -> Self {
        use crate::tokens::primitives::color;

        let default_palette = ColorPalette::default();
        let p = theme.palette_ref().unwrap_or(&default_palette);

        Self {
            neutral: p.neutral.steps().clone(),
            accent: p.accent.steps().clone(),
            red: p.red.steps().clone(),
            green: p.green.steps().clone(),
            amber: p.amber.steps().clone(),
            white: color::WHITE.to_string(),
        }
    }
}

// ============================================================================
// Short CSS Variable Name Mapping
// ============================================================================
//
// Semantic variables use short names --a through --Z to minimize CSS size.
// Rust enum variant names (St::BgApp, St::Primary, etc.) serve as documentation.
//
// Backgrounds:       --a (bg-app), --b (bg-subtle), --c (bg-muted), --d (bg-emphasis),
//                    --e (bg-hover), --f (bg-active)
// Borders:           --g (border-subtle), --h (border-default), --i (border-emphasis)
// Text:              --j (text-muted), --k (text-default), --l (text-high),
//                    --m (text-on-accent)
// Accent scale:      --n1..--n12 (accent-1..accent-12)
// Status:            --o (success), --p (warning), --q (error)
// Surface:           --r (surface), --s (on-surface), --t (surface-raised),
//                    --u (on-surface-raised)
// Primary:           --v (primary), --w (on-primary), --x (primary-hover),
//                    --y (primary-subtle), --z (on-primary-subtle)
// Secondary:         --A (secondary), --B (on-secondary), --C (secondary-hover)
// Muted pair:        --D (muted), --E (on-muted)
// Destructive:       --F (destructive), --G (on-destructive), --H (destructive-hover),
//                    --I (destructive-subtle), --J (on-destructive-subtle)
// Interactive:       --K (focus-ring), --L (border-primary)

/// Generate base/reset CSS.
///
/// Minimal normalization for consistent cross-browser behavior.
/// Uses short CSS variable names.
pub fn generate_base_css() -> &'static str {
    "html{scroll-behavior:smooth}\
     *,*::before,*::after{box-sizing:border-box}\
     body{margin:0;font-family:system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",Roboto,sans-serif;\
     line-height:var(--X3);color:var(--k);background:var(--a)}\
     button,input,select,textarea{font:inherit}\
     table{border-collapse:collapse}\
     img{max-width:100%;height:auto}\n"
}

/// Generate complete theme CSS from a Theme value.
///
/// Produces a single `:root{...}` block with all resolved CSS variables.
/// Mode (light/dark) is resolved server-side: picks the right scale indices.
/// Style preset Q-vars and radius overrides are resolved inline.
/// No `data-theme`, `data-style`, or `data-radius` attribute selectors.
pub fn generate_theme_css(theme: &Theme) -> String {
    let rp = ResolvedPalette::new(theme);
    let is_dark = theme.mode == ThemeMode::Dark;
    let mut css = String::with_capacity(2048);

    css.push_str(":root{");

    // --- Semantic color variables (mode-aware) ---
    if is_dark {
        // Dark mode: invert scales
        v(&mut css, "a", &rp.neutral[11]); // bg-app
        v(&mut css, "b", &rp.neutral[10]); // bg-subtle
        v(&mut css, "c", &rp.neutral[9]);  // bg-muted
        v(&mut css, "d", &rp.neutral[8]);  // bg-emphasis
        v(&mut css, "e", &rp.neutral[7]);  // bg-hover
        v(&mut css, "f", &rp.neutral[6]);  // bg-active
        v(&mut css, "g", &rp.neutral[6]);  // border-subtle
        v(&mut css, "h", &rp.neutral[5]);  // border-default
        v(&mut css, "i", &rp.neutral[4]);  // border-emphasis
        v(&mut css, "j", &rp.neutral[4]);  // text-muted
        v(&mut css, "k", &rp.neutral[1]);  // text-default
        v(&mut css, "l", &rp.neutral[0]);  // text-high
        v(&mut css, "m", &rp.white);       // text-on-accent
        v(&mut css, "r", &rp.neutral[11]); // surface
        v(&mut css, "s", &rp.neutral[0]);  // on-surface
        v(&mut css, "t", &rp.neutral[10]); // surface-raised
        v(&mut css, "u", &rp.neutral[0]);  // on-surface-raised
        v(&mut css, "A", &rp.neutral[8]);  // secondary
        v(&mut css, "B", &rp.neutral[0]);  // on-secondary
        v(&mut css, "C", &rp.neutral[7]);  // secondary-hover
        v(&mut css, "D", &rp.neutral[9]);  // muted
        v(&mut css, "E", &rp.neutral[3]);  // on-muted
        v(&mut css, "I", &rp.red[9]);      // destructive-subtle
        v(&mut css, "J", &rp.red[2]);      // on-destructive-subtle
        v(&mut css, "y", &rp.accent[9]);   // primary-subtle
        v(&mut css, "z", &rp.accent[2]);   // on-primary-subtle
    } else {
        // Light mode
        v(&mut css, "a", &rp.neutral[0]);  // bg-app
        v(&mut css, "b", &rp.neutral[1]);  // bg-subtle
        v(&mut css, "c", &rp.neutral[2]);  // bg-muted
        v(&mut css, "d", &rp.neutral[3]);  // bg-emphasis
        v(&mut css, "e", &rp.neutral[4]);  // bg-hover
        v(&mut css, "f", &rp.neutral[5]);  // bg-active
        v(&mut css, "g", &rp.neutral[5]);  // border-subtle
        v(&mut css, "h", &rp.neutral[6]);  // border-default
        v(&mut css, "i", &rp.neutral[7]);  // border-emphasis
        v(&mut css, "j", &rp.neutral[8]);  // text-muted
        v(&mut css, "k", &rp.neutral[10]); // text-default
        v(&mut css, "l", &rp.neutral[11]); // text-high
        v(&mut css, "m", &rp.white);       // text-on-accent
        v(&mut css, "r", &rp.neutral[0]);  // surface
        v(&mut css, "s", &rp.neutral[11]); // on-surface
        v(&mut css, "t", &rp.neutral[1]);  // surface-raised
        v(&mut css, "u", &rp.neutral[11]); // on-surface-raised
        v(&mut css, "A", &rp.neutral[3]);  // secondary
        v(&mut css, "B", &rp.neutral[11]); // on-secondary
        v(&mut css, "C", &rp.neutral[4]);  // secondary-hover
        v(&mut css, "D", &rp.neutral[2]);  // muted
        v(&mut css, "E", &rp.neutral[10]); // on-muted
        v(&mut css, "I", &rp.red[2]);      // destructive-subtle
        v(&mut css, "J", &rp.red[10]);     // on-destructive-subtle
        v(&mut css, "y", &rp.accent[2]);   // primary-subtle
        v(&mut css, "z", &rp.accent[10]);  // on-primary-subtle
    }

    // Mode-independent vars
    for i in 0..12 {
        vn(&mut css, "n", i + 1, &rp.accent[i]);
    }
    v(&mut css, "o", &rp.green[8]);    // success
    v(&mut css, "p", &rp.amber[8]);    // warning
    v(&mut css, "q", &rp.red[8]);      // error
    v(&mut css, "F", &rp.red[8]);      // destructive
    v(&mut css, "G", &rp.white);       // on-destructive
    v(&mut css, "H", &rp.red[9]);      // destructive-hover
    v(&mut css, "K", &rp.accent[7]);   // focus-ring
    v(&mut css, "L", &rp.accent[6]);   // border-primary

    // --- Style preset: resolve primary/surface/border overrides inline ---
    match theme.style {
        ThemeStyle::Default => {
            v(&mut css, "v", &rp.accent[8]);   // primary
            v(&mut css, "w", &rp.white);       // on-primary
            v(&mut css, "x", &rp.accent[9]);   // primary-hover
        }
        ThemeStyle::Soft => {
            if is_dark {
                v(&mut css, "v", &rp.accent[9]);
                v(&mut css, "w", &rp.accent[2]);
                v(&mut css, "x", &rp.accent[8]);
                v(&mut css, "F", &rp.red[9]);
                v(&mut css, "G", &rp.red[2]);
                v(&mut css, "H", &rp.red[8]);
                v(&mut css, "h", &rp.neutral[if is_dark { 9 } else { 3 }]);
                css.push_str("--g:transparent;");
                v(&mut css, "t", &rp.neutral[if is_dark { 9 } else { 0 }]);
            } else {
                v(&mut css, "v", &rp.accent[2]);
                v(&mut css, "w", &rp.accent[10]);
                v(&mut css, "x", &rp.accent[3]);
                v(&mut css, "F", &rp.red[2]);
                v(&mut css, "G", &rp.red[10]);
                v(&mut css, "H", &rp.red[3]);
                v(&mut css, "h", &rp.neutral[3]);
                v(&mut css, "g", &rp.neutral[2]);
                v(&mut css, "t", &rp.neutral[0]);
            }
        }
        ThemeStyle::Brutalist => {
            v(&mut css, "v", &rp.accent[8]);
            v(&mut css, "w", &rp.white);
            v(&mut css, "x", &rp.accent[9]);
            if is_dark {
                v(&mut css, "h", &rp.neutral[0]);
                v(&mut css, "g", &rp.neutral[2]);
                v(&mut css, "i", &rp.neutral[0]);
                v(&mut css, "t", &rp.neutral[11]);
            } else {
                v(&mut css, "h", &rp.neutral[11]);
                v(&mut css, "g", &rp.neutral[9]);
                v(&mut css, "i", &rp.neutral[11]);
                v(&mut css, "t", &rp.neutral[0]);
            }
        }
        ThemeStyle::Minimal => {
            v(&mut css, "v", &rp.accent[8]);
            v(&mut css, "w", &rp.white);
            v(&mut css, "x", &rp.accent[9]);
            css.push_str("--h:transparent;--g:transparent;");
            v(&mut css, "t", &rp.neutral[if is_dark { 11 } else { 0 }]);
        }
        ThemeStyle::Glass => {
            v(&mut css, "v", &rp.accent[8]);
            v(&mut css, "w", &rp.white);
            v(&mut css, "x", &rp.accent[9]);
        }
        ThemeStyle::Neon => {
            v(&mut css, "v", &rp.accent[8]);
            v(&mut css, "w", &rp.white);
            v(&mut css, "x", &rp.accent[9]);
        }
    }

    // --- Q-vars (theme-style hooks for non-color customization) ---
    match theme.style {
        ThemeStyle::Default => {
            css.push_str("--Qd:var(--Z1);--Qgw:none;");
            css.push_str("--Qb:1px;--Qbl:var(--Qb);--Qbt:var(--Qb);--Qbc:var(--h);--Qbs:solid;");
            css.push_str("--Qol:2px;--Qoo:2px;--Qf:var(--K);");
            css.push_str("--Qbf:none;--Qso:1;--Qgr:none;");
            css.push_str("--Qt:150ms;--Qts:none;");
        }
        ThemeStyle::Soft => {
            css.push_str("--Qd:none;--Qgw:none;");
            css.push_str("--Qb:1px;--Qbl:var(--Qb);--Qbt:var(--Qb);--Qbc:var(--h);--Qbs:solid;");
            css.push_str("--Qol:2px;--Qoo:2px;--Qf:var(--K);");
            css.push_str("--Qbf:none;--Qso:1;--Qgr:none;");
            css.push_str("--Qt:200ms;--Qts:none;");
        }
        ThemeStyle::Brutalist => {
            css.push_str("--Qd:none;--Qgw:none;");
            css.push_str("--Qb:2px;--Qbl:3px;--Qbt:var(--Qb);--Qbc:var(--h);--Qbs:solid;");
            css.push_str("--Qol:3px;--Qoo:0px;--Qf:var(--K);");
            css.push_str("--Qbf:none;--Qso:1;--Qgr:none;");
            css.push_str("--Qt:0ms;--Qts:none;");
        }
        ThemeStyle::Minimal => {
            css.push_str("--Qd:none;--Qgw:none;");
            css.push_str("--Qb:0;--Qbl:var(--Qb);--Qbt:var(--Qb);--Qbc:transparent;--Qbs:none;");
            css.push_str("--Qol:2px;--Qoo:2px;--Qf:var(--K);");
            css.push_str("--Qbf:none;--Qso:1;--Qgr:none;");
            css.push_str("--Qt:200ms;--Qts:none;");
        }
        ThemeStyle::Glass => {
            css.push_str("--Qd:none;--Qgw:none;");
            css.push_str("--Qb:1px;--Qbl:var(--Qb);--Qbt:var(--Qb);--Qbc:rgba(255,255,255,0.1);--Qbs:solid;");
            css.push_str("--Qol:2px;--Qoo:2px;--Qf:var(--K);");
            if is_dark {
                css.push_str("--Qbf:blur(12px);--Qso:0.75;--Qgr:none;");
            } else {
                css.push_str("--Qbf:blur(12px);--Qso:0.85;--Qgr:none;");
            }
            css.push_str("--Qt:200ms;--Qts:none;");
        }
        ThemeStyle::Neon => {
            css.push_str("--Qd:none;--Qgw:0 0 8px var(--n9),0 0 16px var(--n9);");
            css.push_str("--Qb:2px;--Qbl:var(--Qb);--Qbt:var(--Qb);--Qbc:var(--n9);--Qbs:solid;");
            css.push_str("--Qol:2px;--Qoo:2px;--Qf:var(--n9);");
            css.push_str("--Qbf:none;--Qso:1;--Qgr:none;");
            css.push_str("--Qt:50ms;--Qts:0 0 8px currentColor;");
        }
    }

    // --- Radius overrides ---
    match theme.radius {
        RadiusScale::Medium => {} // default, no overrides needed
        RadiusScale::None => css.push_str("--R2:0;--R3:0;--R4:0;"),
        RadiusScale::Small => css.push_str("--R2:var(--R1);--R3:var(--R1);--R4:var(--R2);"),
        RadiusScale::Large => css.push_str("--R2:var(--R3);--R3:var(--R4);--R4:var(--R5);"),
        RadiusScale::Full => css.push_str("--R2:9999px;--R3:9999px;--R4:9999px;"),
    }

    css.push_str("}\n");
    css
}


/// Helper: write `--{name}:{value};`
fn v(css: &mut String, name: &str, value: &str) {
    css.push_str("--");
    css.push_str(name);
    css.push(':');
    css.push_str(value);
    css.push(';');
}

/// Helper: write `--{prefix}{num}:{value};` for numbered variables (accent scale)
fn vn(css: &mut String, prefix: &str, num: usize, value: &str) {
    css.push_str("--");
    css.push_str(prefix);
    use std::fmt::Write;
    let _ = write!(css, "{}:{};", num, value);
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_defaults() {
        let theme = Theme::default();
        assert_eq!(theme.mode, ThemeMode::Light);
        assert_eq!(theme.radius, RadiusScale::Medium);
        assert_eq!(theme.style, ThemeStyle::Default);
        assert!(theme.palette_ref().is_none());
    }

    #[test]
    fn test_theme_builders() {
        let theme = Theme::dark().accent("#5E81AC").radius(RadiusScale::Large);

        assert_eq!(theme.mode, ThemeMode::Dark);
        assert_eq!(theme.radius, RadiusScale::Large);
        assert!(theme.palette_ref().is_some());
    }

    #[test]
    fn test_theme_palette_preset() {
        let theme = Theme::dark_nord();
        assert_eq!(theme.mode, ThemeMode::Dark);
        assert!(theme.palette_ref().is_some());
    }

    #[test]
    fn test_theme_seed_colors() {
        let theme = Theme::dark()
            .accent("#5E81AC")
            .neutral("#2E3440")
            .error("#BF616A")
            .success("#A3BE8C")
            .warning("#EBCB8B");
        assert!(theme.palette_ref().is_some());
    }

    #[test]
    fn test_theme_mode_toggle() {
        assert_eq!(ThemeMode::Light.toggle(), ThemeMode::Dark);
        assert_eq!(ThemeMode::Dark.toggle(), ThemeMode::Light);
    }

    #[test]
    fn test_base_css_size() {
        let css = generate_base_css();
        assert!(css.len() < 400, "Base CSS too large: {} bytes", css.len());
    }

    #[test]
    fn test_generate_theme_css_light() {
        let theme = Theme::light();
        let css = generate_theme_css(&theme);

        // Single :root block, no data-theme selectors
        assert!(css.starts_with(":root{"), "Should start with :root{{");
        assert!(!css.contains("[data-theme"), "Should not contain data-theme selectors");
        assert!(!css.contains("[data-style"), "Should not contain data-style selectors");

        // Should have resolved values
        assert!(css.contains("--a:"), "Missing --a (bg-app)");
        assert!(css.contains("--k:"), "Missing --k (text-default)");
        assert!(css.contains("--v:"), "Missing --v (primary)");
        assert!(css.contains("oklch("), "Should contain resolved oklch values");

        // Should include Q-vars
        assert!(css.contains("--Qd:"), "Missing Q-var --Qd");
        assert!(css.contains("--Qt:"), "Missing Q-var --Qt");
    }

    #[test]
    fn test_generate_theme_css_dark() {
        let theme = Theme::dark();
        let css = generate_theme_css(&theme);

        assert!(css.starts_with(":root{"), "Should start with :root{{");
        assert!(!css.contains("[data-theme"), "No data-theme selectors");
    }

    #[test]
    fn test_generate_theme_css_dark_nord() {
        let theme = Theme::dark_nord();
        let css = generate_theme_css(&theme);

        assert!(css.starts_with(":root{"), "Should start with :root{{");
        assert!(css.contains("--n1:"), "Missing accent scale --n1");
    }

    #[test]
    fn test_generate_theme_css_with_style() {
        let theme = Theme::dark().style(ThemeStyle::Soft);
        let css = generate_theme_css(&theme);

        // Soft style Q-vars should be inline
        assert!(css.contains("--Qd:none"), "Soft should have --Qd:none");
        assert!(css.contains("--Qt:200ms"), "Soft should have --Qt:200ms");
        assert!(!css.contains("[data-style"), "No data-style selectors");
    }

    #[test]
    fn test_generate_theme_css_with_radius() {
        let theme = Theme::light().radius(RadiusScale::Full);
        let css = generate_theme_css(&theme);

        assert!(css.contains("--R2:9999px"), "Full radius should override --R2");
        assert!(!css.contains("[data-radius"), "No data-radius selectors");
    }

    #[test]
    fn test_generate_theme_css_size() {
        let theme = Theme::dark_nord().style(ThemeStyle::Soft).radius(RadiusScale::Large);
        let css = generate_theme_css(&theme);
        // Single block should be compact
        assert!(css.len() < 2048, "Theme CSS too large: {} bytes", css.len());
    }

    #[test]
    fn test_theme_implements_state() {
        assert_eq!(Theme::STORAGE_TYPE, StorageType::Memory);
    }
}
