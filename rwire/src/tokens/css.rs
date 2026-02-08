//! CSS custom property generation from tokens.
//!
//! Generates the `:root` CSS block containing all primitive tokens
//! as CSS custom properties. This is included once in the capsule's
//! `<style>` block.
//!
//! # Output Format
//!
//! ```css
//! :root {
//!   --rw-neutral-1: oklch(0.985 0 0);
//!   --rw-blue-9: oklch(0.55 0.18 250);
//!   --rw-space-4: 1rem;
//!   /* ... */
//! }
//! ```

use super::palette::ColorPalette;
use super::primitives::{color, font_size, font_weight, line_height, radius, shadow, space};
use std::collections::HashSet;

/// Generate CSS custom properties for all primitive tokens.
///
/// This is included once in the capsule's `<style>` block.
/// Returns CSS defining `:root` variables with `--rw-*` prefix.
pub fn generate_primitive_css() -> String {
    let mut css = String::with_capacity(4096);
    css.push_str(":root{\n");

    // Color scales
    write_color_scale(&mut css, "neutral", &NEUTRAL_SCALE);
    write_color_scale(&mut css, "blue", &BLUE_SCALE);
    write_color_scale(&mut css, "red", &RED_SCALE);
    write_color_scale(&mut css, "green", &GREEN_SCALE);
    write_color_scale(&mut css, "amber", &AMBER_SCALE);

    // Special colors
    write_var(&mut css, "white", color::WHITE);
    write_var(&mut css, "black", color::BLACK);

    // Spacing
    write_var(&mut css, "space-0", space::_0);
    write_var(&mut css, "space-1", space::_1);
    write_var(&mut css, "space-2", space::_2);
    write_var(&mut css, "space-3", space::_3);
    write_var(&mut css, "space-4", space::_4);
    write_var(&mut css, "space-5", space::_5);
    write_var(&mut css, "space-6", space::_6);
    write_var(&mut css, "space-8", space::_8);
    write_var(&mut css, "space-10", space::_10);
    write_var(&mut css, "space-12", space::_12);
    write_var(&mut css, "space-16", space::_16);
    write_var(&mut css, "space-20", space::_20);
    write_var(&mut css, "space-24", space::_24);

    // Radius
    write_var(&mut css, "radius-none", radius::NONE);
    write_var(&mut css, "radius-sm", radius::SM);
    write_var(&mut css, "radius-md", radius::MD);
    write_var(&mut css, "radius-lg", radius::LG);
    write_var(&mut css, "radius-xl", radius::XL);
    write_var(&mut css, "radius-2xl", radius::_2XL);
    write_var(&mut css, "radius-full", radius::FULL);

    // Font sizes
    write_var(&mut css, "text-xs", font_size::XS);
    write_var(&mut css, "text-sm", font_size::SM);
    write_var(&mut css, "text-base", font_size::BASE);
    write_var(&mut css, "text-lg", font_size::LG);
    write_var(&mut css, "text-xl", font_size::XL);
    write_var(&mut css, "text-2xl", font_size::_2XL);
    write_var(&mut css, "text-3xl", font_size::_3XL);
    write_var(&mut css, "text-4xl", font_size::_4XL);
    write_var(&mut css, "text-5xl", font_size::_5XL);
    write_var(&mut css, "text-6xl", font_size::_6XL);

    // Font weights
    write_var(&mut css, "font-normal", font_weight::NORMAL);
    write_var(&mut css, "font-medium", font_weight::MEDIUM);
    write_var(&mut css, "font-semibold", font_weight::SEMIBOLD);
    write_var(&mut css, "font-bold", font_weight::BOLD);

    // Line heights
    write_var(&mut css, "leading-tight", line_height::TIGHT);
    write_var(&mut css, "leading-snug", line_height::SNUG);
    write_var(&mut css, "leading-normal", line_height::NORMAL);
    write_var(&mut css, "leading-relaxed", line_height::RELAXED);
    write_var(&mut css, "leading-loose", line_height::LOOSE);

    // Shadows
    write_var(&mut css, "shadow-sm", shadow::SM);
    write_var(&mut css, "shadow-md", shadow::MD);
    write_var(&mut css, "shadow-lg", shadow::LG);
    write_var(&mut css, "shadow-xl", shadow::XL);

    css.push_str("}\n");
    css
}

/// Generate CSS custom properties using a custom color palette.
///
/// Uses the provided palette for all color scales while keeping
/// other primitive tokens (spacing, radius, etc.) from the defaults.
pub fn generate_primitive_css_with_palette(palette: &ColorPalette) -> String {
    let mut css = String::with_capacity(4096);
    css.push_str(":root{\n");

    // Use palette colors instead of hardcoded scales
    css.push_str(&palette.to_css());

    // Special colors (keep defaults)
    write_var(&mut css, "white", color::WHITE);
    write_var(&mut css, "black", color::BLACK);

    // Spacing
    write_var(&mut css, "space-0", space::_0);
    write_var(&mut css, "space-1", space::_1);
    write_var(&mut css, "space-2", space::_2);
    write_var(&mut css, "space-3", space::_3);
    write_var(&mut css, "space-4", space::_4);
    write_var(&mut css, "space-5", space::_5);
    write_var(&mut css, "space-6", space::_6);
    write_var(&mut css, "space-8", space::_8);
    write_var(&mut css, "space-10", space::_10);
    write_var(&mut css, "space-12", space::_12);
    write_var(&mut css, "space-16", space::_16);
    write_var(&mut css, "space-20", space::_20);
    write_var(&mut css, "space-24", space::_24);

    // Radius
    write_var(&mut css, "radius-none", radius::NONE);
    write_var(&mut css, "radius-sm", radius::SM);
    write_var(&mut css, "radius-md", radius::MD);
    write_var(&mut css, "radius-lg", radius::LG);
    write_var(&mut css, "radius-xl", radius::XL);
    write_var(&mut css, "radius-2xl", radius::_2XL);
    write_var(&mut css, "radius-full", radius::FULL);

    // Font sizes
    write_var(&mut css, "text-xs", font_size::XS);
    write_var(&mut css, "text-sm", font_size::SM);
    write_var(&mut css, "text-base", font_size::BASE);
    write_var(&mut css, "text-lg", font_size::LG);
    write_var(&mut css, "text-xl", font_size::XL);
    write_var(&mut css, "text-2xl", font_size::_2XL);
    write_var(&mut css, "text-3xl", font_size::_3XL);
    write_var(&mut css, "text-4xl", font_size::_4XL);
    write_var(&mut css, "text-5xl", font_size::_5XL);
    write_var(&mut css, "text-6xl", font_size::_6XL);

    // Font weights
    write_var(&mut css, "font-normal", font_weight::NORMAL);
    write_var(&mut css, "font-medium", font_weight::MEDIUM);
    write_var(&mut css, "font-semibold", font_weight::SEMIBOLD);
    write_var(&mut css, "font-bold", font_weight::BOLD);

    // Line heights
    write_var(&mut css, "leading-tight", line_height::TIGHT);
    write_var(&mut css, "leading-snug", line_height::SNUG);
    write_var(&mut css, "leading-normal", line_height::NORMAL);
    write_var(&mut css, "leading-relaxed", line_height::RELAXED);
    write_var(&mut css, "leading-loose", line_height::LOOSE);

    // Shadows
    write_var(&mut css, "shadow-sm", shadow::SM);
    write_var(&mut css, "shadow-md", shadow::MD);
    write_var(&mut css, "shadow-lg", shadow::LG);
    write_var(&mut css, "shadow-xl", shadow::XL);

    css.push_str("}\n");
    css
}

/// Generate CSS custom properties for only the primitive tokens that are actually used.
///
/// Takes a set of variable names (with `--rw-` prefix) and only generates CSS
/// for those variables. This reduces CSS size by tree-shaking unused tokens.
pub fn generate_primitive_css_filtered(used_vars: &HashSet<String>) -> String {
    let mut css = String::with_capacity(2048);
    css.push_str(":root{\n");

    // Helper to conditionally write a variable if it's used
    let write_if_used = |css: &mut String, name: &str, value: &str| {
        let var_name = format!("--rw-{}", name);
        if used_vars.contains(&var_name) {
            write_var(css, name, value);
        }
    };

    // Color scales
    for (i, color) in NEUTRAL_SCALE.iter().enumerate() {
        write_if_used(&mut css, &format!("neutral-{}", i + 1), color);
    }
    for (i, color) in BLUE_SCALE.iter().enumerate() {
        write_if_used(&mut css, &format!("blue-{}", i + 1), color);
    }
    for (i, color) in RED_SCALE.iter().enumerate() {
        write_if_used(&mut css, &format!("red-{}", i + 1), color);
    }
    for (i, color) in GREEN_SCALE.iter().enumerate() {
        write_if_used(&mut css, &format!("green-{}", i + 1), color);
    }
    for (i, color) in AMBER_SCALE.iter().enumerate() {
        write_if_used(&mut css, &format!("amber-{}", i + 1), color);
    }

    // Special colors
    write_if_used(&mut css, "white", color::WHITE);
    write_if_used(&mut css, "black", color::BLACK);

    // Spacing
    write_if_used(&mut css, "space-0", space::_0);
    write_if_used(&mut css, "space-1", space::_1);
    write_if_used(&mut css, "space-2", space::_2);
    write_if_used(&mut css, "space-3", space::_3);
    write_if_used(&mut css, "space-4", space::_4);
    write_if_used(&mut css, "space-5", space::_5);
    write_if_used(&mut css, "space-6", space::_6);
    write_if_used(&mut css, "space-8", space::_8);
    write_if_used(&mut css, "space-10", space::_10);
    write_if_used(&mut css, "space-12", space::_12);
    write_if_used(&mut css, "space-16", space::_16);
    write_if_used(&mut css, "space-20", space::_20);
    write_if_used(&mut css, "space-24", space::_24);

    // Radius
    write_if_used(&mut css, "radius-none", radius::NONE);
    write_if_used(&mut css, "radius-sm", radius::SM);
    write_if_used(&mut css, "radius-md", radius::MD);
    write_if_used(&mut css, "radius-lg", radius::LG);
    write_if_used(&mut css, "radius-xl", radius::XL);
    write_if_used(&mut css, "radius-2xl", radius::_2XL);
    write_if_used(&mut css, "radius-full", radius::FULL);

    // Font sizes
    write_if_used(&mut css, "text-xs", font_size::XS);
    write_if_used(&mut css, "text-sm", font_size::SM);
    write_if_used(&mut css, "text-base", font_size::BASE);
    write_if_used(&mut css, "text-lg", font_size::LG);
    write_if_used(&mut css, "text-xl", font_size::XL);
    write_if_used(&mut css, "text-2xl", font_size::_2XL);
    write_if_used(&mut css, "text-3xl", font_size::_3XL);
    write_if_used(&mut css, "text-4xl", font_size::_4XL);
    write_if_used(&mut css, "text-5xl", font_size::_5XL);
    write_if_used(&mut css, "text-6xl", font_size::_6XL);

    // Font weights
    write_if_used(&mut css, "font-normal", font_weight::NORMAL);
    write_if_used(&mut css, "font-medium", font_weight::MEDIUM);
    write_if_used(&mut css, "font-semibold", font_weight::SEMIBOLD);
    write_if_used(&mut css, "font-bold", font_weight::BOLD);

    // Line heights
    write_if_used(&mut css, "leading-tight", line_height::TIGHT);
    write_if_used(&mut css, "leading-snug", line_height::SNUG);
    write_if_used(&mut css, "leading-normal", line_height::NORMAL);
    write_if_used(&mut css, "leading-relaxed", line_height::RELAXED);
    write_if_used(&mut css, "leading-loose", line_height::LOOSE);

    // Shadows
    write_if_used(&mut css, "shadow-sm", shadow::SM);
    write_if_used(&mut css, "shadow-md", shadow::MD);
    write_if_used(&mut css, "shadow-lg", shadow::LG);
    write_if_used(&mut css, "shadow-xl", shadow::XL);

    css.push_str("}\n");
    css
}

// Color scale arrays for iteration
const NEUTRAL_SCALE: [&str; 12] = [
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

const BLUE_SCALE: [&str; 12] = [
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

const RED_SCALE: [&str; 12] = [
    color::RED_1,
    color::RED_2,
    color::RED_3,
    color::RED_4,
    color::RED_5,
    color::RED_6,
    color::RED_7,
    color::RED_8,
    color::RED_9,
    color::RED_10,
    color::RED_11,
    color::RED_12,
];

const GREEN_SCALE: [&str; 12] = [
    color::GREEN_1,
    color::GREEN_2,
    color::GREEN_3,
    color::GREEN_4,
    color::GREEN_5,
    color::GREEN_6,
    color::GREEN_7,
    color::GREEN_8,
    color::GREEN_9,
    color::GREEN_10,
    color::GREEN_11,
    color::GREEN_12,
];

const AMBER_SCALE: [&str; 12] = [
    color::AMBER_1,
    color::AMBER_2,
    color::AMBER_3,
    color::AMBER_4,
    color::AMBER_5,
    color::AMBER_6,
    color::AMBER_7,
    color::AMBER_8,
    color::AMBER_9,
    color::AMBER_10,
    color::AMBER_11,
    color::AMBER_12,
];

/// Write a color scale as CSS variables.
fn write_color_scale(css: &mut String, name: &str, values: &[&str; 12]) {
    for (i, value) in values.iter().enumerate() {
        css.push_str("--rw-");
        css.push_str(name);
        css.push('-');
        // Steps are 1-indexed
        let step = i + 1;
        if step >= 10 {
            css.push_str(&step.to_string());
        } else {
            css.push((b'0' + step as u8) as char);
        }
        css.push(':');
        css.push_str(value);
        css.push_str(";\n");
    }
}

/// Write a single CSS variable.
fn write_var(css: &mut String, name: &str, value: &str) {
    css.push_str("--rw-");
    css.push_str(name);
    css.push(':');
    css.push_str(value);
    css.push_str(";\n");
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
    fn test_generate_primitive_css_structure() {
        let css = generate_primitive_css();

        // Should start with :root
        assert!(css.starts_with(":root{"));
        assert!(css.ends_with("}\n"));

        // Should contain all color scales
        assert!(css.contains("--rw-neutral-1:"));
        assert!(css.contains("--rw-neutral-12:"));
        assert!(css.contains("--rw-blue-9:"));
        assert!(css.contains("--rw-red-9:"));
        assert!(css.contains("--rw-green-9:"));
        assert!(css.contains("--rw-amber-9:"));

        // Should contain spacing
        assert!(css.contains("--rw-space-4:"));

        // Should contain radius
        assert!(css.contains("--rw-radius-md:"));

        // Should contain typography
        assert!(css.contains("--rw-text-base:"));
        assert!(css.contains("--rw-font-medium:"));
        assert!(css.contains("--rw-leading-normal:"));

        // Should contain shadows
        assert!(css.contains("--rw-shadow-sm:"));
    }

    #[test]
    fn test_css_size_budget() {
        let css = generate_primitive_css();

        // Primitive CSS should be under 4KB
        assert!(
            css.len() < 4096,
            "Primitive CSS exceeds budget: {} bytes (max 4096)",
            css.len()
        );

        // Log actual size for tracking
        println!("Primitive CSS size: {} bytes", css.len());
    }

    #[test]
    fn test_minify_css() {
        let input = ":root {\n  --rw-space-4: 1rem;\n  --rw-blue-9: oklch(0.55 0.18 250);\n}\n";
        let output = minify_css(input);

        assert!(!output.contains('\n'));
        assert!(output.contains("--rw-space-4:1rem"));
        assert!(output.contains("--rw-blue-9:oklch(0.55 0.18 250)"));
    }

    #[test]
    fn test_minified_size() {
        let css = generate_primitive_css();
        let minified = minify_css(&css);

        // Minified should be smaller
        assert!(minified.len() < css.len());

        // Minified should still be valid (starts/ends correctly)
        assert!(minified.starts_with(":root{"));
        assert!(minified.ends_with("}"));

        println!(
            "Minified CSS size: {} bytes (saved {} bytes)",
            minified.len(),
            css.len() - minified.len()
        );
    }

    #[test]
    fn test_no_duplicate_variables() {
        let css = generate_primitive_css();

        let vars: Vec<&str> = css
            .lines()
            .filter(|line| line.contains("--rw-"))
            .map(|line| {
                // Extract variable name
                line.trim()
                    .split(':')
                    .next()
                    .unwrap_or("")
                    .trim_start_matches("--rw-")
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
