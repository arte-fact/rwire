---
title: Theme Styles
description: Soft, Solid, Brutalist, Minimal, Glass, and Neon presets
order: 4
---
# Theme Styles

Theme styles control the *feel* of your application -- solid vs subtle buttons, sharp vs rounded
corners, heavy vs invisible borders -- without changing individual components.

`ThemeStyle` is a struct (a small bundle of CSS-remapping callbacks). The core crate ships one
built-in, `ThemeStyle::soft()`, which is also the **default**. Additional presets live in the
`rwire-themes` crate:

```rust
use rwire::theme::{Theme, ThemeStyle};
use rwire_themes::styles;

let theme = Theme::light().style(ThemeStyle::soft());   // built-in default
let theme = Theme::light().style(styles::brutalist());  // from rwire-themes
```

## The Presets

| Preset | Where | Feel |
|--------|-------|------|
| `ThemeStyle::soft()` | core (default) | Tinted backgrounds instead of solid fills; softened borders |
| `styles::solid()` | rwire-themes | Solid accent fills, medium radius, subtle shadows (the classic look) |
| `styles::brutalist()` | rwire-themes | Sharp corners, heavy near-black borders, high contrast |
| `styles::minimal()` | rwire-themes | Near-invisible borders, relies on typography for structure |
| `styles::glass()` | rwire-themes | Translucent, blurred surfaces |
| `styles::neon()` | rwire-themes | Saturated accents and glow |

`styles::ALL` is a slice of all `rwire-themes` preset constructors, handy for building a switcher.

The **Soft** preset, for example, remaps primary and destructive colors to subtle variants
(primary background → a light accent tint, primary text → the darker accent, borders → a lighter
neutral); **Brutalist** pushes borders toward the darkest neutral; **Minimal** makes them
transparent.

## How Styles Work

A style does **not** set a `data-style` attribute or rely on `[data-style="…"]` selectors.
Instead, its callbacks write overrides *inline* into the single `:root{}` block when the theme
CSS is generated:

```css
:root{
  --v:oklch(0.93 0.03 250);  /* primary = accent tint (Soft) */
  --w:oklch(0.25 0.06 250);  /* on-primary = accent text */
  --h:oklch(0.88 0 0);       /* border-default = softer */
}
```

Components use the same `St` tokens regardless of the active style. A `Button::primary("Save")`
renders correctly under every preset.

## Combining with Other Theme Options

Styles compose freely with palettes, dark mode, and radius scales:

```rust
use rwire::theme::{Theme, RadiusScale};
use rwire_themes::{styles, palettes};

Theme::dark()
    .style(styles::brutalist())
    .radius(RadiusScale::Large)
    .palette(palettes::nord())
```

## Switching Styles at Runtime

`Theme` is a state type, so a handler takes `&mut Theme` and sets `theme.style` directly -- no
style index in your own app state, no `data-style` attribute. The built-in theme renderer
re-emits the `:root{}` variables and the client patches them.

```rust
use rwire::{handler};
use rwire::theme::{Theme, ThemeStyle};
use rwire_themes::styles;

#[handler]
fn cycle_theme_style(theme: &mut Theme) {
    // soft (built-in) + every rwire-themes preset
    let mut all = vec![ThemeStyle::soft()];
    all.extend(styles::ALL.iter().map(|f| f()));
    let idx = all.iter().position(|s| *s == theme.style).unwrap_or(0);
    theme.style = all[(idx + 1) % all.len()];
}
```

You can also set one explicitly with `theme.set_style(styles::glass())`.
