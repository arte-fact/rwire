//! Theme system for rwire.
//!
//! Themes define semantic tokens that map primitive values to UI roles.
//! This provides a layer of abstraction between raw colors and their usage.
//!
//! # Philosophy
//!
//! - Theme is determined server-side (no JS theme switching)
//! - Both light and dark CSS are included in capsule
//! - `data-theme` attribute controls active theme
//! - CSS variables handle the actual switching
//!
//! # Example
//!
//! ```ignore
//! use rwire::theme::{Theme, AccentColor};
//!
//! // Light theme with green accent
//! let theme = Theme::light().with_accent(AccentColor::Green);
//!
//! // Dark theme with default blue accent
//! let theme = Theme::dark();
//! ```

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

/// Accent color selection.
///
/// The accent color is used for primary actions, links, and focus states.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AccentColor {
    #[default]
    Blue,
    Red,
    Green,
    Amber,
}

impl AccentColor {
    /// Get the color scale name.
    pub fn as_str(&self) -> &'static str {
        match self {
            AccentColor::Blue => "blue",
            AccentColor::Red => "red",
            AccentColor::Green => "green",
            AccentColor::Amber => "amber",
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
/// Defines the visual appearance of the application.
#[derive(Clone, Copy, Debug, Default)]
pub struct Theme {
    /// Light or dark mode
    pub mode: ThemeMode,
    /// Accent color for primary actions
    pub accent: AccentColor,
    /// Border radius scale
    pub radius: RadiusScale,
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

    /// Set the accent color.
    pub fn with_accent(mut self, accent: AccentColor) -> Self {
        self.accent = accent;
        self
    }

    /// Set the radius scale.
    pub fn with_radius(mut self, radius: RadiusScale) -> Self {
        self.radius = radius;
        self
    }

    /// Generate data attributes for the theme root element.
    ///
    /// Returns a string like `data-theme="dark" data-accent="green"`.
    pub fn data_attrs(&self) -> String {
        let mut attrs = format!("data-theme=\"{}\"", self.mode.as_str());

        // Only add accent attr if not default (blue)
        if self.accent != AccentColor::Blue {
            attrs.push_str(&format!(" data-accent=\"{}\"", self.accent.as_str()));
        }

        // Only add radius attr if not default (medium)
        if self.radius != RadiusScale::Medium {
            attrs.push_str(&format!(" data-radius=\"{}\"", self.radius.as_str()));
        }

        attrs
    }
}

/// Generate base/reset CSS.
///
/// Minimal normalization for consistent cross-browser behavior.
/// Kept small to minimize capsule size.
pub fn generate_base_css() -> &'static str {
    "*,*::before,*::after{box-sizing:border-box}\
     body{margin:0;font-family:system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",Roboto,sans-serif;\
     line-height:var(--rw-leading-normal);color:var(--rw-text-default);background:var(--rw-bg-app)}\
     button,input,select,textarea{font:inherit}\n"
}

/// Generate semantic token CSS for light and dark themes.
///
/// Semantic tokens reference primitive tokens and provide meaningful names
/// for UI roles (background, text, borders, etc.).
pub fn generate_semantic_css() -> String {
    let mut css = String::with_capacity(2048);

    // Light theme (default)
    css.push_str(":root,[data-theme=\"light\"]{\n");
    css.push_str("--rw-bg-app:var(--rw-neutral-1);\n");
    css.push_str("--rw-bg-subtle:var(--rw-neutral-2);\n");
    css.push_str("--rw-bg-muted:var(--rw-neutral-3);\n");
    css.push_str("--rw-bg-emphasis:var(--rw-neutral-4);\n");
    css.push_str("--rw-bg-hover:var(--rw-neutral-5);\n");
    css.push_str("--rw-bg-active:var(--rw-neutral-6);\n");
    css.push_str("--rw-border-subtle:var(--rw-neutral-6);\n");
    css.push_str("--rw-border-default:var(--rw-neutral-7);\n");
    css.push_str("--rw-border-emphasis:var(--rw-neutral-8);\n");
    css.push_str("--rw-text-muted:var(--rw-neutral-9);\n");
    css.push_str("--rw-text-default:var(--rw-neutral-11);\n");
    css.push_str("--rw-text-high:var(--rw-neutral-12);\n");
    css.push_str("--rw-text-on-accent:var(--rw-white);\n");
    // Accent colors (default to blue)
    for i in 1..=12 {
        css.push_str(&format!("--rw-accent-{}:var(--rw-blue-{});\n", i, i));
    }
    css.push_str("--rw-success:var(--rw-green-9);\n");
    css.push_str("--rw-warning:var(--rw-amber-9);\n");
    css.push_str("--rw-error:var(--rw-red-9);\n");
    css.push_str("}\n");

    // Dark theme
    css.push_str("[data-theme=\"dark\"]{\n");
    // Invert the neutral scale for backgrounds
    css.push_str("--rw-bg-app:var(--rw-neutral-12);\n");
    css.push_str("--rw-bg-subtle:var(--rw-neutral-11);\n");
    css.push_str("--rw-bg-muted:var(--rw-neutral-10);\n");
    css.push_str("--rw-bg-emphasis:var(--rw-neutral-9);\n");
    css.push_str("--rw-bg-hover:var(--rw-neutral-8);\n");
    css.push_str("--rw-bg-active:var(--rw-neutral-7);\n");
    // Invert borders
    css.push_str("--rw-border-subtle:var(--rw-neutral-7);\n");
    css.push_str("--rw-border-default:var(--rw-neutral-6);\n");
    css.push_str("--rw-border-emphasis:var(--rw-neutral-5);\n");
    // Invert text
    css.push_str("--rw-text-muted:var(--rw-neutral-5);\n");
    css.push_str("--rw-text-default:var(--rw-neutral-2);\n");
    css.push_str("--rw-text-high:var(--rw-neutral-1);\n");
    css.push_str("}\n");

    css
}

/// Generate accent color override CSS.
///
/// Returns CSS to override the accent scale with a different color.
/// Returns `None` if the accent is the default (blue).
pub fn generate_accent_css(accent: AccentColor) -> Option<String> {
    if accent == AccentColor::Blue {
        return None;
    }

    let name = accent.as_str();
    let mut css = String::with_capacity(512);
    css.push_str(&format!("[data-accent=\"{}\"]{{", name));
    for i in 1..=12 {
        css.push_str(&format!("--rw-accent-{}:var(--rw-{}-{});", i, name, i));
    }
    css.push_str("}\n");
    Some(css)
}

/// Generate radius scale override CSS.
///
/// Returns CSS to override the component radius.
/// Returns `None` if the radius is the default (medium).
pub fn generate_radius_css(radius: RadiusScale) -> Option<&'static str> {
    match radius {
        RadiusScale::Medium => None,
        RadiusScale::None => Some("[data-radius=\"none\"]{--rw-radius-component:0}\n"),
        RadiusScale::Small => {
            Some("[data-radius=\"small\"]{--rw-radius-component:var(--rw-radius-sm)}\n")
        }
        RadiusScale::Large => {
            Some("[data-radius=\"large\"]{--rw-radius-component:var(--rw-radius-xl)}\n")
        }
        RadiusScale::Full => {
            Some("[data-radius=\"full\"]{--rw-radius-component:var(--rw-radius-full)}\n")
        }
    }
}

/// Generate complete theme CSS.
///
/// Includes base reset, primitive tokens, and semantic tokens.
/// This is the complete CSS needed for the capsule.
pub fn generate_theme_css(theme: &Theme) -> String {
    use crate::tokens::css::generate_primitive_css;

    let mut css = String::with_capacity(8192);

    // 1. Base reset
    css.push_str(generate_base_css());

    // 2. Primitive tokens
    css.push_str(&generate_primitive_css());

    // 3. Semantic tokens (light + dark)
    css.push_str(&generate_semantic_css());

    // 4. Accent override (if non-default)
    if let Some(accent_css) = generate_accent_css(theme.accent) {
        css.push_str(&accent_css);
    }

    // 5. Radius override (if non-default)
    if let Some(radius_css) = generate_radius_css(theme.radius) {
        css.push_str(radius_css);
    }

    css
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_defaults() {
        let theme = Theme::default();
        assert_eq!(theme.mode, ThemeMode::Light);
        assert_eq!(theme.accent, AccentColor::Blue);
        assert_eq!(theme.radius, RadiusScale::Medium);
    }

    #[test]
    fn test_theme_builders() {
        let theme = Theme::dark().with_accent(AccentColor::Green).with_radius(RadiusScale::Large);

        assert_eq!(theme.mode, ThemeMode::Dark);
        assert_eq!(theme.accent, AccentColor::Green);
        assert_eq!(theme.radius, RadiusScale::Large);
    }

    #[test]
    fn test_data_attrs_minimal() {
        // Default theme should only have data-theme
        let theme = Theme::default();
        assert_eq!(theme.data_attrs(), "data-theme=\"light\"");
    }

    #[test]
    fn test_data_attrs_full() {
        let theme = Theme::dark().with_accent(AccentColor::Red).with_radius(RadiusScale::Full);

        let attrs = theme.data_attrs();
        assert!(attrs.contains("data-theme=\"dark\""));
        assert!(attrs.contains("data-accent=\"red\""));
        assert!(attrs.contains("data-radius=\"full\""));
    }

    #[test]
    fn test_base_css_size() {
        let css = generate_base_css();
        // Base CSS should be under 300 bytes
        assert!(
            css.len() < 300,
            "Base CSS too large: {} bytes",
            css.len()
        );
    }

    #[test]
    fn test_semantic_css_structure() {
        let css = generate_semantic_css();

        // Should have both light and dark themes
        assert!(css.contains(":root"));
        assert!(css.contains("[data-theme=\"light\"]"));
        assert!(css.contains("[data-theme=\"dark\"]"));

        // Should have semantic tokens
        assert!(css.contains("--rw-bg-app:"));
        assert!(css.contains("--rw-text-high:"));
        assert!(css.contains("--rw-accent-9:"));
    }

    #[test]
    fn test_semantic_css_size() {
        let css = generate_semantic_css();
        // Semantic CSS should be under 1.5KB
        assert!(
            css.len() < 1536,
            "Semantic CSS too large: {} bytes",
            css.len()
        );
        println!("Semantic CSS size: {} bytes", css.len());
    }

    #[test]
    fn test_accent_css() {
        // Default (blue) returns None
        assert!(generate_accent_css(AccentColor::Blue).is_none());

        // Non-default returns CSS
        let css = generate_accent_css(AccentColor::Green).unwrap();
        assert!(css.contains("[data-accent=\"green\"]"));
        assert!(css.contains("--rw-accent-9:var(--rw-green-9)"));
    }

    #[test]
    fn test_radius_css() {
        // Default (medium) returns None
        assert!(generate_radius_css(RadiusScale::Medium).is_none());

        // Non-default returns CSS
        let css = generate_radius_css(RadiusScale::Full).unwrap();
        assert!(css.contains("[data-radius=\"full\"]"));
    }

    #[test]
    fn test_full_theme_css_size() {
        let theme = Theme::dark().with_accent(AccentColor::Red).with_radius(RadiusScale::Large);

        let css = generate_theme_css(&theme);

        // Full theme CSS should be under 6KB
        assert!(
            css.len() < 6144,
            "Full theme CSS too large: {} bytes",
            css.len()
        );
        println!("Full theme CSS size: {} bytes", css.len());
    }

    #[test]
    fn test_theme_css_contains_all_sections() {
        let theme = Theme::default();
        let css = generate_theme_css(&theme);

        // Should contain all sections
        assert!(css.contains("box-sizing")); // Base reset
        assert!(css.contains("--rw-neutral-1:")); // Primitive tokens
        assert!(css.contains("--rw-bg-app:")); // Semantic tokens
    }
}
