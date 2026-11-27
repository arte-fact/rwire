### Added: multi-state renderers — a region can read several states

- `#[renderer]` accepts N `&State` parameters (`fn panel(route: &CurrentRoute, fleet: &Fleet)`),
  with the field-dependency analysis run **per parameter**: a change to a field one
  parameter does not read does not re-render the region, and one state's change
  re-renders it once. The first parameter stays the region's primary type (nested
  matching, single-state accessors). A duplicated state type, a `&mut` parameter, or a
  tuple parameter is a compile error. A state missing at render time yields `None`, as
  before.
- Core: `SyncedRenderer::reads()` (one `Read { type_id, storage, table, deps }` per state)
  and `render_with_states(&dyn Fn(TypeId) -> Option<&dyn Any>)`; `SyncedElement::reads()`.
  The update filter's layer 1 is now "the changed type is among the region's reads" with
  the matching read's mask; subscriptions (`shared_persisted_keys`), discovery, route-swap
  seeding and SSR resolve **every** read, so a region reading two shared states subscribes
  to both keys and paints both in SSR. No wire/protocol change and no new locks: regions
  stay `__synced_<id>` spans, all reads happen under the one shared-cache read guard the
  connection already holds.
- `ElementBuilder::region::<S>(closure)` and `region_with_deps` — readable aliases of
  `synced_with_storage` with `RendererDeps::always()` / an explicit mask.
- `ElementBuilder::synced_multi(specs, render)` and the `Read` / `ReadSpec` types back the
  macro expansion for hand-written multi-state renderers.
- `#[renderer]` parameter that is never read now contributes an **empty** dependency set for
  that state (the region does not re-render on that state's changes) instead of `always()`;
  `always()` is kept only when a parameter is used in a way the analysis cannot see at all
  (e.g. only inside `format!("{:?}", state)`). A renderer whose parameters are all unread
  renders once and only re-renders on `ChangeSet::all()` passes (e.g. a route swap).
- Regions can carry an optional human-readable name — `ElementBuilder::with_name("…")`, and
  `#[renderer]` names its region after the function — printed by `RWIRE_TRACE` as
  `[rwire-trace] se=3 (agent_window) RENDER`.
