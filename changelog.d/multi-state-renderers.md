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
- In a multi-state `#[renderer]`, a parameter that is never read contributes an **empty**
  dependency set for that state: the region does not re-render on that state's changes (it
  still renders once, and on whole-change passes such as a route swap). A multi-state
  renderer whose parameters are all unread therefore renders once and only re-renders on
  `ChangeSet::all()` passes. A **single-state** renderer whose parameter is never read
  keeps `always()` — the behaviour from before multi-state renderers existed (see Changed).
- Regions can carry an optional human-readable name — `ElementBuilder::with_name("…")`, and
  `#[renderer]` names its region after the function — printed by `RWIRE_TRACE` as
  `[rwire-trace] se=3 (agent_window) RENDER`.

### Changed: `#[renderer]` dependency analysis and attributes

- A parameter whose only field reads happen inside a macro token stream the analysis
  cannot see into (e.g. `let x = a.x; format!("{x} {}", a.y)`) now falls back to
  `always()` for that parameter, instead of a field mask that would never re-render on
  the macro-hidden read.
- `#[renderer(always)]` is now honored (the attribute was documented but ignored): every
  parameter's read gets `RendererDeps::always()`, so a trigger renderer such as
  `fn clock(_t: &Tick)` re-renders on any change of its state even though it reads no
  field. Any other `#[renderer(...)]` attribute is a compile error.
- A **single-state** `#[renderer]` whose parameter is never read keeps `always()` (the
  behaviour before multi-state renderers existed); the empty-dependency rule applies only
  to an unread parameter of a **multi-state** renderer, whose read gets an empty mask while
  the other parameters keep theirs.
