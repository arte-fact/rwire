### Added: `ThemeMode::System`, a three-state theme toggle, and a picked mode that survives a reload

- **`ThemeMode::System`** follows the OS (`prefers-color-scheme`). The theme CSS
  carries both modes: the light block on `:root`, the dark one under
  `@media (prefers-color-scheme:dark)`, so the page switches live with the OS,
  with no round trip. `Light` and `Dark` still force a mode (one block).
  - `Theme::mode(ThemeMode)` builder; `ThemeMode::cycle()` (system → light →
    dark → system) and `ThemeMode::parse`. `toggle()` sends `System` to `Light`.
  - Each `:root` block now sets `color-scheme`, so native controls and
    scrollbars follow the mode.
- **`ThemeToggle`, three states:** `ThemeToggleMode::System` (a screen icon,
  new `Icon::Monitor`), `From<ThemeMode> for ThemeToggleMode`
  (`.mode(theme.mode.into())`), and a `title` naming the current mode.
- **A picked mode survives a reload.** The theme `<style>` carries
  `data-rw-mode`. When it changes during a visit, the runtime writes the
  `rw_mode` cookie (one year, scoped to the app's mount: `path=` its
  `base_path`, or `/`); the server then serves the page (its
  first paint included: one capsule per mode, built at startup) and starts the
  connection in that mode. The page response carries `Vary: Accept-Encoding,
  Cookie`. The mode a page starts in is never written, so an
  app's default stays the app's to change.
- **Breaking:** `ThemeMode` gains a `System` variant, and `ThemeToggleMode` too:
  an exhaustive `match` on either needs an arm (`.mode(theme.mode.into())`
  replaces the usual `match`).
- The runtime budget grows by 200 B raw / 80 B gzipped for the cookie write
  (it runs after every batch, so it cannot be lazy).
