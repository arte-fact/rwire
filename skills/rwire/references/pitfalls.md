# rwire — known traps

Each of these has already cost a debugging session somewhere. Read this before debugging
anything that "should work".

## 1. An outbound POST with no timeout

Found in review of `games/mcp-agents` MR !29 (`feat/visual-parity-e3`), in
`crates/web-rwire/src/ui.rs`.

The file has a helper `post_json(…)` that takes a `timeout` and applies `.timeout(…)`. Next to
it, `post_raw(…)` and the local `post(…)` build the same request — `.post(url).bearer_auth(token)
.json(&body).send().await` — **without** `.timeout(…)`. So the prompt relay, the interrupt relay
and the permission relay can hang indefinitely: the handler waits, the `Resolver` never
resolves, and the UI sits there until the 30 s action backstop fires or forever if the call is
not deferred.

Rule: **every outbound HTTP call gets an explicit timeout**, and a size for the response you are
willing to read. Then map the outcome to `Resolver::err` — a hung request is an error you must
report, not a state you leave the UI in. `reqwest`'s per-client default is not a safety net you
should rely on.

## 2. Process-global state shared between connections

Same review. `crates/web-rwire` keeps its per-user data in process globals:

| Where | What |
|---|---|
| `outbox.rs` | `static BOXES: Mutex<Option<Boxes>>` — pending prompts and stop requests |
| `conv.rs` | `static CONVS: Mutex<Option<HashMap<String, Conv>>>` — conversations |
| `conv.rs` | `static OLDER: Mutex<Option<HashMap<u32, String>>>` — pagination tickets |
| `pane.rs` | `static PANES: Mutex<Option<HashMap<String, Pane>>>` — open panes |
| `fleet.rs` | `static FLEET: RwLock<Option<Fleet>>` — fleet snapshot |

Keyed by *agent session*, not by *web connection*. Two browsers looking at the same session share
the same outbox: a message typed by one shows "envoi…" in the other, and "Renvoyer" / "Retirer"
act on someone else's pending send. It is not a leak of credentials, but it is one visitor's
state rendered for another.

The framework already models this correctly — imitate it:

- `Actions` keys every in-flight action by `ActionKey { session, handler_id, param }`
  (`libs/rwire/src/actions.rs`), and `server.rs` calls `Actions::drop_session(session)` when a
  connection goes away.
- `State` with `#[storage(memory)]` is per-connection by construction. `#[storage(shared)]` is
  the only process-global kind, and it is for server-wide facts (metrics, fleet snapshots), not
  for what a visitor typed.

Rule: a hand-rolled side table is keyed by `ctx.session_id()` and purged when that session
disconnects. If you cannot purge it, it does not go in a `static`.

## 3. Tables that are never pruned

Same review. `OLDER` grows one entry per pagination ticket, with `TICKET` an ever-increasing
`AtomicU32`; nothing ever removes an entry. `BOXES.out` accumulates one `Vec<Out>` per session
name and is only emptied one message at a time, never per departing session. Same shape for
`CONVS` and `PANES`. The process is long-lived, so this is unbounded memory growth driven by
uptime and traffic.

The framework's own bounded tables show the pattern:

- the disconnected-session state cache has a **cap** on how many it retains and a **TTL**
  (`server.rs`: the retained-session bound, `evict_expired_sessions`, and the background
  eviction task, 300 s by default);
- `Actions` holds only in-flight loads and unacknowledged errors, removes entries on success, and
  fails a stuck load after 30 s (`LOADING_TIMEOUT`) — bounded by design, not by hope;
- auth tokens are retained only within `AUTH_TTL`.

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

## 7. Shared-state renderers inside memory-state ones

A renderer on `#[storage(shared)]` nested under a parent on `#[storage(memory)]` does not receive
the shared cache. Keep shared renderers nested under shared parents.

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

Measured at the commit this skill ships with: `README.md` says "90+ components",
`libs/rwire-components/src/catalog.rs` holds 64 catalogue entries, `lib.rs` exports 71 names. The
older in-repo skill says "~55". Recount in the code before repeating any of them.
