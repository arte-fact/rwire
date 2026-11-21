### Changed: delegated auth hardening

- **Bounded validations**: one validation per cookie at a time (concurrent
  requests share it), at most `max_concurrent_validations` (default 16) at
  once, each within `timeout` including the wait for a slot (fail closed).
  A validation runs in its own task, so it ends and frees its slot even
  when every request waiting on it is dropped.
  A WebSocket handshake over the admission caps is refused before its cookie
  is validated.
- **Negative cache**: a rejected cookie is remembered for `negative_ttl`
  (default 5 s; `Duration::ZERO` turns it off) instead of re-asking the
  validator on every request. Timeouts and outages are never remembered, and
  a rejection never replaces a success.
- **Cache keys** are the SHA-256 of the cookie, never the cookie; expired
  entries are swept once per TTL (new dependency: `sha2`).
- **Outages vs revocations**: `Validation { Valid, Revoked, Unavailable }`
  and `DelegatedAuth::with_validation`. An open WebSocket closes with 4401
  on a revocation only; while the validator is unreachable it stays up for
  `outage_grace` (default **5 min**) from its last confirmation, then closes
  with 1013 (`CLOSE_HUB_UNAVAILABLE`). New requests stay refused during an
  outage. `DelegatedAuth::new` keeps its contract: `None` is a revocation.
- **`/metrics` is behind the auth gate** when `auth()` or `auth_with()` is
  set (public without a gate, as before); `ServerConfig::public_metrics(true)`
  exposes it. `/health` and `/ready` stay public. Both routes answer `GET`
  and `HEAD` of `/metrics`, with or without a query.
- Re-validation keeps only the session cookie of the handshake; anonymous
  visits are not logged, refused cookies and validator outages are, each
  at most once a minute and under its own message; a
  same-origin login URL is answered 401 instead of redirecting to itself.
