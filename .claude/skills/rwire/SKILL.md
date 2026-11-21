---
name: rwire
description: Building UIs on the rwire framework — components-first workflow, St/At/Av tokens, theming, and the traps a real app hit so you don't have to. Read before writing or reviewing rwire UI code.
---

# Building UIs on rwire

rwire renders synchronous Rust (`#[handler]` / `#[renderer]` / `el(...)` builders) over a
binary wire protocol — no JS/CSS source, no build step. This skill is the practical order of
operations for UI work, distilled from the first big production app on the framework
(claw-rwire). Follow the ladder: **theme → component → token → inline style**, top first.

## 1. The theme owns the aesthetic

Set the look once in `main()` and never re-assert it per element:

```rust
Theme::dark()
    .palette(palettes::nord())
    .style(ThemeStyle::flat())      // terminal look: mono UI font, hairline borders, no shadows
    .radius(RadiusScale::None)      // sharp corners — themed components follow automatically
```

- `ThemeStyle::soft()` is the default (tinted, rounded, shadow-free); `ThemeStyle::flat()`
  adds the monospace UI font via `--Qft`. Custom presets: implement `IntoStyle`.
- `RadiusScale` scales every `Rounded*` token except `RoundedFull` (circles stay circles).
- Because radius/shadow/font are CSS-var based, **components inherit the theme** — never
  sprinkle `FontMono` / `Rounded0` / `FontInheritAll` to enforce a look (the base reset
  already applies `font:inherit` to form controls).
- Colours only via semantic tokens (`St::BgApp`, `St::TextMuted`, `St::BorderDefault`, …),
  never hex. The palette maps them.

## 2. Components before markup

`rwire-components` has ~55 components, all `St`-token based, catalogued in
`libs/rwire-components/src/catalog.rs` (which auto-generates the design-system app —
`apps/rwire-design-system`). Reach for them before hand-rolling:

| You're about to build… | Use instead |
|---|---|
| a button of any kind | `Button::{primary,secondary,ghost,outline,destructive}(…)` — sizes `Xs` (24px dense chrome) to `Lg`; you get the focus-visible ring, hover/disabled/loading states, icon handling free |
| an icon-only button | `Button::icon_only(icon, "Aria label")` — squares itself, carries the aria-label |
| a status chip / label | `Badge::new().intent(…)` + `.shape(BadgeShape::Square)` + `.fill(BadgeFill::Outline)` for the sharp outlined look |
| a selectable filter / tab / view toggle | `Chip::new(label).active(bool).on_click(h)` — chain `.data(…)` for the payload |
| a presence / activity dot | `StatusDot::new().intent(…).pulse(bool).label("running")` |
| a chat input bar | `Composer::new().id(key).placeholder(…).on_submit(h)` — Enter submits, Shift+Enter newline; `.compact(true)` for inline bars |
| a chat / log scroll area | `ChatScroll::new(inner)` — see the trap in §5 |
| an avatar with initials | `Avatar::new().name("Ada Lovelace")` — derives "AL" |
| form fields | `Input::text()/email()/number()`, `Textarea`, `Select`, `Checkbox`, `Switch` — they take `name/placeholder/autocomplete/spellcheck/rows` as methods |
| a plain server-handled form | `rwire::form::Form` + `Field::text/email/password/number/textarea/submit`. `.required()/.min_length()/.max_length()/.pattern()` become native constraints; `Field::textarea` carries a stable id (`field-<name>`), so typed text survives re-renders |
| empty screens | `EmptyState::new().title(…).description(…).action(…)` |
| overlays | `Modal`, `Drawer` (position, backdrop, on_close) |

**Clearing an input after submit**: key the field's `id` by something that changes when the
submit lands (e.g. `format!("composer-{}", messages.len())`) — a fresh element is an empty
element. No client opcode needed. `Composer` documents this pattern.

**When a pattern repeats** (≥2 sites, or it encodes a hard-won fix): promote it into
`rwire-components` instead of copying — add the module, `pub use` it in `lib.rs`, register a
`ComponentEntry` in `catalog.rs` (name/slug/category/variants/bools/`build_demo`), and it
appears in the design-system app automatically. Gate with
`cargo fmt && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.

## 3. Tokens before strings

- **Styles**: `St::*` covers every constant (`grep style_tokens.rs` first). Inline
  `.style(Style::new().set(…))` is *only* for genuinely dynamic values — `calc()`, `vh`
  offsets, depth-based padding, CSS custom properties. If a constant lacks a token, **add the
  token** to `style_tokens.rs` (next free hex index) rather than inlining.
- **Attributes**: `.at(At::Type, Av::Submit)`, `.at_str(At::Name, v)`, `.bool_attr(…)` —
  never string `.attr("k","v")`. Missing key/value? Add it to `attr_tokens.rs`.
- Full-height is `St::HDvh` / `St::MinHDvh` (dynamic viewport) — never raw `100vh`.

## 4. Reactivity + routing rules

- Wrap changing UI in `#[renderer]` (a synced region); static scaffolding stays plain `el()`.
  Keep hot regions small — a token stream should repaint one entry, not a board.
- **Async actions**: a handler that hands work to a background thread calls `ctx.defer()` →
  returns a `Send` `Resolver`; `resolver.ok()` / `.err(msg)` resolves it out-of-band later. The
  triggering `Button` / `Composer` / form-submit then spins in place and clears (or turns
  destructive with the message) **for free** — reflection is on by default (`.reflect(false)`
  opts out), identity is keyless `(session, handler_id, param)`, and a handler that returns
  without deferring never spins. Page-level error surfaces: `ActionErrors` (inline) /
  `ActionToasts` (toast). See `docs/async-action-design.md`.
- Lists: `ItemRef` + `iter_with_ref()` + `on_ref()` — no `data-*` index strings for identity.
  (Plain `.data(…)` payloads on buttons are fine for handler args.)
- Navigate with `Link::route(…)`, never raw `<a href>` (that's a full reload).
- One routing model per page tree: `on_route` **or** `routes` + `outlet()`. A `Router`
  needs an `outlet()` in the page tree (directly or inside a renderer's output): without
  one, `run()` refuses to start with `ROUTER_WITHOUT_OUTLET` instead of serving a page
  whose navigation freezes.
- Enter-to-submit on a textarea: set `data-enter-submit` (the runtime handles
  preventDefault + `requestSubmit`; Shift+Enter falls through to a newline). Don't bind
  keydown yourself — rwire `preventDefault`s bound keydowns, which blocks typing.
- **Auth — pick one gate.** `.auth(user, password)` is rwire's own login form.
  `.auth_with(DelegatedAuth::new("cookie", validator).login_url(url))` delegates to an identity
  provider: the async validator gets the request head (`AuthRequest`: cookies, headers) and
  returns `Option<Identity>` (`id` + opaque `data`, e.g. the token to call the provider on the
  user's behalf). Pages without identity → `303` to `login_url?next=<path>`; WebSocket → `401`.
  Successes only are cached (`cache_ttl`, default 30 s; bounded by `cache_capacity`); open
  sockets are re-validated every TTL and closed (code 4401) once revoked — the client then
  reloads onto the login. Handlers read `ctx.identity()`; renderers have no context, so copy what
  they show into state from `on_route` (it runs on connect and sees the identity). Both gates
  at once → `run()` fails with `AUTH_CONFLICT`. The validator runs on rwire's tokio runtime, so
  it can call reqwest / a hub client directly. Never log the cookie.
- **Runtime: tokio.** `#[tokio::main]` and `Server::…run().await`; inside an existing tokio app
  (axum, an SSE client) it is just a task: `tokio::spawn(server.run_until(stop))`. Handlers run on
  the runtime: `tokio::spawn` / `tokio::time` work there, but never block in a handler (no
  `std::thread::sleep`, no blocking I/O, no `block_on`) — `spawn_blocking` or `ctx.defer()` + a
  thread for that. Background code reaches the UI through `server.shared_state()` taken before
  `run()` (`update_shared_if` broadcasts only real changes; sync, callable from any thread).
  `run_until(fut)` stops cleanly: listener closed, sockets closed with 1001, 5 s grace.

## 4b. Web Push notifications (`rwire-push`)

Standard Web Push (VAPID, RFC 8030/8291/8292), no third-party service.
Example: `examples/push`.

**rwire-push provides:**
- the VAPID key: `VapidKey::load_or_generate(path)` / `from_env(var)`. Keep it: a new key
  orphans every subscription;
- the subscriptions: `Push::new(key, "mailto:…").store("…/subs.json")`, atomic file, mode 0600;
- encryption and the signed POST to each push service: `push.send_push(&Notification)`.
  It returns accepted / removed / failed; 404 and 410 purge the subscription;
- the service-worker half: `Pwa::new(…).push_notifications(true)` (needs the PWA);
- the browser half: the `push` runtime extension (`.serve_push()` on the server).

**The app writes:**
- a button that subscribes: `Button::primary("Notify me").build().push_subscribe(&push)`.
  The permission prompt runs inside that click, the user gesture iOS requires.
  **Never** subscribe on page load;
- optionally a `.push_unsubscribe()` button;
- one `report_input("push-report", on_report())` per page, **outside** re-rendered regions.
  Its handler does `me.push = push.handle_report(ctx.text().unwrap_or(""))` and renders
  from the returned `PushState`: Subscribed / Unsubscribed / Denied / Unsupported / Failed.
  It also fires once on load with the current state;
- the moment to send, e.g. publishing a post. Send from a task (`tokio::spawn`, or
  `ctx.defer()` to make the button spin); never block a handler on it;
- the `Push` handle: handlers take only their state, so keep it in a
  `static OnceLock<Push>`.

**Limits:**
- Push needs a secure context: https, or `localhost` / `127.0.0.1`.
- iOS and iPadOS 16.4+ only inside an installed PWA (home screen), from a user gesture.
  Elsewhere on iOS the report is `Unsupported`.
- A refused permission (`Denied`) can only be undone in the browser's site settings.
- Payload ≤ 3993 bytes (`MAX_PAYLOAD`). One message per subscriber.
- Endpoints are limited to the browsers' push services (`KNOWN_PUSH_HOSTS`) against SSRF.
  A self-hosted service or a test double needs `.allow_endpoint_origin(origin)`.

## 5. Traps a real app hit (each cost a debugging session)

- **Bottom-pinned chat scroll**: `column-reverse` + an **auto bottom margin** on the inner
  column (`St::MbAuto`) — **never** `justify-content` on the scroll container: justify makes
  overflowing content unreachable, while an auto margin collapses to 0 on overflow. This is
  exactly what `ChatScroll` encodes; use it.
- **Width collapse**: a width-spanning child of a flex-*centered* parent needs `St::WFull`
  or it shrinks to content (~0 for empty divs).
- **Stacking tokens**:
  - A later `.st()` token **replaces** the earlier ones whose every property it sets again:
    `.st([DisplayFlex])` then `.st([DisplayNone])` hides the element.
  - A *partial* overlap keeps both, and the shared properties go to the higher token code. The
    stylesheet is kept in code order on every page.
    - `BorderNone` then `BorderB2Accent`: works.
    - `FontInheritAll` then `TextXs`: the size is lost. A debug build prints
      `rwire: st([TextXs]) loses font-size…` once per pair.
  - Pick non-overlapping tokens.
  - Don't put `FontInheritAll` on form controls: the base reset already applies it.
- **Shared-state renderers inside memory-state renderers** don't receive the shared cache —
  keep shared (`App`) renderers nested under shared parents.
- **`#[derive(State)]` field ceiling is 64** per struct — split view models out (re-export
  flat so callers don't churn).
- **Keyed swaps**: when switching what an entire region shows (e.g. another agent's thread),
  key the container `id` by the subject so the client swaps clean instead of diffing across
  subjects.

## 6. Reference

- Component source + catalog: `libs/rwire-components/src/`
- Tokens: `libs/rwire/src/style_tokens.rs`, `attr_tokens.rs`
- Theme system: `libs/rwire/src/theme.rs` (palettes, styles, radius, base CSS)
- Live playground: run `apps/rwire-design-system`
- Reference apps: `../claw-rwire` (large, flat-theme), `apps/rwire-website`, examples/
