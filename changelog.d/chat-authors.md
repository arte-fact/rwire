### Added: authored names that link and qualify themselves

- **`ChatAuthor::name_node(el)`** — the header name can be rendered by a node the
  app supplies, typically a link to the author's own session. The plain `name`
  stays the entry's accessible label, so screen readers and tests still read it.
- **`ChatAuthor::suffix(text)`** — a small muted qualifier right after the name
  ("terminal", "via MCP", "via the clock"): a multi-author thread can say how
  a message reached the conversation without adding chrome to the body.

Both are optional and default to the previous rendering (plain text, no suffix),
so existing transcripts are unchanged.
- A **bubble** keeps its author implied by the side it sits on, but the suffix
  the app set ("terminal") stays visible next to the time.

### Changed: quieter bubbles, closed tool runs, and a foldable detail panel

- **Bubbles** carry no author name: the suffix ("terminal") and the time sit in a
  muted line **under** the block, and the name stays the entry's accessible label.
- **`ChatCalls`** stays closed by default (even while a call runs or after a
  failure), drops the "details" label for the native `<summary>` triangle, and
  indents the expanded calls under their summary.
- **`AppShell::collapsible_panel(true)`** + **`AppShell::panel_toggle()`** — the
  detail column folds away exactly like the sidebar (native checkbox, `:has`
  rules, no round-trip, address untouched); the two toggles now share one shape.
- **`Composer::focus_ring(false)`** — opt out of the card's focus highlight when
  the field is invisible and the surrounding surface already shows the focus.

### Added: the app's own words for a run of tool calls

- **`ChatCallsText`** + **`ChatCalls::text(ChatCallsText)`** — the summary line
  (none / one / many, `{n}` calls, `{tools}` names, the failure count) and the
  three call states (`running`, `ok`, `error`) are now the app's strings. A
  library speaks its own language: the defaults are English, and an app that
  shows another one passes its table in.

  ```rust
  ChatCallsText { running: "in flight", ..ChatCallsText::default() }
  ```

### Changed: the libraries speak English

- Every user-facing word still shipped by `rwire-components` and
  `rwire-markdown` — the `ChatCalls` summary and states, the sample
  `ChatAuthor` suffixes, the `AppShell` details controls — is **English**. The
  words an app owns (author names, tool names, content) are untouched: an app
  brings its own language through the new `*Text` tables rather than inheriting
  the library's.

### Added: a compact composer with the send button in the row

- **`Composer::compact(true)`** is now **one box**: the field and the send
  button share the same rounded shell (border, shadow and focus highlight on
  the box, not on the field), so a thread's steer bar no longer looks like a
  card inside a card. No actions row, no hint.
- **`Composer::send_icon(Icon)`** — the glyph on the send button is the app's
  to choose (default `ArrowUp`).
- **`Composer::send_icon_only(bool)`** — drop the written label and keep it as
  the accessible name and tooltip. Defaults to the form factor: icon-only in
  the compact box, label beside the glyph in the pill.

### Changed: code blocks are one card with a copy button

- A fenced block (and `rwire_markdown::highlight_code`) is now **one card**: a
  header strip with the language (uppercase, muted) and a **copy button**
  (`data-copy`, so the runtime's clipboard affordance does it; the glyph turns
  into a check for two seconds), above the `<pre><code>`.
- The code ground `--Qc` is a step **darker than the page** in both modes
  (`color-mix(in srgb, var(--a) 86%, #000)`), in the `solid` and `studio`
  styles: a dark panel on the light grey page, a deeper panel on nord0.

### Added: an empty composer guards its send button

- `Composer` marks its form `data-send-guard` and ships a one-rule sheet: while
  the field shows its **placeholder** (i.e. holds nothing) the submit button is
  dimmed and out of the click path. Pure CSS (`:has(:placeholder-shown)`) — no
  round-trip, no runtime change; a composer without a placeholder cannot be read
  for emptiness, so the guard needs one.
- The compact row's field takes `PySm`, which gives it the height of the 36 px
  send button on one line: text and button share a centre line.
