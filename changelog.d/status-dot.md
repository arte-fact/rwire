### Added: status dots at 3:1 (`Tone::dot`), `StatusDot::ring`, `size`, `tone` and `Hollow`

- **Dot colors, one per tone:** `--Y0d` (neutral), `--Yad` (accent), `--Yid`
  (info), `--Ysd` (success), `--Ywd` (warning), `--Yxd` (danger), `--Y1d…--Y6d`
  (categorical), read through `Tone::dot()` (`St::TextDotNeutral…`,
  `St::TextDotCat1…6`, as `color`). Each reaches 3:1, the WCAG threshold for
  graphics, on every ground a dot sits on: app, subtle, surface, raised (a
  hovered row), the selected row and the sidebar. It starts from the tone's
  own color (a style may name its neutral dot in `--Y0d`) and takes the nearest
  that passes when that does not.
- **`StatusDot`:** `.tone(Tone)` (any tone, a categorical one included),
  `.ring(bool)` (a 3 px halo at 22 %, `St::DotRing`),
  `.size(StatusDotSize::Sm | Md)` (9 px or 12 px, `St::SizeDotSm` /
  `SizeDotMd`), and `StatusDotIntent::Hollow` (a 1.5 px neutral circle,
  `St::DotHollow`). The dot is drawn with `currentColor`; a label takes the
  tone's readable ink. A dot without a label is `aria-hidden`.
- **Reduced motion:** `St::AnimatePulse` stops under `prefers-reduced-motion`.
- **Behaviour change:** the default dot is 9 px (it was `0.5rem`) and its
  colors are the dot colors (they were the raw scale steps, some under 3:1 on
  light grounds); the label's color is the tone's ink.
- `contrast.rs`: every dot at 3:1 on every dot ground in every built-in theme;
  studio + Nord dots equal to the old mcp-agents front where its value passes.
