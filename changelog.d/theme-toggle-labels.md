### Added: `ThemeToggle::labels` (translatable accessible name and tooltips)

- `ThemeToggle::labels(ThemeToggleLabels { toggle, light, dark, system })` sets
  the button's `aria-label` and the `title` naming each state. The default is
  English, as before (`Toggle theme`, `Theme: light`, `Theme: dark`,
  `Theme: follows the system`).
