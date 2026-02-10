---
title: Color Palettes
description: Nord palette, auto-generated scales, and semantic color system
order: 2
---
# Color Palettes

rwire uses a 12-step color scale system inspired by Radix Colors. Each scale maps steps to UI roles, and a `ColorPalette` bundles five scales (neutral, accent, red, green, amber) into a complete color system.

## Quick Setup

```rust
use rwire::capsule_gen::CapsuleConfig;
use rwire::theme::Theme;

// Default Oklch palette
let config = CapsuleConfig::new();

// Custom accent from a single color (auto-generates 12-step scale)
let config = CapsuleConfig::new()
    .theme(Theme::dark().accent("#5E81AC"));

// Nord preset
let config = CapsuleConfig::dark_nord();
```

## The 12-Step Scale

Each color scale follows a consistent purpose mapping:

| Steps | Role | Example Tokens |
|-------|------|----------------|
| 1-2 | Backgrounds | `BgApp`, `BgSubtle` |
| 3-4 | Muted/emphasis | `BgMuted`, `BgEmphasis` |
| 5-6 | Interactive | `BgHover`, `BgActive` |
| 7-8 | Borders | `BorderSubtle`, `BorderDefault` |
| 9-10 | Solid fills | Primary buttons, accent |
| 11-12 | Text | `TextDefault`, `TextHigh` |

## Semantic Color Tokens

Style tokens reference semantic CSS variables that are resolved at build time from the active palette. Components never need conditional color logic.

```rust
// Text hierarchy
St::TextHigh       // highest contrast text
St::TextDefault    // body text
St::TextMuted      // secondary text

// Background layers
St::BgApp          // page background
St::BgSubtle       // section background
St::BgSurface      // card background

// Accent colors
St::TextAccent     // links, labels (accent step 11)
St::BgAccentSubtle // tinted backgrounds (accent step 3)

// Status colors
// St::BgSuccess (green-9), St::BgWarning (amber-9), St::BgError (red-9)
```

## Single-Color Scales

Generate a full 12-step scale from one seed color using `ColorScale::from_color()`. The seed becomes step 9 (primary solid), and steps 1-8 and 10-12 are derived automatically using oklch lightness and chroma ramps.

```rust
use rwire::tokens::palette::ColorScale;

// From hex
let scale = ColorScale::from_color("#5E81AC");

// From oklch
let scale = ColorScale::from_color("oklch(0.55 0.08 250)");
```

Supported formats: `#RRGGBB`, `#RGB`, and `oklch(L C H)`.

## Nord Palette

The built-in Nord preset maps arctic-inspired colors to the 12-step scale:

```rust
use rwire::tokens::ColorPalette;
let palette = ColorPalette::nord();
```

- **Neutral**: Snow Storm (light) to Polar Night (dark)
- **Accent**: Nord Frost blues (#5E81AC primary)
- **Red**: Aurora Red (#BF616A)
- **Green**: Aurora Green (#A3BE8C)
- **Amber**: Aurora Yellow (#EBCB8B)

## Custom Palettes

For full control, build a palette from explicit 12-step hex scales:

```rust
use rwire::tokens::palette::{ColorPalette, ColorScale};
use rwire::theme::Theme;

let palette = ColorPalette::new()
    .with_neutral(ColorScale::from_hex([
        "#FAFAFA", "#F5F5F5", "#E5E5E5", "#D4D4D4",
        "#A3A3A3", "#737373", "#525252", "#404040",
        "#262626", "#1C1C1C", "#141414", "#0A0A0A",
    ]))
    .with_accent(ColorScale::from_hex([
        "#EFF6FF", "#DBEAFE", "#BFDBFE", "#93C5FD",
        "#60A5FA", "#3B82F6", "#2563EB", "#1D4ED8",
        "#1E40AF", "#1E3A8A", "#172554", "#0F172A",
    ]));

let theme = Theme::dark().palette(palette);
```

Or from oklch tuples for perceptually uniform scales:

```rust
let scale = ColorScale::from_oklch([
    (0.98, 0.0, 0.0), (0.90, 0.0, 0.0), // ...12 steps
]);
```

## Seed-Based Themes

For the simplest possible custom theme, set seed colors directly on `Theme`. Each seed auto-generates a full 12-step scale:

```rust
use rwire::theme::Theme;

let theme = Theme::dark()
    .accent("#5E81AC")    // primary actions
    .neutral("#2E3440");  // backgrounds/text
```

Optional seeds: `.error("#BF616A")`, `.success("#A3BE8C")`, `.warning("#EBCB8B")`. Any omitted role uses the default oklch palette.

## Build-Time Resolution

Colors are resolved to final values during capsule generation. The output CSS contains direct oklch values instead of `var()` indirection:

```css
:root,[data-theme="light"]{
  --a:oklch(0.985 0 0);    /* bg-app */
  --b:oklch(0.97 0 0);     /* bg-subtle */
  --k:oklch(0.25 0.02 250); /* text-default */
  --n9:oklch(0.55 0.08 250); /* accent step 9 */
}
[data-theme="dark"]{
  --a:oklch(0.15 0 0);     /* bg-app (inverted) */
  --k:oklch(0.97 0 0);     /* text-default (inverted) */
}
```

Short variable names (`--a` through `--L`, `--n1`..`--n12`) minimize CSS size. The Rust `St` enum variant names (`St::BgApp`, `St::TextDefault`) serve as human-readable documentation.

Switching palettes changes all colors at once without touching component code.
