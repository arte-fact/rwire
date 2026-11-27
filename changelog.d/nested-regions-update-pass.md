### Fixed: nested synced regions in update passes

Three bugs found by a downstream front's e2e suite, all in how an update
pass walks nested regions (server-level tests over a real WebSocket:
`libs/rwire/tests/nested_regions_update.rs`):

- **A change of an outer state now reaches level 3+.** The re-render cascade
  stopped at a nested region whose own output was unchanged (often just the
  placeholder for the next level), so a grand-child region was re-rendered
  only when its parent's output changed. The cascade now always descends
  into a re-rendered region's nested regions — their renderers are re-created
  by the parent's fresh render and may capture new values — while emission
  is still deduped per region by content hash.
- **A region first created during an update pass no longer renders empty.**
  It can read a memory state the connection never seeded (memory states are
  seeded at first render and on view swap only), and the update pass's state
  map had no entry for it: no symbols were interned and the inline render was
  skipped, leaving an empty span. The pass now renders such a region from a
  fresh default of its state, and `reconcile_nested_regions` seeds the
  connection's state map with every discovered region's type, so later passes
  and handlers on that type find the connection's copy.
- **A region whose position shifts (a sibling inserted before it) renders its
  content.** Positional matching mints the shifted region anew (documented
  for identity-less regions; keyed regions keep their id via
  `with_identity`); the new span is now filled inline from its state instead
  of staying empty.
- **No double render of an unchanged nested region.** A child the cascade
  re-rendered with the parent's fresh renderer — but whose output was
  unchanged — is no longer rendered a second time by the layer-1 pass with its
  stored (older) renderer when its deps match the ChangeSet (CPU-only cost;
  regression test counts the renderer invocations).
- **Shared state first read by a region discovered mid-pass**: the connection
  now lazily creates the shared-cache entry and subscribes to its broadcasts
  when the update pass registers the new region, so the next pass injects the
  real copy (the discovery pass itself renders the state's default, as a seed
  region would).

No wire-format change, no API break.
