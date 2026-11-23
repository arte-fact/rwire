# rwire — the model

Facts below are read from the tree at the merge commit this skill ships with; see
`verification.md` for how to pin and re-check.

## 1. The shape

```
Rust server                              Browser
┌──────────────────────┐   binary opcodes   ┌──────────────────┐
│ state + handlers     │ ─────────────────> │ ~13 KB runtime   │
│ ElementBuilder tree  │ <───────────────── │ DOM + events     │
└──────────────────────┘     events         └──────────────────┘
```

The server is authoritative. The browser never holds application state: it holds the DOM and a
name cache (element/event/attribute maps) that the server fills lazily, per connection. The
capsule ships the runtime plus global CSS (~17 KB for a minimal app); names and utility CSS
arrive on first use and are deduped per connection, so a bigger app does not mean a bigger
initial payload (`README.md`, "Capsule Size"; `apps/rwire-docs/docs/05-advanced/tree-shaking.md`).

Consequences worth stating out loud before you design anything:

- every non-cosmetic interaction is a WebSocket round-trip;
- server memory scales with open connections — one process, no horizontal-scaling story:
  partition, don't pool (`README.md`, "Status & scope");
- the static first paint shows **default-state** content until the socket connects.

## 2. Server-side rendering

`ssr(true)` renders the requested route to static HTML **per request**, from the shared state,
plus the utility CSS its classes reference — otherwise that CSS would arrive lazily, after first
paint. The capsule carries a slot for each (`SSR_HTML_SLOT`, `SSR_CSS_SLOT` in
`libs/rwire/src/server.rs`), filled from `to_static_html_and_styles`.

Because that paint has **no visitor identity**, SSR switches itself off behind an auth gate unless
the app opts in with `CapsuleConfig::ssr_behind_auth(true)` (`server.rs`, the `config.ssr && gated`
branch, with a printed notice).

What that means in practice: the first paint is real markup, indexable and instant, but it is not
*yours* — it carries no identity and no per-connection state. Anything per-user is filled in after
the socket opens: via `on_route` (which runs on connect and sees the identity) or a handler, never
by assuming the SSR pass knew who you are.

## 3. The binary protocol

Each frame is a batch: an optional symbol-table header, then opcodes, then `BATCH_END`. Strings
are interned in a per-connection symbol table — `SYMBOLS` defines it, `SYMBOLS_EXTEND` appends
to it in later batches, indices from `0x80` up are session-specific (`SYMBOL_SESSION_START`) —
and referenced by index.

| Opcode | Name | Format |
|---|---|---|
| `0x02` | CREATE | `[type]` → ref |
| `0x03` | CREATE_SYNCED | create a synced region |
| `0x05` | GET_SYNCED | fetch an existing synced region |
| `0x10` | SET_CLASS | `[ref, sym]` |
| `0x11` | SET_TEXT | `[ref, sym]` |
| `0x12` | SET_ATTR | `[ref, attr, val]` |
| `0x14` | SET_DATA | `[ref, key, val]` |
| `0x20` | APPEND | `[parent, child]` |
| `0x25` | CLEAR_CHILDREN | `[ref]` |
| `0x31` | BIND_REMOTE | `[ref, event, handler]` |
| `0x34` | BIND_REMOTE_PARAM | `[ref, event, handler, len, params…]` |
| `0x42` | DEF_HANDLER | declare a handler index |
| `0x47`–`0x4E` | targets / selectors | client-side toggle and exclusive-select classes |
| `0x50` | BIND_RESIZE | pointer-drag handle that resizes the previous sibling (SplitPane) |
| `0x60`–`0x63` | form validation | set validation, show/clear error, set required |
| `0x70`–`0x73` | route | push / replace / inline variants |
| `0x81`–`0x86` | styles | set, utility, property, multi, composite, table |
| `0x87` | STYLE_DEF | lazy delivery of utility/pseudo/breakpoint CSS rules |
| `0x88` | MAP_DEF | lazy delivery of element/event/attribute/token names |
| `0xF0` | SYMBOLS | `[count, …strings]` |
| `0xF1` | SYMBOLS_EXTEND | `[count, start_index, …strings]` |
| `0xF3` | EVENT_ACK | the server handled one client event |
| `0xFF` | BATCH_END | end of batch |

Full list: `libs/rwire/src/protocol/opcodes.rs`. Integers use varint
(`protocol/varint.rs`), which is why indices and lengths stay cheap.

**You never write opcodes.** You build an `ElementBuilder` tree with `el(El::Div)…` and rwire
encodes it. The opcode table matters for two things only: reading a capture or a log, and
understanding that a wire change is a breaking change with no compatibility matrix (the runtime
ships from the same binary).

## 4. State

```rust
#[derive(State, Default)]
#[storage(memory)]          // see the three kinds below
struct Counter { count: i32 }
```

`StorageType` (`libs/rwire/src/state.rs`):

| Storage | Scope | Survives | Use for |
|---|---|---|---|
| `memory` (default) | one connection | reconnects within the session TTL | per-user UI state, view models |
| `persisted` | SQLite table | process restarts | real data |
| `shared` | one instance for the whole process | everything but a restart | server-wide live data (metrics, fleets) |

`shared` is the one to be careful with: it is genuinely global. That is fine for a fleet
snapshot, wrong for anything a single visitor typed.

**Field ceiling: 64 per `State` struct.** Renderer dependencies are tracked in a `u64` mask,
one bit per field; the derive macro rejects more (`MAX_STATE_FIELDS` in
`libs/rwire-macros/src/lib.rs`, with the explicit diagnostic). Split the view model instead of
fighting it.

Reconnection: the server keeps disconnected-session states for a day (`session_eviction_task`
spawned with `Duration::from_secs(86_400)` in `server.rs`; the comment above it still says
"5-minute", ignore it) and retains at most `MAX_CACHED_SESSIONS` (10 000) of them;
`evict_expired_sessions` drops the expired ones. So a phone that switches apps comes back to the
same state, and memory stays bounded.

## 5. Pages and handlers

Pages are routes:

```rust
Router::new()
    .page("/", home)
    .page("/agents/:machine/:session", thread)
    .not_found(missing)
```

`Router::page(pattern, view)` / `Router::not_found` (`libs/rwire/src/router.rs`), with
`RouteParams::get` and `::wildcard` read from the context. One routing model per page tree —
`on_route`, or `routes` + `outlet()`; a `Router` without an `outlet()` anywhere in the page tree
makes `run()` refuse to start (`ROUTER_WITHOUT_OUTLET`) rather than serve a page whose navigation
silently freezes.

### Navigating from a handler

`ctx.navigate(url)` pushes the URL onto the browser's history (so Back returns here);
`ctx.replace_route(url)` changes it without adding an entry (`state.rs`, `EventContext`). Both
only queue the navigation: it is flushed to that connection **after** the handler's DOM update
lands, so the page you navigate to is already rendered from the state you just changed.

Use them when a handler creates or selects a resource whose URL is only known server-side — a new
item, a picked session. For a plain link in markup, use `rwire::Link::to(href, text)`; a raw `<a href>` (or
`rwire_components::Link`) is a full reload and loses the socket.

Handlers mutate, renderers render:

```rust
#[handler]
fn increment(state: &mut Counter) { state.count += 1; }

#[renderer]
fn render_count(state: &Counter) -> ElementBuilder {
    el(El::Span).text(&state.count.to_string())
}
```

After a handler returns, the renderers that read a changed field re-run (`RendererDeps`: one
bit per field in a `u64` mask, or `RendererDeps::always()`), and only the changed elements go
on the wire. Keep re-rendered regions small: a renderer around a whole board repaints the board
for one token.

`EventContext` (`state.rs`) is what a handler uses to reach out:

| Call | Gives you |
|---|---|
| `ctx.item_ref::<T>()` | the `ItemRef<T>` bound by `on_ref(…)` — type-safe row identity |
| `ctx.item_index()` | the row index, for positional payloads |
| `ctx.session_id()` | the connection's session id — **the key for any per-user side table** |
| `ctx.identity()` | the `Identity` installed by the auth gate (`delegated_auth`), or `None` |
| `ctx.text()` | the payload string of the control |
| `ctx.navigate(url)` | push `url` onto the browser's history (back returns here) |
| `ctx.replace_route(url)` | change the URL without adding a history entry |
| `ctx.defer()` | a `Resolver` for async work (see below) |

Lists use `ItemRef` + `iter_with_ref()` + `on_ref()` — no `data-*` index strings for identity.
Plain `.data(…)` payloads on buttons are fine as handler arguments.

## 6. Async actions (`defer` → `Resolver`)

A handler that hands work to a background thread calls `ctx.defer()`. That marks the action
`Loading`, broadcasts it, and returns a `Resolver`:

```rust
#[handler]
fn send(state: &mut App, ctx: &EventContext) {
    let resolver = ctx.defer();
    let token = /* … */;
    tokio::spawn(async move {
        match post(token).await {
            Ok(()) => resolver.ok(),
            Err(e) => resolver.err(format!("hub unreachable: {e}")),
        }
    });
}
```

The triggering control then spins in place and clears (or turns destructive with the message) by
itself: reflection is on by default, `.reflect(false)` opts out.

Identity of an action is keyless — `(session, handler_id, param)` — assembled by the framework,
so you never invent a correlation key. Success **removes** the entry, so the registry only ever
holds in-flight loads and unacknowledged errors and stays bounded; a worker that crashes or
forgets to resolve is failed by a 30 s backstop (`LOADING_TIMEOUT` in `libs/rwire/src/actions.rs`),
and `Actions::drop_session` clears a departing session's entries (`server.rs`, on disconnect).

This is the pattern to imitate for *any* per-connection side table: session-scoped key, removed
on disconnect, bounded by a timeout. See `pitfalls.md` for what happens when you don't.

## 7. Auth

Pick one gate, and only one:

- `.auth(user, password)` — rwire's own login form;
- `.auth_with(DelegatedAuth::new("cookie", validator).login_url(url))` — an external authority
  decides. The async validator receives the request head (`AuthRequest`: cookies, headers) and
  returns `Option<Identity>` (`id` + opaque `data`, typically the token to call the provider on
  the user's behalf). Pages without identity → `303` to `login_url?next=<path>`; WebSocket →
  `401`. Successes are cached for `cache_ttl` (30 s default, bounded by `cache_capacity`), open
  sockets are re-validated every TTL and closed with `4401` once revoked; if the validator is
  unreachable, sockets survive `outage_grace` then close with `1013`, and new requests are
  refused (fail closed). Handlers read `ctx.identity()`.

Both at once makes `run()` fail with `AUTH_CONFLICT`. The validator runs on rwire's tokio
runtime, so it can use tokio clients (reqwest, a hub SDK) directly — but never block the thread.
Never log the cookie.
