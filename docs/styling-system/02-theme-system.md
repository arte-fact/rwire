# Phase 2: Theme System

## Goal

Define semantic tokens that map primitives to UI roles, with light/dark theme support.

## rwire Philosophy Alignment

| Principle | How This Phase Aligns |
|-----------|----------------------|
| Zero runtime | Theme selected at connection, no JS theme switching |
| Minimal bandwidth | Single `data-theme` attribute, CSS does the rest |
| Minimal capsule | Both themes share same class names, differ only in `:root` vars |

## Design Decision: Theme Switching

**Option A**: JavaScript theme toggle (runtime cost)
**Option B**: Server-side theme selection (zero runtime) ✓

rwire chooses Option B: theme is determined server-side (user preference, system setting, or explicit choice). The capsule CSS includes both light and dark variable sets; the server sets `data-theme` attribute on root element.

```html
<!-- Server decides theme -->
<div id="rw" data-theme="dark">...</div>
```

## Implementation

### Step 2.1: Semantic Token Types

**File: `rwire/src/theme.rs`**

```rust
//! Theme system for rwire.
//!
//! Themes define semantic tokens that reference primitive values.
//! This provides a layer of abstraction between raw colors and UI usage.

use crate::tokens::primitives::color;

/// Available theme variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
    /// System preference (requires JS, not recommended)
    System,
}

impl ThemeMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
            ThemeMode::System => "system",
        }
    }
}

/// Accent color selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AccentColor {
    #[default]
    Blue,
    Red,
    Green,
    Amber,
}

impl AccentColor {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccentColor::Blue => "blue",
            AccentColor::Red => "red",
            AccentColor::Green => "green",
            AccentColor::Amber => "amber",
        }
    }
}

/// Theme configuration.
#[derive(Clone, Debug)]
pub struct Theme {
    pub mode: ThemeMode,
    pub accent: AccentColor,
    pub radius: RadiusScale,
}

/// Border radius scaling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RadiusScale {
    None,    // Sharp corners
    Small,   // Subtle rounding
    #[default]
    Medium,  // Default rounding
    Large,   // Pronounced rounding
    Full,    // Pill shapes
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            mode: ThemeMode::default(),
            accent: AccentColor::default(),
            radius: RadiusScale::default(),
        }
    }
}

impl Theme {
    pub fn light() -> Self {
        Self::default()
    }

    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            ..Self::default()
        }
    }

    pub fn with_accent(mut self, accent: AccentColor) -> Self {
        self.accent = accent;
        self
    }

    pub fn with_radius(mut self, radius: RadiusScale) -> Self {
        self.radius = radius;
        self
    }
}
```

### Step 2.2: Semantic CSS Generation

**File: `rwire/src/theme.rs` (continued)**

```rust
impl Theme {
    /// Generate semantic CSS variables for this theme.
    /// Returns CSS that defines semantic tokens referencing primitives.
    pub fn generate_semantic_css() -> String {
        let mut css = String::with_capacity(2048);

        // Light theme (default)
        css.push_str(":root, [data-theme=\"light\"] {\n");
        css.push_str("  /* Backgrounds */\n");
        css.push_str("  --rw-bg-app: var(--rw-neutral-1);\n");
        css.push_str("  --rw-bg-subtle: var(--rw-neutral-2);\n");
        css.push_str("  --rw-bg-muted: var(--rw-neutral-3);\n");
        css.push_str("  --rw-bg-emphasis: var(--rw-neutral-4);\n");
        css.push_str("  --rw-bg-hover: var(--rw-neutral-5);\n");
        css.push_str("  --rw-bg-active: var(--rw-neutral-6);\n");
        css.push_str("\n");
        css.push_str("  /* Borders */\n");
        css.push_str("  --rw-border-subtle: var(--rw-neutral-6);\n");
        css.push_str("  --rw-border-default: var(--rw-neutral-7);\n");
        css.push_str("  --rw-border-emphasis: var(--rw-neutral-8);\n");
        css.push_str("\n");
        css.push_str("  /* Text */\n");
        css.push_str("  --rw-text-high: var(--rw-neutral-12);\n");
        css.push_str("  --rw-text-default: var(--rw-neutral-11);\n");
        css.push_str("  --rw-text-muted: var(--rw-neutral-9);\n");
        css.push_str("  --rw-text-on-accent: var(--rw-white);\n");
        css.push_str("\n");
        css.push_str("  /* Accent (default blue, can be overridden) */\n");
        css.push_str("  --rw-accent-1: var(--rw-blue-1);\n");
        css.push_str("  --rw-accent-2: var(--rw-blue-2);\n");
        css.push_str("  --rw-accent-3: var(--rw-blue-3);\n");
        css.push_str("  --rw-accent-4: var(--rw-blue-4);\n");
        css.push_str("  --rw-accent-5: var(--rw-blue-5);\n");
        css.push_str("  --rw-accent-6: var(--rw-blue-6);\n");
        css.push_str("  --rw-accent-7: var(--rw-blue-7);\n");
        css.push_str("  --rw-accent-8: var(--rw-blue-8);\n");
        css.push_str("  --rw-accent-9: var(--rw-blue-9);\n");
        css.push_str("  --rw-accent-10: var(--rw-blue-10);\n");
        css.push_str("  --rw-accent-11: var(--rw-blue-11);\n");
        css.push_str("  --rw-accent-12: var(--rw-blue-12);\n");
        css.push_str("\n");
        css.push_str("  /* Status colors */\n");
        css.push_str("  --rw-success: var(--rw-green-9);\n");
        css.push_str("  --rw-warning: var(--rw-amber-9);\n");
        css.push_str("  --rw-error: var(--rw-red-9);\n");
        css.push_str("}\n\n");

        // Dark theme
        css.push_str("[data-theme=\"dark\"] {\n");
        css.push_str("  /* Backgrounds (inverted) */\n");
        css.push_str("  --rw-bg-app: var(--rw-neutral-12);\n");
        css.push_str("  --rw-bg-subtle: var(--rw-neutral-11);\n");
        css.push_str("  --rw-bg-muted: var(--rw-neutral-10);\n");
        css.push_str("  --rw-bg-emphasis: var(--rw-neutral-9);\n");
        css.push_str("  --rw-bg-hover: var(--rw-neutral-8);\n");
        css.push_str("  --rw-bg-active: var(--rw-neutral-7);\n");
        css.push_str("\n");
        css.push_str("  /* Borders (inverted) */\n");
        css.push_str("  --rw-border-subtle: var(--rw-neutral-7);\n");
        css.push_str("  --rw-border-default: var(--rw-neutral-6);\n");
        css.push_str("  --rw-border-emphasis: var(--rw-neutral-5);\n");
        css.push_str("\n");
        css.push_str("  /* Text (inverted) */\n");
        css.push_str("  --rw-text-high: var(--rw-neutral-1);\n");
        css.push_str("  --rw-text-default: var(--rw-neutral-2);\n");
        css.push_str("  --rw-text-muted: var(--rw-neutral-4);\n");
        css.push_str("}\n");

        css
    }

    /// Generate accent color override CSS.
    /// Only needed if using non-default accent.
    pub fn generate_accent_override(accent: AccentColor) -> Option<String> {
        if accent == AccentColor::Blue {
            return None; // Blue is default, no override needed
        }

        let name = accent.as_str();
        let mut css = String::with_capacity(512);
        css.push_str(&format!("[data-accent=\"{}\"] {{\n", name));
        for i in 1..=12 {
            css.push_str(&format!("  --rw-accent-{}: var(--rw-{}-{});\n", i, name, i));
        }
        css.push_str("}\n");
        Some(css)
    }

    /// Generate radius scale override CSS.
    pub fn generate_radius_override(scale: RadiusScale) -> Option<String> {
        match scale {
            RadiusScale::Medium => None, // Default, no override
            RadiusScale::None => Some(
                "[data-radius=\"none\"] { --rw-radius-component: 0; }\n".to_string()
            ),
            RadiusScale::Small => Some(
                "[data-radius=\"small\"] { --rw-radius-component: var(--rw-radius-sm); }\n".to_string()
            ),
            RadiusScale::Large => Some(
                "[data-radius=\"large\"] { --rw-radius-component: var(--rw-radius-xl); }\n".to_string()
            ),
            RadiusScale::Full => Some(
                "[data-radius=\"full\"] { --rw-radius-component: var(--rw-radius-full); }\n".to_string()
            ),
        }
    }
}
```

### Step 2.3: Theme Application in Capsule

**File: `rwire/src/capsule_gen.rs` (modification)**

```rust
// Add to capsule HTML generation:
impl CapsuleGenerator {
    pub fn generate_theme_root(&self, theme: &Theme) -> String {
        let mut attrs = format!("data-theme=\"{}\"", theme.mode.as_str());

        if theme.accent != AccentColor::Blue {
            attrs.push_str(&format!(" data-accent=\"{}\"", theme.accent.as_str()));
        }

        if theme.radius != RadiusScale::Medium {
            attrs.push_str(&format!(" data-radius=\"{:?}\"", theme.radius).to_lowercase());
        }

        attrs
    }
}
```

### Step 2.4: Base Styles

**File: `rwire/src/theme.rs` (continued)**

```rust
/// Generate base/reset styles.
/// Minimal normalization for consistent cross-browser behavior.
pub fn generate_base_css() -> String {
    r#"*,*::before,*::after{box-sizing:border-box}
body{margin:0;font-family:system-ui,-apple-system,sans-serif;line-height:var(--rw-leading-normal);color:var(--rw-text-default);background:var(--rw-bg-app)}
button,input,select,textarea{font:inherit}
"#.to_string()
}
```

### Step 2.5: Combined Theme CSS Generator

```rust
/// Generate complete theme CSS for capsule.
pub fn generate_full_theme_css(theme: &Theme) -> String {
    use crate::tokens::css::generate_primitive_css;

    let mut css = String::with_capacity(8192);

    // 1. Base reset
    css.push_str(&generate_base_css());

    // 2. Primitive tokens
    css.push_str(&generate_primitive_css());

    // 3. Semantic tokens (light + dark)
    css.push_str(&Theme::generate_semantic_css());

    // 4. Accent override (if needed)
    if let Some(accent_css) = Theme::generate_accent_override(theme.accent) {
        css.push_str(&accent_css);
    }

    // 5. Radius override (if needed)
    if let Some(radius_css) = Theme::generate_radius_override(theme.radius) {
        css.push_str(&radius_css);
    }

    css
}
```

## Deliverables

- [ ] `rwire/src/theme.rs` — Theme struct and CSS generation
- [ ] Update `rwire/src/lib.rs` to export `theme` module
- [ ] Update `capsule_gen.rs` to include theme CSS
- [ ] `rwire/tests/theme.rs` — theme tests

## Size Budget

| Component | Max Size |
|-----------|----------|
| Base CSS | < 200 bytes |
| Semantic CSS (light) | < 1KB |
| Semantic CSS (dark) | < 500 bytes |
| Accent override | < 300 bytes |
| Total theme CSS | < 2KB |

## Theme Usage Example

```rust
use rwire::{Theme, ThemeMode, AccentColor};

// Simple: light theme, blue accent
let theme = Theme::default();

// Custom: dark theme, red accent, large radius
let theme = Theme::dark()
    .with_accent(AccentColor::Red)
    .with_radius(RadiusScale::Large);

// In server setup
Server::new()
    .theme(theme)
    .serve(app);
```

## Next Phase

[Phase 3: Variant System](./03-variant-system.md) — CVA-inspired component variants.
