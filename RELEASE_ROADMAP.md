# rwire Public Release Roadmap

Tracking document for taking rwire from "excellent private codebase" to a credible public
0.1 release. Sourced from a full framework review (2026-07-02): architecture, protocol,
runtime, macros, security posture, docs, tests, and the consumer apps
(`../claw-rwire`, `../llama-modnitor-rwire`).

**Verdict:** engineering maturity is unusually high for a 5-month project (audit trail,
round-trip tests, docs site, live dogfooding). What's missing is almost entirely release
*packaging* (Phase 2, mechanical) plus two adopter-facing gaps (T1, T2) that should land
before any announcement because they're the first things a skeptical evaluator will test.

**Suggested sequence:** Phase 1 (housekeeping) → Phase 2 (mechanics) → T1 + T2 →
announce as experimental 0.1 with remaining items stated as known limitations.

## Baseline (already release-grade — do not regress)

- Security audit 2026-06-29: 15/16 findings resolved (CSPRNG session IDs, admission
  control, WS frame-size limits, event rate limiting, constant-time auth compare,
  `HttpOnly; SameSite=Strict` + auto-`Secure` cookies). See `AUDIT_ROADMAP.md`.
- Binary decoder bounds-checked, length-validated, 64KB payload cap, no reachable panic.
- Zero `unsafe` in the framework; zero clippy warnings across 14 crates.
- Cross-language wire tests: Rust encoder → real JS parser (`tests/wire_roundtrip.rs`).
- 26-page docs site; ops story (health/ready, Prometheus, Docker, PWA, reconnect overlay).
- No protocol-compat burden post-release: the JS runtime ships from the same binary that
  speaks the protocol, so the wire format can keep churning freely.

## Status legend

`[ ]` todo `[~]` in progress `[x]` done `[-]` won't fix / deferred (note why)

## Progress

| Phase | Total | Done |
|-------|-------|------|
| 1 — Workspace & consumers | 3 | 0 |
| 2 — Release mechanics | 5 | 0 |
| 3 — Technical gaps | 6 | 0 |
| 4 — Positioning & launch | 3 | 0 |
| **All** | **17** | **0** |

---

## Phase 1 — Workspace & consumers

### W1 — Migrate llama-modnitor-rwire into the workspace
- **Status:** `[ ]`
- **Location:** `../llama-modnitor-rwire` → `apps/llama-modnitor/`
- **Problem:** Out-of-tree consumers compile rwire against a foreign workspace: nothing
  in rwire's checks builds them, so breaking framework changes rot them silently. Cargo
  requires members hierarchically below the workspace root (verified — sibling-path
  members are rejected), so integration means physically moving the app.
- **Why this app and not claw-rwire:** small (~5.5K lines, 2 commits of history), no
  secrets or live data beyond a `.env`, no external path deps, already treated as the
  "example reference app" by claw-rwire's docs. claw-rwire stays separate: personal app,
  `.env` + live SQLite data, optional `../isola` path dep, own stricter lint rules, and
  its agent's mid-edit breakage must not fail rwire's workspace checks (observed live
  2026-07-02).
- **Fix:** Move the directory, add to `[workspace] members`, drop its `Cargo.lock`,
  fix path deps (`../rwire/libs/rwire` → `libs/rwire`), add data/log paths to
  `.gitignore`, verify `cargo test --workspace`.
- **Acceptance:** `cargo clippy --workspace --all-targets` and `cargo test --workspace`
  cover the app with zero warnings; a deliberate breaking change in `builder.rs` fails
  the workspace build.
- **Effort:** ~1h.

### W2 — Consumer smoke-check for out-of-tree apps
- **Status:** `[ ]`
- **Location:** `CLAUDE.md` "Before Committing"; optionally a `check-consumers.sh`.
- **Problem:** claw-rwire (staying out-of-tree per W1) can still rot silently.
- **Fix:** Add to the pre-commit checklist:
  `[ -d ../claw-rwire ] && cargo check --manifest-path ../claw-rwire/Cargo.toml`.
  Agents read CLAUDE.md, so this is self-enforcing for agent-driven work.
- **Acceptance:** Checklist documents the command; a breaking rwire change is caught
  before commit when the sibling checkout exists.
- **Effort:** minutes.

### W3 — Update stale consumer claim in CLAUDE.md
- **Status:** `[ ]`
- **Location:** `CLAUDE.md` Deprecation Process: "The only consumers are internal examples."
- **Problem:** False since claw-rwire and llama-modnitor-rwire exist; agents doing
  breaking changes plan migrations from this line.
- **Fix:** Rewrite to name the real consumers and point at W1/W2 checks.
- **Acceptance:** CLAUDE.md reflects reality.
- **Effort:** minutes.

---

## Phase 2 — Release mechanics (mechanical, days total)

### R1 — LICENSE file
- **Status:** `[ ]`
- **Problem:** README declares MIT; no `LICENSE` file exists. Blocks legal reuse and
  `cargo publish`.
- **Fix:** Add `LICENSE` (MIT) at root; consider dual MIT/Apache-2.0 (Rust convention).
- **Acceptance:** File exists; `license` field in crate metadata matches (R3).
- **Effort:** minutes.

### R2 — CI pipeline
- **Status:** `[ ]`
- **Location:** `.github/workflows/` (none exist).
- **Problem:** Zero CI. The "serious project" signal for a public repo, and the guard
  for every other item here.
- **Fix:** GitHub Actions: `cargo fmt --check`, `cargo clippy --workspace --all-targets
  -- -D warnings`, `cargo test --workspace`. Node must be present for the wire
  round-trip harness (`tests/*.mjs`). Cache cargo. Optionally a capsule-size regression
  check (fail if the generated capsule exceeds a budget) — the ~17KB claim is a
  headline feature and deserves a tripwire.
- **Acceptance:** CI green on main; a clippy warning or failing round-trip test blocks.
- **Effort:** ~half a day.

### R3 — Crate metadata + publishability
- **Status:** `[ ]`
- **Location:** `libs/*/Cargo.toml` (all currently bare: no description/license/repo).
- **Problem:** `cargo publish` fails without metadata; path deps need `version =`
  fields; crates.io name availability for `rwire` (and `rwire-*`) is unverified —
  check *before* getting attached to the name in announcements.
- **Fix:** Add `description`, `license`, `repository`, `keywords`, `categories`,
  `readme` to the five lib crates (use `[workspace.package]` inheritance); add
  `version` to path deps; `cargo publish --dry-run` each crate in dependency order
  (macros → rwire → components/themes/markdown).
- **Acceptance:** `cargo publish --dry-run` succeeds for all five lib crates.
- **Effort:** ~half a day.

### R4 — CHANGELOG + versioning policy
- **Status:** `[ ]`
- **Problem:** CLAUDE.md declares "experimental phase; breaking changes allowed; no
  versioning" — fine private, unworkable once strangers depend on the crates.
- **Fix:** Add `CHANGELOG.md` (start at 0.1.0). Policy: 0.x semver discipline —
  breaking changes bump minor, docs state loudly that the protocol is unstable (which
  is fine — see Baseline note on protocol compat). Update CLAUDE.md's deprecation
  section to match.
- **Acceptance:** CHANGELOG exists; CLAUDE.md and README state the stability policy.
- **Effort:** ~1h.

### R5 — Community files
- **Status:** `[ ]`
- **Fix:** `CONTRIBUTING.md` (build/test instructions incl. the Node harness, code
  rules distilled from CLAUDE.md, the runtime-modification pointer from P2), issue
  templates, optionally a Code of Conduct.
- **Acceptance:** Files exist; CONTRIBUTING explains how to run *all* checks locally.
- **Effort:** ~half a day.

---

## Phase 3 — Technical gaps (adopter-facing)

### T1 — Keyed list diffing 🔴 announce-blocker
- **Status:** `[ ]`
- **Location:** runtime `me`/`mk` morph (`capsule_gen.rs`); already top of README roadmap.
- **Problem:** Morph reuses nodes by id, else positionally. List *reorders* shuffle
  focus/caret/scroll state across items. Dynamic lists are the bread and butter of real
  apps and the first thing a skeptical evaluator tests; "efficient updates" is the
  framework's load-bearing claim.
- **Fix direction:** key children (e.g. from `ItemRef` identity, which already exists
  and is stable) and match by key in `mk` before positional fallback. Extend the wire
  format only if needed (a per-child key varint on list children).
- **Acceptance:** A reorder of a list with focused inputs preserves each item's focus,
  caret, and scroll; covered by a morph test (`tests/morph_test.mjs`) and a wire
  round-trip test.
- **Effort:** days — the hardest item here; budget accordingly.

### T2 — WebSocket Origin validation 🔴 announce-blocker (cheap)
- **Status:** `[ ]`
- **Location:** WS upgrade path in `server.rs` (no `Origin` check exists — verified).
- **Problem:** `SameSite=Strict` already keeps session cookies off cross-site
  handshakes, so practical risk is low, but an explicit Origin allowlist is table
  stakes for a public networked framework and reviewers will look for it.
- **Fix:** Validate `Origin` against the request `Host` by default; `ServerConfig`
  option for extra allowed origins (reverse-proxy setups). Reject with 403 before
  upgrade.
- **Acceptance:** Cross-origin handshake rejected in a test; same-origin and configured
  origins pass; documented in `05-advanced/config.md`.
- **Effort:** ~half a day.

### T3 — Event delegation for large lists
- **Status:** `[ ]` (post-launch OK; document as limitation)
- **Problem:** One listener per bound element; large lists get heavy on bind and on
  morph re-bind.
- **Fix direction:** delegate per event type at a container/document level, resolve the
  handler from `__hk`/params at dispatch. Already on README roadmap.
- **Acceptance:** A 1k-row list binds O(1) listeners per event type; behavior identical.

### T4 — SSR / static first paint
- **Status:** `[ ]` (post-launch OK; document as limitation)
- **Problem:** First render requires WS connect; crawlers and no-JS see a blank page.
  Weak for content sites — ironically including the rwire docs site itself.
- **Fix direction:** server renders initial HTML into the capsule (the builder tree is
  already known at request time — see `static_html.rs` test groundwork), runtime
  attaches/morphs over it on connect.
- **Acceptance:** `curl /` returns meaningful HTML for a routed page; docs site is
  crawlable.

### T5 — Auth middleware
- **Status:** `[ ]` (post-launch OK; document as limitation)
- **Problem:** Current auth is a single user/password gate. Real apps need sessions
  with identity, login flows, and per-handler authorization.
- **Fix direction:** grow from claw-rwire's real needs (it has `auth.rs`) rather than
  speculating — same dogfooding loop as everything else.

### T6 — Scaling story documentation
- **Status:** `[ ]`
- **Problem:** Server memory scales with open connections; single-process; no
  horizontal-scaling / session-affinity guidance. Fine for the target niche
  (self-hosted, internal tools) but must be stated, not discovered.
- **Fix:** A docs page: memory-per-connection expectations, connection limits
  (`ServerConfig`), sticky-session requirement behind load balancers, and what
  `shared`/`persisted` state do and don't give you across processes.
- **Acceptance:** Docs page exists; README links it from a "limitations" section.
- **Effort:** ~half a day.

---

## Phase 4 — Positioning & launch

### P1 — Public comparison page
- **Status:** `[ ]`
- **Location:** `docs/comparative-study.md` exists; not public-facing.
- **Problem:** "Why this over Phoenix LiveView / Blazor Server / htmx / Leptos?" is
  every reader's first question.
- **Fix:** Edit the study into a docs-site page + README section. Frame honestly:
  a different point in the design space — binary protocol, per-connection runtime
  tree-shaking, compile-time reactivity, built for self-hosted software — not a
  LiveView challenger.
- **Effort:** ~half a day.

### P2 — Runtime contributor guide
- **Status:** `[ ]`
- **Location:** `RUNTIME_JS` in `capsule_gen.rs` (~13KB hand-minified string).
- **Problem:** The most safety-critical file is a wall for contributors, and it keeps
  growing (morph, reconnect, PWA, desync self-heal). The round-trip tests de-risk
  changes but don't explain them.
- **Fix:** Short doc: runtime structure (opcode loop, morph, event send path), naming
  conventions, how to modify safely (edit → `wire_roundtrip` + `morph_test.mjs` →
  size check). Decide explicitly whether/when to move to a readable source + minify
  build step — deferring is fine, silently drifting is not.
- **Acceptance:** Doc exists and is linked from CONTRIBUTING (R5).
- **Effort:** ~half a day.

### P3 — Launch framing
- **Status:** `[ ]`
- **Fix:** Announce as **experimental 0.1**, aimed at self-hosted tools, dashboards,
  internal and personal software. State known limitations up front (T3–T6, latency
  model, scale ceiling) — the AUDIT_ROADMAP/this-file style of public honesty *is*
  the credibility strategy. Lead with the two ideas nobody else has: lazy
  per-connection tree-shaking over the wire, and compile-time dependency bitmasks.
- **Acceptance:** README states scope, stability, and limitations before the feature
  list does.
