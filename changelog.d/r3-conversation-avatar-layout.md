### Added: conversation entry layout (R3, iteration 1) — `ChatLayout::Avatar`, bubbles, reading prose

- `ChatTranscript::layout(ChatLayout::Avatar)`: a grid with a square avatar
  (30 px, 26 px on a phone, radius 8) beside a header (author name, tag,
  `El::Time`) and the body. Consecutive entries from the same author group
  under one avatar and header.
- `ChatAuthor::tone(Tone)` and `ChatAuthor::initials(&str)` drive the avatar
  square (a `Tone::Cat(n)` for a runtime; the raised ground for the user).
- `ChatEntry::bubble(bool)`: a radius-12 bubble on the raised ground, 14.5 px;
  on by default for `ChatAuthor::user`.
- `ChatEntry::prose(ProseFace::Read | Ui)`: `Read` (the theme's reading face,
  `--Qfr`, 15.5 px / 15 px on a phone, line height 1.6) by default for an
  agent; `ProseFace` is exported from `rwire` and applies the face to any
  element.
- `ChatTranscript::max_width(Measure::Reading)`: the 760 px centred reading
  column, 20 px gutters (16 px on a phone), 18 px between entries, on
  `BgCanvas`. New `St` tokens cover the layout; no inline styles.
- Gallery: a "Conversation" demo. Measured against the /app reference:
  column 760 / 20 / 16 / 18, avatar 30-26 r8, bubble r12 raise 14.5 px,
  prose Source Serif 4 15.5-15 px lh 1.6, thread nord4 light / nord0 dark.
