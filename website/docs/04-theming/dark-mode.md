---
title: Dark Mode
description: Implementing light and dark themes
order: 3
---
# Dark Mode

rwire ships both light and dark CSS in every capsule. Switching themes is a matter of changing the `data-theme` attribute on the root element -- no page reload, no server round-trip.

```rust
use rwire::theme::{Theme, ThemeMode};

// Start with dark mode
let theme = Theme::dark();

// Start with light mode (the default)
let theme = Theme::light();
```

## How It Works

The capsule CSS includes two blocks of semantic variables with resolved color values:

```css
:root,[data-theme="light"]{
  --a:oklch(0.985 0 0);   /* bg-app */
  --k:oklch(0.25 0 0);    /* text-default */
  --j:oklch(0.55 0 0);    /* text-muted */
  /* ...light mappings */
}

[data-theme="dark"]{
  --a:oklch(0.15 0 0);    /* bg-app (inverted) */
  --k:oklch(0.97 0 0);    /* text-default (inverted) */
  --j:oklch(0.55 0 0);    /* text-muted (inverted) */
  /* ...dark mappings */
}
```

When `data-theme` changes, all semantic tokens (`BgApp`, `TextDefault`, `TextMuted`, etc.) automatically resolve to the correct value. Components never need conditional color logic.

## ThemeToggle Component

The built-in `ThemeToggle` component renders a button that switches between light and dark modes:

```rust
use rwire::components::{ThemeToggle, ThemeToggleMode};

// In your header or navigation
ThemeToggle::new()
    .mode(ThemeToggleMode::Light)  // current mode (controls icon)
    .on_toggle(toggle_theme())
    .build()
```

The toggle shows a moon icon in light mode and a sun icon in dark mode.

## Managing Theme State

Connect the toggle to your application state:

```rust
use rwire::{State, handler, renderer};
use rwire::theme::ThemeMode;
use rwire::components::{ThemeToggle, ThemeToggleMode};

#[derive(State, Default)]
#[storage(memory)]
struct AppState {
    dark_mode: bool,
}

#[handler]
fn toggle_theme(state: &mut AppState) {
    state.dark_mode = !state.dark_mode;
}

#[renderer]
fn render_toggle(state: &AppState) -> ElementBuilder {
    let mode = if state.dark_mode {
        ThemeToggleMode::Dark
    } else {
        ThemeToggleMode::Light
    };

    ThemeToggle::new()
        .mode(mode)
        .on_toggle(toggle_theme())
        .build()
}
```

The server updates the `data-theme` attribute on the root element when the handler fires, and CSS variables do the rest.

## No Conditional Code

Because semantic tokens reference CSS variables, components are theme-agnostic:

```rust
// This card looks correct in both light and dark mode
el(El::Div).st([
    St::BgSurface,        // white in light, dark gray in dark
    St::TextDefault,      // near-black in light, near-white in dark
    St::BorderDefault,    // adapts to theme
    St::RoundedMd,
    St::PxLg,
    St::PyMd,
])
```

There is no `if dark_mode { ... }` anywhere. The semantic layer handles the mapping, and the palette step inversion ensures consistent contrast ratios across both themes.
