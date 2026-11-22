### Added: design-system gallery examples for disabled states with a reason

- The `Composer` page gains a `disabled` toggle: the composer turns read-only
  and shows why (`Composer::disabled(reason)`), and the code snippet follows.
- The `Button` page gains a `title` text field: the native tooltip, which
  still shows when the button is disabled (`Button::title`).
- The playground's variant and toggle rows wrap, so the Button page no longer
  scrolls sideways at 375 px.
