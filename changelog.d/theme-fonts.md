### Added: theme font stacks (`Theme::fonts`, `St::FontRead`) and self-served web fonts (`CapsuleConfig::font_face`)

- **`Theme::fonts(Fonts { ui, read, mono })`** sets three `font-family` stacks:
  `ui` → `--Qft` (the `body`, the whole interface), `read` → `--Qfr` (new: long-form
  reading), `mono` → `--Qm` (`St::FontMono`, code). Builder form:
  `Fonts::new().ui(…).read(…).mono(…)`. A stack left unset keeps the style's
  default; a stack set wins over the style's own (`flat`'s monospace UI).
- **`St::FontRead`** (`font-family:var(--Qfr,inherit)`): the reading face, or the
  body face when the theme names none.
- **`CapsuleConfig::font_face(family, weights, path)`** declares an `@font-face`
  over a woff2 file the app serves itself (`asset()` / `asset_dir()`):
  `font_face("Inter", 100..=900, "/fonts/inter.woff2")`. One weight or a
  variable font's range; `font-display:swap`; the `base_path` mount is
  prefixed. A URL (`https://…`, `//…`) panics at startup: no third-party
  request.
