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
