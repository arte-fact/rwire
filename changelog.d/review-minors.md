### Fixed: runtime details from review

- `persist_task` runs SQLite writes on the blocking pool
  (`spawn_blocking`), not on a worker that drives connections.
- The reverse proxy's upstream socket sets `TCP_NODELAY`, like accepted
  sockets.
- A failed `accept()` is logged and the server keeps serving (after a
  100 ms pause) instead of stopping.
- `ServerWithRoot::run_on(listener)` runs on a listener the caller bound
  (port 0 in tests); `integration.rs` and `router_outlet.rs` use free ports
  and no startup sleeps.
