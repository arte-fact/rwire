### Changed: resting chips rest on nothing; compact chip padding

- An inactive `Chip` has a transparent ground (it was the page's), with the
  hairline and the soft ink; the active chip is unchanged.
- Compact density pads chips `2.4px 9.6px` (`--Qzcy` / `--Qzcx`: the old
  mcp-agents front's `.15rem .6rem` at its 16 px root, in px so an app's own
  root size does not shrink them).
