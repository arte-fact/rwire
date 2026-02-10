//! Theme system for rwire.
//!
//! Colors are resolved at build time: the server knows the full theme configuration
//! (palette, accent color, mode) so we flatten `var(--rw-neutral-1)` references into
//! direct CSS values like `oklch(0.985 0 0)`. Only light/dark switching remains
//! runtime via `data-theme` attribute.
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

use crate::tokens::ColorPalette;

/// Theme mode (light or dark).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl ThemeMode {
    /// Get the data-theme attribute value.
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
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
    /// All non-default style variants (for CSS generation).
    pub const ALL: &[ThemeStyle] = &[
        ThemeStyle::Soft,
        ThemeStyle::Brutalist,
        ThemeStyle::Minimal,
        ThemeStyle::Glass,
        ThemeStyle::Neon,
    ];

    /// All variants including Default (for UI cycling).
    pub const ALL_WITH_DEFAULT: &[ThemeStyle] = &[
        ThemeStyle::Default,
        ThemeStyle::Soft,
        ThemeStyle::Brutalist,
        ThemeStyle::Minimal,
        ThemeStyle::Glass,
        ThemeStyle::Neon,
    ];

    /// Get the data-style attribute value.
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
    /// Get the data-radius attribute value.
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

/// Theme configuration.
///
/// Defines the visual appearance of the application. Colors are configured
/// via builder methods that accept CSS color strings (hex, oklch).
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

    /// Generate data attributes for the theme root element.
    ///
    /// Returns a string like `data-theme="dark"`.
    pub fn data_attrs(&self) -> String {
        let mut attrs = format!("data-theme=\"{}\"", self.mode.as_str());

        // Only add radius attr if not default (medium)
        if self.radius != RadiusScale::Medium {
            attrs.push_str(&format!(" data-radius=\"{}\"", self.radius.as_str()));
        }

        // Only add style attr if not default
        if self.style != ThemeStyle::Default {
            attrs.push_str(&format!(" data-style=\"{}\"", self.style.as_str()));
        }

        attrs
    }
}

// ============================================================================
// Resolved Palette — flattens all color indirection at build time
// ============================================================================

/// A fully resolved color palette where all indirection has been eliminated.
///
/// Instead of `var(--rw-neutral-1)`, semantic variables get the actual CSS value
/// like `oklch(0.985 0 0)`. This struct is constructed once at capsule build time.
pub struct ResolvedPalette {
    pub neutral: [String; 12],
    pub accent: [String; 12],
    pub red: [String; 12],
    pub green: [String; 12],
    pub amber: [String; 12],
    pub white: String,
    pub black: String,
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
            black: color::BLACK.to_string(),
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

/// Generate resolved semantic CSS for light and dark themes.
///
/// All color values are resolved from `ResolvedPalette` — no `var(--rw-neutral-*)` indirection.
/// Uses short CSS variable names (`--a` through `--L`).
pub fn generate_resolved_semantic_css(rp: &ResolvedPalette) -> String {
    let mut css = String::with_capacity(2048);

    // Light theme (default)
    css.push_str(":root,[data-theme=\"light\"]{");
    // Backgrounds
    v(&mut css, "a", &rp.neutral[0]);  // bg-app
    v(&mut css, "b", &rp.neutral[1]);  // bg-subtle
    v(&mut css, "c", &rp.neutral[2]);  // bg-muted
    v(&mut css, "d", &rp.neutral[3]);  // bg-emphasis
    v(&mut css, "e", &rp.neutral[4]);  // bg-hover
    v(&mut css, "f", &rp.neutral[5]);  // bg-active
    // Borders
    v(&mut css, "g", &rp.neutral[5]);  // border-subtle
    v(&mut css, "h", &rp.neutral[6]);  // border-default
    v(&mut css, "i", &rp.neutral[7]);  // border-emphasis
    // Text
    v(&mut css, "j", &rp.neutral[8]);  // text-muted
    v(&mut css, "k", &rp.neutral[10]); // text-default
    v(&mut css, "l", &rp.neutral[11]); // text-high
    v(&mut css, "m", &rp.white);       // text-on-accent
    // Accent scale (1-12)
    for i in 0..12 {
        vn(&mut css, "n", i + 1, &rp.accent[i]);
    }
    // Status
    v(&mut css, "o", &rp.green[8]);    // success
    v(&mut css, "p", &rp.amber[8]);    // warning
    v(&mut css, "q", &rp.red[8]);      // error
    // Surface pairs
    v(&mut css, "r", &rp.neutral[0]);  // surface
    v(&mut css, "s", &rp.neutral[11]); // on-surface
    v(&mut css, "t", &rp.neutral[1]);  // surface-raised
    v(&mut css, "u", &rp.neutral[11]); // on-surface-raised
    // Primary pairs (accent-based)
    v(&mut css, "v", &rp.accent[8]);   // primary
    v(&mut css, "w", &rp.white);       // on-primary
    v(&mut css, "x", &rp.accent[9]);   // primary-hover
    v(&mut css, "y", &rp.accent[2]);   // primary-subtle
    v(&mut css, "z", &rp.accent[10]);  // on-primary-subtle
    // Secondary pairs (neutral-based)
    v(&mut css, "A", &rp.neutral[3]);  // secondary
    v(&mut css, "B", &rp.neutral[11]); // on-secondary
    v(&mut css, "C", &rp.neutral[4]);  // secondary-hover
    // Muted pairs
    v(&mut css, "D", &rp.neutral[2]);  // muted
    v(&mut css, "E", &rp.neutral[10]); // on-muted
    // Destructive pairs
    v(&mut css, "F", &rp.red[8]);      // destructive
    v(&mut css, "G", &rp.white);       // on-destructive
    v(&mut css, "H", &rp.red[9]);      // destructive-hover
    v(&mut css, "I", &rp.red[2]);      // destructive-subtle
    v(&mut css, "J", &rp.red[10]);     // on-destructive-subtle
    // Interactive
    v(&mut css, "K", &rp.accent[7]);   // focus-ring
    v(&mut css, "L", &rp.accent[6]);   // border-primary
    css.push_str("}\n");

    // Dark theme — invert scales
    css.push_str("[data-theme=\"dark\"]{");
    // Backgrounds (inverted)
    v(&mut css, "a", &rp.neutral[11]); // bg-app
    v(&mut css, "b", &rp.neutral[10]); // bg-subtle
    v(&mut css, "c", &rp.neutral[9]);  // bg-muted
    v(&mut css, "d", &rp.neutral[8]);  // bg-emphasis
    v(&mut css, "e", &rp.neutral[7]);  // bg-hover
    v(&mut css, "f", &rp.neutral[6]);  // bg-active
    // Borders (inverted)
    v(&mut css, "g", &rp.neutral[6]);  // border-subtle
    v(&mut css, "h", &rp.neutral[5]);  // border-default
    v(&mut css, "i", &rp.neutral[4]);  // border-emphasis
    // Text (inverted)
    v(&mut css, "j", &rp.neutral[4]);  // text-muted
    v(&mut css, "k", &rp.neutral[1]);  // text-default
    v(&mut css, "l", &rp.neutral[0]);  // text-high
    // Surface pairs (inverted)
    v(&mut css, "r", &rp.neutral[11]); // surface
    v(&mut css, "s", &rp.neutral[0]);  // on-surface
    v(&mut css, "t", &rp.neutral[10]); // surface-raised
    v(&mut css, "u", &rp.neutral[0]);  // on-surface-raised
    // Secondary (inverted)
    v(&mut css, "A", &rp.neutral[8]);  // secondary
    v(&mut css, "B", &rp.neutral[0]);  // on-secondary
    v(&mut css, "C", &rp.neutral[7]);  // secondary-hover
    // Muted (inverted)
    v(&mut css, "D", &rp.neutral[9]);  // muted
    v(&mut css, "E", &rp.neutral[3]);  // on-muted
    // Destructive subtle (inverted)
    v(&mut css, "I", &rp.red[9]);      // destructive-subtle
    v(&mut css, "J", &rp.red[2]);      // on-destructive-subtle
    // Primary subtle (inverted)
    v(&mut css, "y", &rp.accent[9]);   // primary-subtle
    v(&mut css, "z", &rp.accent[2]);   // on-primary-subtle
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

/// Generate Q-var base CSS.
///
/// Q-vars provide theme-style hooks for non-color customization (shadows, borders,
/// transitions, etc.). Components use `var(--Qx)` to adapt to different ThemeStyles.
pub fn generate_q_var_base_css() -> &'static str {
    ":root{\
     --Qd:var(--Z1);--Qgw:none;\
     --Qb:1px;--Qbl:var(--Qb);--Qbt:var(--Qb);--Qbc:var(--h);--Qbs:solid;\
     --Qol:2px;--Qoo:2px;--Qf:var(--K);\
     --Qbf:none;--Qso:1;--Qgr:none;\
     --Qt:150ms;\
     --Qts:none}\n"
}

/// Generate ThemeStyle preset override CSS with resolved colors.
///
/// Returns CSS that remaps semantic variables for a given style preset.
/// Returns `None` if the style is the default.
pub fn generate_resolved_style_css(style: ThemeStyle, rp: &ResolvedPalette) -> Option<String> {
    match style {
        ThemeStyle::Default => None,
        ThemeStyle::Soft => {
            let mut css = String::with_capacity(512);
            // Light: tinted primaries, softer borders, elevated surfaces
            css.push_str("[data-style=\"soft\"]{");
            v(&mut css, "v", &rp.accent[2]);   // primary = accent-3
            v(&mut css, "w", &rp.accent[10]);  // on-primary = accent-11
            v(&mut css, "x", &rp.accent[3]);   // primary-hover = accent-4
            v(&mut css, "F", &rp.red[2]);      // destructive = red-3
            v(&mut css, "G", &rp.red[10]);     // on-destructive = red-11
            v(&mut css, "H", &rp.red[3]);      // destructive-hover = red-4
            v(&mut css, "h", &rp.neutral[3]);  // border-default = neutral-4
            v(&mut css, "g", &rp.neutral[2]);  // border-subtle = neutral-3
            v(&mut css, "t", &rp.neutral[0]);  // surface-raised = neutral-1
            css.push_str("--Qd:none;--Qt:200ms;");
            css.push_str("}\n");
            // Dark: muted primaries
            css.push_str("[data-theme=\"dark\"][data-style=\"soft\"]{");
            v(&mut css, "v", &rp.accent[9]);   // primary = accent-10
            v(&mut css, "w", &rp.accent[2]);   // on-primary = accent-3
            v(&mut css, "x", &rp.accent[8]);   // primary-hover = accent-9
            v(&mut css, "F", &rp.red[9]);      // destructive = red-10
            v(&mut css, "G", &rp.red[2]);      // on-destructive = red-3
            v(&mut css, "H", &rp.red[8]);      // destructive-hover = red-9
            v(&mut css, "h", &rp.neutral[9]);  // border-default = neutral-10
            css.push_str("--g:transparent;");   // border-subtle
            v(&mut css, "t", &rp.neutral[9]);  // surface-raised = neutral-10
            css.push_str("}\n");
            Some(css)
        }
        ThemeStyle::Brutalist => {
            let mut css = String::with_capacity(512);
            // Light: heavy borders, high contrast
            css.push_str("[data-style=\"brutalist\"]{");
            v(&mut css, "h", &rp.neutral[11]); // border-default = neutral-12
            v(&mut css, "g", &rp.neutral[9]);  // border-subtle = neutral-10
            v(&mut css, "i", &rp.neutral[11]); // border-emphasis = neutral-12
            v(&mut css, "t", &rp.neutral[0]);  // surface-raised = neutral-1
            css.push_str("--Qd:none;--Qb:2px;--Qbl:3px;--Qt:0ms;--Qol:3px;--Qoo:0px;");
            css.push_str("}\n");
            // Dark: invert
            css.push_str("[data-theme=\"dark\"][data-style=\"brutalist\"]{");
            v(&mut css, "h", &rp.neutral[0]);  // border-default = neutral-1
            v(&mut css, "g", &rp.neutral[2]);  // border-subtle = neutral-3
            v(&mut css, "i", &rp.neutral[0]);  // border-emphasis = neutral-1
            v(&mut css, "t", &rp.neutral[11]); // surface-raised = neutral-12
            css.push_str("}\n");
            Some(css)
        }
        ThemeStyle::Minimal => {
            let mut css = String::with_capacity(256);
            // Light: no borders
            css.push_str("[data-style=\"minimal\"]{--h:transparent;--g:transparent;");
            v(&mut css, "t", &rp.neutral[0]);  // surface-raised = neutral-1
            css.push_str("--Qd:none;--Qb:0;--Qbc:transparent;--Qbs:none;--Qt:200ms;");
            css.push_str("}\n");
            // Dark: no borders
            css.push_str("[data-theme=\"dark\"][data-style=\"minimal\"]{--h:transparent;--g:transparent;");
            v(&mut css, "t", &rp.neutral[11]); // surface-raised = neutral-12
            css.push_str("}\n");
            Some(css)
        }
        ThemeStyle::Glass => {
            let mut css = String::with_capacity(512);
            css.push_str("[data-style=\"glass\"]{");
            css.push_str("--Qd:none;--Qb:1px;--Qt:200ms;--Qbf:blur(12px);--Qso:0.85;--Qbc:rgba(255,255,255,0.1);--Qgr:none;--Qol:2px;");
            css.push_str("}\n");
            css.push_str("[data-theme=\"dark\"][data-style=\"glass\"]{");
            css.push_str("--Qso:0.75;--Qbc:rgba(255,255,255,0.06);");
            css.push_str("}\n");
            // Fallback for browsers without backdrop-filter
            css.push_str("@supports not (backdrop-filter:blur(0px)){[data-style=\"glass\"]{--Qso:1;--Qbf:none}}\n");
            Some(css)
        }
        ThemeStyle::Neon => {
            let mut css = String::with_capacity(512);
            css.push_str("[data-style=\"neon\"]{");
            css.push_str("--Qd:none;--Qb:2px;--Qt:50ms;--Qgw:0 0 8px var(--n9),0 0 16px var(--n9);--Qts:0 0 8px currentColor;--Qbc:var(--n9);--Qol:2px;--Qf:var(--n9);");
            css.push_str("}\n");
            Some(css)
        }
    }
}

/// Generate palette override CSS scoped to `[data-palette="name"]`.
///
/// Same structure as `generate_resolved_semantic_css` but scoped under
/// an attribute selector so apps can switch palettes at runtime.
pub fn generate_palette_override_css(name: &str, rp: &ResolvedPalette) -> String {
    let mut css = String::with_capacity(2048);

    // Light overrides
    css.push_str(&format!("[data-palette=\"{}\"]{{", name));
    // Backgrounds
    v(&mut css, "a", &rp.neutral[0]);
    v(&mut css, "b", &rp.neutral[1]);
    v(&mut css, "c", &rp.neutral[2]);
    v(&mut css, "d", &rp.neutral[3]);
    v(&mut css, "e", &rp.neutral[4]);
    v(&mut css, "f", &rp.neutral[5]);
    // Borders
    v(&mut css, "g", &rp.neutral[5]);
    v(&mut css, "h", &rp.neutral[6]);
    v(&mut css, "i", &rp.neutral[7]);
    // Text
    v(&mut css, "j", &rp.neutral[8]);
    v(&mut css, "k", &rp.neutral[10]);
    v(&mut css, "l", &rp.neutral[11]);
    v(&mut css, "m", &rp.white);
    // Accent scale
    for i in 0..12 {
        vn(&mut css, "n", i + 1, &rp.accent[i]);
    }
    // Status
    v(&mut css, "o", &rp.green[8]);
    v(&mut css, "p", &rp.amber[8]);
    v(&mut css, "q", &rp.red[8]);
    // Surface pairs
    v(&mut css, "r", &rp.neutral[0]);
    v(&mut css, "s", &rp.neutral[11]);
    v(&mut css, "t", &rp.neutral[1]);
    v(&mut css, "u", &rp.neutral[11]);
    // Primary pairs
    v(&mut css, "v", &rp.accent[8]);
    v(&mut css, "w", &rp.white);
    v(&mut css, "x", &rp.accent[9]);
    v(&mut css, "y", &rp.accent[2]);
    v(&mut css, "z", &rp.accent[10]);
    // Secondary
    v(&mut css, "A", &rp.neutral[3]);
    v(&mut css, "B", &rp.neutral[11]);
    v(&mut css, "C", &rp.neutral[4]);
    // Muted
    v(&mut css, "D", &rp.neutral[2]);
    v(&mut css, "E", &rp.neutral[10]);
    // Destructive
    v(&mut css, "F", &rp.red[8]);
    v(&mut css, "G", &rp.white);
    v(&mut css, "H", &rp.red[9]);
    v(&mut css, "I", &rp.red[2]);
    v(&mut css, "J", &rp.red[10]);
    // Interactive
    v(&mut css, "K", &rp.accent[7]);
    v(&mut css, "L", &rp.accent[6]);
    css.push_str("}\n");

    // Dark overrides
    css.push_str(&format!("[data-theme=\"dark\"][data-palette=\"{}\"]{{", name));
    v(&mut css, "a", &rp.neutral[11]);
    v(&mut css, "b", &rp.neutral[10]);
    v(&mut css, "c", &rp.neutral[9]);
    v(&mut css, "d", &rp.neutral[8]);
    v(&mut css, "e", &rp.neutral[7]);
    v(&mut css, "f", &rp.neutral[6]);
    v(&mut css, "g", &rp.neutral[6]);
    v(&mut css, "h", &rp.neutral[5]);
    v(&mut css, "i", &rp.neutral[4]);
    v(&mut css, "j", &rp.neutral[4]);
    v(&mut css, "k", &rp.neutral[1]);
    v(&mut css, "l", &rp.neutral[0]);
    v(&mut css, "r", &rp.neutral[11]);
    v(&mut css, "s", &rp.neutral[0]);
    v(&mut css, "t", &rp.neutral[10]);
    v(&mut css, "u", &rp.neutral[0]);
    v(&mut css, "A", &rp.neutral[8]);
    v(&mut css, "B", &rp.neutral[0]);
    v(&mut css, "C", &rp.neutral[7]);
    v(&mut css, "D", &rp.neutral[9]);
    v(&mut css, "E", &rp.neutral[3]);
    v(&mut css, "I", &rp.red[9]);
    v(&mut css, "J", &rp.red[2]);
    v(&mut css, "y", &rp.accent[9]);
    v(&mut css, "z", &rp.accent[2]);
    css.push_str("}\n");

    css
}

/// Generate radius scale override CSS.
///
/// Returns CSS to override the component radius.
/// Returns `None` if the radius is the default (medium).
pub fn generate_radius_css(radius: RadiusScale) -> Option<&'static str> {
    match radius {
        RadiusScale::Medium => None,
        RadiusScale::None => Some("[data-radius=\"none\"]{--R2:0;--R3:0;--R4:0}\n"),
        RadiusScale::Small => {
            Some("[data-radius=\"small\"]{--R2:var(--R1);--R3:var(--R1);--R4:var(--R2)}\n")
        }
        RadiusScale::Large => {
            Some("[data-radius=\"large\"]{--R2:var(--R3);--R3:var(--R4);--R4:var(--R5)}\n")
        }
        RadiusScale::Full => {
            Some("[data-radius=\"full\"]{--R2:9999px;--R3:9999px;--R4:9999px}\n")
        }
    }
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
    fn test_data_attrs_minimal() {
        let theme = Theme::default();
        assert_eq!(theme.data_attrs(), "data-theme=\"light\"");
    }

    #[test]
    fn test_data_attrs_full() {
        let theme = Theme::dark().radius(RadiusScale::Full);
        let attrs = theme.data_attrs();
        assert!(attrs.contains("data-theme=\"dark\""));
        assert!(attrs.contains("data-radius=\"full\""));
    }

    #[test]
    fn test_base_css_size() {
        let css = generate_base_css();
        assert!(css.len() < 400, "Base CSS too large: {} bytes", css.len());
    }

    #[test]
    fn test_resolved_semantic_css_structure() {
        let theme = Theme::default();
        let rp = ResolvedPalette::new(&theme);
        let css = generate_resolved_semantic_css(&rp);

        // Should have both light and dark themes
        assert!(css.contains(":root"), "Missing :root");
        assert!(css.contains("[data-theme=\"light\"]"), "Missing light theme");
        assert!(css.contains("[data-theme=\"dark\"]"), "Missing dark theme");

        // Should use short variable names with resolved values
        assert!(css.contains("--a:"), "Missing --a (bg-app)");
        assert!(css.contains("--k:"), "Missing --k (text-default)");
        assert!(css.contains("--v:"), "Missing --v (primary)");

        // Should contain resolved oklch values, NOT var() references
        assert!(css.contains("oklch("), "Should contain resolved oklch values");
        assert!(!css.contains("var(--rw-neutral"), "Should not contain var(--rw-neutral-*) indirection");
        assert!(!css.contains("var(--rw-accent"), "Should not contain var(--rw-accent-*) indirection");
    }

    #[test]
    fn test_resolved_semantic_css_size() {
        let theme = Theme::default();
        let rp = ResolvedPalette::new(&theme);
        let css = generate_resolved_semantic_css(&rp);
        // Resolved CSS should be much smaller than old 3KB
        assert!(css.len() < 2048, "Resolved semantic CSS too large: {} bytes", css.len());
    }

    #[test]
    fn test_radius_css() {
        assert!(generate_radius_css(RadiusScale::Medium).is_none());

        let css = generate_radius_css(RadiusScale::Full).unwrap();
        assert!(css.contains("[data-radius=\"full\"]"));
    }

    #[test]
    fn test_full_theme_css_size() {
        let theme = Theme::dark().radius(RadiusScale::Large);
        let rp = ResolvedPalette::new(&theme);
        let css = generate_resolved_semantic_css(&rp);
        assert!(css.len() < 2048, "Resolved CSS too large: {} bytes", css.len());
    }

    #[test]
    fn test_style_css_default_returns_none() {
        let rp = ResolvedPalette::new(&Theme::default());
        assert!(generate_resolved_style_css(ThemeStyle::Default, &rp).is_none());
    }

    #[test]
    fn test_style_css_soft() {
        let rp = ResolvedPalette::new(&Theme::default());
        let css = generate_resolved_style_css(ThemeStyle::Soft, &rp).unwrap();
        assert!(css.contains("[data-style=\"soft\"]"));
        assert!(css.contains("--v:"), "Should contain --v (primary)");
    }

    #[test]
    fn test_style_css_brutalist() {
        let rp = ResolvedPalette::new(&Theme::default());
        let css = generate_resolved_style_css(ThemeStyle::Brutalist, &rp).unwrap();
        assert!(css.contains("[data-style=\"brutalist\"]"));
        assert!(css.contains("--h:"), "Should contain --h (border-default)");
    }

    #[test]
    fn test_style_css_minimal() {
        let rp = ResolvedPalette::new(&Theme::default());
        let css = generate_resolved_style_css(ThemeStyle::Minimal, &rp).unwrap();
        assert!(css.contains("[data-style=\"minimal\"]"));
        assert!(css.contains("--h:transparent"));
    }

    #[test]
    fn test_data_attrs_with_style() {
        let theme = Theme::light().style(ThemeStyle::Soft);
        let attrs = theme.data_attrs();
        assert!(attrs.contains("data-style=\"soft\""));
    }

    #[test]
    fn test_data_attrs_default_style_omitted() {
        let theme = Theme::default();
        assert!(!theme.data_attrs().contains("data-style"));
    }

    #[test]
    fn test_resolved_palette_accent_from_seed() {
        // Custom accent should use the seed color's generated scale
        let theme = Theme::default().accent("#00FF00");
        let rp = ResolvedPalette::new(&theme);
        let css = generate_resolved_semantic_css(&rp);
        // The accent should contain oklch values (from generated scale)
        assert!(css.contains("oklch("), "Accent should have oklch values");
    }

    #[test]
    fn test_no_primitive_color_vars() {
        let theme = Theme::default();
        let rp = ResolvedPalette::new(&theme);
        let css = generate_resolved_semantic_css(&rp);

        // Should NOT contain any --rw-neutral-*, --rw-blue-*, etc. color primitives
        assert!(!css.contains("--rw-neutral-"), "Should not emit primitive color vars");
        assert!(!css.contains("--rw-blue-"), "Should not emit primitive color vars");
        assert!(!css.contains("--rw-red-"), "Should not emit primitive color vars");
    }
}
