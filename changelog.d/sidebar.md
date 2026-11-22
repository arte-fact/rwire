### Added: `Sidebar` (a full-height column with slots)

- `Sidebar::new().header(el).search(el).filters(el).body(el).footer(el).aria_label(…)`:
  an `<aside>` on the theme's sidebar ground (`St::BgSidebar`), full height.
  The body takes the remaining height and scrolls on its own; the footer stays
  at the bottom, over a hairline.
- Safe areas: the column clears the left inset (`St::PlSafe`), the header the
  top one (`St::PtSafe`), the footer the bottom one (`St::PbSafe`).
- The footer is padded like the old mcp-agents front: 10 px 12 px, the bottom
  safe area added (`St::PtSidebarFoot`, `St::PbSidebarFootSafe`).
