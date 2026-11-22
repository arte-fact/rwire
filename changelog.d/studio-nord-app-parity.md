### Changed: studio + Nord take mcp-agents' old front colors where contrast allows

- **Rule:** with `studio` and `palettes::nord()`, a token takes the old
  front's exact value whenever it reads (4.5:1 for text, 3:1 for a mark) on
  every ground it is used on (app, subtle, surface, raised for a hovered row,
  canvas, sidebar); otherwise the nearest value that does.
- **`ColorPalette::categorical_inks([(light, dark), …])`:** the ink each
  categorical color should take per mode, kept when it reads, else nudged to
  the nearest that does. `nord()` carries the old front's runtime inks
  (claude, vibe, pi) and its wait and permission inks.
- **Tone inks first, then their grounds:** a categorical ink is chosen against
  the plain grounds, then its tinted ground is the strongest tint the ink
  still reads on (it used to be a fixed tint, which pushed dark-mode inks
  toward white).
- **`BorderCat1…6` use the ink** (`--Yni`), like an outlined badge's text,
  instead of the raw color.
- **A style can name the faint ink it wants** (`--j2`): kept if it reads on
  every plain ground, else the nearest that does.
- **studio:** soft ink `--j` is `nord3` / `nord4`; the faint ink aims at the
  old front's; in dark mode the success fill is `nord14` with dark text on it.
- `contrast.rs` has the table "studio + Nord = old front" (exact values) and
  the named gaps with their ratios.
