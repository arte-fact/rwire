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
