---
title: Custom Themes
description: Creating custom palettes and extending the token system
order: 5
---
# Custom Themes

rwire's theming system is configured through `CapsuleConfig`. You can customize the color palette, accent color, border radius, font, and style preset -- all in one place.

```rust
use rwire::capsule_gen::{CapsuleConfig, FontFace};
use rwire::theme::{Theme, AccentColor, RadiusScale, ThemeStyle};
use rwire::tokens::ColorPalette;

let config = CapsuleConfig::new()
    .theme(
        Theme::dark()
            .with_accent(AccentColor::Green)
            .with_radius(RadiusScale::Large)
            .with_style(ThemeStyle::Soft)
    )
    .palette(ColorPalette::nord())
    .font(FontFace::google("Inter", &[400, 600]));
```

## Custom Color Palettes

Build a palette by providing 12-step color scales for each role:

```rust
use rwire::tokens::palette::{ColorPalette, ColorScale};

let palette = ColorPalette::new()
    .with_neutral(ColorScale::from_hex([
        "#FAFAF9", "#F5F5F4", "#E7E5E4", "#D6D3D1",
        "#A8A29E", "#78716C", "#57534E", "#44403C",
        "#292524", "#1C1917", "#141210", "#0C0A09",
    ]))
    .with_accent(ColorScale::from_hex([
        "#F0FDF4", "#DCFCE7", "#BBF7D0", "#86EFAC",
        "#4ADE80", "#22C55E", "#16A34A", "#15803D",
        "#166534", "#14532D", "#0F3D22", "#052E16",
    ]))
    .with_red(ColorScale::from_hex([
        "#FEF2F2", "#FEE2E2", "#FECACA", "#FCA5A5",
        "#F87171", "#EF4444", "#DC2626", "#B91C1C",
        "#991B1B", "#7F1D1D", "#6B1414", "#450A0A",
    ]));
// .with_green() and .with_amber() follow the same pattern
```

The palette generates CSS variables (`--rw-neutral-1` through `--rw-neutral-12`, etc.) that semantic tokens reference.

## Accent Colors

Four accent colors are built in: Blue (default), Red, Green, and Amber. The accent controls primary buttons, links, focus rings, and other interactive elements.

```rust
Theme::light().with_accent(AccentColor::Green)
```

This sets `data-accent="green"` on the root element, remapping `--rw-accent-1` through `--rw-accent-12` to the green scale.

## Radius Scale

Control component border radius globally:

```rust
Theme::light().with_radius(RadiusScale::Full)   // pill shapes
Theme::light().with_radius(RadiusScale::Large)   // pronounced rounding
Theme::light().with_radius(RadiusScale::Medium)  // default
Theme::light().with_radius(RadiusScale::Small)   // subtle rounding
Theme::light().with_radius(RadiusScale::None)    // sharp corners
```

The radius scale overrides `--rw-radius-component`, which all components reference.

## Custom Fonts

Load Google Fonts or self-hosted fonts:

```rust
// Google Fonts with specific weights
let config = CapsuleConfig::new()
    .font(FontFace::google("Inter", &[400, 500, 600, 700]));

// Self-hosted font
let config = CapsuleConfig::new()
    .font(FontFace::custom("MyFont")
        .src("/fonts/myfont.woff2", "woff2")
        .weight(400));
```

Google Fonts generate an `@import` tag in the capsule HTML. The base CSS sets `font-family: system-ui, ...` by default, so adding a custom font requires updating the body font via additional CSS.

## Adding Custom CSS

For styles beyond what tokens cover, pass extra CSS alongside the capsule config:

```rust
let extra_css = r#"
    .custom-gradient {
        background: linear-gradient(135deg, var(--rw-accent-3), var(--rw-accent-5));
    }
"#;
```

Reference semantic CSS variables (`--rw-accent-*`, `--rw-bg-*`, `--rw-text-*`) in custom CSS to ensure it adapts to palette and theme changes.

## How It All Fits Together

The capsule CSS is assembled in layers:

1. **Base reset** -- Box-sizing, body margins, system font
2. **Primitive tokens** -- Raw color scales from the palette
3. **Semantic tokens** -- Light and dark mappings
4. **Accent override** -- Remaps accent scale (if non-default)
5. **Radius override** -- Adjusts component radius (if non-default)
6. **Style preset** -- Remaps semantic variables (if non-default)
7. **Component CSS** -- Tree-shaken token classes

Each layer is optional and only included when needed. The total theme CSS stays under 8KB.
