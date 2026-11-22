### Added: `Chip::count` / `Chip::href` / `ChipGroup`, `Badge::tone` / `::mono` / `BadgeSize::Xs`, search-input density

- `Chip::count(n)` renders a counter after the label. `Chip::href(path)` renders a
  link chip (`<a href>`) whose `aria-pressed` reflects `active`. The chip is now
  fully rounded (radius 999) and an active chip uses the accent tone's solid fill
  with its `on_solid` ink (`Tone::Accent.solid()` / `on_solid()`).
- `ChipGroup::new(label).chips([...])` renders a `role="group"` container with
  `aria-label`.
- `Badge::tone(Tone)` uses `tone.ground()` as background and `tone.ink()` as text,
  plus `tone.border()` in the `Outline` fill. `Badge::mono(bool)` switches to the
  mono font (`St::FontMono`). `BadgeSize::Xs` is the runtime badge: 10.5 px mono,
  5 px radius.
- `Input::search().size(InputSize::Sm)` now follows the /app density: 13 px text,
  8 px radius, `.45rem .7rem` padding; the 44 px touch target still comes from the
  theme's coarse-pointer rule.
