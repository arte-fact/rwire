# Renderers that read several states — design study

Status: study, no production code. Source: the mcp-agents trial
(`docs/essai-rwire.md` §5, gap 2, branch `feat/migration-front` of
`games/mcp-agents`). Prototype: `libs/rwire/tests/multi_state_pattern.rs`.

## 1. The problem

A `#[renderer]` takes one state: `fn view(s: &S) -> ElementBuilder`. A view
that needs **per-connection** state (the selected item, the route, a draft)
*and* **shared** state (data published for everyone) has no direct way to read
both.

The trial's view: the selected session lives in each tab's URL
(`CurrentRoute`), the fleet of sessions is the same for everyone and is
refreshed by a background task reading the hub's event stream. Its
workaround:

- the fleet lives in a process-global `RwLock<Option<Fleet>>`, outside rwire;
- every route renderer reads `CurrentRoute` and, inside, takes that lock;
- each fleet snapshot calls `notify_all(TypeId::of::<CurrentRoute>())`.

What that costs:

- **Every route-reading region of every connection re-renders** on each
  snapshot, whether it shows fleet data or not: `notify_all` bypasses the
  dependency masks (the change set is `all`), and the hash dedup only saves
  the wire, not the render.
- **The data is invisible to rwire**: no subscription, no SSR paint of it, no
  per-field dependency tracking, and a renderer that takes a foreign lock
  while the connection renders (see §4.5).
- `notify_all` uses `try_send` on each connection's bounded channel: a slow
  connection may drop the notification and stay stale until the next one.

## 2. How rendering works today

The facts the options have to fit:

| Piece | Today | Where |
|---|---|---|
| Region | `SyncedElement { id, renderer, state_type_id, deps, parent }`: **one** state type, one `RendererDeps` mask | `builder.rs` `SyncedElement` |
| Renderer | `SyncedRenderer::render_with_state(&dyn Any)`, `state_type_id()`, `storage_type()`, `table_name()`, `deps()` | `builder.rs` `trait SyncedRenderer` |
| States at render time | the connection builds `HashMap<TypeId, &dyn Any>`: its memory states, plus the shared/persisted cache copies for every key it knows | `server.rs` `build_type_update`, `render_initial_dom` |
| Which regions re-render | "layer 1": `se.state_type_id == changed_type`; then `deps.needs_update(changes)`; then a content hash | `builder.rs` `build_synced_update_with_known_symbols` |
| Nested regions | a re-rendered parent re-renders its existing children with its **fresh renderers** (a closure child captures values from the parent); children are matched by identity, else by state type in document order | same function, "A nested region's content is a function of its parent's render" |
| Subscriptions | `shared_persisted_keys(handlers, synced)` maps each region's / handler's state type to a cache key (`__shared__:{table}`, `{table}:{session}`) | `server.rs` |
| Discovery | render with default states, collect regions, add defaults for new types, repeat until stable (open, route swap) | `handle_websocket`, `view_state_keys` |
| SSR | `to_static_html_and_styles(state_for)`, `state_for(renderer) -> Option<&dyn Any>` by the renderer's one type | `server.rs` `render_ssr` |
| Locks | one `RwLock` over the shared cache; a render holds its **read** guard; writers (`update_shared*`) take the write guard and **release it before** broadcasting; memory states are owned by the connection task, no lock | `server.rs` |
| Wire | regions are `__synced_<id>` spans; nothing on the wire knows about state types | protocol |

**Key observation.** A *closure* region nested in a renderer already combines
two states. It is documented on `ElementBuilder::synced_with_storage`:

```rust
#[renderer]
fn root(me: &Me) -> ElementBuilder {
    let token = me.token;
    ElementBuilder::synced_with_storage::<Room, _>(
        move |room| board(room, token),
        RendererDeps::always(),
    )
}
```

The outer region depends on `Me`, the inner on `Room`. A `Room` change
re-renders the inner region with the token it captured. A `Me` change
re-renders the outer region, whose fresh closure replaces the inner one's,
and the inner region re-renders with it.

The prototype test reproduces the trial's case with this pattern, on today's
`main`, with no lock and no `notify_all`:
- two tabs on `/s/a` and `/s/b`;
- a shared `Fleet` published from outside;
- each tab sees its own session after a publish, and follows a route change.

## 3. The options

### (a) Several state parameters: `#[renderer] fn f(a: &A, b: &B)`

The macro analyses field accesses **per parameter** (as it does today for the
one), giving one `RendererDeps` per state.

The core gains a multi-read region:
- `SyncedRenderer::reads() -> &[Read]`, where `Read { type_id, storage, table, deps }`;
- `render_with_states(&dyn Fn(TypeId) -> Option<&dyn Any>)`;
- `state_type_id()` stays as the **primary** type (the first parameter), for nested matching and for the existing single-state API.

### (b) A tuple of states: `fn f((a, b): (&A, &B))`

Same internals as (a); only the signature differs. Pattern-matching a tuple
makes the per-parameter dependency analysis harder (field accesses through a
destructured binding), reads worse, and buys nothing. **Rejected.**

### (c) A tracked read: `ctx.read::<B>()` inside a renderer

The renderer keeps one parameter and reads other states through a context
that records which types were read. The region's read set is whatever the
last render touched (dynamic, like signals).

- **Pro:** conditional reads, any number of states, no macro change for the
  signature.
- **Con:**
  - a type first read during a render may not be in the state map yet (a shared state the connection never knew), so `read` must return `Option` and the region needs a second render once it is subscribed;
  - field-level dependencies are lost unless the accessor also tracks fields, so the region re-renders on any change of `B`;
  - the read set changes per render, so subscriptions and the layer-1 filter must be updated after every render;
  - SSR must replay it through `state_for`.

It is the most machinery for the least static knowledge.

### (d) The nested closure region, made the official pattern

Today's mechanism (§2), documented and given a readable entry point. Nothing
changes in the core.

- **Pro:** works now (the prototype); each state keeps its own dependency
  tracking (outer: field mask of `A`; inner: the deps given to the region); a
  publish of `B` re-renders only the inner regions, never the outer;
  subscriptions, discovery, SSR and locks are the existing, tested paths.
- **Con:**
  - verbose: the name `synced_with_storage` and the turbofish hide the intent;
  - values from `A` must be **owned** in the closure (`'static`): clone or compute what is needed. Capturing a whole large `A` means cloning it on each outer render;
  - one extra region (one `<span id="__synced_N">` wrapper) per combination;
  - handlers have the same one-state limit (a handler mutates one state). Not in this study's scope, but it is the next wall an app hits.

## 4. Impact of (a), the only option that changes the core

### 4.1 `RendererDeps` and the update filter
- `SyncedElement` carries `reads: Vec<(TypeId, RendererDeps)>` (the single-state case is a one-element list, so existing regions are unchanged).
- Layer 1 becomes "the changed type is in `reads`", and the mask check uses that read's deps.
- One state change re-renders the region once, as today.

### 4.2 Subscriptions
- `shared_persisted_keys` iterates over every read, not one type per region.
- A region reading two shared states subscribes to both keys; a broadcast of either reaches it.
- No change to `SharedServerState`.

### 4.3 Region discovery
- The discovery loop adds a default for **each** read type it has not seen.
- `view_state_keys` (route swap) and `render_initial_dom` already work on a type → state map, so they take the extra types without change in shape.

### 4.4 SSR
- `render_static_region` resolves each read through `state_for` (memory → default, shared → cache copy, `CurrentRoute` → the requested route) and calls `render_with_states`.
- No new rule.
- A renderer reading `CurrentRoute` and a shared state gets a correct SSR paint, which the trial's global-lock workaround cannot give (rwire doesn't know about that lock).

### 4.5 Locks: acquisition order and deadlocks
- All reads happen inside **one** render, under the **one** shared-cache read guard the connection already holds; memory states are the connection's own. **No new lock, no new acquisition order.**
- Writers take the write guard and release it before broadcasting, so a writer never waits on a renderer while holding it.
- The real deadlock risk is the trial's workaround: a renderer taking an **application** lock (`FLEET.read()`) while rwire holds the cache read guard. The deadlock appears if a task:
  1. holds `FLEET.write()`;
  2. calls `update_shared*` (cache write) while a connection is blocked on `FLEET.read()` inside a render that holds the cache read guard.

  That is a lock-order inversion: cache, then FLEET in the render; FLEET, then cache in the task. The trial's code doesn't do it today (it calls `notify_all`, which takes no cache lock), but nothing stops the next change from doing it.
- Both (a) and (d) remove the reason to take application locks in renderers. The docs should say so explicitly.

### 4.6 Nested regions
- Matching stays keyed by the **primary** type (first read) plus identity.
- A multi-read child is matched like a single-read child of its first type.

### 4.7 Protocol
- **None.** Regions are spans with ids; which states fed a render is server-side only.

### 4.8 Macro
- `#[renderer]` accepts N `&T` parameters (no `&mut`, no tuple), each `T: State`.
- It emits `reads` with per-parameter field masks, and a `render_with_states` that downcasts each.
- Errors:
  - a duplicated type is a compile error;
  - a missing state at render time returns `None`, as today when the one state is absent.

## 5. Comparison

| | Works today | Core change | Per-field deps on each state | Publish of B re-renders | SSR | Lock risk | Ergonomics |
|---|---|---|---|---|---|---|---|
| (a) several params | no | moderate (region reads, filter, subscriptions, discovery, SSR, macro) | yes, per param | the region | yes | none added | best |
| (b) tuple | no | as (a) | harder | the region | yes | none added | worse than (a) |
| (c) `ctx.read` | no | largest (dynamic read sets, second render) | no (unless tracked accessors) | the region | via replay | none added | flexible, implicit |
| (d) nested closure | **yes** | none (docs + a helper name) | outer: yes; inner: given | **only the inner region** | yes | none | verbose |
| trial workaround | yes | — | no | every route region of every connection | **no** | **lock-order inversion possible** | — |

## 6. Recommendation

**Make (d) official now; build (a) on the same foundations when an app needs
the sugar. Do not do (b) or (c).**

The trial's need is met today by (d), with better behaviour than its
workaround:
- a publish re-renders only the nested regions;
- the data is subscribed and painted by SSR;
- no application lock in renderers.

(a) is the nicer API and fits the current architecture without protocol or
locking changes. But it touches the update filter, subscriptions, discovery,
SSR and the macro, all at once, for something users can already express.

### Plan

1. **Document (d)** (docs + `SKILL.md`):
   - "combining per-connection and shared state", with the trial's case;
   - the warning against application locks in renderers (§4.5);
   - the `'static` capture rule.

   Keep `tests/multi_state_pattern.rs` as the regression test of the pattern (this branch).
2. **A readable entry point for (d):** `ElementBuilder::region::<S>(move |s: &S| …)`, storage-aware like `synced_with_storage`, with `RendererDeps::always()`, plus `region_with_deps`. It is a thin alias, no behaviour change.
   - Tests: the prototype written with the alias; the alias's region is subscribed like the long form.
3. **Migrate the trial** (mcp-agents, their side):
   - `Fleet` becomes `#[storage(shared)]`, published with `update_shared_changed`;
   - the route renderers nest a `region::<Fleet>`;
   - remove `FLEET` and `notify_all`.

   Then measure render counts per snapshot before and after.
4. **If wanted, (a)** in three PRs, each green on its own:
   1. Core: `SyncedElement.reads`, layer-1 membership with per-read deps, subscriptions over all reads, discovery defaults per read, SSR per read. Driven by a hand-written two-state `SyncedRenderer` in tests, no macro yet. Tests:
      - each state's change re-renders the region, the other state's untouched fields don't;
      - both shared keys are subscribed;
      - SSR paints both;
      - discovery seeds an unseen second type;
      - single-state regions unchanged (the existing suite).
   2. Macro: N parameters, per-parameter masks, compile errors (duplicated type, `&mut`, tuple), trybuild-style tests.
   3. Docs: (a) as the first choice, (d) as the way to capture computed values or to nest.
5. **Out of scope, noted:** handlers that read several states (same one-state limit); the per-route `<title>` (needs the runtime).

### Test plan summary

| Step | Tests |
|---|---|
| 1 | `multi_state_pattern.rs`: per-tab selection × shared publish, route change followed (in this branch) |
| 2 | the same through `region::<S>`; subscription of the alias's state |
| 4.1 | two-state region: re-render on each state, per-read masks, both subscriptions, SSR of both, discovery of the second type, existing suite unchanged |
| 4.2 | macro: N params, masks per param, compile-fail cases |
