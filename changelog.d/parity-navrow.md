### Fixed: a one-line `NavRow` is as tall as its title's line

- The trailing cell is a flex cell, so its badge adds no line strut; the title
  sets its line height (1.5). A one-line row is 32.4 px on a desktop in compact
  density (it was about 1.5 to 4.5 px taller), 44 px under a finger.
- Compact row padding is `6.08px 8.8px` (`--Qzp`: the old front's `.38rem
  .55rem` at its 16 px root, in px).
