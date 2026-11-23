# rwire — known traps

Each of these has already cost a debugging session somewhere. Read this before debugging
anything that "should work".

## 1. An outbound call with no timeout

A handler (or the task it defers to) that calls out over HTTP without an explicit timeout can
hang: the `Resolver` never resolves and the control spins until the 30 s action backstop
(`LOADING_TIMEOUT`) fails it — or forever if the call was not deferred at all.

Rule: **every outbound HTTP call gets an explicit timeout**, and a bound on the response you are
willing to read. Map the outcome to `Resolver::err(msg)`: a hung request is an error you report,
not a state you leave the UI in. Do not count on a client library's default.

Seen in: the mcp-agents web front (`crates/web-rwire/src/ui.rs`, review of MR !29, November
2026) — `post_raw` and `post` lacked the `.timeout(…)` that `post_json` had; the MR branch has
since added one.

## 2. Process-global state shared between connections

A `static` table keyed by something other than the web connection (an agent session, a
document id…) is shared by every browser that looks at the same thing: one visitor's pending
send, draft or open pane shows up — and can be acted on — in another visitor's page.

The framework already models this correctly — imitate it:

- `Actions` keys every in-flight action by `ActionKey { session, handler_id, param }`
  (`libs/rwire/src/actions.rs`), and `server.rs` calls `Actions::drop_session(session)` when a
  connection goes away.
- `State` with `#[storage(memory)]` is per-connection by construction. `#[storage(shared)]` is
  the only process-global kind, and it is for server-wide facts (metrics, fleet snapshots), not
  for what a visitor typed.

Rule: a hand-rolled side table is keyed by `ctx.session_id()` and purged when that session
disconnects. If you cannot purge it, it does not go in a `static`.

Seen in: the mcp-agents web front, same review — `static BOXES` (`outbox.rs`), `CONVS`
(`conv.rs`), `PANES` (`pane.rs`), keyed by agent session, not by connection.

## 3. Tables that are never pruned

A long-lived process plus a table that only grows (one entry per ticket, per session name, per
page opened) is unbounded memory growth driven by uptime and traffic.

The framework's own bounded tables show the pattern:

- the disconnected-session state cache: a **cap** (`MAX_CACHED_SESSIONS`, 10 000) and a **TTL**
  (one day, enforced by `session_eviction_task` → `evict_expired_sessions` in `server.rs`);
- `Actions` holds only in-flight loads and unacknowledged errors, removes entries on success, and
  fails a stuck load after 30 s (`LOADING_TIMEOUT`) — bounded by design, not by hope;
- login tokens are retained only within `AUTH_TTL` (`server.rs`).

Rule: every table a handler can grow needs three things — a **key you can delete**, a **cap or
TTL**, and the **hook that fires** (session drop, TTL sweep). If you can only answer two of the
three, it is a leak with a delay.

## 4. Stacking `St` tokens

- A later `.st()` token **replaces** an earlier one whose every property it sets again:
  `.st([DisplayFlex])` then `.st([DisplayNone])` hides the element.
- On a **partial** overlap both stay, and the shared properties go to the higher token code; the
  stylesheet is kept in code order on every page.
  - `BorderNone` then `BorderB2Accent`: works.
  - `FontInheritAll` then `TextXs`: the size is lost. A debug build prints
    `rwire: st([TextXs]) loses font-size…` once per pair.
- Pick non-overlapping tokens. Don't put `FontInheritAll` on form controls — the base reset
  already applies `font: inherit`.

## 5. Bottom-pinned chat scroll

`column-reverse` plus an **auto bottom margin** on the inner column (`St::MbAuto`) — **never**
`justify-content` on the scroll container. `justify-content` makes overflowing content
unreachable; an auto margin collapses to 0 on overflow. `ChatScroll` encodes exactly this; use
it.

## 6. Width collapse

A width-spanning child of a flex-*centered* parent needs `St::WFull`, or it shrinks to content —
which is ~0 for an empty div. Symptom: an element that renders fine once it has text and
vanishes while loading.

## 7. Shared-state regions inside memory-state renderers

To show shared state inside a per-connection view, nest a **closure region** the documented way:
`ElementBuilder::synced_with_storage::<Room, _>(move |room| view(room, token), RendererDeps::always())`
(`CLAUDE.md`, "Multiplayer"; `../empire`'s `empire-web` is the reference). An older note in this
repository said a shared `#[renderer]` nested under a memory one "does not receive the shared
cache"; it was not re-verified on current `main` — prefer the closure region, and test the nesting
if you rely on anything else.

## 8. The 64-field `State` ceiling

Renderer dependencies are a `u64` mask, one bit per field; `#[derive(State)]` rejects a struct
with more than 64 fields (`MAX_STATE_FIELDS` in `libs/rwire-macros/src/lib.rs`). Split the view
model — and re-export flat so callers don't churn.

## 9. Keyed swaps when the whole region changes subject

When switching what an entire region shows (another agent's thread, another machine's detail),
key the container `id` by the subject. The client then swaps clean instead of diffing across
subjects and leaving stale spans.

## 10. Ordering traps in builders

- On `Button`, set `title` / `data` / `aria_label` / `class` / `hover` / `attr` **before**
  `on_click`; chained after, they land on the box-less reflection wrapper and debug builds panic.
- A `Router` with no `outlet()` anywhere in its page tree makes `run()` refuse to start
  (`ROUTER_WITHOUT_OUTLET`) — better than a page whose navigation silently freezes.
- Enter-to-submit on a textarea: set `data-enter-submit`; do not bind keydown yourself, rwire
  `preventDefault`s bound keydowns and you lose typing.

## 11. Numbers in the docs drift

Measured on `main`: `README.md` says "90+ components", `libs/rwire-components/src/catalog.rs`
holds 69 catalogue records (five of them guide pages), `lib.rs` has 71 `pub use` lines (151 exported names). The older
in-repo skill says "~55". Recount in the code before repeating any of them.
