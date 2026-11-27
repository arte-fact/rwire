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

### Review fixes (independent review of this MR)

- **Nested multi-state regions now update when their parent re-renders**: the cascade in
  `build_synced_update_with_known_symbols` rendered matched children with the single-state
  `render_with_state`, which returns `None` for a multi-state region (its secondary reads
  stay unresolved) — while the id had already landed in `cascade_touched`, so the outer loop
  skipped the child and it stayed stale (0 bytes) next to a correct single-state sibling.
  The cascade now renders through `render_in_pass` (every read resolved from the pass map or
  a fresh default of that read). `IdentityRenderer` (the `with_identity` wrapper) also
  delegates `reads()` / `render_with_states` / `default_state_for` now, so a multi-state
  region keeps all its reads through the wrapper. Test (failed first):
  `a_nested_multi_state_region_is_updated_by_the_parent_cascade`.
- `#[renderer(always)]` is implemented (it was documented but the attribute was ignored):
  every parameter's read gets `RendererDeps::always()`, so a trigger renderer such as
  `fn clock(_t: &Tick)` re-renders on any change of its state even though it reads no
  field; any other `#[renderer(...)]` attribute is a compile error. Tests:
  `the_renderer_always_attribute_forces_always_on_every_read` plus macro unit tests.
- Field reads inside macro token streams are no longer lost: the per-parameter analysis now
  scans macro tokens for the parameter (`ident_used_in_token_stream`), so
  `let x = a.x; format!("{x} {}", a.y)` falls back to `always()` for that parameter instead
  of a `{x}` mask that would never re-render on an `a.y` change. Tests:
  `a_field_read_only_inside_a_macro_falls_back_to_always` plus `renderer_analysis_tests`.
- `SyncedElement::reads()` now returns a borrowed `&[Read]` cached at construction — no
  per-region `Vec` allocation per update pass — and `render_in_pass` takes that cached
  slice. **API break:** `SyncedElement::reads()` changed from `Vec<Read>` to `&[Read]`
  (the API is introduced by this branch, so nothing outside it consumes the old shape).
- Out-of-tree consumer check: `claw-rwire` **exists** at github.com/arte-fact/claw-rwire
  (the earlier fragment wrongly claimed it was absent). Cloned, its `path = "../rwire/..."`
  deps pointed at this branch's head with a throwaway symlink override (never committed
  downstream): `cargo check --all-targets` **passes with 0 warnings**. Downstream
  `mcp-agents` (`crates/web-rwire`) and `life-board` (`lot/b-web`) pass against this
  branch with a throwaway path override.

### Re-review round 3 fixes (independent re-review of this MR)

- **Secondary reads of a mid-pass-discovered region are now seeded/cached**
  (`build_type_update` + `reconcile_nested_regions` previously looked only at a region's
  **primary** read). `reconcile_nested_regions` iterates `region.reads()` and seeds the
  connection's state map with every read's `default_state_for`; the mid-pass shared-cache
  entry is created for every read (found via `reads()`, default via
  `create_default_state_for`) and now covers persisted `{table}:{session}` keys too, not
  only `__shared__:` ones, and the connection is subscribed. Without this, a multi-state
  region discovered mid-pass whose secondary memory state has a per-default-distinct
  `Default` rendered from a fresh throwaway default on every pass — the emit-pass text was
  never interned, so the region's span stayed **empty** on the wire. Failing-first tests
  (each verified to fail with the fix reverted): `..._seeds_its_secondary_memory_read`
  (counter `Default` starting at 7, stable across passes),
  `..._caches_and_subscribes_its_secondary_shared_read`,
  `..._caches_its_secondary_persisted_read` in tests/nested_regions_update.rs.
- **IdentityRenderer (`with_identity`) delegation is now tested**: a keyed multi-state
  region (`#[renderer]` with 2 state params wrapped in `with_identity`) re-renders on a
  secondary-state change, on a primary change with the secondary read missing from the
  pass map (`default_state_for`), and a prepend keeps the keyed children's wire ids while
  the parent cascade re-renders them through the wrapper. Removing any of the
  `reads()` / `render_with_states` / `default_state_for` delegations fails these tests
  (verified individually). Tests: `a_keyed_multi_state_region_rerenders_when_any_of_its_states_changes`,
  `a_prepend_keeps_keyed_multi_state_ids_and_the_cascade_renders_them` in
  tests/multi_state_renderers.rs.
