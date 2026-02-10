---
title: Theme Styles
description: Default, Soft, Brutalist, and Minimal presets
order: 4
---
# Theme Styles

Theme styles control the *feel* of your application -- solid vs subtle buttons, sharp vs rounded corners, heavy vs invisible borders -- without changing individual components. Four presets are available:

```rust
use rwire::theme::{Theme, ThemeStyle};

let theme = Theme::light().style(ThemeStyle::Soft);
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
Theme::light().style(ThemeStyle::Soft)
```

The Soft preset remaps primary and destructive colors to their subtle variants:
- Primary background becomes a light accent tint (accent step 3)
- Primary text becomes the darker accent (accent step 11)
- Borders shift to a lighter neutral

### Brutalist

Sharp corners, heavy borders, high contrast. Borders use the darkest neutral for maximum definition.

```rust
Theme::light().style(ThemeStyle::Brutalist)
```

The Brutalist preset pushes borders to extremes:
- Default borders become near-black (neutral step 12)
- Subtle borders become prominent (neutral step 10)

### Minimal

Near-invisible borders, generous spacing, relies on typography hierarchy for structure.

```rust
Theme::light().style(ThemeStyle::Minimal)
```

The Minimal preset removes visual noise:
- Default borders become transparent
- Subtle borders become transparent

## How Styles Work

Each preset sets a `data-style` attribute on the root element. CSS rules scoped to `[data-style="soft"]`, etc., remap semantic variables with resolved color values.

```css
[data-style="soft"]{
  --v:oklch(0.93 0.03 250);  /* primary = accent tint */
  --w:oklch(0.25 0.06 250);  /* on-primary = accent text */
  --h:oklch(0.88 0 0);       /* border-default = softer */
}
```

Components use the same `St` tokens regardless of the active style. A `Button::primary("Save")` renders correctly across all four presets.

## Combining with Other Theme Options

Styles compose freely with palettes, dark mode, and radius scales:

```rust
use rwire::theme::{Theme, ThemeStyle, RadiusScale};
use rwire::capsule_gen::CapsuleConfig;

let config = CapsuleConfig::new()
    .theme(
        Theme::dark()
            .style(ThemeStyle::Soft)
            .radius(RadiusScale::Large)
            .accent("#5E81AC")
    );
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
