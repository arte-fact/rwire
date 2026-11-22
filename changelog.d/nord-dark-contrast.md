### Fixed: unreadable "subtle" pairs (dark Nord and most built-in themes)

- **Many pairs fell below 4.5:1.** In dark Nord, `TextWarning` on
  `BgWarningSubtle` was orange on salmon (1.17:1), and light text on
  `BgAccentSubtle` (a selected row) was 1.15:1. A WCAG check over every
  built-in palette, style and mode found 151 pairs below 4.5:1.
- **The cause: fixed scale steps.** Hand-made palettes do not put the same
  lightness at the same step. In dark mode, step 9 is a saturated mid-tone,
  not a dark tint. The status text colours were the fill colours, whatever the
  ground.
- **The theme now computes the subtle grounds and their text**
  (`subtle_pairs`):
  - **Grounds.** In dark mode, the hue laid over the app background (a dark
    tint). In light mode, the palette's pale steps, unchanged.
  - **Texts.** The first of the palette's own steps that reaches 4.5:1. Failing
    that, the brand step darkened (lightened in dark mode) until it does, so
    the hue is kept. The neutral ends are only a last resort; no built-in theme
    needs them.
  - **Status texts.** `TextSuccess` / `TextWarning` / `TextError` read new
    variables (`--o3` / `--p3` / `--q3`), chosen against their subtle
    background and the plain grounds they also land on: app, subtle, surface
    and raised surface (`--a` / `--b` / `--r` / `--t`), as the style finally
    sets them. `Bg*` fills and borders keep `--o` / `--p` / `--q`.
  - **Accent-subtle ground.** `BgAccentSubtle` reads a new mode-aware
    `--n0`, chosen so that the ordinary text reads on it. It no longer reads
    the fixed accent step 3.
- **Test.** `rwire-themes/tests/contrast.rs` checks 22 text/ground pairs over
  11 palettes, 8 styles and both modes against 4.5:1.
