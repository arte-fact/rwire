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
- The small search field and the extra-small badge use tokens, not an inline
  style, so an app can override them with `.st()`: `St::TextSearchSm` (13 px),
  `St::PxSearchSm` / `St::PySearchSm` (.7rem / .45rem), `St::TextBadgeXs`
  (10.5 px), `St::RoundedBadgeXs` (5 px). The search keeps `RoundedMd`
  (`--R2`, 8 px with `studio`).
- **A link chip** (`Chip::href`) is client-routed (`data-route`, like
  `NavRow`) and, when active, marked `aria-current="page"`; it never carries
  `aria-pressed` (not allowed on a link).
- **A button chip is a toggle only when `active` is set:** `aria-pressed`
  appears with `.active(true)` or `.active(false)`, not on a chip that never
  says (behaviour change).
- **A counted chip** names itself with a separator: `aria-label="Failed, 3"`.
- **The small search matches the old mcp-agents front:** its height comes from
  its padding (13 px × 1.5 + .45rem × 2 + the border: about 36 px; no fixed
  height), on the surface (`BgSurface`), in the ordinary ink (`TextDefault`);
  44 px under a finger still.
