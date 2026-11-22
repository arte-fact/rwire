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

/// Theme mode: light, dark, or following the operating system.
///
/// `Light` and `Dark` force a mode. `System` emits both: the light variables on
/// `:root` and the dark ones under `@media (prefers-color-scheme: dark)`, so the
/// page follows the OS setting live, without a round trip.
///
/// A mode the user picks (a handler setting `theme.mode`) survives a reload:
/// the runtime keeps it in the `rw_mode` cookie, and the server starts the
/// next page and connection in that mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
    /// Follow `prefers-color-scheme`.
    System,
}

impl ThemeMode {
    /// Get the string representation (`light`, `dark`, `system`).
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
            ThemeMode::System => "system",
        }
    }

    /// Parse [`ThemeMode::as_str`]'s output.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "light" => Some(ThemeMode::Light),
            "dark" => Some(ThemeMode::Dark),
            "system" => Some(ThemeMode::System),
            _ => None,
        }
    }

    /// Toggle between light and dark mode (two-state toggles). `System`
    /// toggles to `Light`.
    pub fn toggle(&self) -> Self {
        match self {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark | ThemeMode::System => ThemeMode::Light,
        }
    }

    /// The next mode of a three-state toggle: System → Light → Dark → System.
    pub fn cycle(&self) -> Self {
        match self {
            ThemeMode::System => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::System,
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
        Self {
            id,
            label,
            color_css,
            q_css,
        }
    }

    /// Built-in Soft style (the framework default).
    ///
    /// Subtle tinted backgrounds, large radius, no shadows.
    pub fn soft() -> Self {
        Self::new("soft", "Soft", soft_color_css, soft_q_css)
    }

    /// Built-in Flat style — the terminal aesthetic.
    ///
    /// Soft's hairline borders and shadow-free surfaces, plus a **monospace UI font** for the
    /// whole app (`--Qft`). Pair with [`RadiusScale::None`] for sharp corners:
    ///
    /// ```ignore
    /// Theme::dark().style(ThemeStyle::flat()).radius(RadiusScale::None)
    /// ```
    pub fn flat() -> Self {
        Self::new("flat", "Flat", soft_color_css, flat_q_css)
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
    /// Root font size in px below / from the `sm` breakpoint (see [`Theme::base_font_size`]).
    base_font: Option<(u8, u8)>,
    /// Font stacks (see [`Theme::fonts`]).
    fonts: Fonts,
    /// Control and row dimensions (see [`Theme::density`]).
    density: Density,
    /// The viewport width below which controls get 44 px touch targets
    /// (see [`Theme::touch_below`]); `None`: the `sm` breakpoint (640).
    touch_below: Option<u16>,
}

/// The responsive rules of a three-column `AppShell` under `scope` (a
/// selector for its root): below `drawer` px the panel becomes a drawer over
/// the main column; below `one_column` px a single column shows, the sidebar
/// or the main one as the root's `data-mobile` says (`list` / `main`), and
/// the panel covers the screen. The theme emits them for every shell at its
/// defaults (one column below [`Theme::touch_below`], else 760; drawer below
/// 1100); a shell with its own thresholds emits its own.
#[doc(hidden)]
pub fn shell_layout_css(scope: &str, one_column: u16, drawer: u16) -> String {
    let (d, o) = (drawer.saturating_sub(1), one_column.saturating_sub(1));
    format!(
        "@media (max-width:{d}.98px){{\
         {scope} [data-shell=panel]{{position:fixed;top:0;right:0;bottom:0;z-index:40;max-width:100vw;box-shadow:var(--Qd,0 8px 24px rgba(0,0,0,.25))}}\
         {scope} [data-pane=panel]{{display:none}}}}\
         @media (max-width:{o}.98px){{\
         {scope} [data-shell=handle]{{display:none}}\
         {scope} [data-shell=sidebar]{{width:100%!important;border-right:0}}\
         {scope} [data-shell=panel]{{left:0;width:100%!important}}\
         {scope}[data-mobile=list] [data-shell=main],{scope}[data-mobile=main] [data-shell=sidebar]{{display:none}}}}\n"
    )
}

/// How dense the controls and lists are, for a site or for a tool.
///
/// It sets the `--Qz*` hooks: control height (`--Qzh`, `St::HControl`), control
/// text (`--Qzt`, `St::TextControl`), tab padding (`--Qzy`, `St::PyTab`), chip
/// padding (`--Qzcx` / `--Qzcy`, `St::PxChip` / `St::PyChip`) and list-row padding (`--Qzp`,
/// `St::PadRow`), icon buttons' square and glyph (`--Qzi`, `--Qzg`, `--Qzq`:
/// `Button::icon_only` and `ThemeToggle`, 32 px with an 18 px glyph in
/// `Compact`). `Button`, `Input`, `Select` (default size), `Chip`, `Tabs` and
/// `ThemeToggle` read them. Whatever the density, interactive controls are at least
/// 44 px under a coarse pointer or below the `sm` breakpoint (base CSS).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Density {
    /// Sized for a site (the default).
    #[default]
    Comfortable,
    /// Sized for a tool: 32 px controls, tighter tabs, chips and rows.
    Compact,
}

impl Density {
    /// The `--Qz*` declarations.
    fn css(self) -> &'static str {
        match self {
            // The tokens' fallbacks are the comfortable sizes.
            Density::Comfortable => "",
            Density::Compact => {
                "--Qzh:2rem;--Qzt:var(--T2);--Qzy:var(--S2);--Qzcx:var(--S2);--Qzcy:.125rem;--Qzp:.38rem .55rem;\
                 --Qzi:32px;--Qzg:18px;--Qzq:0;"
            }
        }
    }
}

/// The theme's three font stacks, each a CSS `font-family` list. A stack left
/// `None` keeps the style's (or the browser's) default.
///
/// | Stack | Variable | Read by |
/// |---|---|---|
/// | `ui` | `--Qft` | `body`: the whole interface |
/// | `read` | `--Qfr` | `St::FontRead`: long-form reading (a reply, an article) |
/// | `mono` | `--Qm` | `St::FontMono`, code |
///
/// ```ignore
/// Theme::new().fonts(
///     Fonts::new()
///         .ui("Inter, system-ui, sans-serif")
///         .read("'Source Serif 4', Georgia, serif")
///         .mono("'JetBrains Mono', ui-monospace, monospace"),
/// )
/// ```
///
/// Web fonts are loaded with [`CapsuleConfig::font_face`](crate::CapsuleConfig::font_face),
/// from files the app serves itself.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fonts {
    /// The interface face (`--Qft`).
    pub ui: Option<String>,
    /// The reading face (`--Qfr`, [`St::FontRead`](crate::St::FontRead)).
    pub read: Option<String>,
    /// The monospace face (`--Qm`, [`St::FontMono`](crate::St::FontMono)).
    pub mono: Option<String>,
}

impl Fonts {
    /// No stack set.
    pub fn new() -> Self {
        Self::default()
    }

    /// The interface stack (`--Qft`).
    pub fn ui(mut self, stack: impl Into<String>) -> Self {
        self.ui = Some(stack.into());
        self
    }

    /// The reading stack (`--Qfr`).
    pub fn read(mut self, stack: impl Into<String>) -> Self {
        self.read = Some(stack.into());
        self
    }

    /// The monospace stack (`--Qm`).
    pub fn mono(mut self, stack: impl Into<String>) -> Self {
        self.mono = Some(stack.into());
        self
    }

    /// `--Qft` / `--Qfr` / `--Qm` for the stacks set. A stack's own `;`, `{`
    /// or `}` would end the declaration: they are dropped.
    fn css(&self, css: &mut String) {
        for (var, stack) in [("Qft", &self.ui), ("Qfr", &self.read), ("Qm", &self.mono)] {
            if let Some(stack) = stack {
                let clean: String = stack
                    .chars()
                    .filter(|c| !matches!(c, ';' | '{' | '}'))
                    .collect();
                css_var(css, var, clean.trim());
            }
        }
    }
}

// Theme implements State manually (can't use derive macro from within rwire crate).
impl State for Theme {
    const STORAGE_TYPE: StorageType = StorageType::Memory;
}

impl Theme {
    // Field-index constants for fine-grained reactivity, matching the
    // `#[derive(State)]` convention (positional index, uppercased field name).
    // Required because Theme's `State` impl is hand-written, not derived.
    /// Field index of `mode`.
    pub const FIELD_MODE: u8 = 0;
    /// Field index of `radius`.
    pub const FIELD_RADIUS: u8 = 1;
    /// Field index of `style`.
    pub const FIELD_STYLE: u8 = 2;
    /// Field index of `palette`.
    pub const FIELD_PALETTE: u8 = 3;
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

    /// Set the interface, reading and monospace font stacks (see [`Fonts`]).
    /// They win over a style's own (`flat`'s monospace UI, for one).
    pub fn fonts(mut self, fonts: Fonts) -> Self {
        self.fonts = fonts;
        self
    }

    /// The font stacks set with [`Theme::fonts`].
    pub fn fonts_ref(&self) -> &Fonts {
        &self.fonts
    }

    /// The viewport width (px) below which interactive controls get 44 px touch
    /// targets, even under a fine pointer (a coarse pointer gets them at any
    /// width). The default is the `sm` breakpoint, 640; an app whose "phone"
    /// layout starts wider sets its own (mcp-agents: 760). The `sm`
    /// breakpoint itself does not move.
    pub fn touch_below(mut self, px: u16) -> Self {
        self.touch_below = Some(px);
        self
    }

    /// The density set with [`Theme::density`].
    pub fn current_density(&self) -> Density {
        self.density
    }

    /// Set the density of controls and lists (see [`Density`]).
    pub fn density(mut self, density: Density) -> Self {
        self.density = density;
        self
    }

    /// Set the mode: `Light` or `Dark` force it, `System` follows the OS.
    pub fn mode(mut self, mode: ThemeMode) -> Self {
        self.mode = mode;
        self
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

    /// Root font size: `mobile_px` on narrow screens, `desktop_px` from the `sm`
    /// breakpoint (640px). Every `rem`-based token scales with it, so a dense
    /// mobile app can run at 14px without touching a single component.
    pub fn base_font_size(mut self, mobile_px: u8, desktop_px: u8) -> Self {
        self.base_font = Some((mobile_px, desktop_px));
        self
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
    // `data-rw-mode`: the runtime keeps a mode changed during the visit in the
    // `rw_mode` cookie, so it survives a reload (see [`ThemeMode`]).
    el(El::Style)
        .attr("data-rw-mode", theme.mode.as_str())
        .text(&css)
}

/// Create an `ElementBuilder` representing the theme synced region.
///
/// This returns a synced element that, when included in the element tree,
/// renders to a `<style>` element with CSS variable declarations.
/// On theme state changes, the synced element system patches the `<style>`
/// content, causing an instant browser restyle.
pub(crate) fn theme_synced_builder() -> ElementBuilder {
    ElementBuilder::synced::<Theme, _>(theme_renderer)
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
    /// The categorical colors (`--Y1…--Y6`): the palette's own, or six
    /// derived from its scales when it names none.
    pub categorical: Vec<String>,
    /// The palette's preferred categorical inks, `(light, dark)`.
    pub categorical_inks: Vec<(String, String)>,
    /// The palette's preferred marks on the categorical solid fills.
    pub categorical_on_solids: Vec<String>,
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
            categorical: if p.categorical.is_empty() {
                // Six hues the palette already has, far enough apart to tell.
                vec![
                    p.accent.steps()[8].clone(),
                    p.green.steps()[8].clone(),
                    p.amber.steps()[8].clone(),
                    p.red.steps()[8].clone(),
                    p.accent.steps()[5].clone(),
                    p.neutral.steps()[7].clone(),
                ]
            } else {
                p.categorical.clone()
            },
            categorical_inks: p.categorical_inks.clone(),
            categorical_on_solids: p.categorical_on_solids.clone(),
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
// Status:            --o (success), --p (warning), --q (error): fills and borders;
//                    --o3, --p3, --q3: their text, readable on the app and subtle grounds
// Subtle pairs:      --M/--M1 (success), --N/--N1 (warning), --O/--O1 (info),
//                    --P/--P1 (error), --n0 (accent-subtle background)
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
/// Uses short CSS variable names. On touch screens, no tap highlight (the
/// translucent flash mobile browsers paint over anything clickable — a
/// whole tap-to-continue screen would flash) and no double-tap zoom, so a
/// double tap is an app gesture; pinch zoom stays.
pub fn generate_base_css() -> &'static str {
    "html{scroll-behavior:smooth;-webkit-tap-highlight-color:transparent;touch-action:manipulation}\
     *,*::before,*::after{box-sizing:border-box}\
     body{margin:0;font-family:var(--Qft,system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",Roboto,sans-serif);\
     line-height:var(--X3);color:var(--k);background:var(--a)}\
     button,input,select,textarea{font:inherit}\
     table{border-collapse:collapse}\
     img{max-width:100%;height:auto}\
     *{scrollbar-width:thin;scrollbar-color:var(--c) transparent}\
     ::-webkit-scrollbar{width:6px;height:6px}\
     ::-webkit-scrollbar-track{background:transparent}\
     ::-webkit-scrollbar-thumb{background:var(--c);border-radius:3px}\
     ::-webkit-scrollbar-thumb:hover{background:var(--e)}\
     [data-busy]{cursor:progress;opacity:.6;transition:opacity .25s .2s}\
     [id^=__synced_]:not([class]),[data-rwr]:not([class]){display:contents}\
\n"
}

/// Generate complete theme CSS from a Theme value.
///
/// Produces a `:root{...}` block with all resolved CSS variables for a forced
/// mode (`Light`, `Dark`); for `System`, the light block followed by the dark
/// one under `@media (prefers-color-scheme:dark)`.
/// Mode is resolved server-side: picks the right scale indices.
/// Style preset callbacks generate color overrides and Q-vars inline.
/// No `data-theme`, `data-style`, or `data-radius` attribute selectors.
pub fn generate_theme_css(theme: &Theme) -> String {
    let mut css = match theme.mode {
        ThemeMode::Light => root_block(theme, false),
        ThemeMode::Dark => root_block(theme, true),
        // Light on `:root`, dark when the OS asks for it: the browser switches
        // live when the OS setting changes.
        ThemeMode::System => {
            let mut css = root_block(theme, false);
            css.push_str("@media (prefers-color-scheme:dark){");
            css.push_str(root_block(theme, true).trim_end());
            css.push_str("}\n");
            css
        }
    };
    if let Some((mobile, desktop)) = theme.base_font {
        use std::fmt::Write;
        let _ = writeln!(
            css,
            "html{{font-size:{mobile}px}}@media(min-width:640px){{html{{font-size:{desktop}px}}}}"
        );
    }
    // Three-column shells at the theme's thresholds (a shell with its own emits
    // its own rules, under `[data-shell-bp]`).
    css.push_str(&shell_layout_css(
        "[data-shell-root]:not([data-shell-bp])",
        theme.touch_below.unwrap_or(760),
        1100,
    ));
    // 44 px touch targets under a coarse pointer, or below `touch_below` (the
    // `sm` breakpoint by default), for every interactive control whatever its
    // classes. Zero specificity (`:where`): an app's own min-height wins. Icon
    // buttons sized by `--Qzi` (whose min-width would beat `:where`) get 44 px
    // squares through the variable itself.
    // Checkboxes, radios and sliders keep their size.
    {
        use std::fmt::Write;
        let below = theme.touch_below.unwrap_or(640);
        let _ = writeln!(
            css,
            "@media (pointer:coarse),(max-width:{}.98px){{:root{{--Qzi:44px}}\
             :where(button,select,input:not([type=checkbox],[type=radio],[type=range],[type=hidden]),[role=tab],[role=button]){{min-height:44px}}\
             :where(button,[role=tab],[role=button]){{min-width:44px}}}}",
            below.saturating_sub(1)
        );
    }
    css
}

/// The `:root{…}` block of one resolved mode (`is_dark`), newline-terminated.
/// It also sets `color-scheme`, so native controls and scrollbars follow.
fn root_block(theme: &Theme, is_dark: bool) -> String {
    let rp = ResolvedPalette::new(theme);
    let mut css = String::with_capacity(2048);

    css.push_str(":root{");
    css.push_str(if is_dark {
        "color-scheme:dark;"
    } else {
        "color-scheme:light;"
    });

    // --- Semantic color variables (mode-aware) ---
    if is_dark {
        // Dark mode: invert scales
        css_var(&mut css, "a", &rp.neutral[11]); // bg-app
        css_var(&mut css, "b", &rp.neutral[10]); // bg-subtle
        css_var(&mut css, "c", &rp.neutral[9]); // bg-muted
        css_var(&mut css, "d", &rp.neutral[8]); // bg-emphasis
        css_var(&mut css, "e", &rp.neutral[7]); // bg-hover
        css_var(&mut css, "f", &rp.neutral[6]); // bg-active
        css_var(&mut css, "g", &rp.neutral[6]); // border-subtle
        css_var(&mut css, "h", &rp.neutral[5]); // border-default
        css_var(&mut css, "i", &rp.neutral[4]); // border-emphasis
        css_var(&mut css, "j", &rp.neutral[4]); // text-muted
        css_var(&mut css, "k", &rp.neutral[1]); // text-default
        css_var(&mut css, "l", &rp.neutral[0]); // text-high
        css_var(&mut css, "m", &rp.white); // text-on-accent
        css_var(&mut css, "r", &rp.neutral[11]); // surface
        css_var(&mut css, "s", &rp.neutral[0]); // on-surface
        css_var(&mut css, "t", &rp.neutral[10]); // surface-raised
        css_var(&mut css, "u", &rp.neutral[0]); // on-surface-raised
        css_var(&mut css, "A", &rp.neutral[8]); // secondary
        css_var(&mut css, "B", &rp.neutral[0]); // on-secondary
        css_var(&mut css, "C", &rp.neutral[7]); // secondary-hover
        css_var(&mut css, "D", &rp.neutral[9]); // muted
        css_var(&mut css, "E", &rp.neutral[3]); // on-muted
    } else {
        // Light mode
        css_var(&mut css, "a", &rp.neutral[0]); // bg-app
        css_var(&mut css, "b", &rp.neutral[1]); // bg-subtle
        css_var(&mut css, "c", &rp.neutral[2]); // bg-muted
        css_var(&mut css, "d", &rp.neutral[3]); // bg-emphasis
        css_var(&mut css, "e", &rp.neutral[4]); // bg-hover
        css_var(&mut css, "f", &rp.neutral[5]); // bg-active
        css_var(&mut css, "g", &rp.neutral[5]); // border-subtle
        css_var(&mut css, "h", &rp.neutral[6]); // border-default
        css_var(&mut css, "i", &rp.neutral[7]); // border-emphasis
        css_var(&mut css, "j", &rp.neutral[8]); // text-muted
        css_var(&mut css, "k", &rp.neutral[10]); // text-default
        css_var(&mut css, "l", &rp.neutral[11]); // text-high
        css_var(&mut css, "m", &rp.white); // text-on-accent
        css_var(&mut css, "r", &rp.neutral[0]); // surface
        css_var(&mut css, "s", &rp.neutral[11]); // on-surface
        css_var(&mut css, "t", &rp.neutral[1]); // surface-raised
        css_var(&mut css, "u", &rp.neutral[11]); // on-surface-raised
        css_var(&mut css, "A", &rp.neutral[3]); // secondary
        css_var(&mut css, "B", &rp.neutral[11]); // on-secondary
        css_var(&mut css, "C", &rp.neutral[4]); // secondary-hover
        css_var(&mut css, "D", &rp.neutral[2]); // muted
        css_var(&mut css, "E", &rp.neutral[10]); // on-muted
    }

    // Mode-independent vars
    for (i, color) in rp.accent.iter().enumerate() {
        vn(&mut css, "n", i + 1, color);
    }
    css_var(&mut css, "o", &rp.green[8]); // success
    css_var(&mut css, "p", &rp.amber[8]); // warning
    css_var(&mut css, "q", &rp.red[8]); // error
                                        // On-colors for the solid semantic backgrounds: white on dark fills, the
                                        // darkest neutral on bright fills (relative luminance threshold).
    css_var(&mut css, "o2", &on_solid_color(&rp.green[8], &rp));
    css_var(&mut css, "p2", &on_solid_color(&rp.amber[8], &rp));
    css_var(&mut css, "q2", &on_solid_color(&rp.red[8], &rp));
    css_var(&mut css, "F", &rp.red[8]); // destructive
    css_var(&mut css, "G", &rp.white); // on-destructive
    css_var(&mut css, "H", &rp.red[9]); // destructive-hover
    if is_dark {
        css_var(&mut css, "K", &rp.accent[5]); // focus-ring (brighter in dark)
        css_var(&mut css, "L", &rp.accent[4]); // border-primary (brighter in dark)
    } else {
        css_var(&mut css, "K", &rp.accent[7]); // focus-ring
        css_var(&mut css, "L", &rp.accent[6]); // border-primary
    }

    // --- Canvas: the sunken ground of a reading area (`St::BgCanvas`), under
    // cards that stay `BgSurface`. The app background unless a style remaps it.
    css.push_str("--Qw:var(--a);");

    // --- Style preset: resolve primary/surface/border overrides inline ---
    (theme.style.color_css)(&mut css, &rp, is_dark);

    // Subtle backgrounds and the text set on them, readable by construction.
    // After the style: the status texts are checked against the plain grounds
    // (`a`, `b`, `r`, `t`) as the style finally sets them.
    subtle_pairs(&mut css, &rp, is_dark);
    tone_pairs(&mut css, &rp, is_dark);
    dot_colors(&mut css, &rp, is_dark);

    // --- Q-vars (theme-style hooks for non-color customization) ---
    (theme.style.q_css)(&mut css, is_dark);

    // --- Font stacks: after the style's hooks, so the theme's own win ---
    theme.fonts.css(&mut css);

    // --- Density: control and row dimensions (`--Qz*`) ---
    css.push_str(theme.density.css());

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

/// The least WCAG contrast a text color gets against its ground (AA, body text).
pub const MIN_TEXT_CONTRAST: f64 = 4.5;

/// The least WCAG contrast of a graphic: a mark, a logotype, a decorative glyph.
pub const MIN_MARK_CONTRAST: f64 = 3.0;

/// `#rrggbb` → linear-light sRGB channels.
fn linear_rgb(hex: &str) -> [f64; 3] {
    let h = hex.trim_start_matches('#');
    let ch = |i: usize| {
        let c = h
            .get(i..i + 2)
            .and_then(|s| u8::from_str_radix(s, 16).ok())
            .unwrap_or(0) as f64
            / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    [ch(0), ch(2), ch(4)]
}

/// WCAG 2 relative luminance of a `#rrggbb` color.
fn wcag_luminance(hex: &str) -> f64 {
    let [r, g, b] = linear_rgb(hex);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// WCAG 2 contrast ratio between two `#rrggbb` colors (1 to 21).
pub fn contrast_ratio(a: &str, b: &str) -> f64 {
    let (x, y) = (wcag_luminance(a), wcag_luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// `a` laid over `b` at opacity `t` (linear light), as `#rrggbb`.
fn mix(a: &str, b: &str, t: f64) -> String {
    let (x, y) = (linear_rgb(a), linear_rgb(b));
    let enc = |c: f64| {
        let c = c.clamp(0.0, 1.0);
        let s = if c <= 0.003_130_8 {
            12.92 * c
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        };
        (s * 255.0).round() as u8
    };
    let m = |i: usize| enc(x[i] * t + y[i] * (1.0 - t));
    format!("#{:02x}{:02x}{:02x}", m(0), m(1), m(2))
}

/// The first candidate readable on every ground in `against` (contrast at
/// least [`MIN_TEXT_CONTRAST`]), else the one that comes closest.
fn first_readable(candidates: &[String], against: &[String]) -> String {
    let worst = |c: &String| {
        against
            .iter()
            .map(|g| contrast_ratio(c, g))
            .fold(f64::INFINITY, f64::min)
    };
    candidates
        .iter()
        .find(|c| worst(c) >= MIN_TEXT_CONTRAST)
        .or_else(|| {
            candidates
                .iter()
                .max_by(|a, b| worst(a).total_cmp(&worst(b)))
        })
        .cloned()
        .unwrap_or_default()
}

/// The subtle backgrounds (success, warning, info, error, destructive,
/// primary, accent) and the text set on them, plus the status text colors.
///
/// Palettes are hand-made scales whose steps do not mean the same lightness
/// everywhere, so fixed steps gave unreadable pairs (orange on salmon in dark
/// Nord). Here the grounds come from the palette (light mode: its pale steps;
/// dark mode: the hue laid over the app background, a dark tint), and each
/// text is the first of the palette's own steps, in order of preference, that
/// reaches [`MIN_TEXT_CONTRAST`] on its ground, then the brand step darkened
/// (dark mode: lightened) until it does, so the hue stays; the ends of the
/// neutral scale, then black or white, are the last resort.
fn subtle_pairs(css: &mut String, rp: &ResolvedPalette, is_dark: bool) {
    use crate::tokens::palette::css_color_to_hex as hex;
    let app = hex(if is_dark {
        &rp.neutral[11]
    } else {
        &rp.neutral[0]
    });
    // The plain grounds a status text also lands on (app, subtle, surface,
    // raised surface), as declared so far, style overrides included.
    let plain_grounds: Vec<String> = ["a", "b", "r", "t"]
        .iter()
        .filter_map(|name| declared_color(css, name))
        .map(|c| hex(&c))
        .collect();
    let ends = [
        hex(&rp.neutral[0]),
        hex(&rp.neutral[11]),
        "#ffffff".to_string(),
        "#000000".to_string(),
    ];
    // Toward the readable side: black in light mode, white in dark mode.
    let toward = if is_dark { "#ffffff" } else { "#000000" };
    let steps = |scale: &[String; 12], order: &[usize]| -> Vec<String> {
        let brand = hex(&scale[8]);
        order
            .iter()
            .map(|&i| hex(&scale[i]))
            // Then the brand step darkened (lightened) until it reads: the hue stays.
            .chain(
                [0.7, 0.55, 0.4, 0.25]
                    .iter()
                    .map(|&t| mix(&brand, toward, t)),
            )
            .chain(ends.iter().cloned())
            .collect()
    };
    // Text on a tinted ground: dark mode wants the pale end, light mode the dark end.
    let on_order: &[usize] = if is_dark { &[2, 1, 0] } else { &[10, 11] };
    // A status color as text: as close to the brand step (8) as reads.
    let status_order: &[usize] = if is_dark {
        &[8, 7, 6, 5, 4, 3, 2, 1, 0]
    } else {
        &[8, 9, 10, 11]
    };
    let ground = |scale: &[String; 12], light_step: usize| {
        if is_dark {
            mix(&hex(&scale[8]), &app, 0.22)
        } else {
            hex(&scale[light_step])
        }
    };

    // (background var, text var, status text var, scale, light-mode ground step)
    type Pair<'a> = (&'a str, &'a str, Option<&'a str>, &'a [String; 12], usize);
    let pairs: [Pair; 6] = [
        ("M", "M1", Some("o3"), &rp.green, 1),
        ("N", "N1", Some("p3"), &rp.amber, 1),
        ("O", "O1", None, &rp.accent, 1),
        ("P", "P1", Some("q3"), &rp.red, 1),
        ("I", "J", None, &rp.red, 2),
        ("y", "z", None, &rp.accent, 2),
    ];
    for (bg, text, status, scale, light_step) in pairs {
        let g = ground(scale, light_step);
        css_var(
            css,
            text,
            &first_readable(&steps(scale, on_order), std::slice::from_ref(&g)),
        );
        if let Some(status) = status {
            css_var(
                css,
                status,
                &first_readable(
                    &steps(scale, status_order),
                    &[std::slice::from_ref(&g), &plain_grounds[..]].concat(),
                ),
            );
        }
        css_var(css, bg, &g);
    }

    // The accent-subtle ground (a selected row) carries the ordinary text
    // colors, which do not change: the ground adapts to them.
    let (text, high) = if is_dark {
        (hex(&rp.neutral[1]), hex(&rp.neutral[0]))
    } else {
        (hex(&rp.neutral[10]), hex(&rp.neutral[11]))
    };
    let a8 = hex(&rp.accent[8]);
    let accent_grounds: Vec<String> = if is_dark {
        [0.25, 0.18, 0.12, 0.06]
            .iter()
            .map(|&t| mix(&a8, &app, t))
            .collect()
    } else {
        vec![
            hex(&rp.accent[2]),
            hex(&rp.accent[1]),
            hex(&rp.accent[0]),
            mix(&a8, &app, 0.15),
            mix(&a8, &app, 0.08),
        ]
    };
    css_var(css, "n0", &first_readable(&accent_grounds, &[text, high]));
}

/// The categorical and neutral tones (`Tone::Cat(n)`, `Tone::Neutral`) and the
/// faint ink, computed per mode against the plain grounds as the style left
/// them (called after it, like [`subtle_pairs`]).
///
/// For each categorical color `n` (1 to 6): `--Yn` the color itself (a dot, a
/// solid fill), `--Yni` its ink (text and outline: the palette's preferred ink,
/// else the hue, darkened in light mode or lightened in dark mode, the nearest
/// that reads at [`MIN_TEXT_CONTRAST`] on `--a`/`--b`/`--r`/`--t`), `--Yng` the
/// strongest tint of the hue that ink still reads on, and `--Yno` the text on
/// the solid fill. The neutral tone (gray) gets `--Y0g` and
/// `--Y0i`. `--j2` is the faint ink: the palest neutral step that still reads at
/// AA on the plain grounds (below `--k` and `--j`).
fn tone_pairs(css: &mut String, rp: &ResolvedPalette, is_dark: bool) {
    use crate::tokens::palette::css_color_to_hex as hex;
    let plain: Vec<String> = ["a", "b", "r", "t"]
        .iter()
        .filter_map(|name| declared_color(css, name))
        .map(|c| hex(&c))
        .collect();
    let app = plain
        .first()
        .cloned()
        .unwrap_or_else(|| hex(&rp.neutral[0]));
    let toward = if is_dark { "#ffffff" } else { "#000000" };
    let ends = [
        hex(&rp.neutral[0]),
        hex(&rp.neutral[11]),
        "#ffffff".to_string(),
        "#000000".to_string(),
    ];
    // `preferred`, then it pushed toward the readable side, step by step: the
    // first that reads is the nearest to it.
    let near = |preferred: &str| -> Vec<String> {
        let p = hex(preferred);
        std::iter::once(p.clone())
            .chain((1..=80).map(|i| mix(toward, &p, f64::from(i) * 0.01)))
            .collect()
    };
    let tone =
        |css: &mut String, n: usize, color: &str, preferred: Option<&str>, mark: Option<&str>| {
            let c = hex(color);
            let inks: Vec<String> = preferred
                .map(&near)
                .unwrap_or_default()
                .into_iter()
                .chain(std::iter::once(c.clone()))
                .chain(
                    [0.2, 0.35, 0.5, 0.65, 0.8]
                        .iter()
                        .map(|&t| mix(toward, &c, t)),
                )
                .chain(ends.iter().cloned())
                .collect();
            // The ink first, as near its preferred value as the plain grounds allow;
            // then the strongest tint of the hue that ink still reads on. Only when
            // no tint does is the ink chosen against the ground too.
            let tints: &[f64] = if is_dark {
                &[0.22, 0.17, 0.12, 0.08, 0.05, 0.03]
            } else {
                &[0.16, 0.12, 0.09, 0.06, 0.03]
            };
            let ink = first_readable(&inks, &plain);
            let (ground, ink) = match tints
                .iter()
                .map(|&t| mix(&c, &app, t))
                .find(|g| contrast_ratio(&ink, g) >= MIN_TEXT_CONTRAST)
            {
                Some(ground) => (ground, ink),
                None => {
                    // The faintest tint: the ink moves least from its preferred value.
                    let ground = mix(&c, &app, tints[tints.len() - 1]);
                    let against = [std::slice::from_ref(&ground), &plain[..]].concat();
                    let ink = first_readable(&inks, &against);
                    (ground, ink)
                }
            };
            css_var(css, &format!("Y{n}g"), &ground);
            css_var(css, &format!("Y{n}i"), &ink);
            if n > 0 {
                // A mark (logotype, decorative glyph) on the solid fill: the preferred
                // one if it reaches 3:1, else the nearest that does, else the text's.
                let on_solid = first_readable(&ends, std::slice::from_ref(&c));
                let mark = mark
                    .map(|m| {
                        let m = hex(m);
                        let away = if wcag_luminance(&m) > wcag_luminance(&c) {
                            "#ffffff"
                        } else {
                            "#000000"
                        };
                        std::iter::once(m.clone())
                            .chain((1..=100).map(|i| mix(away, &m, f64::from(i) * 0.01)))
                            .find(|x| contrast_ratio(x, &c) >= MIN_MARK_CONTRAST)
                            .unwrap_or_else(|| on_solid.clone())
                    })
                    .unwrap_or_else(|| on_solid.clone());
                css_var(css, &format!("Y{n}m"), &mark);
                css_var(css, &format!("Y{n}"), &c);
                css_var(css, &format!("Y{n}o"), &on_solid);
            }
        };
    tone(css, 0, &rp.neutral[6], None, None);
    for (i, color) in rp.categorical.iter().take(6).enumerate() {
        let preferred = rp.categorical_inks.get(i).map(|(light, dark)| {
            if is_dark {
                dark.as_str()
            } else {
                light.as_str()
            }
        });
        let mark = rp.categorical_on_solids.get(i).map(String::as_str);
        tone(css, i + 1, color, preferred, mark);
    }
    // A palette with fewer than six: the rest repeat it, so every token resolves.
    let given = rp.categorical.len().clamp(1, 6);
    for n in given + 1..=6 {
        let from = (n - 1) % given + 1;
        for suffix in ["", "g", "i", "o", "m"] {
            css_var(
                css,
                &format!("Y{n}{suffix}"),
                &format!("var(--Y{from}{suffix})"),
            );
        }
    }

    // The faint ink: the palest neutral step that still reads on every plain ground.
    let faint_order: &[usize] = if is_dark {
        &[5, 4, 3, 2, 1, 0]
    } else {
        &[6, 7, 8, 9, 10, 11]
    };
    // A style may name the faint ink it wants (`--j2`): kept if it reads, else
    // the nearest that does.
    let faint: Vec<String> = declared_color(css, "j2")
        .map(|c| near(&c))
        .unwrap_or_default()
        .into_iter()
        .chain(faint_order.iter().map(|&i| hex(&rp.neutral[i])))
        .collect();
    css_var(css, "j2", &first_readable(&faint, &plain));
}

/// The status dots' colors (`--Y?d`), one per tone, each reaching
/// [`MIN_MARK_CONTRAST`] (a dot is a graphic) on every ground a dot sits on:
/// app, subtle, surface, raised (a hovered row), the selected row (`--n0`) and
/// the sidebar (`--Qs`, when the style sets one). Each starts from the tone's
/// own color (a style may name the neutral one it wants in `--Y0d`) and, when
/// that does not reach 3:1, takes the nearest that does (darkened in light
/// mode, lightened in dark mode). Called after the style and the tones.
fn dot_colors(css: &mut String, rp: &ResolvedPalette, is_dark: bool) {
    use crate::tokens::palette::css_color_to_hex as hex;
    let grounds: Vec<String> = ["a", "b", "r", "t", "n0", "Qs"]
        .iter()
        .filter_map(|name| declared_color(css, name))
        .map(|c| hex(&c))
        .collect();
    let toward = if is_dark { "#ffffff" } else { "#000000" };
    let dot = |preferred: &str| -> String {
        let p = hex(preferred);
        std::iter::once(p.clone())
            .chain((1..=100).map(|i| mix(toward, &p, f64::from(i) * 0.01)))
            .find(|c| {
                grounds
                    .iter()
                    .all(|g| contrast_ratio(c, g) >= MIN_MARK_CONTRAST)
            })
            .unwrap_or_else(|| toward.to_string())
    };
    let declared = |name: &str, fallback: &str| {
        declared_color(css, name).unwrap_or_else(|| fallback.to_string())
    };
    let neutral = declared("Y0d", &rp.neutral[6]);
    let wanted = [
        ("Y0d", neutral),
        ("Yad", declared("v", &rp.accent[8])),
        ("Yid", rp.accent[8].clone()),
        ("Ysd", declared("o", &rp.green[8])),
        ("Ywd", declared("p", &rp.amber[8])),
        ("Yxd", declared("q", &rp.red[8])),
    ];
    for (name, color) in wanted {
        css_var(css, name, &dot(&color));
    }
    let given = rp.categorical.len().clamp(1, 6);
    for n in 1..=6 {
        match rp.categorical.get(n - 1).filter(|_| n <= given) {
            Some(color) => css_var(css, &format!("Y{n}d"), &dot(color)),
            None => {
                let from = (n - 1) % given + 1;
                css_var(css, &format!("Y{n}d"), &format!("var(--Y{from}d)"));
            }
        }
    }
}

/// The last value declared for `--name` in `css`, when it is a plain color
/// (not `transparent`, not a `var(…)`), else None.
fn declared_color(css: &str, name: &str) -> Option<String> {
    let key = format!("--{name}:");
    let at = css
        .match_indices(&key)
        .filter(|(i, _)| *i == 0 || matches!(css.as_bytes()[i - 1], b';' | b'{'))
        .map(|(i, _)| i + key.len())
        .last()?;
    let value = css[at..].split(';').next()?.trim();
    let plain = value.starts_with('#') || value.starts_with("oklch(") || value.starts_with("rgb");
    plain.then(|| value.to_string())
}

/// Relative luminance of a hex color (0.0-1.0). Non-hex values (oklch…)
/// return None — the caller falls back to a safe default.
fn hex_luminance(hex: &str) -> Option<f64> {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let channel = |s: &str| u16::from_str_radix(s, 16).ok().map(|v| v as f64 / 255.0);
    let (r, g, b) = (channel(&h[0..2])?, channel(&h[2..4])?, channel(&h[4..6])?);
    // WCAG relative luminance (approximation sans linéarisation fine)
    Some(0.2126 * r + 0.7152 * g + 0.0722 * b)
}

/// Luminance of an `oklch(L C H)` color on the same scale as [`hex_luminance`]:
/// the color is converted to its sRGB hex (gamut-clamped) and measured there, so
/// one formula and one threshold serve both notations. `L` is a number (0-1) or
/// a percentage, `C` a number or a percentage (100% = 0.4), `H` a number with an
/// optional `deg`; an alpha (`/ a`) is ignored. Anything else returns None.
fn oklch_luminance(color: &str) -> Option<f64> {
    let inner = color
        .trim()
        .strip_prefix("oklch(")?
        .strip_suffix(')')?
        .split('/')
        .next()?;
    let parts: Vec<&str> = inner.split_whitespace().collect();
    let [l, c, h] = parts.as_slice() else {
        return None;
    };
    let number = |s: &str| s.parse::<f32>().ok().filter(|v| v.is_finite());
    let scaled = |s: &str, full: f32| match s.strip_suffix('%') {
        Some(pct) => number(pct).map(|v| v / 100.0 * full),
        None => number(s),
    };
    let l = scaled(l, 1.0)?.clamp(0.0, 1.0);
    let c = scaled(c, 0.4)?.max(0.0);
    let h = number(h.strip_suffix("deg").unwrap_or(h))?;
    hex_luminance(&crate::tokens::palette::oklch_to_hex(l, c, h))
}

/// On-color for a solid semantic background: white on dark fills, the darkest
/// neutral of the palette on bright fills (threshold 0.55). Hex and `oklch()`
/// fills are measured; any other notation falls back to white.
fn on_solid_color(bg: &str, rp: &ResolvedPalette) -> String {
    match hex_luminance(bg).or_else(|| oklch_luminance(bg)) {
        Some(l) if l > 0.55 => rp.neutral[11].clone(),
        _ => rp.white.clone(),
    }
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
        css_var(css, "v", &rp.accent[7]); // primary — brighter for dark bg
        css_var(css, "w", &rp.accent[0]); // on-primary
        css_var(css, "x", &rp.accent[6]); // primary-hover
        css_var(css, "F", &rp.red[7]); // destructive
        css_var(css, "G", &rp.red[0]); // on-destructive
        css_var(css, "H", &rp.red[6]); // destructive-hover
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

fn flat_q_css(css: &mut String, is_dark: bool) {
    soft_q_css(css, is_dark);
    // The whole UI runs on the mono stack (the base CSS reads `--Qft` for `body`).
    css.push_str("--Qft:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;");
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

    /// E2 criterion: `System` emits the light block on `:root` and the dark one
    /// under `prefers-color-scheme: dark`; `Light` and `Dark` force their mode.
    #[test]
    fn system_emits_both_modes_and_light_dark_force_one() {
        let light = generate_theme_css(&Theme::light());
        let dark = generate_theme_css(&Theme::dark());
        let system = generate_theme_css(&Theme::new().mode(ThemeMode::System));

        // Forced: one block, no media query, the mode's own colors.
        for (css, scheme) in [(&light, "light"), (&dark, "dark")] {
            assert!(!css.contains("prefers-color-scheme"), "{css}");
            // One color block (the touch-target rule's `:root{--Qzi}` aside).
            assert_eq!(css.matches(":root{color-scheme").count(), 1, "{css}");
            assert!(css.starts_with(&format!(":root{{color-scheme:{scheme};")));
        }
        assert_ne!(light, dark);

        // System: exactly the light block, then the dark block in the media query
        // (the mode-independent rules, such as touch targets, follow once).
        let block = |css: &str| css.split("@media (pointer").next().unwrap().to_string();
        let (light, dark) = (block(&light), block(&dark));
        let (head, tail) = system
            .split_once("@media (prefers-color-scheme:dark){")
            .expect("a dark media block");
        assert_eq!(head, light, "the light block, unchanged, on :root");
        let inner = block(tail);
        let inner = inner
            .trim_end()
            .strip_suffix('}')
            .expect("closed media block");
        assert_eq!(inner, dark.trim_end(), "the dark block, unchanged");

        // The base font size rule follows both blocks, once.
        let sized =
            generate_theme_css(&Theme::new().mode(ThemeMode::System).base_font_size(14, 15));
        assert_eq!(sized.matches("html{font-size:14px}").count(), 1);
        assert!(sized.rfind("html{font-size").unwrap() > sized.find("@media (prefers").unwrap());
    }

    #[test]
    fn fonts_feed_the_ui_read_and_mono_variables() {
        let fonts = Fonts::new()
            .ui("Inter, system-ui, sans-serif")
            .read("'Source Serif 4', Georgia, serif")
            .mono("'JetBrains Mono', ui-monospace, monospace");
        let css = generate_theme_css(&Theme::new().fonts(fonts.clone()));
        assert!(css.contains("--Qft:Inter, system-ui, sans-serif;"), "{css}");
        assert!(
            css.contains("--Qfr:'Source Serif 4', Georgia, serif;"),
            "{css}"
        );
        assert!(
            css.contains("--Qm:'JetBrains Mono', ui-monospace, monospace;"),
            "{css}"
        );
        // The theme's stacks win over a style's own: flat sets a mono UI first.
        let flat = generate_theme_css(&Theme::new().style(ThemeStyle::flat()).fonts(fonts));
        assert!(
            flat.rfind("--Qft:Inter").unwrap() > flat.find("--Qft:ui-monospace").unwrap(),
            "{flat}"
        );
        // Unset stacks emit nothing; a stack cannot end the declaration.
        let css = generate_theme_css(&Theme::new().fonts(Fonts::new().read("x;--a:red}")));
        assert!(!css.contains("--Qm:") && !css.contains("--Qft:"), "{css}");
        assert!(css.contains("--Qfr:x--a:red;"), "{css}");
        // In System mode, both blocks carry them.
        let sys = generate_theme_css(
            &Theme::new()
                .mode(ThemeMode::System)
                .fonts(Fonts::new().ui("Inter")),
        );
        assert_eq!(sys.matches("--Qft:Inter;").count(), 2, "{sys}");
    }

    #[test]
    fn font_read_reads_the_reading_stack() {
        assert_eq!(crate::St::FontRead.css(), "font-family:var(--Qfr,inherit)");
    }

    #[test]
    fn density_sets_the_control_hooks() {
        // Comfortable: nothing emitted, the tokens' fallbacks apply.
        let comfy = generate_theme_css(&Theme::new());
        let root = comfy.split("@media (pointer").next().unwrap();
        assert!(!root.contains("--Qz"), "{comfy}");
        let compact = generate_theme_css(&Theme::new().density(Density::Compact));
        assert!(compact.contains("--Qzh:2rem;"), "{compact}");
        assert!(compact.contains("--Qzp:.38rem .55rem;"));
        assert!(compact.contains("--Qzy:var(--S2);"));
        // Icon buttons: 32 px squares, 18 px glyphs, no padding (`/app`).
        assert!(
            compact.contains("--Qzi:32px;--Qzg:18px;--Qzq:0;"),
            "{compact}"
        );
        assert!(St::SqIconButton
            .css()
            .starts_with("width:var(--Qzi,2.25rem);"));
        assert!(St::SqIconToggle.css().starts_with("width:var(--Qzi,auto);"));
        // The tokens read them, with the comfortable sizes as fallbacks.
        use crate::St;
        assert_eq!(St::HControl.css(), "height:var(--Qzh,2.25rem)");
        assert_eq!(St::TextControl.css(), "font-size:var(--Qzt,var(--T2))");
        assert_eq!(St::PyTab.css(), "padding-block:var(--Qzy,var(--S3))");
        assert_eq!(St::PxChip.css(), "padding-inline:var(--Qzcx,var(--S2))");
        assert_eq!(St::PyChip.css(), "padding-block:var(--Qzcy,var(--S1))");
        assert_eq!(St::PadRow.css(), "padding:var(--Qzp,var(--S2) var(--S3))");
    }

    #[test]
    fn touch_targets_are_44px_under_a_coarse_pointer_or_below_sm() {
        let base = generate_theme_css(&Theme::new());
        let rule = base
            .split_once("@media (pointer:coarse),(max-width:639.98px){")
            .expect("the touch-target media block")
            .1;
        assert!(rule.contains(":where(button,select,input:not("), "{rule}");
        assert!(
            rule.contains("[role=tab],[role=button]){min-height:44px}"),
            "{rule}"
        );
        assert!(rule.contains("{min-width:44px}"), "{rule}");
        // Zero specificity (`:where`): an app's own min-height still wins.
        assert!(!rule.contains("button{min-height"));
        // Checkboxes, radios and sliders keep their size.
        assert!(rule.contains("[type=checkbox],[type=radio],[type=range]"));
    }

    /// Whether the 44 px touch-target rule of `css` applies to a fine pointer
    /// on a viewport `width` px wide (its `max-width` branch).
    fn touch_rule_applies(css: &str, width: f64) -> bool {
        let max = css
            .split_once("@media (pointer:coarse),(max-width:")
            .expect("the touch-target media block")
            .1
            .split_once("px)")
            .unwrap()
            .0;
        width <= max.parse::<f64>().unwrap()
    }

    #[test]
    fn touch_below_sets_where_the_44px_rule_starts() {
        // Default: below `sm` (640): a fine pointer at 700 px keeps its sizes.
        let default = generate_theme_css(&Theme::new());
        assert!(!touch_rule_applies(&default, 700.0));
        assert!(touch_rule_applies(&default, 639.0));
        // mcp-agents' phone threshold: 760.
        let app = generate_theme_css(&Theme::new().touch_below(760));
        assert!(touch_rule_applies(&app, 700.0));
        assert!(!touch_rule_applies(&app, 760.0));
        assert!(!touch_rule_applies(&app, 800.0));
        // In System mode the rule is emitted once, after both blocks.
        let sys = generate_theme_css(&Theme::new().mode(ThemeMode::System).touch_below(760));
        assert_eq!(sys.matches("{min-height:44px}").count(), 1, "{sys}");
    }

    #[test]
    fn modes_cycle_parse_and_toggle() {
        use ThemeMode::*;
        assert_eq!(
            [System.cycle(), Light.cycle(), Dark.cycle()],
            [Light, Dark, System]
        );
        for m in [Light, Dark, System] {
            assert_eq!(ThemeMode::parse(m.as_str()), Some(m));
        }
        assert_eq!(ThemeMode::parse("auto"), None);
        assert_eq!(
            [Light.toggle(), Dark.toggle(), System.toggle()],
            [Dark, Light, Light]
        );
    }

    #[test]
    fn the_theme_style_names_its_mode() {
        let html = theme_renderer(&Theme::new().mode(ThemeMode::System)).to_static_html();
        assert!(html.starts_with("<style data-rw-mode=\"system\""), "{html}");
    }

    #[test]
    fn flat_style_sets_the_mono_ui_font() {
        let theme = Theme::dark().style(ThemeStyle::flat());
        let css = generate_theme_css(&theme);
        assert!(css.contains("--Qft:ui-monospace"), "mono UI font var");
        assert!(
            css.contains("--Qd:none"),
            "keeps soft's shadow-free surfaces"
        );
        assert!(generate_base_css().contains("font-family:var(--Qft,"));
    }

    #[test]
    fn test_theme_defaults() {
        let theme = Theme::default();
        assert_eq!(theme.mode, ThemeMode::Light);
        assert_eq!(theme.radius, RadiusScale::Medium);
        assert_eq!(theme.style, ThemeStyle::soft());
        assert!(theme.palette_ref().is_none());
    }

    #[test]
    fn base_font_size_emits_root_rule_only_when_set() {
        assert!(!generate_theme_css(&Theme::dark()).contains("html{font-size"));
        let css = generate_theme_css(&Theme::dark().base_font_size(14, 16));
        assert!(css.contains("html{font-size:14px}"));
        assert!(css.contains("@media(min-width:640px){html{font-size:16px}}"));
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
        // +~70B (2026-09): the tapped control waiting on the server (`data-busy`, set by the
        // runtime at the tap) dims after 200ms; client-set, so it can't ride STYLE_DEF.
        // +~70B (R1): unstyled renderer wrappers (live `__synced_N`, static `data-rwr`)
        // add no box, so a region never breaks a grid or a flex row.
        assert!(css.len() < 900, "Base CSS too large: {} bytes", css.len());
    }

    #[test]
    fn test_generate_theme_css_light() {
        let theme = Theme::light();
        let css = generate_theme_css(&theme);

        // Single :root block, no data-theme selectors
        assert!(css.starts_with(":root{"), "Should start with :root{{");
        assert!(
            !css.contains("[data-theme"),
            "Should not contain data-theme selectors"
        );
        assert!(
            !css.contains("[data-style"),
            "Should not contain data-style selectors"
        );

        // Should have resolved values
        assert!(css.contains("--a:"), "Missing --a (bg-app)");
        assert!(css.contains("--k:"), "Missing --k (text-default)");
        assert!(css.contains("--v:"), "Missing --v (primary)");
        assert!(
            css.contains("oklch("),
            "Should contain resolved oklch values"
        );

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

        assert!(
            css.contains("--R2:9999px"),
            "Full radius should override --R2"
        );
        assert!(!css.contains("[data-radius"), "No data-radius selectors");
    }

    #[test]
    fn test_generate_theme_css_size() {
        let theme = Theme::dark()
            .style(ThemeStyle::soft())
            .radius(RadiusScale::Large);
        let css = generate_theme_css(&theme);
        // Single block should be compact.
        // +~400B (R1): the neutral and six categorical tones (`--Y0g…--Y6o`), the faint ink.
        // +~200B (R1): 44 px touch targets below `Theme::touch_below` (a theme setting).
        // +~650B (R2): the three-column shell's drawer and single-column rules, whose
        // one-column threshold follows `Theme::touch_below`.
        assert!(css.len() < 3500, "Theme CSS too large: {} bytes", css.len());
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

    /// The default palette is oklch: the bright amber fill gets the dark text, the
    /// darker green and red fills keep white — in the generated CSS too.
    #[test]
    fn on_colors_follow_oklch_fills_of_the_default_palette() {
        let rp = ResolvedPalette::new(&Theme::default());
        let dark = rp.neutral[11].clone();
        assert_eq!(rp.amber[8], "oklch(0.75 0.18 85)");
        assert_eq!(on_solid_color(&rp.amber[8], &rp), dark);
        assert_eq!(on_solid_color(&rp.green[8], &rp), rp.white);
        assert_eq!(on_solid_color(&rp.red[8], &rp), rp.white);
        for theme in [Theme::default(), Theme::dark()] {
            let css = generate_theme_css(&theme);
            assert!(
                css.contains(&format!("--p2:{dark}")),
                "warning on-color: {css}"
            );
            assert!(css.contains(&format!("--o2:{}", rp.white)));
            assert!(css.contains(&format!("--q2:{}", rp.white)));
        }
    }

    #[test]
    fn on_solid_color_reads_oklch_numbers_and_percentages() {
        let rp = ResolvedPalette::new(&Theme::default());
        let dark = rp.neutral[11].clone();
        for bright in [
            "oklch(0.75 0.18 85)",
            "oklch(75% 0.18 85)",
            "oklch(75% 45% 85deg)",
            "oklch(0.95 0.02 250 / 0.5)",
            " oklch(0.9 0 0) ",
        ] {
            assert_eq!(on_solid_color(bright, &rp), dark, "{bright}");
        }
        for deep in ["oklch(0.3 0.1 250)", "oklch(30% 0.1 250)", "oklch(0 0 0)"] {
            assert_eq!(on_solid_color(deep, &rp), rp.white, "{deep}");
        }
    }

    /// The oklch path measures on the hex scale: a gray sits on the same side of the
    /// threshold in both notations.
    #[test]
    fn oklch_and_hex_luminance_agree() {
        let mid_gray = crate::tokens::palette::oklch_to_hex(0.6, 0.0, 0.0);
        for (oklch, hex) in [
            ("oklch(1 0 0)", "#ffffff"),
            ("oklch(0 0 0)", "#000000"),
            ("oklch(0.6 0 0)", mid_gray.as_str()),
        ] {
            let (a, b) = (oklch_luminance(oklch).unwrap(), hex_luminance(hex).unwrap());
            assert!((a - b).abs() < 1e-9, "{oklch} {a} vs {hex} {b}");
        }
    }

    #[test]
    fn on_solid_color_keeps_the_hex_rule() {
        let rp = ResolvedPalette::new(&Theme::default());
        assert_eq!(on_solid_color("#ffd700", &rp), rp.neutral[11]);
        assert_eq!(on_solid_color("#1e3a8a", &rp), rp.white);
        // 0.2126*0x80 + 0.7152*0x80 + 0.0722*0x80, over 255: just under the threshold.
        assert!((hex_luminance("#808080").unwrap() - 128.0 / 255.0).abs() < 1e-9);
        assert_eq!(on_solid_color("#808080", &rp), rp.white);
    }

    #[test]
    fn on_solid_color_falls_back_to_white_on_unreadable_values() {
        let rp = ResolvedPalette::new(&Theme::default());
        for value in [
            "var(--x)",
            "rgb(255 255 0)",
            "oklch(abc 0.1 20)",
            "oklch(0.9 0.1)",
            "oklch(0.9 0.1 20 30)",
            "oklch(0.9 0.1 20",
            "oklch(NaN 0 0)",
            "",
        ] {
            assert_eq!(oklch_luminance(value), None, "{value:?}");
            assert_eq!(on_solid_color(value, &rp), rp.white, "{value:?}");
        }
    }

    #[test]
    fn mix_and_contrast_helpers() {
        assert_eq!(mix("#ff0000", "#0000ff", 1.0), "#ff0000");
        assert_eq!(mix("#ff0000", "#0000ff", 0.0), "#0000ff");
        // Linear-light mixing: half white over black is #bcbcbc, not #808080.
        assert_eq!(mix("#ffffff", "#000000", 0.5), "#bcbcbc");
        assert!((contrast_ratio("#000000", "#ffffff") - 21.0).abs() < 1e-9);
        let grounds = ["#ffffff".to_string()];
        // The first readable candidate wins, in order.
        let pick = first_readable(
            &["#eeeeee".into(), "#595959".into(), "#000000".into()],
            &grounds,
        );
        assert_eq!(pick, "#595959");
        // None readable: the closest.
        let pick = first_readable(&["#eeeeee".into(), "#aaaaaa".into()], &grounds);
        assert_eq!(pick, "#aaaaaa");
    }

    #[test]
    fn status_text_has_its_own_var_and_fills_keep_theirs() {
        use crate::style_tokens::St;
        assert_eq!(St::TextWarning.css(), "color:var(--p3)");
        assert_eq!(St::BgWarning.css(), "background:var(--p)");
        assert_eq!(St::BgAccentSubtle.css(), "background:var(--n0)");
        for theme in [Theme::light(), Theme::dark()] {
            let css = generate_theme_css(&theme);
            for v in [
                "--o3:", "--p3:", "--q3:", "--n0:", "--N:", "--N1:", "--y:", "--z:",
            ] {
                assert!(css.contains(v), "{v} missing");
            }
        }
    }
}
