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
/// `ThemeStyle` is a struct holding CSS generation callbacks, making it
/// extensible without `dyn` or `Box`. The struct is `Copy + Clone + Send + Sync`.
///
/// Use [`ThemeStyle::soft()`] for the built-in default, or create custom styles
/// via [`ThemeStyle::new()`] with your own CSS generation functions.
///
/// # Example
///
/// ```ignore
/// use rwire::{ThemeStyle, ResolvedPalette, css_var};
///
/// let custom = ThemeStyle::new(
///     "my-style", "My Style",
///     |css, rp, is_dark| { css_var(css, "v", &rp.accent[8]); },
///     |css, _is_dark| { css.push_str("--Qd:none;"); },
/// );
/// ```
#[derive(Clone, Copy)]
pub struct ThemeStyle {
    id: &'static str,
    label: &'static str,
    color_css: fn(&mut String, &ResolvedPalette, bool),
    q_css: fn(&mut String, bool),
}

impl Default for ThemeStyle {
    fn default() -> Self {
        Self::soft()
    }
}

impl ThemeStyle {
    /// Create a custom style preset.
    pub const fn new(
        id: &'static str,
        label: &'static str,
        color_css: fn(&mut String, &ResolvedPalette, bool),
        q_css: fn(&mut String, bool),
    ) -> Self {
        Self { id, label, color_css, q_css }
    }

    /// Built-in Soft style (the framework default).
    ///
    /// Subtle tinted backgrounds, large radius, no shadows.
    pub fn soft() -> Self {
        Self::new("soft", "Soft", soft_color_css, soft_q_css)
    }

    /// Get the style identifier string.
    pub fn id(&self) -> &'static str {
        self.id
    }

    /// Get the human-readable label.
    pub fn label(&self) -> &'static str {
        self.label
    }
}

impl PartialEq for ThemeStyle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ThemeStyle {}

impl std::fmt::Debug for ThemeStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThemeStyle").field("id", &self.id).finish()
    }
}

// ============================================================================
// Extensibility Traits
// ============================================================================

/// Convert a custom style enum/value into a [`ThemeStyle`].
///
/// Implement this trait on your own enum to use it with [`Theme::style()`].
///
/// # Example
///
/// ```ignore
/// use rwire::{ThemeStyle, IntoStyle, ResolvedPalette, css_var};
///
/// enum MyStyle { Fancy, Retro }
///
/// impl IntoStyle for MyStyle {
///     fn into_style(self) -> ThemeStyle {
///         match self {
///             MyStyle::Fancy => ThemeStyle::new("fancy", "Fancy", fancy_colors, fancy_q),
///             MyStyle::Retro => ThemeStyle::new("retro", "Retro", retro_colors, retro_q),
///         }
///     }
/// }
/// ```
pub trait IntoStyle {
    fn into_style(self) -> ThemeStyle;
}

impl IntoStyle for ThemeStyle {
    fn into_style(self) -> ThemeStyle {
        self
    }
}

/// Convert a custom palette enum/value into a [`ColorPalette`].
///
/// Implement this trait on your own enum to use it with [`Theme::palette()`].
pub trait IntoPalette {
    fn into_palette(self) -> ColorPalette;
}

impl IntoPalette for ColorPalette {
    fn into_palette(self) -> ColorPalette {
        self
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
/// // Full control with custom style
/// let theme = Theme::dark().style(ThemeStyle::soft());
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
    /// Create a new theme with default settings (light mode, Soft style).
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

    /// Set a full color palette.
    ///
    /// Accepts any type implementing [`IntoPalette`], including [`ColorPalette`]
    /// directly or custom palette enums.
    pub fn palette(mut self, palette: impl IntoPalette) -> Self {
        self.palette = Some(palette.into_palette());
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

    /// Set the visual style preset (builder).
    ///
    /// Accepts any type implementing [`IntoStyle`], including [`ThemeStyle`]
    /// directly or custom style enums.
    pub fn style(mut self, style: impl IntoStyle) -> Self {
        self.style = style.into_style();
        self
    }

    /// Set the visual style on a mutable theme reference.
    pub fn set_style(&mut self, style: impl IntoStyle) {
        self.style = style.into_style();
    }

    /// Set the palette on a mutable theme reference.
    ///
    /// Accepts any type implementing [`IntoPalette`].
    pub fn set_palette(&mut self, palette: impl IntoPalette) {
        self.palette = Some(palette.into_palette());
    }

    /// Clear the palette back to None (framework default colors).
    pub fn clear_palette(&mut self) {
        self.palette = None;
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
///
/// External style implementations receive this in their `color_css` callback
/// to access palette colors.
pub struct ResolvedPalette {
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
     img{max-width:100%;height:auto}\
     *{scrollbar-width:thin;scrollbar-color:var(--c) transparent}\
     ::-webkit-scrollbar{width:6px;height:6px}\
     ::-webkit-scrollbar-track{background:transparent}\
     ::-webkit-scrollbar-thumb{background:var(--c);border-radius:3px}\
     ::-webkit-scrollbar-thumb:hover{background:var(--e)}\n"
}

/// Generate complete theme CSS from a Theme value.
///
/// Produces a single `:root{...}` block with all resolved CSS variables.
/// Mode (light/dark) is resolved server-side: picks the right scale indices.
/// Style preset callbacks generate color overrides and Q-vars inline.
/// No `data-theme`, `data-style`, or `data-radius` attribute selectors.
pub fn generate_theme_css(theme: &Theme) -> String {
    let rp = ResolvedPalette::new(theme);
    let is_dark = theme.mode == ThemeMode::Dark;
    let mut css = String::with_capacity(2048);

    css.push_str(":root{");

    // --- Semantic color variables (mode-aware) ---
    if is_dark {
        // Dark mode: invert scales
        css_var(&mut css, "a", &rp.neutral[11]); // bg-app
        css_var(&mut css, "b", &rp.neutral[10]); // bg-subtle
        css_var(&mut css, "c", &rp.neutral[9]);  // bg-muted
        css_var(&mut css, "d", &rp.neutral[8]);  // bg-emphasis
        css_var(&mut css, "e", &rp.neutral[7]);  // bg-hover
        css_var(&mut css, "f", &rp.neutral[6]);  // bg-active
        css_var(&mut css, "g", &rp.neutral[6]);  // border-subtle
        css_var(&mut css, "h", &rp.neutral[5]);  // border-default
        css_var(&mut css, "i", &rp.neutral[4]);  // border-emphasis
        css_var(&mut css, "j", &rp.neutral[4]);  // text-muted
        css_var(&mut css, "k", &rp.neutral[1]);  // text-default
        css_var(&mut css, "l", &rp.neutral[0]);  // text-high
        css_var(&mut css, "m", &rp.white);       // text-on-accent
        css_var(&mut css, "r", &rp.neutral[11]); // surface
        css_var(&mut css, "s", &rp.neutral[0]);  // on-surface
        css_var(&mut css, "t", &rp.neutral[10]); // surface-raised
        css_var(&mut css, "u", &rp.neutral[0]);  // on-surface-raised
        css_var(&mut css, "A", &rp.neutral[8]);  // secondary
        css_var(&mut css, "B", &rp.neutral[0]);  // on-secondary
        css_var(&mut css, "C", &rp.neutral[7]);  // secondary-hover
        css_var(&mut css, "D", &rp.neutral[9]);  // muted
        css_var(&mut css, "E", &rp.neutral[3]);  // on-muted
        css_var(&mut css, "I", &rp.red[9]);      // destructive-subtle
        css_var(&mut css, "J", &rp.red[2]);      // on-destructive-subtle
        css_var(&mut css, "y", &rp.accent[9]);   // primary-subtle
        css_var(&mut css, "z", &rp.accent[2]);   // on-primary-subtle
    } else {
        // Light mode
        css_var(&mut css, "a", &rp.neutral[0]);  // bg-app
        css_var(&mut css, "b", &rp.neutral[1]);  // bg-subtle
        css_var(&mut css, "c", &rp.neutral[2]);  // bg-muted
        css_var(&mut css, "d", &rp.neutral[3]);  // bg-emphasis
        css_var(&mut css, "e", &rp.neutral[4]);  // bg-hover
        css_var(&mut css, "f", &rp.neutral[5]);  // bg-active
        css_var(&mut css, "g", &rp.neutral[5]);  // border-subtle
        css_var(&mut css, "h", &rp.neutral[6]);  // border-default
        css_var(&mut css, "i", &rp.neutral[7]);  // border-emphasis
        css_var(&mut css, "j", &rp.neutral[8]);  // text-muted
        css_var(&mut css, "k", &rp.neutral[10]); // text-default
        css_var(&mut css, "l", &rp.neutral[11]); // text-high
        css_var(&mut css, "m", &rp.white);       // text-on-accent
        css_var(&mut css, "r", &rp.neutral[0]);  // surface
        css_var(&mut css, "s", &rp.neutral[11]); // on-surface
        css_var(&mut css, "t", &rp.neutral[1]);  // surface-raised
        css_var(&mut css, "u", &rp.neutral[11]); // on-surface-raised
        css_var(&mut css, "A", &rp.neutral[3]);  // secondary
        css_var(&mut css, "B", &rp.neutral[11]); // on-secondary
        css_var(&mut css, "C", &rp.neutral[4]);  // secondary-hover
        css_var(&mut css, "D", &rp.neutral[2]);  // muted
        css_var(&mut css, "E", &rp.neutral[10]); // on-muted
        css_var(&mut css, "I", &rp.red[2]);      // destructive-subtle
        css_var(&mut css, "J", &rp.red[10]);     // on-destructive-subtle
        css_var(&mut css, "y", &rp.accent[2]);   // primary-subtle
        css_var(&mut css, "z", &rp.accent[10]);  // on-primary-subtle
    }

    // Mode-independent vars
    for i in 0..12 {
        vn(&mut css, "n", i + 1, &rp.accent[i]);
    }
    css_var(&mut css, "o", &rp.green[8]);    // success
    css_var(&mut css, "p", &rp.amber[8]);    // warning
    css_var(&mut css, "q", &rp.red[8]);      // error
    css_var(&mut css, "F", &rp.red[8]);      // destructive
    css_var(&mut css, "G", &rp.white);       // on-destructive
    css_var(&mut css, "H", &rp.red[9]);      // destructive-hover
    // Semantic subtle pairs for status colors (mode-aware)
    if is_dark {
        css_var(&mut css, "M", &rp.green[9]);  // success-subtle bg
        css_var(&mut css, "M1", &rp.green[2]); // on-success-subtle text
        css_var(&mut css, "N", &rp.amber[9]);  // warning-subtle bg
        css_var(&mut css, "N1", &rp.amber[2]); // on-warning-subtle text
        css_var(&mut css, "O", &rp.accent[9]); // info-subtle bg
        css_var(&mut css, "O1", &rp.accent[2]);// on-info-subtle text
        css_var(&mut css, "P", &rp.red[9]);    // error-subtle bg (alias for --I)
        css_var(&mut css, "P1", &rp.red[2]);   // on-error-subtle text
    } else {
        css_var(&mut css, "M", &rp.green[1]);
        css_var(&mut css, "M1", &rp.green[10]);
        css_var(&mut css, "N", &rp.amber[1]);
        css_var(&mut css, "N1", &rp.amber[10]);
        css_var(&mut css, "O", &rp.accent[1]);
        css_var(&mut css, "O1", &rp.accent[10]);
        css_var(&mut css, "P", &rp.red[1]);
        css_var(&mut css, "P1", &rp.red[10]);
    }
    if is_dark {
        css_var(&mut css, "K", &rp.accent[5]);   // focus-ring (brighter in dark)
        css_var(&mut css, "L", &rp.accent[4]);   // border-primary (brighter in dark)
    } else {
        css_var(&mut css, "K", &rp.accent[7]);   // focus-ring
        css_var(&mut css, "L", &rp.accent[6]);   // border-primary
    }

    // --- Style preset: resolve primary/surface/border overrides inline ---
    (theme.style.color_css)(&mut css, &rp, is_dark);

    // --- Q-vars (theme-style hooks for non-color customization) ---
    (theme.style.q_css)(&mut css, is_dark);

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

/// Write `--{name}:{value};` to a CSS string.
///
/// Helper for building CSS variable declarations in style callbacks.
pub fn css_var(css: &mut String, name: &str, value: &str) {
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

// ============================================================================
// Built-in Soft Style Implementation
// ============================================================================

fn soft_color_css(css: &mut String, rp: &ResolvedPalette, is_dark: bool) {
    if is_dark {
        css_var(css, "v", &rp.accent[7]);  // primary — brighter for dark bg
        css_var(css, "w", &rp.accent[0]);  // on-primary
        css_var(css, "x", &rp.accent[6]);  // primary-hover
        css_var(css, "F", &rp.red[7]);     // destructive
        css_var(css, "G", &rp.red[0]);     // on-destructive
        css_var(css, "H", &rp.red[6]);     // destructive-hover
        css_var(css, "h", &rp.neutral[8]); // border-default
        css.push_str("--g:transparent;");
        css_var(css, "t", &rp.neutral[9]); // surface-raised
    } else {
        css_var(css, "v", &rp.accent[2]);
        css_var(css, "w", &rp.accent[10]);
        css_var(css, "x", &rp.accent[3]);
        css_var(css, "F", &rp.red[2]);
        css_var(css, "G", &rp.red[10]);
        css_var(css, "H", &rp.red[3]);
        css_var(css, "h", &rp.neutral[3]);
        css_var(css, "g", &rp.neutral[2]);
        css_var(css, "t", &rp.neutral[0]);
    }
}

fn soft_q_css(css: &mut String, _is_dark: bool) {
    css.push_str("--Qd:none;--Qgw:none;");
    css.push_str("--Qb:1px;--Qbl:var(--Qb);--Qbt:var(--Qb);--Qbc:var(--h);--Qbs:solid;");
    css.push_str("--Qol:2px;--Qoo:2px;--Qf:var(--K);");
    css.push_str("--Qbf:none;--Qso:1;--Qgr:none;");
    css.push_str("--Qt:200ms;--Qts:none;");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_defaults() {
        let theme = Theme::default();
        assert_eq!(theme.mode, ThemeMode::Light);
        assert_eq!(theme.radius, RadiusScale::Medium);
        assert_eq!(theme.style, ThemeStyle::soft());
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
        assert!(css.len() < 600, "Base CSS too large: {} bytes", css.len());
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
    fn test_generate_theme_css_with_style() {
        let theme = Theme::dark().style(ThemeStyle::soft());
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
        let theme = Theme::dark().style(ThemeStyle::soft()).radius(RadiusScale::Large);
        let css = generate_theme_css(&theme);
        // Single block should be compact
        assert!(css.len() < 2048, "Theme CSS too large: {} bytes", css.len());
    }

    #[test]
    fn test_theme_implements_state() {
        assert_eq!(Theme::STORAGE_TYPE, StorageType::Memory);
    }

    #[test]
    fn test_theme_style_equality() {
        assert_eq!(ThemeStyle::soft(), ThemeStyle::soft());
        let custom = ThemeStyle::new("soft", "Custom Soft", soft_color_css, soft_q_css);
        assert_eq!(ThemeStyle::soft(), custom); // same id
        let other = ThemeStyle::new("other", "Other", soft_color_css, soft_q_css);
        assert_ne!(ThemeStyle::soft(), other); // different id
    }

    #[test]
    fn test_custom_style() {
        fn my_color(css: &mut String, rp: &ResolvedPalette, _is_dark: bool) {
            css_var(css, "v", &rp.accent[8]);
            css_var(css, "w", &rp.white);
            css_var(css, "x", &rp.accent[9]);
        }
        fn my_q(css: &mut String, _is_dark: bool) {
            css.push_str("--Qd:none;--Qgw:none;");
            css.push_str("--Qb:1px;--Qbl:var(--Qb);--Qbt:var(--Qb);--Qbc:var(--h);--Qbs:solid;");
            css.push_str("--Qol:2px;--Qoo:2px;--Qf:var(--K);");
            css.push_str("--Qbf:none;--Qso:1;--Qgr:none;");
            css.push_str("--Qt:100ms;--Qts:none;");
        }
        let custom = ThemeStyle::new("custom", "Custom", my_color, my_q);
        let theme = Theme::dark().style(custom);
        let css = generate_theme_css(&theme);
        assert!(css.contains("--Qt:100ms"), "Custom Q-var should be present");
    }

    #[test]
    fn test_into_style_trait() {
        struct MyStyle;
        impl IntoStyle for MyStyle {
            fn into_style(self) -> ThemeStyle {
                ThemeStyle::soft()
            }
        }
        let theme = Theme::dark().style(MyStyle);
        assert_eq!(theme.style, ThemeStyle::soft());
    }

    #[test]
    fn test_into_palette_trait() {
        let theme = Theme::dark().palette(ColorPalette::default());
        assert!(theme.palette_ref().is_some());
    }

    #[test]
    fn test_set_style() {
        let mut theme = Theme::dark();
        let custom = ThemeStyle::new("custom", "Custom", soft_color_css, soft_q_css);
        theme.set_style(custom);
        assert_eq!(theme.style.id(), "custom");
    }
}
