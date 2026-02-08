---
title: Theme Styles
description: Default, Soft, Brutalist, and Minimal presets
order: 4
---
# Theme Styles

Theme styles control the *feel* of your application -- solid vs subtle buttons, sharp vs rounded corners, heavy vs invisible borders -- without changing individual components. Four presets are available:

```rust
use rwire::theme::{Theme, ThemeStyle};

let theme = Theme::light().with_style(ThemeStyle::Soft);
```

## The Four Presets

### Default

Solid accent fills, medium border radius, subtle shadows. This is the standard look.

```rust
Theme::new() // ThemeStyle::Default is the default
```

### Soft

Tinted backgrounds instead of solid fills. Buttons use light accent tints with darker accent text. Borders are softened.

```rust
Theme::light().with_style(ThemeStyle::Soft)
```

The Soft preset remaps primary and destructive colors to their subtle variants:
- `--rw-primary` becomes `var(--rw-accent-3)` (tinted background)
- `--rw-on-primary` becomes `var(--rw-accent-11)` (accent text)
- Borders shift to `var(--rw-neutral-5)` (lighter)

### Brutalist

Sharp corners, heavy borders, high contrast. Borders use the darkest neutral for maximum definition.

```rust
Theme::light().with_style(ThemeStyle::Brutalist)
```

The Brutalist preset pushes borders to extremes:
- `--rw-border-default` becomes `var(--rw-neutral-12)` (near-black)
- `--rw-border-subtle` becomes `var(--rw-neutral-10)` (still prominent)

### Minimal

Near-invisible borders, generous spacing, relies on typography hierarchy for structure.

```rust
Theme::light().with_style(ThemeStyle::Minimal)
```

The Minimal preset removes visual noise:
- `--rw-border-default` becomes `transparent`
- `--rw-border-subtle` becomes `transparent`

## How Styles Work

Each preset sets a `data-style` attribute on the root element. CSS rules scoped to `[data-style="soft"]`, etc., remap semantic variables without touching component tokens.

```css
[data-style="soft"] {
  --rw-primary: var(--rw-accent-3);
  --rw-on-primary: var(--rw-accent-11);
  --rw-destructive: var(--rw-red-3);
  --rw-on-destructive: var(--rw-red-11);
}
```

Components use the same `St` tokens regardless of the active style. A `Button::primary("Save")` renders correctly across all four presets.

## Combining with Other Theme Options

Styles compose freely with palettes, accent colors, dark mode, and radius scales:

```rust
use rwire::theme::{Theme, ThemeStyle, AccentColor, RadiusScale};
use rwire::tokens::ColorPalette;

let theme = Theme::dark()
    .with_style(ThemeStyle::Soft)
    .with_accent(AccentColor::Green)
    .with_radius(RadiusScale::Large);

let config = CapsuleConfig::new()
    .theme(theme)
    .palette(ColorPalette::nord());
```

## Cycling Styles at Runtime

You can let users switch styles by storing the current style in state and rebuilding the theme:

```rust
#[derive(State, Default)]
#[storage(memory)]
struct AppState {
    style_index: usize,
}

const STYLES: [ThemeStyle; 4] = [
    ThemeStyle::Default,
    ThemeStyle::Soft,
    ThemeStyle::Brutalist,
    ThemeStyle::Minimal,
];

#[handler]
fn cycle_style(state: &mut AppState) {
    state.style_index = (state.style_index + 1) % STYLES.len();
}
```

The server re-renders the root element with the updated `data-style` attribute, and all components instantly reflect the new preset.
