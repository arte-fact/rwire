### Added: `styles::studio()` and the canvas surface (`--Qw`, `St::BgCanvas`)

- **`rwire_themes::styles::studio()`** (also `Style::Studio`, in `styles::ALL`): the
  look of apps for working with an AI. Stepped surfaces (a canvas under the
  reading area, lighter cards on it, a raised level), 1 px hairlines, one soft
  shadow for floating elements (`--Qd`, and `--Z2` / `St::ShadowMd`), radii of
  8, 10 and 14 px (`--R2`, `--R3`, `--R4`).
  - With `palettes::nord()`: light ground `nord4`, cards lighter than `nord6`,
    sidebar `nord5`, raised `nord6`; dark ground `nord0`, cards `nord1`, raised
    `nord2`, hairlines `nord3`. Primary: `nord10` one step darker in light
    mode (white text on it reads at AA), `nord8` in dark mode.
- **Canvas:** `--Qw`, the sunken ground of a reading area (a conversation, a
  document), read by the new `St::BgCanvas`. The theme sets it to the app
  background (`var(--a)`); a `ThemeStyle` remaps it. Cards on it stay
  `BgSurface`.
- `contrast.rs` covers the studio style (with every palette) and text on
  `BgCanvas`, and checks studio + Nord in both modes, primary button included.
- **Breaking:** `Style` gains a `Studio` variant (an exhaustive `match` needs
  an arm).
