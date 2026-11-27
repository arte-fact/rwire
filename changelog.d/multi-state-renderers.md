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
- Out-of-tree consumer check: `../claw-rwire` is **absent** on this machine (checked next
  to the rwire clone and the usual workspace locations), so the
  `cargo check --manifest-path ../claw-rwire/Cargo.toml` smoke check from CLAUDE.md could
  not be run here. Downstream `mcp-agents` (`crates/web-rwire`) and `life-board`
  (`lot/b-web`) pass against this branch with a throwaway path override.
- Known gap left to the base branch (`fix/nested-regions-update-pass`, T2's functions):
  `build_type_update`'s lazily-created shared-cache entries and `reconcile_nested_regions'`
  state seeding still look only at a region's **primary** read, so a secondary shared or
  memory read of a region first discovered mid-pass is neither cached nor seeded until a
  handler touches that type. Details in MR !2.
