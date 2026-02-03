# Phase 1: Token Foundation

## Goal

Define primitive design tokens as Rust constants that compile to CSS custom properties.

## rwire Philosophy Alignment

| Principle | How This Phase Aligns |
|-----------|----------------------|
| Zero runtime | Tokens are `const` — resolved at compile time |
| Minimal bandwidth | Token values never sent over wire, only class names |
| Minimal capsule | CSS variables defined once in `:root`, ~500 bytes |

## Implementation

### Step 1.1: Create Token Module Structure

**File: `rwire/src/tokens/mod.rs`**
```rust
//! Design tokens for rwire styling system.
//!
//! Tokens are organized in three tiers:
//! - `primitives`: Raw values (colors, spacing, etc.)
//! - `semantic`: Context-aware aliases (defined in theme.rs)
//! - `css`: CSS custom property generation

pub mod primitives;
pub mod css;

pub use primitives::*;
```

### Step 1.2: Define Primitive Color Tokens

**File: `rwire/src/tokens/primitives.rs`**

Use Oklch color space for perceptual uniformity and wide gamut support.

```rust
//! Primitive design tokens — raw values without semantic meaning.

/// 12-step color scales following Radix Colors methodology.
/// Each scale provides colors for: backgrounds (1-4), borders (5-8), text (9-12).
pub mod color {
    // Neutral scale (gray)
    pub const NEUTRAL_1: &str = "oklch(0.99 0 0)";
    pub const NEUTRAL_2: &str = "oklch(0.97 0 0)";
    pub const NEUTRAL_3: &str = "oklch(0.94 0 0)";
    pub const NEUTRAL_4: &str = "oklch(0.91 0 0)";
    pub const NEUTRAL_5: &str = "oklch(0.87 0 0)";
    pub const NEUTRAL_6: &str = "oklch(0.83 0 0)";
    pub const NEUTRAL_7: &str = "oklch(0.77 0 0)";
    pub const NEUTRAL_8: &str = "oklch(0.70 0 0)";
    pub const NEUTRAL_9: &str = "oklch(0.55 0 0)";
    pub const NEUTRAL_10: &str = "oklch(0.50 0 0)";
    pub const NEUTRAL_11: &str = "oklch(0.40 0 0)";
    pub const NEUTRAL_12: &str = "oklch(0.25 0 0)";

    // Blue scale (default accent)
    pub const BLUE_1: &str = "oklch(0.99 0.01 250)";
    pub const BLUE_2: &str = "oklch(0.97 0.02 250)";
    pub const BLUE_3: &str = "oklch(0.93 0.04 250)";
    pub const BLUE_4: &str = "oklch(0.88 0.07 250)";
    pub const BLUE_5: &str = "oklch(0.82 0.10 250)";
    pub const BLUE_6: &str = "oklch(0.76 0.12 250)";
    pub const BLUE_7: &str = "oklch(0.68 0.14 250)";
    pub const BLUE_8: &str = "oklch(0.60 0.16 250)";
    pub const BLUE_9: &str = "oklch(0.55 0.18 250)";   // Primary solid
    pub const BLUE_10: &str = "oklch(0.50 0.19 250)";  // Hover
    pub const BLUE_11: &str = "oklch(0.45 0.16 250)";  // Low contrast text
    pub const BLUE_12: &str = "oklch(0.30 0.12 250)";  // High contrast text

    // Red scale (destructive/error)
    pub const RED_1: &str = "oklch(0.99 0.01 25)";
    pub const RED_2: &str = "oklch(0.97 0.02 25)";
    pub const RED_3: &str = "oklch(0.93 0.05 25)";
    pub const RED_4: &str = "oklch(0.88 0.09 25)";
    pub const RED_5: &str = "oklch(0.82 0.12 25)";
    pub const RED_6: &str = "oklch(0.76 0.14 25)";
    pub const RED_7: &str = "oklch(0.68 0.16 25)";
    pub const RED_8: &str = "oklch(0.60 0.18 25)";
    pub const RED_9: &str = "oklch(0.55 0.20 25)";
    pub const RED_10: &str = "oklch(0.50 0.21 25)";
    pub const RED_11: &str = "oklch(0.45 0.18 25)";
    pub const RED_12: &str = "oklch(0.30 0.14 25)";

    // Green scale (success)
    pub const GREEN_1: &str = "oklch(0.99 0.01 145)";
    pub const GREEN_2: &str = "oklch(0.97 0.02 145)";
    pub const GREEN_3: &str = "oklch(0.93 0.05 145)";
    pub const GREEN_4: &str = "oklch(0.88 0.08 145)";
    pub const GREEN_5: &str = "oklch(0.82 0.11 145)";
    pub const GREEN_6: &str = "oklch(0.76 0.13 145)";
    pub const GREEN_7: &str = "oklch(0.68 0.15 145)";
    pub const GREEN_8: &str = "oklch(0.60 0.16 145)";
    pub const GREEN_9: &str = "oklch(0.55 0.17 145)";
    pub const GREEN_10: &str = "oklch(0.50 0.18 145)";
    pub const GREEN_11: &str = "oklch(0.45 0.15 145)";
    pub const GREEN_12: &str = "oklch(0.30 0.12 145)";

    // Amber scale (warning)
    pub const AMBER_1: &str = "oklch(0.99 0.01 85)";
    pub const AMBER_2: &str = "oklch(0.97 0.03 85)";
    pub const AMBER_3: &str = "oklch(0.93 0.06 85)";
    pub const AMBER_4: &str = "oklch(0.88 0.10 85)";
    pub const AMBER_5: &str = "oklch(0.82 0.13 85)";
    pub const AMBER_6: &str = "oklch(0.76 0.15 85)";
    pub const AMBER_7: &str = "oklch(0.70 0.16 85)";
    pub const AMBER_8: &str = "oklch(0.64 0.17 85)";
    pub const AMBER_9: &str = "oklch(0.75 0.18 85)";  // Brighter for visibility
    pub const AMBER_10: &str = "oklch(0.70 0.19 85)";
    pub const AMBER_11: &str = "oklch(0.50 0.14 85)";
    pub const AMBER_12: &str = "oklch(0.35 0.10 85)";

    /// White for contrast text on solid backgrounds
    pub const WHITE: &str = "oklch(1 0 0)";
    /// Black for contrast text on light backgrounds
    pub const BLACK: &str = "oklch(0 0 0)";
}

/// Spacing scale using rem units (base 16px).
/// Follows 4px grid: 4, 8, 12, 16, 20, 24, 32, 40, 48, 64, 80, 96
pub mod space {
    pub const _0: &str = "0";
    pub const _1: &str = "0.25rem";   // 4px
    pub const _2: &str = "0.5rem";    // 8px
    pub const _3: &str = "0.75rem";   // 12px
    pub const _4: &str = "1rem";      // 16px
    pub const _5: &str = "1.25rem";   // 20px
    pub const _6: &str = "1.5rem";    // 24px
    pub const _8: &str = "2rem";      // 32px
    pub const _10: &str = "2.5rem";   // 40px
    pub const _12: &str = "3rem";     // 48px
    pub const _16: &str = "4rem";     // 64px
    pub const _20: &str = "5rem";     // 80px
    pub const _24: &str = "6rem";     // 96px
}

/// Border radius scale.
pub mod radius {
    pub const NONE: &str = "0";
    pub const SM: &str = "0.125rem";  // 2px
    pub const MD: &str = "0.375rem";  // 6px
    pub const LG: &str = "0.5rem";    // 8px
    pub const XL: &str = "0.75rem";   // 12px
    pub const _2XL: &str = "1rem";    // 16px
    pub const FULL: &str = "9999px";
}

/// Font size scale.
pub mod font_size {
    pub const XS: &str = "0.75rem";   // 12px
    pub const SM: &str = "0.875rem";  // 14px
    pub const BASE: &str = "1rem";    // 16px
    pub const LG: &str = "1.125rem";  // 18px
    pub const XL: &str = "1.25rem";   // 20px
    pub const _2XL: &str = "1.5rem";  // 24px
    pub const _3XL: &str = "1.875rem"; // 30px
    pub const _4XL: &str = "2.25rem"; // 36px
}

/// Font weight scale.
pub mod font_weight {
    pub const NORMAL: &str = "400";
    pub const MEDIUM: &str = "500";
    pub const SEMIBOLD: &str = "600";
    pub const BOLD: &str = "700";
}

/// Line height scale.
pub mod line_height {
    pub const TIGHT: &str = "1.25";
    pub const SNUG: &str = "1.375";
    pub const NORMAL: &str = "1.5";
    pub const RELAXED: &str = "1.625";
    pub const LOOSE: &str = "2";
}

/// Shadow definitions.
pub mod shadow {
    pub const SM: &str = "0 1px 2px 0 rgb(0 0 0 / 0.05)";
    pub const MD: &str = "0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)";
    pub const LG: &str = "0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)";
    pub const XL: &str = "0 20px 25px -5px rgb(0 0 0 / 0.1), 0 8px 10px -6px rgb(0 0 0 / 0.1)";
}

/// Transition timing.
pub mod transition {
    pub const FAST: &str = "150ms";
    pub const NORMAL: &str = "200ms";
    pub const SLOW: &str = "300ms";
}
```

### Step 1.3: CSS Variable Generation

**File: `rwire/src/tokens/css.rs`**

```rust
//! CSS custom property generation from tokens.

use super::primitives::{color, space, radius, font_size, font_weight, line_height, shadow};

/// Generate CSS custom properties for primitive tokens.
/// This is included once in the capsule's <style> block.
pub fn generate_primitive_css() -> String {
    let mut css = String::with_capacity(2048);
    css.push_str(":root {\n");

    // Color scales
    write_color_scale(&mut css, "neutral", &[
        color::NEUTRAL_1, color::NEUTRAL_2, color::NEUTRAL_3, color::NEUTRAL_4,
        color::NEUTRAL_5, color::NEUTRAL_6, color::NEUTRAL_7, color::NEUTRAL_8,
        color::NEUTRAL_9, color::NEUTRAL_10, color::NEUTRAL_11, color::NEUTRAL_12,
    ]);
    write_color_scale(&mut css, "blue", &[
        color::BLUE_1, color::BLUE_2, color::BLUE_3, color::BLUE_4,
        color::BLUE_5, color::BLUE_6, color::BLUE_7, color::BLUE_8,
        color::BLUE_9, color::BLUE_10, color::BLUE_11, color::BLUE_12,
    ]);
    write_color_scale(&mut css, "red", &[
        color::RED_1, color::RED_2, color::RED_3, color::RED_4,
        color::RED_5, color::RED_6, color::RED_7, color::RED_8,
        color::RED_9, color::RED_10, color::RED_11, color::RED_12,
    ]);
    write_color_scale(&mut css, "green", &[
        color::GREEN_1, color::GREEN_2, color::GREEN_3, color::GREEN_4,
        color::GREEN_5, color::GREEN_6, color::GREEN_7, color::GREEN_8,
        color::GREEN_9, color::GREEN_10, color::GREEN_11, color::GREEN_12,
    ]);
    write_color_scale(&mut css, "amber", &[
        color::AMBER_1, color::AMBER_2, color::AMBER_3, color::AMBER_4,
        color::AMBER_5, color::AMBER_6, color::AMBER_7, color::AMBER_8,
        color::AMBER_9, color::AMBER_10, color::AMBER_11, color::AMBER_12,
    ]);

    // Special colors
    css.push_str(&format!("  --rw-white: {};\n", color::WHITE));
    css.push_str(&format!("  --rw-black: {};\n", color::BLACK));

    // Spacing
    css.push_str(&format!("  --rw-space-0: {};\n", space::_0));
    css.push_str(&format!("  --rw-space-1: {};\n", space::_1));
    css.push_str(&format!("  --rw-space-2: {};\n", space::_2));
    css.push_str(&format!("  --rw-space-3: {};\n", space::_3));
    css.push_str(&format!("  --rw-space-4: {};\n", space::_4));
    css.push_str(&format!("  --rw-space-5: {};\n", space::_5));
    css.push_str(&format!("  --rw-space-6: {};\n", space::_6));
    css.push_str(&format!("  --rw-space-8: {};\n", space::_8));
    css.push_str(&format!("  --rw-space-10: {};\n", space::_10));
    css.push_str(&format!("  --rw-space-12: {};\n", space::_12));
    css.push_str(&format!("  --rw-space-16: {};\n", space::_16));
    css.push_str(&format!("  --rw-space-20: {};\n", space::_20));
    css.push_str(&format!("  --rw-space-24: {};\n", space::_24));

    // Radius
    css.push_str(&format!("  --rw-radius-none: {};\n", radius::NONE));
    css.push_str(&format!("  --rw-radius-sm: {};\n", radius::SM));
    css.push_str(&format!("  --rw-radius-md: {};\n", radius::MD));
    css.push_str(&format!("  --rw-radius-lg: {};\n", radius::LG));
    css.push_str(&format!("  --rw-radius-xl: {};\n", radius::XL));
    css.push_str(&format!("  --rw-radius-2xl: {};\n", radius::_2XL));
    css.push_str(&format!("  --rw-radius-full: {};\n", radius::FULL));

    // Font sizes
    css.push_str(&format!("  --rw-text-xs: {};\n", font_size::XS));
    css.push_str(&format!("  --rw-text-sm: {};\n", font_size::SM));
    css.push_str(&format!("  --rw-text-base: {};\n", font_size::BASE));
    css.push_str(&format!("  --rw-text-lg: {};\n", font_size::LG));
    css.push_str(&format!("  --rw-text-xl: {};\n", font_size::XL));
    css.push_str(&format!("  --rw-text-2xl: {};\n", font_size::_2XL));
    css.push_str(&format!("  --rw-text-3xl: {};\n", font_size::_3XL));
    css.push_str(&format!("  --rw-text-4xl: {};\n", font_size::_4XL));

    // Font weights
    css.push_str(&format!("  --rw-font-normal: {};\n", font_weight::NORMAL));
    css.push_str(&format!("  --rw-font-medium: {};\n", font_weight::MEDIUM));
    css.push_str(&format!("  --rw-font-semibold: {};\n", font_weight::SEMIBOLD));
    css.push_str(&format!("  --rw-font-bold: {};\n", font_weight::BOLD));

    // Line heights
    css.push_str(&format!("  --rw-leading-tight: {};\n", line_height::TIGHT));
    css.push_str(&format!("  --rw-leading-snug: {};\n", line_height::SNUG));
    css.push_str(&format!("  --rw-leading-normal: {};\n", line_height::NORMAL));
    css.push_str(&format!("  --rw-leading-relaxed: {};\n", line_height::RELAXED));
    css.push_str(&format!("  --rw-leading-loose: {};\n", line_height::LOOSE));

    // Shadows
    css.push_str(&format!("  --rw-shadow-sm: {};\n", shadow::SM));
    css.push_str(&format!("  --rw-shadow-md: {};\n", shadow::MD));
    css.push_str(&format!("  --rw-shadow-lg: {};\n", shadow::LG));
    css.push_str(&format!("  --rw-shadow-xl: {};\n", shadow::XL));

    css.push_str("}\n");
    css
}

fn write_color_scale(css: &mut String, name: &str, values: &[&str; 12]) {
    for (i, value) in values.iter().enumerate() {
        css.push_str(&format!("  --rw-{}-{}: {};\n", name, i + 1, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_primitive_css() {
        let css = generate_primitive_css();
        assert!(css.starts_with(":root {"));
        assert!(css.contains("--rw-neutral-1:"));
        assert!(css.contains("--rw-blue-9:"));
        assert!(css.contains("--rw-space-4:"));
        assert!(css.contains("--rw-radius-md:"));
        assert!(css.ends_with("}\n"));
    }

    #[test]
    fn test_css_size() {
        let css = generate_primitive_css();
        // Should be under 3KB for primitive tokens
        assert!(css.len() < 3072, "CSS too large: {} bytes", css.len());
    }
}
```

### Step 1.4: Integration Test

**File: `rwire/tests/tokens.rs`**

```rust
use rwire::tokens::css::generate_primitive_css;

#[test]
fn test_primitive_css_is_valid() {
    let css = generate_primitive_css();

    // Basic structural validation
    assert!(css.contains(":root"));
    assert!(css.contains("--rw-"));

    // No duplicate properties
    let lines: Vec<&str> = css.lines()
        .filter(|l| l.contains("--rw-"))
        .collect();
    let unique: std::collections::HashSet<_> = lines.iter().collect();
    assert_eq!(lines.len(), unique.len(), "Duplicate CSS properties found");
}
```

## Deliverables

- [ ] `rwire/src/tokens/mod.rs` — module structure
- [ ] `rwire/src/tokens/primitives.rs` — all primitive tokens
- [ ] `rwire/src/tokens/css.rs` — CSS generation
- [ ] `rwire/tests/tokens.rs` — integration tests
- [ ] Update `rwire/src/lib.rs` to export `tokens` module

## Size Budget

| Component | Max Size |
|-----------|----------|
| Primitive CSS | < 3KB |
| After gzip | < 800 bytes |

## Next Phase

[Phase 2: Theme System](./02-theme-system.md) — semantic tokens and theme variants.
