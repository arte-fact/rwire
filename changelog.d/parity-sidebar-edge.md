### Changed: `Sidebar` draws its right edge

- `Sidebar` has a 1 px hairline on its right edge (`St::BorderRDefault`,
  `--h`) by default; `.border(false)` drops it (a sidebar on another edge or
  in a drawer). The sidebar owns its edge: the three-column `AppShell`'s
  sidebar column no longer draws one (no double line).
