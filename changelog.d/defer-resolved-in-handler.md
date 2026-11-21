### Fixed: a deferred action resolved before its handler returns

- `ctx.defer()` followed by `resolver.ok()` / `resolver.err(..)` before the
  handler returned (inline, or by work the handler waited for) was lost: the
  server marked the action loading only after the handler returned, over the
  resolution, and the control spun until the 30 s backstop failed it with
  "timed out". Such a resolution is now kept and applied when the handler
  returns, with no loading phase: success leaves the control idle, an error
  shows at once. Resolving after the handler returns, and the timeout for an
  action never resolved, are unchanged. No API change.
