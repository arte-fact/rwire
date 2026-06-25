---
title: Custom Themes
description: Creating custom palettes and extending the token system
order: 5
---
# Custom Themes

rwire's theming system is configured through `Theme` and `CapsuleConfig`. You can customize the color palette, accent color, border radius, font, and style preset -- all on the `Theme` builder.

```rust
use rwire::capsule_gen::{CapsuleConfig, FontFace};
use rwire::theme::{Theme, RadiusScale, ThemeStyle};

let config = CapsuleConfig::new()
    .theme(
        Theme::dark()
            .accent("#5E81AC")
            .radius(RadiusScale::Large)
            .style(ThemeStyle::soft())
    )
    .font(FontFace::google("Inter", &[400, 600]));
```

## Custom Accent Color

Set a custom accent color from a single CSS color. A full 12-step scale is auto-generated:

```rust
use rwire::theme::Theme;

// From hex
let theme = Theme::dark().accent("#5E81AC");

// From oklch
let theme = Theme::dark().accent("oklch(0.55 0.08 250)");
```

The accent controls primary buttons, links, focus rings, and other interactive elements. The 12-step scale is derived at build time using oklch lightness and chroma ramps.

## Custom Color Palettes

For full control, build a palette by providing 12-step color scales for each role:

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

Then pass it to the `Theme`:

```rust
let config = CapsuleConfig::new()
    .theme(Theme::dark().palette(palette));
```

## Seed-Based Themes

For the simplest approach, set seed colors directly on `Theme`. Each seed auto-generates a complete 12-step scale:

```rust
use rwire::theme::Theme;

let theme = Theme::dark()
    .accent("#5E81AC")    // primary actions
    .neutral("#2E3440")   // backgrounds and text
    .error("#BF616A")     // error states
    .success("#A3BE8C")   // success states
    .warning("#EBCB8B");  // warning states
```

Any omitted role uses the default oklch palette.

## Radius Scale

Control component border radius globally:

```rust
Theme::light().radius(RadiusScale::Full)   // pill shapes
Theme::light().radius(RadiusScale::Large)   // pronounced rounding
Theme::light().radius(RadiusScale::Medium)  // default
Theme::light().radius(RadiusScale::Small)   // subtle rounding
Theme::light().radius(RadiusScale::None)    // sharp corners
```

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
        background: linear-gradient(135deg, var(--n3), var(--n5));
    }
"#;
```

Reference short semantic CSS variables (`--a` through `--L`, `--n1`..`--n12`) or primitive variables (`--S*` spacing, `--R*` radius, `--T*` text size, `--Z*` shadow) in custom CSS.

## How It All Fits Together

The capsule CSS is assembled in layers:

1. **Base reset** -- Box-sizing, body margins, system font
2. **Non-color primitives** -- Spacing, radius, typography, shadows (tree-shaken)
3. **Color primitives** -- Only palette colors still referenced by tokens (tree-shaken)
4. **Resolved semantics** -- Light and dark mappings with direct oklch values
5. **Radius override** -- Adjusts component radius (if non-default)
6. **Style preset** -- Remaps semantic variables for Soft/Brutalist/Minimal
7. **Component CSS** -- Tree-shaken token classes

All color values are resolved at build time from the palette. There is no runtime accent switching -- the server knows the full theme configuration and emits final CSS values. Only light/dark mode switching remains at runtime via the `data-theme` attribute.

The total theme CSS stays under 4KB.
