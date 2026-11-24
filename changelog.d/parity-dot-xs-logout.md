### Changed: `StatusDotSize::Xs`; `Icon::LogOut` points into a door on the right

- `StatusDotSize::Xs`: a 7 px dot that never takes a halo (a connection
  indicator; `St::SizeDotXs`).
- **Behaviour change:** `Icon::LogOut` draws the door on the right with the
  arrow going out into it (the old mcp-agents front's), instead of the door on
  the left. `Icon::svg_path()` is now public.
- **Breaking:** `StatusDotSize` gains `Xs` (an exhaustive `match` needs an
  arm).
