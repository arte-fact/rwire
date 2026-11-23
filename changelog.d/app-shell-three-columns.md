### Added: a three-column `AppShell` (sidebar, main, panel) with resize handles and thresholds

- **API:** `AppShell::new().sidebar(el).main(el).panel(Option<el>).sidebar_width(264).panel_width(340).collapsed(bool).resizable(on_resize).mobile(MobilePane::List | Main).breakpoints(one_column, drawer)`.
  Any of the new options lays the shell out in a full-height row: the sidebar
  (`St::WShellSidebar`, `--Qsw`), the main column, the panel
  (`St::WShellPanel`, `--Qpw`). Without them, the shell is unchanged (header,
  sidebar, main on a grid).
- **Thresholds:** below 1100 px the panel becomes a drawer over the main
  column; below 760 px one column shows, the one `mobile(…)` names, and the
  panel covers the screen. The theme CSS carries these rules for every shell;
  its one-column threshold follows `Theme::touch_below` when set.
  `.breakpoints(one, drawer)` gives a shell its own (rules scoped by
  `data-shell-bp`). The `sm` breakpoint does not move.
- **Resize handles** (`BIND_RESIZE`, `role="separator"`, `St::ShellHandle`,
  an accent line on hover): a drag sizes the column client-side; on release
  the `resizable` handler gets `ctx.data("pane")` (`sidebar` / `panel`) and
  `ctx.data("w")` (px); a double-click restores the default width and sends an
  empty `w`.
- **Runtime:** a resize handle with `data-rz="n"` sizes its next sibling (a
  right-hand panel), reports the width on release (a click carrying
  `data-w`) and resets it on double-click. Existing handles (`SplitPane`)
  gain the double-click reset. Budget +300 B raw / +100 B gzip.
- **Review fixes:**
  - A plain (native) click on a handle sends nothing; a drag sends once, a
    double-click one reset; the report goes through the handle's own click
    binding only (it no longer bubbles to an ancestor's handler, `SplitPane`'s
    included). `pointercancel` ends a drag; the width stays between 8rem and the
    parent's width minus 256 px; handles that size the next sibling have their
    own binding key.
  - Without a visible sidebar (none, or `collapsed`), a phone shows the main
    column (it could show an empty screen).
  - The one-column threshold never exceeds the drawer's (`touch_below(1200)`,
    `.breakpoints(1200, 1100)`).
  - The rules use child combinators: a shell nested in another's main column
    keeps its own layout.
  - `landmarks(false)` renders the columns as `<div>`s (a shell embedded in a
    page that has its own `<main>`); the panel is `<aside aria-label="Panel">`.
  - **Behaviour change:** `AppShell`'s default sidebar width is 264 px (it was
    260), in the legacy layout too.
  - The drawer is the app's to manage (close with `panel(None)`, focus); see
    the module docs.
