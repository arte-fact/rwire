### Changed: lighter HTTP answers for the capsule

- **Short 404**: a path the app has no page for gets a one-line `404 Not
  Found` instead of the 20–30 KB page. With the router, unless it declares a
  not-found view (`Router::not_found`, then the page is served with 404 so it
  renders); without one, an app with `on_route` still gets the page on every
  path, and an app with neither only at `/`.
- **`CapsuleConfig::favicon(bytes, content_type)`** serves `/favicon.ico`
  and links it from the `<head>` (replacing the PWA's `<link rel="icon">`).
- **`CapsuleConfig::title(text)`**: the page's `<title>` (replacing the PWA
  name's). One title per app; no per-route title yet (documented).
- **gzip**: the page (fixed capsule compressed once at startup, 27.6 KB →
  9.8 KB; the `ssr(true)` page compressed per request, ~0.6 ms), text
  `asset()`s and the bundled extension modules, for clients whose
  `Accept-Encoding` takes gzip, with `Vary: Accept-Encoding`. New dependency:
  `flate2` (pure-Rust backend).
