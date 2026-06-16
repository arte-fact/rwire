# Concern 2: collect_tokens_from Ignores Renderers

## Problem

`collect_tokens_from()` walked element trees for router views but skipped synced elements (renderers). Any renderer inside a router page had its tokens silently omitted.

## Solution

`collect_tokens_from()` now handles synced elements:

1. Reads the compile-time `token_inventory()` (covers all branches)
2. Creates a default state and renders the synced element (discovers runtime tokens from called functions)
3. Recurses into the rendered output

This gives router views the same tree-shaking coverage as the root element tree.

### Files Modified

- `libs/rwire/src/builder.rs` — `collect_tokens_from()` synced element handling
