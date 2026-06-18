# Reactivity

rwire is a **state-reactivity** framework. The server owns all state; the
browser is a thin renderer. You mutate state, and the framework diffs the
affected `#[renderer]` regions and patches each connected client over the binary
protocol. Per-region content-hash dedup means an unchanged render sends no bytes.

## Renderers bind to a state type

A `#[renderer]` function takes `&SomeState` and returns an `ElementBuilder`. It
is associated with `TypeId::of::<SomeState>()`. When that state type changes, the
region re-renders; regions bound to other state types are skipped.

```rust
#[renderer]
fn render_count(state: &Counter) -> ElementBuilder {
    Text::heading1(state.count.to_string()).build()
}
```

## What triggers a re-render

There are two triggers, both handled by the connection loop in
`libs/rwire/src/server.rs`:

1. **Client events.** A client event runs its `#[handler]`, which mutates state;
   the connection re-renders the regions for that handler's state type and pushes
   the diff.
2. **Broadcasts.** A `BroadcastMsg::StateChanged { key, state_type_id, changes }`
   arriving on the connection's broadcast channel re-renders the regions for
   `state_type_id` and pushes the diff. This is how state mutated **outside** a
   client event reaches the browser.

## Pushing updates from background tasks

For process-global state mutated by a background task (e.g. a metrics poller),
call `SharedServerState::notify_all`:

```rust
let shared = server.shared_state();        // before run()
std::thread::spawn(move || loop {
    update_my_shared_state();
    shared.notify_all(TypeId::of::<MyState>(), ChangeSet::all());
    std::thread::sleep(interval);
});
```

`notify_all` signals every connection to re-render that state type; the framework
diffs and pushes. The data itself must be reachable by the renderer — typically a
shared lock the renderer reads.

## Cross-tab sync for persisted state

For `#[storage(persisted)]` state, a handler mutation marks the cache dirty and
calls `shared.broadcast(key, type_id, changes, except_conn_id=self)`. Other
connections subscribed to that key receive the broadcast and re-render, so edits
in one tab appear in others.

## Nested renderers

Nested synced regions (a renderer inside another, even bound to a different
state) are supported via the `CREATE_SYNCED` opcode. When a parent re-renders it
re-emits each nested child with its original wrapper id, so the child keeps its
value and stays updatable — it is not destroyed by the parent clearing its
children. Verified at the protocol level by `libs/rwire/tests/nested_renderer.rs`
and end-to-end in the browser by the `examples/nested` app (bump the outer state,
confirm the inner region survives and still updates). Flat, sibling renderers
remain the simplest pattern and are fine to prefer.

## Tree-shaking

There is **no tree-shaking gap** and nothing to declare manually. The capsule is
split by token kind:

- **Small `u8` enums** (`El`/`Ev`/`At`/`Av` + inline style prop/value maps) are
  **shipped whole** — the full set is ~1-2 KB, and a missing entry would be a
  structural break, so tree-shaking is not worth the risk there.
- **CSS rules** (`.u`/`.h`/`.b` utility/pseudo/breakpoint classes) are delivered
  **lazily over the wire**: the static capsule ships only globals (reset, all CSS
  variables, theme, keyframes, composites), and each class rule is sent via the
  `STYLE_DEF` opcode the first time a connection actually references it, deduped
  per connection (`ConnectionState.sent_css`). The set is therefore exact and
  automatic — a rule used only in a deeply-nested plain helper still arrives the
  moment that branch first renders.

Because `sent_css` lives exactly as long as the WebSocket connection (and the
client's dynamic stylesheet), a hard refresh starts a fresh connection with an
empty set and re-receives every rule in the initial batch — they cannot drift.

Composites (`.c{id}`) are detected by a one-time startup analysis; their id set is
fixed there, so their CSS stays static and is never missing. The full design and
history is in `docs/tree-shaking-redesign.md`.
