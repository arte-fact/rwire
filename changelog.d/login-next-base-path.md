### Fixed: login `next` under a base path; `/metrics` off switch

- Under a `base_path`, the delegated login redirect's `next` carries the
  prefix (`/rwire/s/x` → `next=%2Frwire%2Fs%2Fx`), and the loop guard
  compares the login page with the full path.
- `ServerConfig::serve_metrics(false)` turns `/metrics` off entirely (it is
  then an ordinary path); `/health` and `/ready` stay.
