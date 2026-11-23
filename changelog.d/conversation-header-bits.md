### Added: header bits (R3, vibe-18) — `PageHeader`, `Badge::status`, `DangerGhost`, `ChatDivider`/`ChatNote`, `TypingIndicator::tone`, responsive label helpers

- `PageHeader`: a `<header>` row (min-height 56 px, bottom border) with an
  `h1` title (15 px, weight 650), an optional mono suffix (11.5 px, faint,
  with a `title` tooltip), an optional mono subtitle (12 px), an actions
  slot, and a menu button hidden from `Bp::Md` up (44 px touch target).
- `Badge::status(tone, label)`: a raised pill with a status dot in `tone`
  and a 12 px / 600 label; the label collapses to screen-reader-only below
  `Bp::Sm` via `label_hidden_below`.
- `ButtonIntent::DangerGhost` (`DangerGhost`): danger text on a neutral
  bordered ghost button, 36 px tall, 44 px on a phone; `label_hidden_below`
  collapses the label to screen-reader-only below the given breakpoint.
- `ChatDivider::day(label)`: a centered faint 11.5 px day divider with
  0.04 em tracking, `role="separator"` and an `aria-label`.
- `ChatNote::note(text)`: a centered muted 12 px note row.
- `TypingIndicator::tone(Tone)`: the three dots take a tone (e.g. success
  for the writing row of a transcript).
- New `St` tokens for the responsive helpers: `SrOnlyBelow*` /
  `HiddenFrom*` at `Bp::Sm/Md/Lg/Xl`, exposed as `St::sr_only_below(bp)` and
  `St::hidden_from(bp)`; plus `Text15`, `MinHHeader`, `Tracking04em`,
  `SqDot04`. No inline styles.
- Gallery: a "Page header" demo and divider/note/writing rows in the
  "Conversation" demo. Measured at 1280 and 375 px, light and dark.
