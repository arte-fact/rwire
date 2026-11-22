### Added: `Markdown::face(ProseFace)` (R3, iteration 2) — the reading face for chat answers

- `Markdown::face(ProseFace::Read)` sets the whole block in the theme's
  reading face (`--Qfr`, 15.5 px / 15 px on a phone, line height 1.6) spanning
  its container — inside a chat's reading column, the column is the cap, so
  the `65ch` prose cap is dropped. Headings, lists, quotes and tables keep
  their theme tokens; the app has nothing left to restyle.
- Fenced code blocks in the reading face run mono at 12.5 px on the theme's
  code ground, radius 10 (`--R3`), with `overflow-x:auto`: a long line scrolls
  inside the block, never the page (`highlight_code_face`; `highlight_code`
  keeps the interface face).
- `ParseOptions` gains a `face` field (`Ui` by default — docs rendering is
  unchanged). The `javascript:` link and remote-image neutralisation is
  unchanged and now also tested under the reading face.
- Gallery: the Conversation demo shows a long Markdown answer (headings,
  list, quote, table, inline code, a long fenced block). Measured at 1280
  and 375 px, light and dark: prose 15.5/15 px lh 1.6 serif, code block
  12.5 px mono r10 `overflow-x:auto` with `scrollWidth > clientWidth` while
  the page `scrollWidth == innerWidth`.
- `rwire-components` now depends on `rwire-markdown` (gallery demo only).
