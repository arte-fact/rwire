### Fixed: links read at AA in dark mode (`--lk`, `St::TextLink`)

- Links used the accent's step 11 in both modes: 2.28:1 on Nord's dark ground.
  They now read `--lk` (`St::TextLink`), computed per mode to reach 4.5:1 on
  the app, surface and sidebar grounds: the accent's step 11 in light mode and
  step 6 in dark mode when they read, else the nearest that does (a style may
  name its own in `--lk`).
- `rwire_components::Link` and `rwire::router::Link::to` / `Link::touch` use
  it (`Link::to` had no color: the browser's default blue).
- studio + Nord: dark links are nord8 `#88c0d0` (the old front's `--link`);
  light links are `#466389`, as the accent's step 11 (`#4c6b94`, 4.05:1) and
  the old front's `#4b6d96` (3.96:1) are under AA on nord4.
- `contrast.rs` checks the link on the app, surface and sidebar grounds in
  every built-in theme and mode.
