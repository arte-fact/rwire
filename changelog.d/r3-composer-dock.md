### Added: composer dock and send options (R3, iteration 6) — `ComposerDock`, `send_icon`, `min_rows`, `hint_hidden_below`, `reset_on_ack`

- `Composer::send_icon(Icon::ArrowUp)`: an icon-only send button, a 34 px
  square (44 px under a finger via the theme's touch rule), radius 9, the
  label in `aria-label` (`St::Sq34`, `St::Rounded9`).
- `Composer::min_rows(2)`: the field's starting row count (default 1).
- `Composer::hint_hidden_below(Bp)`: the hint is `display:none` below `bp`
  and flex from `bp` up (e.g. hidden under 768 px).
- `Composer::reset_on_ack(true)`: the field carries `data-reset-on-ack`; the
  runtime empties it at the `EVENT_ACK` of a successful submit and keeps the
  text when the submit's action came back errored (`data-action-error` marker
  added to the errored phase of a reflected action). Runtime: 4 new Node
  tests (ack clears, error keeps, detached form keeps, Enter submits /
  Shift+Enter / IME do not). Size budget +250B raw / +120B gzip.
- `ComposerDock::new(composer)`: a sticky bottom bar (full width, canvas,
  gutters 20/16 px, bottom safe-area) with the composer as a card centred at
  the 760 px reading measure, radius 14, soft shadow (`--Z2`).
- The default field cap (40vh) and padding are now tokens (`MaxH40vh`,
  `PadField`); an inline style remains only for a caller-provided
  `max_height`. Field text is 14.5 px (`St::Text14_5`).
- Gallery: a "ComposerDock" entry and the docked composer under the
  Conversation demo. Measured at 1280/375 light/dark: field 14.5 px rows=2,
  send 34 px r9 (44 px at 375) aria-label, hint flex/none around md, dock
  sticky bottom canvas 20/16 px gutters, card r14 soft shadow centred at 760.
- Enter submits, Shift+Enter newlines, IME composition does not submit —
  already the runtime's `data-enter-submit` behavior, now pinned by tests.
