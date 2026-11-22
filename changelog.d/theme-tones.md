### Added: categorical colors, `Tone`, the neutral tone pair and the faint ink

- **`ColorPalette::categorical([...])`**: up to 6 colors with no meaning of state
  (a runtime, an author). Each becomes `--Y1…--Y6` (the color), `--Yng` (a
  tinted ground), `--Yni` (its ink: the hue darkened in light mode or lightened
  in dark mode until it reads at AA on that ground and on the app, subtle,
  surface and raised grounds) and `--Yno` (the text on the solid fill), per
  mode. A palette that names none gets six from its own scales; one with fewer
  than six repeats them. `palettes::nord()` provides Frost and Aurora (blue,
  purple, teal, orange, yellow, cyan).
- **`rwire::Tone { Neutral, Accent, Info, Success, Warning, Danger, Cat(u8) }`**
  maps a color role to its tokens: `ground()`, `ink()`, `solid()`,
  `on_solid()`, `border()`. Components take it in a later lot (R2).
- **New tokens:** `St::BgCat1…6`, `BgCat1Subtle…6`, `TextCat1…6`,
  `TextOnCat1…6`, `BorderCat1…6`; `St::BgNeutralSubtle` / `St::TextNeutral`
  (`--Y0g` / `--Y0i`, the gray tone pair); `St::TextFaint` (`--j2`, the faint ink:
  the palest neutral step still at AA on the plain grounds, below
  `TextDefault` and `TextMuted`).
- `contrast.rs` checks every tone ink on its ground and on the plain grounds,
  the text on each solid fill, and the faint ink: 68 pairs over every built-in
  palette, style and mode. For studio + Nord, also the soft ink (`TextMuted`)
  on every ground and the text on the sidebar.
- **Breaking:** `ColorPalette` gains a public `categorical` field: a struct
  literal needs it (`categorical: Vec::new()`).
