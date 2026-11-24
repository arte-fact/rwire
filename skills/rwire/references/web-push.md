# rwire — Web Push (`rwire-push`)

Standard Web Push (VAPID, RFC 8030/8291/8292), no third-party service. Example: `examples/push`.
This page is kept from the in-repo skill and re-checked against `libs/rwire-push/src`.

## What the crate provides

- **The VAPID key** — `VapidKey::load_or_generate(path)` / `VapidKey::from_env(var)`. Keep it: a
  new key orphans every subscription.
- **The subscriptions** — `Push::new(key, "mailto:…").store("…/subs.json")`, atomic file,
  mode 0600.
- **Encryption and the signed POST** — `push.send_push(&Notification)`, returning accepted /
  removed / failed; `404` and `410` purge the subscription.
- **The service-worker half** — `Pwa::new(…).push_notifications(true)` (needs the PWA).
- **The browser half** — the `push` runtime extension, `.serve_push()` on the server.

## What the app writes

- A button that subscribes: `Button::primary("Notify me").build().push_subscribe(&push)`. The
  permission prompt runs **inside that click** — the user gesture iOS requires. Never subscribe
  on page load.
- Optionally `.push_unsubscribe()`.
- One `report_input("push-report", on_report())` per page, **outside** re-rendered regions. Its
  handler does `me.push = push.handle_report(ctx.text().unwrap_or(""))` and renders from the
  returned `PushState` (Unknown — the default, nothing reported yet — / Subscribed /
  Unsubscribed / Denied / Unsupported / Failed). It also fires
  once on load with the current state.
- The moment to send, e.g. publishing a post. Send from a task (`tokio::spawn`; add `ctx.defer()` before it
  so the button spins until the task resolves) — never block a handler on it.
- The `Push` handle: handlers take only their state, so keep it in a `static OnceLock<Push>`.

## Limits

- Push needs a secure context: https, or `localhost` / `127.0.0.1`.
- iOS and iPadOS 16.4+ only inside an installed PWA (home screen), from a user gesture;
  elsewhere on iOS the report is `Unsupported`.
- A refused permission (`Denied`) can only be undone in the browser's site settings.
- Payload ≤ 3993 bytes (`MAX_PAYLOAD` in `ece.rs`). One message per subscriber.
- Endpoints are restricted to the browsers' push services (`KNOWN_PUSH_HOSTS`) against SSRF. A
  self-hosted service or a test double needs `.allow_endpoint_origin(origin)`.
