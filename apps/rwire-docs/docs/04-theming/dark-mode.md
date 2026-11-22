---
title: Dark Mode
description: Implementing light and dark themes
order: 3
---
# Dark Mode

`Theme` is a framework-provided state type. The capsule ships one `:root{}` block of CSS
variables for the *current* mode; switching modes is a handler that mutates `&mut Theme`, and a
built-in renderer re-emits the `:root{}` block, which the synced `<style>` element patches over
the wire. It's a server round-trip, but a tiny one — only the variable block changes, and every
component re-colors automatically because they reference the semantic variables.

```rust
use rwire::theme::{Theme, ThemeMode};

// Start in dark mode
let theme = Theme::dark();

// Start in light mode (the default)
let theme = Theme::light();
```

Pass the theme to the server with `.theme(...)` (typically from a `#[theme]` function).

## Following the System

`ThemeMode::System` follows the operating system's setting (`prefers-color-scheme`). The theme
CSS then carries both modes, light on `:root` and dark under a media query, so the page switches
live when the OS does, with no round trip:

```rust
let theme = Theme::new().mode(ThemeMode::System);
```

```css
:root{color-scheme:light;--a:…;--k:…}
@media (prefers-color-scheme:dark){:root{color-scheme:dark;--a:…;--k:…}}
```

`Light` and `Dark` force a mode (only their block is emitted). A three-state toggle cycles with
`theme.mode.cycle()` (system → light → dark → system) and shows the current state with
`ThemeToggle::new().mode(theme.mode.into())`.

**The user's pick survives a reload.** When a handler changes the mode during a visit, the
runtime keeps the new mode in the `rw_mode` cookie, and the server starts the next page (its first
paint included) and connection in that mode. The mode a page starts in is never written, so the
app's own default stays the app's to change.

## How It Works

The capsule's `<style>` contains a single `:root{}` block with the resolved color values for the
current mode:

```css
:root{
  --a:oklch(0.985 0 0);   /* bg-app */
  --k:oklch(0.25 0 0);    /* text-default */
  --l:oklch(0.55 0 0);    /* text-muted */
  /* ...the rest of the semantic variables */
}
```

There are **no** `[data-theme]` selectors. When a handler flips `theme.mode`, the framework
re-renders this block with the dark values and patches the `<style>` element via the synced
element system. All semantic tokens (`BgApp`, `TextDefault`, `TextMuted`, …) then resolve to the
new values automatically — components never need conditional color logic.

## ThemeToggle Component

The built-in `ThemeToggle` renders a button that switches between light and dark modes:

```rust
use rwire_components::{ThemeToggle, ThemeToggleMode};

ThemeToggle::new()
    .mode(ThemeToggleMode::Light)  // current mode (controls the icon)
    .on_toggle(toggle_theme())
    .build()
```

The toggle shows a moon icon in light mode and a sun icon in dark mode.

## Wiring It Up

Because `Theme` is itself a state type, the handler takes `&mut Theme` and the renderer reads
`&Theme` directly — you do **not** track a `dark_mode` flag in your own app state:

```rust
use rwire::{handler, renderer, ElementBuilder};
use rwire::theme::{Theme, ThemeMode};
use rwire_components::{ThemeToggle, ThemeToggleMode};

#[handler]
fn toggle_theme(theme: &mut Theme) {
    theme.mode = theme.mode.toggle();
}

#[renderer]
fn render_toggle(theme: &Theme) -> ElementBuilder {
    ThemeToggle::new()
        .mode(theme.mode.into())
        .on_toggle(toggle_theme())
        .build()
}
```

When the handler fires, the built-in theme renderer re-emits the `:root{}` variables and the
client patches them — no `data-theme` attribute, no page reload.

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

There is no `if dark_mode { ... }` anywhere. The semantic layer handles the mapping, and the
palette step inversion keeps contrast ratios consistent across both modes.
