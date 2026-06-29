# PWA Support — Implementation Roadmap

Tracking doc for adding installable-PWA support to rwire via a high-level
`CapsuleConfig::pwa(Pwa)` config. See the settled design in chat for rationale.

**Status legend:** `[ ]` todo · `[~]` in progress · `[x]` done · `[-]` deferred

## Settled decisions

- Offline = installable + cached shell for instant boot; **offline shows a runtime
  reconnect/offline overlay** (no real-content caching).
- Manifest `theme_color`/`background_color`: **static, configurable**, default **derived
  from the theme's initial mode** (new `oklch_to_hex`).
- Reconnect: **harden** the existing loop (visible overlay, network-aware, `/ready` probe).
- Icons: **embed PNGs** via `include_bytes!`, with a **built-in default glyph** (zero-asset install).
- Overlay: **always-on** (every app's runtime).
- SW update: **on next launch** (waiting SW; no `skipWaiting`/`claim`).
- HTTPS: **log once** if a page request arrives without `X-Forwarded-Proto: https` while PWA is on.
- `start_url`/`scope`: default `"/"`, overridable.

## Progress

| Batch | Total | Done |
|-------|-------|------|
| 1 — Foundations | 2 | 2 |
| 2 — PWA module  | 8 | 0 |
| **All** | **10** | **2** |

---

## Batch 1 — Foundations (independently useful; ships as commit 1)

### B1.1 — `oklch_to_hex`
- **Status:** `[x]` Done. `oklch_to_hex(l,c,h)` in `tokens/palette.rs` (Oklab→linear-sRGB,
  per-channel gamut clamp). Tests: exact white/black, hex→oklch→hex round-trip ≤3/255 on
  Nord colors, out-of-gamut clamps without panic.
- Add the inverse of the existing hex→oklch in `tokens/palette.rs`: oklch → oklab →
  linear sRGB → gamma → `#rrggbb`, with sRGB-gamut clipping for out-of-gamut oklch.
- **Acceptance:** round-trips known colors within tolerance; clips out-of-gamut without
  panicking; unit tests vs reference values.

### B1.2 — Always-on reconnect/offline overlay (`RUNTIME_JS`)
- **Status:** `[x]` Done. `connect()` hardened: themed overlay (`#__rwov`) shown 600ms
  after disconnect (debounced so fast reconnects don't flash), "Reconnecting…" vs
  "You’re offline" (`navigator.onLine`), Retry button → `connect()`. `online`/`offline`
  listeners (reconnect on `online`); reachability probe moved from `fetch(pathname)` →
  `fetch('/ready')` (SW-safe). Runtime JS ~14KB, `node --check` clean. Verified live:
  killing the server shows the overlay; reconnect hides it.
- Harden `connect()`: a fixed overlay shown while disconnected (`rn`), hidden on
  `onopen`; "Reconnecting…" vs "You're offline" (`navigator.onLine`); **Retry** button →
  `connect()`. Listen to `online`/`offline` (reset backoff + reconnect on `online`).
  Change the reachability probe from `fetch(pathname)` → `fetch('/ready')`.
- **Acceptance:** disconnect shows the overlay; reconnect hides it; offline shows the
  offline copy; runtime JS still syntax-checks (`node --check`); capsule still builds;
  size stays well under budget.

---

## Batch 2 — PWA module, config & serving (commit 2)

### B2.1 — `pwa.rs` types + builder
- **Status:** `[ ]`
- `Pwa`, `PwaDisplay`, `PwaIcon` + builder methods (name/short_name/description/display/
  icon/maskable_icon/theme_color/background_color/start_url/scope).

### B2.2 — Default glyph asset
- **Status:** `[ ]`
- Embed a 512×512 rwire-mark PNG used when no icons are supplied.

### B2.3 — `PwaAssets::freeze`
- **Status:** `[ ]`
- Generate manifest JSON (derive colors via `oklch_to_hex` from theme initial mode),
  `sw.js` (cache-versioned by shell hash; `/ready`·`/health`·`/metrics` excluded; no
  `skipWaiting`/`claim`), the `<head>` fragment, and the icon `(path, mime, bytes)` list.

### B2.4 — `CapsuleConfig.pwa` field + `.pwa()` builder
- **Status:** `[ ]`

### B2.5 — Head injection
- **Status:** `[ ]`
- `generate_styled_capsule` gains a `{pwa_head}` slot + the SW-register snippet (empty
  when not configured).

### B2.6 — Server wiring
- **Status:** `[ ]`
- `run()`: freeze → `Arc<PwaAssets>`, inject head, thread into `handle_client`.
- GET routes (same block as `/health`): `/manifest.webmanifest`, `/sw.js`,
  `/pwa/icon-*.png` via a `serve_static(mime, bytes)` helper.
- HTTPS **log-once** (`AtomicBool`) on first insecure page request while PWA is on.

### B2.7 — `lib.rs` exports
- **Status:** `[ ]`
- `pub mod pwa;` + re-export `Pwa`, `PwaDisplay`.

### B2.8 — Tests + live verification
- **Status:** `[ ]`
- Unit: manifest shape + derived colors, sw.js contents (shell list, probe exclusion, no
  `skipWaiting`), head tags, route content-types, `oklch_to_hex` round-trip.
- Live: enable PWA on one app; Chrome shows it installable; offline → overlay; manifest +
  sw.js + icon served with correct content-types.

---

## Verification per change

```bash
cargo clippy --workspace --all-targets   # 0 warnings
cargo test --workspace
# runtime JS: node --check on the extracted RUNTIME_JS / sw.js
```

## Commit plan

1. **Foundations** — B1.1 + B1.2 (oklch_to_hex + always-on reconnect overlay).
2. **PWA** — B2.* on top.
