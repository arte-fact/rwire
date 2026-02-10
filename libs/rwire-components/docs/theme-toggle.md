---
title: ThemeToggle
description: Button for switching between light and dark themes
order: 605
component: theme-toggle
---

## Import

```rust
use rwire_components::{ThemeToggle, ThemeToggleMode, ToggleSize};
```

## Usage

```rust
ThemeToggle::new()
    .on_toggle(toggle_theme())
    .build()
```

## With Current Mode

```rust
// Display correct icon based on current theme
ThemeToggle::new()
    .mode(ThemeToggleMode::Dark)  // shows moon icon
    .on_toggle(toggle_theme())
    .build()

ThemeToggle::new()
    .mode(ThemeToggleMode::Light) // shows sun icon
    .on_toggle(toggle_theme())
    .build()
```

## Sizes

```rust
ThemeToggle::new().size(ToggleSize::Sm).build()
ThemeToggle::new().size(ToggleSize::Md).build() // default
ThemeToggle::new().size(ToggleSize::Lg).build()
```

## Integration

```rust
#[handler]
fn toggle_theme(theme: &mut Theme) {
    theme.mode = theme.mode.toggle();
}
```

## Accessibility

- Renders a `<button>` with sun/moon icon
- Icon changes based on the current mode
- Keyboard accessible via standard button interaction
