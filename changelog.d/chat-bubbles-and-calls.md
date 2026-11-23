### Added: chat bubbles, grouped tool calls, and a foldable sidebar

- **`ChatEntry::bubble(true)`** / **`ChatItem::bubble()`** — the local user's turn
  as a right-aligned tinted block (width follows its content, up to a share of the
  column, the author's name kept for screen readers); model output stays plain and
  full width. No rail, no repeated header: the side of the block says who speaks.
- **`ChatCalls`** (new component) — a run of tool calls collapsed into one
  transcript block: a summary line ("4 appels d'outils · Bash, Read, Edit"),
  each call with its state (`ChatCallState::Running|Ok|Error`) and its own native
  disclosure for input and output. Opens by itself while a call runs or after a
  failure, stays closed once everything succeeded; pure `<details>`, no round-trip.
- **`AppShell::collapsible_sidebar(true)`** — the visitor folds the sidebar away
  from a native checkbox and two labels (fold on the sidebar's corner, unfold on
  the main column's, shown once folded). CSS-only (`:has`), so folding keeps the
  shell's layout rules and costs no server round-trip; the checkbox stays
  keyboard reachable.
