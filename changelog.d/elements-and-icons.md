### Added: `El::Time` and `El::B` elements, `Icon::Stop`/`PanelLeft`/`PanelRight`/`SidebarCollapse` icons

- `El::Time` (code `0x47`, tag `time`) and `El::B` (code `0x48`, tag `b`) join the
  `El` element table. They render at SSR like any other element and their tag
  name reaches the client through `MAP_DEF`, so `el(El::Time)` / `el(El::B)`
  work over the wire with no runtime change.
- Four new stroke icons in `rwire::icons` (24x24 viewBox, `currentColor`, no
  hardcoded fill): `Icon::Stop` (a square), `Icon::PanelLeft`, `Icon::PanelRight`
  (a panel with its divider on the left/right) and `Icon::SidebarCollapse`
  (a panel with a collapse arrow).
