### Added: authored names that link and qualify themselves

- **`ChatAuthor::name_node(el)`** — the header name can be rendered by a node the
  app supplies, typically a link to the author's own session. The plain `name`
  stays the entry's accessible label, so screen readers and tests still read it.
- **`ChatAuthor::suffix(text)`** — a small muted qualifier right after the name
  (« terminal », « via MCP », « via horloge »): a multi-author thread can say how
  a message reached the conversation without adding chrome to the body.

Both are optional and default to the previous rendering (plain text, no suffix),
so existing transcripts are unchanged.
- A **bubble** keeps its author implied by the side it sits on, but the suffix
  the app set (« terminal ») stays visible next to the time.

### Changed: quieter bubbles, closed tool runs, and a foldable detail panel

- **Bubbles** carry no author name: the suffix (« terminal ») and the time sit in a
  muted line **under** the block, and the name stays the entry's accessible label.
- **`ChatCalls`** stays closed by default (even while a call runs or after a
  failure), drops the « détails » label for the native `<summary>` triangle, and
  indents the expanded calls under their summary.
- **`AppShell::collapsible_panel(true)`** + **`AppShell::panel_toggle()`** — the
  detail column folds away exactly like the sidebar (native checkbox, `:has`
  rules, no round-trip, address untouched); the two toggles now share one shape.
- **`Composer::focus_ring(false)`** — opt out of the card's focus highlight when
  the field is invisible and the surrounding surface already shows the focus.
