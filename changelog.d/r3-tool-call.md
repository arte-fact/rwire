### Added: `ToolCall` (R3, iteration 4) — a tool invocation card in a conversation

- `ToolCall::new(name, command).status(ToolStatus::Running | Ok | Err).output(text)`:
  a native `<details>` **closed by default**, radius 10 (`--R3`), 13 px, on the
  raised ground with a hairline border.
- The summary holds on one line: the name in weight 650, the command in mono
  with `text-overflow: ellipsis`, the status on the right (`Running` muted,
  `ok` green, `err` red). The opened body is the output in a `pre` capped at
  320 px (`St::MaxH320`), scrolling inside — a long line never scrolls the
  page.
- Serves as a `ChatItem::row()` or a `ChatEntry::detail` — it builds a plain
  `ElementBuilder`. New tokens: `MaxH320`, `PadToolSummary`.
- Gallery: a "ToolCall" entry (variants Running/Ok/Err) and ok + err tool
  calls in the Conversation demo. Measured at 1280/375 light/dark: closed by
  default, radius 10, 13 px, one-line summary (padding 8×12), status ok/err
  in the success/error ink, pre max-height 320 px with scrollWidth 1326 >
  clientWidth while the page scrollWidth == innerWidth.
