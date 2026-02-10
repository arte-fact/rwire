//! Predefined style presets for rwire themes.
//!
//! Each style preset controls the visual *feel* of components (solid vs subtle,
//! sharp vs soft) without changing individual components.
//!
//! # Usage
//!
//! ```ignore
//! use rwire::theme::Theme;
//! use rwire_themes::styles;
//!
//! let theme = Theme::dark().style(styles::brutalist());
//! ```

use rwire::theme::{css_var, IntoStyle, ResolvedPalette, ThemeStyle};

/// All extended style presets (excludes built-in Soft).
///
/// Useful for building UI controls that cycle through styles.
pub const ALL: &[fn() -> ThemeStyle] = &[solid, brutalist, minimal, glass, neon];

/// Solid accents, medium radius, subtle shadows (the classic default look).
pub fn solid() -> ThemeStyle {
    ThemeStyle::new("solid", "Solid", solid_color_css, solid_q_css)
}

/// Sharp corners, heavy borders, high contrast.
pub fn brutalist() -> ThemeStyle {
    ThemeStyle::new("brutalist", "Brutalist", brutalist_color_css, brutalist_q_css)
}

/// Near-zero borders, large spacing, text hierarchy.
pub fn minimal() -> ThemeStyle {
    ThemeStyle::new("minimal", "Minimal", minimal_color_css, minimal_q_css)
}

/// Glassmorphism: frosted surfaces, backdrop-blur, subtle borders.
pub fn glass() -> ThemeStyle {
    ThemeStyle::new("glass", "Glass", glass_color_css, glass_q_css)
}

/// Cyberpunk/Neon: glowing borders, text-shadow, dark-first.
pub fn neon() -> ThemeStyle {
    ThemeStyle::new("neon", "Neon", neon_color_css, neon_q_css)
}

/// Convenience enum for all extended styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Solid,
    Brutalist,
    Minimal,
    Glass,
    Neon,
}

impl Style {
    /// All variants.
    pub const ALL: &[Style] = &[
        Style::Solid,
        Style::Brutalist,
        Style::Minimal,
        Style::Glass,
        Style::Neon,
    ];

    /// Get the human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            Style::Solid => "Solid",
            Style::Brutalist => "Brutalist",
            Style::Minimal => "Minimal",
            Style::Glass => "Glass",
            Style::Neon => "Neon",
        }
    }
}

impl IntoStyle for Style {
    fn into_style(self) -> ThemeStyle {
        match self {
            Style::Solid => solid(),
            Style::Brutalist => brutalist(),
            Style::Minimal => minimal(),
            Style::Glass => glass(),
            Style::Neon => neon(),
        }
    }
}

// ============================================================================
// Solid Style (old "Default")
// ============================================================================

fn solid_color_css(css: &mut String, rp: &ResolvedPalette, _is_dark: bool) {
    css_var(css, "v", &rp.accent[8]);   // primary
    css_var(css, "w", &rp.white);       // on-primary
    css_var(css, "x", &rp.accent[9]);   // primary-hover
}

fn solid_q_css(css: &mut String, _is_dark: bool) {
    css.push_str("--Qd:var(--Z1);--Qgw:none;");
    css.push_str("--Qb:1px;--Qbl:var(--Qb);--Qbt:var(--Qb);--Qbc:var(--h);--Qbs:solid;");
    css.push_str("--Qol:2px;--Qoo:2px;--Qf:var(--K);");
    css.push_str("--Qbf:none;--Qso:1;--Qgr:none;");
    css.push_str("--Qt:150ms;--Qts:none;");
}

// ============================================================================
// Brutalist Style
// ============================================================================

fn brutalist_color_css(css: &mut String, rp: &ResolvedPalette, is_dark: bool) {
    css_var(css, "v", &rp.accent[8]);
    css_var(css, "w", &rp.white);
    css_var(css, "x", &rp.accent[9]);
    if is_dark {
        css_var(css, "h", &rp.neutral[0]);
        css_var(css, "g", &rp.neutral[2]);
        css_var(css, "i", &rp.neutral[0]);
        css_var(css, "t", &rp.neutral[11]);
    } else {
        css_var(css, "h", &rp.neutral[11]);
        css_var(css, "g", &rp.neutral[9]);
        css_var(css, "i", &rp.neutral[11]);
        css_var(css, "t", &rp.neutral[0]);
    }
}

fn brutalist_q_css(css: &mut String, _is_dark: bool) {
    css.push_str("--Qd:none;--Qgw:none;");
    css.push_str("--Qb:2px;--Qbl:3px;--Qbt:var(--Qb);--Qbc:var(--h);--Qbs:solid;");
    css.push_str("--Qol:3px;--Qoo:0px;--Qf:var(--K);");
    css.push_str("--Qbf:none;--Qso:1;--Qgr:none;");
    css.push_str("--Qt:0ms;--Qts:none;");
}

// ============================================================================
// Minimal Style
// ============================================================================

fn minimal_color_css(css: &mut String, rp: &ResolvedPalette, is_dark: bool) {
    css_var(css, "v", &rp.accent[8]);
    css_var(css, "w", &rp.white);
    css_var(css, "x", &rp.accent[9]);
    css.push_str("--h:transparent;--g:transparent;");
    css_var(css, "t", &rp.neutral[if is_dark { 11 } else { 0 }]);
}

fn minimal_q_css(css: &mut String, _is_dark: bool) {
    css.push_str("--Qd:none;--Qgw:none;");
    css.push_str("--Qb:0;--Qbl:var(--Qb);--Qbt:var(--Qb);--Qbc:transparent;--Qbs:none;");
    css.push_str("--Qol:2px;--Qoo:2px;--Qf:var(--K);");
    css.push_str("--Qbf:none;--Qso:1;--Qgr:none;");
    css.push_str("--Qt:200ms;--Qts:none;");
}

// ============================================================================
// Glass Style
// ============================================================================

fn glass_color_css(css: &mut String, rp: &ResolvedPalette, _is_dark: bool) {
    css_var(css, "v", &rp.accent[8]);
    css_var(css, "w", &rp.white);
    css_var(css, "x", &rp.accent[9]);
}

fn glass_q_css(css: &mut String, is_dark: bool) {
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

// ============================================================================
// Neon Style
// ============================================================================

fn neon_color_css(css: &mut String, rp: &ResolvedPalette, _is_dark: bool) {
    css_var(css, "v", &rp.accent[8]);
    css_var(css, "w", &rp.white);
    css_var(css, "x", &rp.accent[9]);
}

fn neon_q_css(css: &mut String, _is_dark: bool) {
    css.push_str("--Qd:none;--Qgw:0 0 8px var(--n9),0 0 16px var(--n9);");
    css.push_str("--Qb:2px;--Qbl:var(--Qb);--Qbt:var(--Qb);--Qbc:var(--n9);--Qbs:solid;");
    css.push_str("--Qol:2px;--Qoo:2px;--Qf:var(--n9);");
    css.push_str("--Qbf:none;--Qso:1;--Qgr:none;");
    css.push_str("--Qt:50ms;--Qts:0 0 8px currentColor;");
}

#[cfg(test)]
mod tests {
    use super::*;
    use rwire::theme::{generate_theme_css, Theme};

    #[test]
    fn test_solid_style() {
        let theme = Theme::dark().style(solid());
        let css = generate_theme_css(&theme);
        assert!(css.contains("--Qd:var(--Z1)"), "Solid should have shadow Q-var");
        assert!(css.contains("--Qt:150ms"), "Solid transition");
    }

    #[test]
    fn test_brutalist_style() {
        let theme = Theme::light().style(brutalist());
        let css = generate_theme_css(&theme);
        assert!(css.contains("--Qb:2px"), "Brutalist thick borders");
        assert!(css.contains("--Qt:0ms"), "Brutalist no transition");
    }

    #[test]
    fn test_minimal_style() {
        let theme = Theme::dark().style(minimal());
        let css = generate_theme_css(&theme);
        assert!(css.contains("--Qbs:none"), "Minimal no border style");
    }

    #[test]
    fn test_glass_style() {
        let theme = Theme::dark().style(glass());
        let css = generate_theme_css(&theme);
        assert!(css.contains("--Qbf:blur(12px)"), "Glass backdrop blur");
        assert!(css.contains("--Qso:0.75"), "Glass dark opacity");
    }

    #[test]
    fn test_glass_style_light() {
        let theme = Theme::light().style(glass());
        let css = generate_theme_css(&theme);
        assert!(css.contains("--Qso:0.85"), "Glass light opacity");
    }

    #[test]
    fn test_neon_style() {
        let theme = Theme::dark().style(neon());
        let css = generate_theme_css(&theme);
        assert!(css.contains("--Qts:0 0 8px currentColor"), "Neon text shadow");
    }

    #[test]
    fn test_style_enum() {
        let theme = Theme::dark().style(Style::Solid);
        assert_eq!(theme.style, solid());
    }

    #[test]
    fn test_all_styles_produce_valid_css() {
        for style_fn in ALL {
            let style = style_fn();
            let theme = Theme::dark().style(style);
            let css = generate_theme_css(&theme);
            assert!(css.starts_with(":root{"), "Style {} should produce valid CSS", style.id());
            assert!(css.contains("--Qd:"), "Style {} missing Q-var --Qd", style.id());
        }
    }
}
