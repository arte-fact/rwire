### Changed: no application in the libraries, and a gallery rule

- **`StatusDot` docs** demo `.intent(StatusDotIntent::Warning)` for a pending decision,
  instead of the raw palette index `Tone::Cat(5)`.
- **`MobilePane::List` is now `MobilePane::Sidebar`** (breaking, no wire change: the
  `data-mobile` value stays `list`). The enum names the column a phone shows, not the
  content one app puts in it.
- Component docs describe a default by its role (`The default brand row:`), not by
  reference to the front it was measured on.
- **New guard, `tests/generic_api.rs`**: (1) no library source may name an application
  (its product name, host or paths); (2) every module declared in `lib.rs` must appear
  in the gallery `src/catalog.rs`, or be exempted in `NO_DEMO` with a reason. An
  affordance only one app needs is a design flaw, not a feature.
