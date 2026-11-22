### Added: `Theme::density` (`--Qz*` hooks) and 44 px touch targets

- **`Theme::density(Density::Compact | Comfortable)`** sets the `--Qz*` hooks:
  control height `--Qzh` (`St::HControl`), control text `--Qzt`
  (`St::TextControl`), tab padding `--Qzy` (`St::PyTab`), chip padding
  `--Qzcx` / `--Qzcy` (`St::PxChip` / `St::PyChip`, inline and block apart so a
  caller's `.st([St::PxMd])` still replaces the inline one), list-row padding `--Qzp` (`St::PadRow`). `Comfortable` (the
  default) emits nothing: the tokens' fallbacks are today's sizes. `Compact`:
  32 px controls, tighter tabs, chips and rows.
- **Icon buttons:** `--Qzi` (square), `--Qzg` (glyph), `--Qzq` (padding).
  `Compact` makes `Button::icon_only` and `ThemeToggle` (default sizes) 32 ×
  32 px with an 18 px glyph (`St::SqIconButton`, `St::GlyphIconButton`,
  `St::SqIconToggle`, `St::PxIconToggle` / `St::PyIconToggle`, `St::GlyphIconToggle`); their
  radius is the theme's `--R2` (8 px with `studio`).
- **Readers:** `Button`, `Input` and `Select` at their default size (height and
  text), `Button::icon_only` and `ThemeToggle` (square and glyph), `Chip`
  (padding), `Tabs` (text, padding). Rendered sizes are unchanged under the
  default density.
- **Behaviour change: 44 px touch targets.** Under `(pointer: coarse)` or below
  the `sm` breakpoint, buttons, selects, text inputs, `role=tab` and
  `role=button` get `min-height: 44px` (buttons and tabs also `min-width`),
  whatever the density. The rule is in the theme CSS (its threshold is
  `Theme::touch_below`, see that entry) with zero specificity
  (`:where`), so an app's own `min-height` still wins. Checkboxes, radios and
  sliders keep their size.
