### Added: connection lifecycle (`on_connect`, `on_disconnect`, `ctx.spawn`)

- `ServerWithRoot::on_connect(handler)` runs a `#[handler]` once per WebSocket
  connection, after its states are ready and before its first render: what it
  writes is in the first paint. Its `EventContext` carries the session id and,
  under delegated authentication, the identity. Until now an app reached these
  through `on_route` or the `DelegatedAuth` validator.
- `ServerWithRoot::on_disconnect(handler)` runs once at close, whatever the
  cause: the client left, heartbeat, re-validation, an error, a panicking
  handler, or `run_until`'s shutdown. On shared state it broadcasts like an
  event (presence).
- `EventContext::spawn(fut)` starts a tokio task owned by the connection. The
  task is aborted when the connection closes, before `on_disconnect` runs. It
  returns `false` and drops the future when no live connection owns the
  context (`on_disconnect`, or a hand-built context).
