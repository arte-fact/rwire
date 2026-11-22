### Fixed: renderer wrappers add no box (static paint and live page alike)

- A synced region (a `#[renderer]`'s output) is wrapped in a `<span>`. The
  static paint (SSR) wrote a `<div>` instead, so the same page laid out
  differently before and after the WebSocket's first frame, and both wrappers
  were boxes of their own: a region inside a grid or a flex row became one
  item, breaking the layout (mcp-agents patched it with its own
  `display: contents` rule).
- The static paint now writes `<span data-rwr>`, like the live
  `<span id="__synced_N">`, and the base CSS makes both `display:contents`
  when they carry no class: `[id^=__synced_]:not([class]),[data-rwr]:not([class])`.
  A wrapper styled with `.st(…)` (`render_x().st([St::Flex1])`) keeps its box.
- **Behaviour change:** an unstyled region no longer creates a box. A layout
  that relied on the wrapper being an inline (live) or block (static) box
  needs a styled wrapper (`.st(…)` on the region).
