### Added: chat bubbles, grouped tool calls, and a foldable sidebar

- **`ChatEntry::bubble(true)`** / **`ChatItem::bubble()`** — the local user's turn
  as a right-aligned tinted block (width follows its content, up to a share of the
  column, the author's name kept for screen readers); model output stays plain and
  full width.
- **`ChatEntry::rail(false)`** / **`ChatItem::rail()`** — the entry's left accent
  rail (a 2px tone strip) is optional: off, the body runs flush with the column,
  no lateral border, the text sitting on the page.
- **`ChatCalls`** (new component) — a run of tool calls collapsed into one
  transcript block: a summary line ("4 appels d'outils · Bash, Read, Edit"),
  each call with its state (`ChatCallState::Running|Ok|Error`) and its own native
  disclosure for input and output. Opens by itself while a call runs or after a
  failure, stays closed once everything succeeded; pure `<details>`, no round-trip.
  The run carries no chrome of its own (no background, no rounding, no padding):
  it reads as a line of the transcript, never as a message bubble.
- **`AppShell::collapsible_sidebar(true)`** + **`AppShell::sidebar_toggle()`** —
  the visitor folds the sidebar away from a native checkbox; the shell hides the
  column and its handle in CSS (`:has`), and the app places the toggle label in a
  header of its own, so the control sits in the flow instead of floating over a
  column (no server round-trip, checkbox keyboard reachable).
