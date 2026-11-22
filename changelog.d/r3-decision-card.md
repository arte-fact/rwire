### Added: `DecisionCard` + `DecisionAction` (R3, iteration 5) — validations in the thread

- `DecisionCard::new(title)`: a `Warning`-toned card by default — the theme's
  amber at 12 % (`St::BgWarning12`, `color-mix(in srgb, var(--p) 12%, …)`),
  radius 12, padding 14 × 16 (`St::PadDecision`), hairline border. `.tone()`
  switches to another tone's tinted ground.
- Header: title (13 px, weight 650) + the tool as a mono tag (`decision-tag`);
  `body` in a preformatted block (mono 12.5 px, radius 8, code ground,
  `overflow-x:auto`); `reason`; a collapsible `details(summary, el)` for the
  terminal dialogue.
- Actions: one `<button>` per choice, two lines (label + hint, the hint also
  as `title`): `DecisionAction::new(label).hint(text).intent(…).on_click(h)`,
  `.disabled(true)` for the read-only preview.
- `resolved(Some(text))` replaces the actions with a `role=status` line;
  `error(Some(text))` adds a `role=alert` line (hub refusal).
- Gallery: a "DecisionCard" entry (Pending/Resolved/Refused) and pending +
  resolved cards in the Conversation demo. Measured at 1280/375 light/dark:
  padding 14×16, radius 12, amber 12 % ground, title 13 px/650, tag mono
  12 px, body 12.5 px mono r8, two-line actions, resolved `role=status`.
