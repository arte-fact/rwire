# rwire — known traps

Each of these has already cost a debugging session somewhere. Read this before debugging
anything that "should work". Every section ends with where it was seen: a commit, a review, or a
dated report.

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
(`conv.rs`), `PANES` (`pane.rs`), keyed by agent session, not by connection; and `WIDTHS` /
`LAYOUT` / `SENT` moved into a per-connection `Ui` state only after review (mcp-agents
`a3e5a7b`, 2026-11-23).

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

Seen in: rwire `7b62fa9` "fix(rwire): keep disconnected sessions for a day" (2026-08-31), the
model to copy; and the mcp-agents front, where `outbox` and `infos` had no pruning until
`a3e5a7b` (2026-11-23).

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

## 6. Width collapse, and wrappers that have no box at all

A width-spanning child of a flex-*centered* parent needs `St::WFull`, or it shrinks to content —
which is ~0 for an empty div. Symptom: an element that renders fine once it has text and vanishes
while loading.

The same symptom has a second cause: **a synced wrapper has no box**. A synced region is emitted
as a `<span data-rwr>` and, without classes, the base CSS makes it `display: contents` — in the
static paint as in the live page (`builder.rs`, the `region` branch of the static renderer).
Action reflectors are the same by construction: `.st([St::DisplayContents]).boxless(…)`
(`actions.rs`). Ask one of those for a width, a padding or a border and nothing visible happens;
set an attribute or a style on a boxless builder and a debug build panics (`builder.rs`, the
`boxless` field exists precisely to catch that).

Rule: put layout on real components, and measure in the browser at 1280 **and** 375 px before
assuming a wrapper has geometry.

Seen in: claw-rwire `0b5c722`, restoring uniform tab panels "now that rwire nested removal is
fixed" — a workaround for a framework bug that was later fixed (claw-rwire, 2026-07).

## 7. One state per handler and per renderer

A `#[handler]` and a `#[renderer]` take **exactly one** state parameter. Wanting two is a type
error, not a design discussion.

Read the second state through a nested closure region:

```rust
ElementBuilder::synced_with_storage::<Room, _>(
    move |room| view(room, token),
    rwire::RendererDeps::always(),
)
```

(`CLAUDE.md`, "Multiplayer"; `../empire`'s `empire-web` is the reference implementation.)

An older note in this repository said a shared `#[renderer]` nested under a memory one "does not
receive the shared cache"; it was not re-verified on current `main` — prefer the closure region,
and test the nesting if you rely on anything else.

Seen in: web-rwire, MR !29 (games/mcp-agents, 2026-11-23) — the pattern used to render a shared
room inside a per-connection page.

## 8. The 64-field `State` ceiling

Renderer dependencies are a `u64` mask, one bit per field; `#[derive(State)]` rejects a struct
with more than 64 fields (`MAX_STATE_FIELDS` in `libs/rwire-macros/src/lib.rs`). Split the view
model — and re-export flat so callers don't churn.

## 9. Keyed swaps when the whole region changes subject

When switching what an entire region shows (another agent's thread, another machine's detail),
key the container `id` by the subject. The client then swaps clean instead of diffing across
subjects and leaving stale spans.

This is about the **container**; see §12 for the fields inside it, where the opposite holds.

## 10. Ordering traps in builders

- On `Button`, set `title` / `data` / `aria_label` / `class` / `hover` / `attr` **before**
  `on_click`; chained after, they land on the box-less reflection wrapper and debug builds panic.
- A `Router` with no `outlet()` anywhere in its page tree makes `run()` refuse to start
  (`ROUTER_WITHOUT_OUTLET`) — better than a page whose navigation silently freezes.
- Enter-to-submit on a textarea: set `data-enter-submit`; do not bind keydown yourself, rwire
  `preventDefault`s bound keydowns and you lose typing.

## 11. Folded/open state re-closes when a neighbour updates

Collapse state — a tool-call block, a detail panel, a row — kept at screen level is reset as soon
as any other state of that screen re-renders. The user opens a block, something else changes, the
block closes on its own.

Rule: keep the state **per element, keyed by the item id, in its own state region**. Never derive
it from the whole conversation or the whole page state, and never store one boolean for a list of
foldable things.

Seen in: Benoît's preview feedback on MR !29 (games/mcp-agents, 2026-11-23) — chat tool-call
blocks re-closed when other attributes of the conversation updated.

## 12. Do not re-key synced fields on every render

Re-keying a synced input — a fresh `id` or key on every render — stamps the **stale value and
caret** onto the newly created element: text jumps back, the cursor lands at the end, or a
half-typed word is lost.

Rule: a synced field keeps a stable key, and the runtime moves it and restores focus on reorder.
Rely on that instead of fighting it; add a key only when the *subject* changes (§9), not when the
value does.

Seen in: rwire `e4fcc73` "fix(runtime): stale value/caret stamped onto re-keyed editor fields"
(2026-07-07); the neighbourhood also produced `3ee6270` (caret restored on undo/redo) and
`ba42f64` (explicit multiline textarea geometry).

## 13. `<select>` values are not read uniformly

Read every `<select>` change through `ctx.text()` — one contract for every select, rather than
a payload shape per call site.

Seen in: claw-rwire `6d20b19` (2026-07-07).

## 14. A drawer that must remember its state is a renderer

A drawer, panel or popover wired outside a `#[renderer]` can stop responding: it does not
re-render when the state it shows changes.

Rule: the component owns its dependencies as a renderer — it is re-rendered because what it
shows changed, not because the page did.

Seen in: claw-rwire `661e148` — "the CI logs drawer opens again — it owns its deps as a
`#[renderer]`" (2026-07-09).

## 15. Markdown: no remote URLs by default

`rwire-markdown` does **not** load images from another origin: `remote_images` is `false`
(`libs/rwire-markdown/src/markdown.rs`), and turning it on with `.remote_images(true)` lets a
third-party server learn who reads the page and when. Link and image targets pass an allowlist
(`libs/rwire/src/url_safety.rs`): relative paths, `?query`, `#anchor`, `http:`, `https:`,
`mailto:` — images also accept raster `data:image/*`. Everything else is dropped, silently.

Rule: never inject a remote URL into rendered markdown or into a pre-rendered attribute, and do
not "fix" a missing image by enabling remote loading without deciding who gets told about your
readers.

Seen in: rwire `3c6f616` "fix(markdown): no remote images by default; links keep http/https/mailto
only" (2026-11-21); the attribute denylist (`is_executable_attr`) came with PRs #14 and #19.

## 16. Every pane gets a 375 px and a keyboard pass

A panel that works on a desktop can be unusable on a phone and unusable from the keyboard, and
neither shows up in a unit test.

Before a merge, for each drawer, panel and modal:

- at 375 px: it opens, it is reachable, it does not push a horizontal scrollbar, and its close
  control is tappable (≥ 44 × 44 px — `parity.mjs` fails below that unless the `/app`
  reference is just as small);
- from the keyboard: Escape closes it, focus enters it and comes back where it was, and the close
  control is in the tab order.

Seen in: preview feedback on MR !29 (Benoît, 2026-11-23) — right panel not openable on mobile,
broken close-button style on the left panel, broken session↔`/` navigation; and PR #23, where the
drawer had no backdrop, Escape or focus handling at all.

## 17. Numbers in the docs drift

Measured on `main`: `README.md` says "90+ components", `libs/rwire-components/src/catalog.rs`
holds 69 catalogue records (five of them guide pages), `lib.rs` has 71 `pub use` lines (151
exported names). The older in-repo skill says "~55". Recount in the code before repeating any of
them.
