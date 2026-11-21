### Added: `outlet_guard(false)` for a conditional outlet

- The router's startup check (`ROUTER_WITHOUT_OUTLET`) renders regions at their
  default state, so an `outlet()` that only appears in another state (behind a
  login) was refused. `ServerWithRoot::outlet_guard(false)` turns the check off;
  the server then says at startup that no outlet was seen, and routes once the
  outlet renders. The error message names the option and no longer points at an
  internal tool path.
