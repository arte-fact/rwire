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

Review round 2 (independent review of this branch) follow-ups, all in
`libs/rwire/tests/nested_regions_update.rs` plus growth-bound comments in
`server.rs`:

- The update-answer assertions are now opcode-level: a small batch parser
  (`wire::parse`) requires the text to be carried by a `SET_TEXT` on a ref
  appended under a `CREATE_SYNCED`/`GET_SYNCED` wrapper of the same batch.
  Raw symbol bytes alone cannot prove the emit fix (symbol collection interns
  even when the span stays empty) — verified by reverting the inline fill: the
  four minted-anew-region tests fail with "not painted by a SET_TEXT inside a
  synced span", then pass again with the fix restored.
- Failing-first coverage of the two server-side changes: a mid-pass-discovered
  region whose memory state mints a distinct token per `Default` must render
  from the connection's seeded copy (token stable across passes; without the
  `reconcile_nested_regions` seeding it changes every pass), and a
  `#[storage(shared)]` state first read mid-pass gets a shared-cache entry +
  subscription in `build_type_update`, so a later background publish reaches
  the client with the real shared copy.
- Measured cascade cost: the cascade walks the whole subtree of a re-rendered
  region regardless of the children's dep masks — a depth-12 chain with
  narrow (`FIELD_LEAF`-only) masks and a `FIELD_TOP`-only change costs exactly
  12 renderer invocations per pass (counter test).
- Growth bound documented at both discovery sites: `states` and
  `subscribed_keys` grow by at most one entry per distinct state type / table
  (a per-app compile-time constant), never per region or per pass; disconnect
  drops them all.

No wire-format change, no API break.
