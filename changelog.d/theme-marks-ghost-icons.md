### Added: categorical marks (3:1), ghost icon buttons, square icon buttons

- **Marks on categorical fills:** `ColorPalette::categorical_on_solids([...])`
  sets the color a logotype or an `aria-hidden` decorative glyph takes on each
  categorical fill: `--Ynm`, `St::TextCat1Mark…6`, `Tone::on_solid_mark()`.
  It is kept when it reaches 3:1 (the WCAG threshold for graphics), else the
  nearest that does. **Never for read text**: `TextOnCatN` stays at 4.5:1.
  `palettes::nord()` gives nord6 (`#ECEFF4`) for every category: on `Cat(1)`
  (nord10) that is 3.5:1, the old mcp-agents brand "m".
  `rwire::theme::MIN_MARK_CONTRAST` (3.0) joins `MIN_TEXT_CONTRAST`.
- **Ghost icon buttons:** `Button::icon_only(…).intent(ButtonIntent::Ghost)`
  and the new `ThemeToggle::ghost()` rest on nothing (transparent ground, a
  transparent 1 px stroke: `St::BorderGhost`) in the soft ink (`TextMuted`);
  on hover they take the raised ground, the hairline and the ink
  (`BgSurfaceRaised`, `BorderDefault`, `TextDefault`). A ghost button with
  text is unchanged.
- **Square icon buttons:** `--Qzi` now sets width, min-width and height, and
  the square does not shrink in a flex row (`flex-shrink:0`): a compact
  `ThemeToggle` measured 20 × 32 in a crowded row, it is 32 × 32 (44 × 44
  under a finger).
