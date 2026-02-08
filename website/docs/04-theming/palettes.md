---
title: Color Palettes
description: Nord palette and semantic color system
order: 2
---
# Color Palettes

rwire uses a 12-step color scale system inspired by Radix Colors. Each scale maps steps to UI roles, and a `ColorPalette` bundles five scales (neutral, accent, red, green, amber) into a complete color system.

```rust
use rwire::tokens::ColorPalette;
use rwire::capsule_gen::CapsuleConfig;

// Use the built-in Nord palette
let config = CapsuleConfig::new()
    .palette(ColorPalette::nord());

// Or use the default Oklch-based palette
let config = CapsuleConfig::new();
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

Style tokens reference semantic CSS variables, not raw colors. These variables adapt automatically based on the active palette and theme mode.

```rust
// Text hierarchy
St::TextHigh       // --rw-text-high (highest contrast)
St::TextDefault    // --rw-text-default (body text)
St::TextMuted      // --rw-text-muted (secondary text)

// Background layers
St::BgApp          // --rw-bg-app (page background)
St::BgSubtle       // --rw-bg-subtle (section background)
St::BgSurface      // --rw-surface (card background)

// Accent colors
St::TextAccent     // --rw-accent-11 (links, labels)
St::BgAccentSubtle // --rw-accent-3 (tinted backgrounds)

// Status colors
// --rw-success (green-9), --rw-warning (amber-9), --rw-error (red-9)
```

## Nord Palette

The built-in Nord preset maps arctic-inspired colors to the 12-step scale:

```rust
let palette = ColorPalette::nord();
```

- **Neutral**: Snow Storm (light) to Polar Night (dark)
- **Accent**: Nord Frost blues (#5E81AC primary)
- **Red**: Aurora Red (#BF616A)
- **Green**: Aurora Green (#A3BE8C)
- **Amber**: Aurora Yellow (#EBCB8B)

## Custom Palettes

Build a palette from scratch using `ColorScale`:

```rust
use rwire::tokens::palette::{ColorPalette, ColorScale};

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
```

You can also use Oklch values for perceptually uniform scales:

```rust
let scale = ColorScale::from_oklch([
    (0.98, 0.0, 0.0), (0.90, 0.0, 0.0), // ...12 steps
]);
```

## CSS Variables Under the Hood

The palette generates CSS custom properties on `:root`:

```css
:root {
  --rw-neutral-1: #ECEFF4;
  --rw-neutral-2: #E5E9F0;
  /* ...through step 12 */
  --rw-blue-1: #F0F4F8;
  --rw-blue-9: #5E81AC;
  /* ...all scales */
}
```

Semantic tokens then reference these primitives: `--rw-bg-app: var(--rw-neutral-1)`. Switching palettes swaps all colors at once without touching component code.
