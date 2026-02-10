//! CSS custom property generation from tokens.
//!
//! Generates the `:root` CSS block containing primitive tokens
//! as CSS custom properties. This is included once in the capsule's
//! `<style>` block.

use super::primitives::{color, font_size, font_weight, line_height, radius, shadow, space};
use std::collections::HashSet;

/// Single source of truth for all non-color primitive CSS variables.
///
/// Each entry is `(name, value)` where the emitted variable is `--{name}:{value}`.
/// Short names: S=spacing, R=radius, T=text size, W=weight, X=leading, Z=shadow.
pub(crate) const NONCOLOR_PRIMITIVES: &[(&str, &str)] = &[
    // Spacing (--S{n})
    ("S0", space::_0),
    ("S1", space::_1),
    ("S2", space::_2),
    ("S3", space::_3),
    ("S4", space::_4),
    ("S5", space::_5),
    ("S6", space::_6),
    ("S8", space::_8),
    ("S10", space::_10),
    ("S12", space::_12),
    ("S16", space::_16),
    ("S20", space::_20),
    ("S24", space::_24),
    // Radius (--R{n}, numbered 0-6)
    ("R0", radius::NONE),
    ("R1", radius::SM),
    ("R2", radius::MD),
    ("R3", radius::LG),
    ("R4", radius::XL),
    ("R5", radius::_2XL),
    ("R6", radius::FULL),
    // Font sizes (--T{n}, numbered 1-10)
    ("T1", font_size::XS),
    ("T2", font_size::SM),
    ("T3", font_size::BASE),
    ("T4", font_size::LG),
    ("T5", font_size::XL),
    ("T6", font_size::_2XL),
    ("T7", font_size::_3XL),
    ("T8", font_size::_4XL),
    ("T9", font_size::_5XL),
    ("T10", font_size::_6XL),
    // Font weights (--W{n}, CSS weight ÷ 100)
    ("W4", font_weight::NORMAL),
    ("W5", font_weight::MEDIUM),
    ("W6", font_weight::SEMIBOLD),
    ("W7", font_weight::BOLD),
    // Line heights (--X{n}, numbered 1-5)
    ("X1", line_height::TIGHT),
    ("X2", line_height::SNUG),
    ("X3", line_height::NORMAL),
    ("X4", line_height::RELAXED),
    ("X5", line_height::LOOSE),
    // Shadows (--Z{n}, numbered 1-4)
    ("Z1", shadow::SM),
    ("Z2", shadow::MD),
    ("Z3", shadow::LG),
    ("Z4", shadow::XL),
];

// Color scale arrays for iteration
const NEUTRAL_SCALE: [&str; 12] = [
    color::NEUTRAL_1,  color::NEUTRAL_2,  color::NEUTRAL_3,  color::NEUTRAL_4,
    color::NEUTRAL_5,  color::NEUTRAL_6,  color::NEUTRAL_7,  color::NEUTRAL_8,
    color::NEUTRAL_9,  color::NEUTRAL_10, color::NEUTRAL_11, color::NEUTRAL_12,
];

const BLUE_SCALE: [&str; 12] = [
    color::BLUE_1,  color::BLUE_2,  color::BLUE_3,  color::BLUE_4,
    color::BLUE_5,  color::BLUE_6,  color::BLUE_7,  color::BLUE_8,
    color::BLUE_9,  color::BLUE_10, color::BLUE_11, color::BLUE_12,
];

const RED_SCALE: [&str; 12] = [
    color::RED_1,  color::RED_2,  color::RED_3,  color::RED_4,
    color::RED_5,  color::RED_6,  color::RED_7,  color::RED_8,
    color::RED_9,  color::RED_10, color::RED_11, color::RED_12,
];

const GREEN_SCALE: [&str; 12] = [
    color::GREEN_1,  color::GREEN_2,  color::GREEN_3,  color::GREEN_4,
    color::GREEN_5,  color::GREEN_6,  color::GREEN_7,  color::GREEN_8,
    color::GREEN_9,  color::GREEN_10, color::GREEN_11, color::GREEN_12,
];

const AMBER_SCALE: [&str; 12] = [
    color::AMBER_1,  color::AMBER_2,  color::AMBER_3,  color::AMBER_4,
    color::AMBER_5,  color::AMBER_6,  color::AMBER_7,  color::AMBER_8,
    color::AMBER_9,  color::AMBER_10, color::AMBER_11, color::AMBER_12,
];

/// Write a single CSS variable.
fn write_var(css: &mut String, name: &str, value: &str) {
    css.push_str("--");
    css.push_str(name);
    css.push(':');
    css.push_str(value);
    css.push_str(";\n");
}

/// Generate CSS custom properties for only non-color primitive tokens.
///
/// Emits spacing, radius, typography, and shadow tokens. Color variables
/// are no longer emitted — they are resolved at build time via `ResolvedPalette`.
pub fn generate_noncolor_primitive_css() -> String {
    let mut css = String::with_capacity(1024);
    css.push_str(":root{\n");

    for &(name, value) in NONCOLOR_PRIMITIVES {
        write_var(&mut css, name, value);
    }

    css.push_str("}\n");
    css
}

/// Generate CSS custom properties for only the non-color primitives that are actually used.
///
/// Takes a set of variable names (with `--` prefix) and only generates CSS
/// for those variables. This tree-shakes spacing, radius, typography, and shadow
/// tokens so small apps don't pay for unused vars.
pub(crate) fn generate_primitive_css_filtered(used_vars: &HashSet<String>) -> String {
    if used_vars.is_empty() {
        return String::new();
    }

    let mut css = String::with_capacity(512);

    for &(name, value) in NONCOLOR_PRIMITIVES {
        let var_name = format!("--{}", name);
        if used_vars.contains(&var_name) {
            write_var(&mut css, name, value);
        }
    }

    if css.is_empty() {
        return String::new();
    }

    let mut result = String::with_capacity(css.len() + 10);
    result.push_str(":root{\n");
    result.push_str(&css);
    result.push_str("}\n");
    result
}

/// Generate CSS custom properties for only the color tokens that are actually used.
///
/// Takes a set of variable names (with `--` prefix) and only generates CSS
/// for those variables. This is used to tree-shake color primitives that are
/// directly referenced by St tokens (e.g., `--P4` for green-4, `--O9` for red-9).
///
/// Short names: N=neutral, U=blue, O=red, P=green, M=amber, Y=special (Yw/Yb).
pub(crate) fn generate_color_css_filtered(used_vars: &HashSet<String>) -> String {
    if used_vars.is_empty() {
        return String::new();
    }

    let mut css = String::with_capacity(512);

    let write_if_used = |css: &mut String, name: &str, value: &str| {
        let var_name = format!("--{}", name);
        if used_vars.contains(&var_name) {
            write_var(css, name, value);
        }
    };

    // Color scales (short prefix + 1-based index)
    for (i, c) in NEUTRAL_SCALE.iter().enumerate() {
        write_if_used(&mut css, &format!("N{}", i + 1), c);
    }
    for (i, c) in BLUE_SCALE.iter().enumerate() {
        write_if_used(&mut css, &format!("U{}", i + 1), c);
    }
    for (i, c) in RED_SCALE.iter().enumerate() {
        write_if_used(&mut css, &format!("O{}", i + 1), c);
    }
    for (i, c) in GREEN_SCALE.iter().enumerate() {
        write_if_used(&mut css, &format!("P{}", i + 1), c);
    }
    for (i, c) in AMBER_SCALE.iter().enumerate() {
        write_if_used(&mut css, &format!("M{}", i + 1), c);
    }

    // Special colors
    write_if_used(&mut css, "Yw", color::WHITE);
    write_if_used(&mut css, "Yb", color::BLACK);

    if css.is_empty() {
        return String::new();
    }

    let mut result = String::with_capacity(css.len() + 10);
    result.push_str(":root{\n");
    result.push_str(&css);
    result.push_str("}\n");
    result
}

/// Minify CSS by removing unnecessary whitespace.
///
/// This is a simple implementation suitable for the small token CSS.
/// For larger CSS, consider a dedicated minifier.
pub fn minify_css(css: &str) -> String {
    let mut result = String::with_capacity(css.len());
    let mut prev_char = ' ';

    for c in css.chars() {
        match c {
            '\n' | '\r' | '\t' => {
                // Skip newlines and tabs
            }
            ' ' => {
                // Skip spaces after certain characters
                if !matches!(prev_char, '{' | '}' | ':' | ';' | ',' | ' ') {
                    result.push(c);
                }
            }
            _ => {
                // Remove space before certain characters
                if matches!(c, '{' | '}' | ':' | ';' | ',') && prev_char == ' ' {
                    result.pop();
                }
                result.push(c);
            }
        }
        if c != '\n' && c != '\r' && c != '\t' {
            prev_char = c;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_noncolor_primitive_css_structure() {
        let css = generate_noncolor_primitive_css();

        // Should start with :root
        assert!(css.starts_with(":root{"));
        assert!(css.ends_with("}\n"));

        // Should NOT contain color scales
        assert!(!css.contains("--N1:"), "Should not emit color vars");
        assert!(!css.contains("--U9:"), "Should not emit color vars");

        // Should contain spacing (short names)
        assert!(css.contains("--S4:"));

        // Should contain radius (short names)
        assert!(css.contains("--R2:"));

        // Should contain typography (short names)
        assert!(css.contains("--T3:"));
        assert!(css.contains("--W5:"));
        assert!(css.contains("--X3:"));

        // Should contain shadows (short names)
        assert!(css.contains("--Z1:"));
    }

    #[test]
    fn test_noncolor_css_size_budget() {
        let css = generate_noncolor_primitive_css();

        // Non-color CSS should be much smaller than old full primitive CSS
        assert!(
            css.len() < 2048,
            "Non-color CSS exceeds budget: {} bytes (max 2048)",
            css.len()
        );
    }

    #[test]
    fn test_noncolor_primitives_table_count() {
        // Verify we have the expected number of entries
        assert_eq!(NONCOLOR_PRIMITIVES.len(), 43);
    }

    #[test]
    fn test_color_css_filtered() {
        let mut used = HashSet::new();
        used.insert("--P4".to_string());
        used.insert("--O9".to_string());

        let css = generate_color_css_filtered(&used);

        // Should contain only the requested colors (short names)
        assert!(css.contains("--P4:"));
        assert!(css.contains("--O9:"));
        // Should NOT contain others
        assert!(!css.contains("--U1:"));
        assert!(!css.contains("--N1:"));
    }

    #[test]
    fn test_color_css_filtered_empty() {
        let used = HashSet::new();
        let css = generate_color_css_filtered(&used);

        // Should return empty string when no colors are used
        assert_eq!(css, "");
    }

    #[test]
    fn test_minify_css() {
        let input = ":root {\n  --S4: 1rem;\n  --U9: oklch(0.55 0.18 250);\n}\n";
        let output = minify_css(input);

        assert!(!output.contains('\n'));
        assert!(output.contains("--S4:1rem"));
        assert!(output.contains("--U9:oklch(0.55 0.18 250)"));
    }

    #[test]
    fn test_minified_size() {
        let css = generate_noncolor_primitive_css();
        let minified = minify_css(&css);

        // Minified should be smaller
        assert!(minified.len() < css.len());

        // Minified should still be valid (starts/ends correctly)
        assert!(minified.starts_with(":root{"));
        assert!(minified.ends_with("}"));
    }

    #[test]
    fn test_no_duplicate_variables() {
        let css = generate_noncolor_primitive_css();

        let vars: Vec<&str> = css
            .lines()
            .filter(|line| line.starts_with("--"))
            .map(|line| {
                line.trim()
                    .split(':')
                    .next()
                    .unwrap_or("")
            })
            .collect();

        let unique: std::collections::HashSet<_> = vars.iter().collect();
        assert_eq!(
            vars.len(),
            unique.len(),
            "Duplicate CSS variables detected"
        );
    }
}
