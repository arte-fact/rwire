# rwire Public Release Roadmap

Tracking document for taking rwire from "excellent private codebase" to a credible public
0.1 release. Sourced from a full framework review (2026-07-02): architecture, protocol,
runtime, macros, security posture, docs, tests, and the consumer apps
(`../claw-rwire`, `../llama-modnitor-rwire`).

**Verdict:** engineering maturity is unusually high for a 5-month project (audit trail,
round-trip tests, docs site, live dogfooding). What's missing is almost entirely release
*packaging* (Phase 2, mechanical) plus two adopter-facing gaps (T1, T2) that should land
before any announcement because they're the first things a skeptical evaluator will test.

Extended 2026-07-06 with the runtime-extraction decision (Phase 5, supersedes P2) and
the content & editing feature track (Phase 6).

**Suggested sequence:** Phase 1 (housekeeping) → Phase 2 (mechanics) → Phase 5 (runtime
extraction) → T1 + T2 → announce as experimental 0.1 with remaining items stated as
known limitations. Phase 6 (features) starts after the extraction and can interleave
with the release track — nothing in it blocks 0.1.

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
| 4 — Positioning & launch | 3 | 1 |
| 5 — Runtime extraction | 3 | 0 |
| 6 — Content & editing | 8 | 0 |
| **All** | **28** | **1** |

(P2 counts as closed: superseded by Phase 5.)

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
- **Effort:** days — the hardest item here; budget accordingly. Do it in the extracted
  TypeScript runtime (after RT1–RT3): typed modules and per-function unit tests are
  exactly what this surgery needs — land Phase 5 first.

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

### P2 — Runtime maintenance policy (superseded by Phase 5)
- **Status:** `[-]` Superseded 2026-07-06 → **Phase 5 (runtime extraction)**.
- **History, kept so the decision isn't re-litigated blind:**
  - **v1 (2026-07-02):** keep the runtime a hand-minified, Rust-embedded string.
    Rationale: firmware-not-codebase — a small fixed-instruction interpreter with
    write-once stability, evolution happening server-side; pure-Rust toolchain worth
    protecting from npm. Revisit trigger: "re-open if runtime surgery becomes
    routinely painful."
  - **v2 (2026-07-06):** the trigger fired early — the Phase 6 feature track (content
    streaming, editor gutter/scroll-sync, dirty tracking) plus T1 (keyed diffing) make
    the runtime an actively-developed codebase, which invalidates the firmware premise.
    New decision: extract to a separate TypeScript repo (`../rwire-runtime`) that
    releases a minified artifact rwire vendors. This keeps what v1 was actually
    protecting — the rwire repo stays 100% Rust, `cargo build` stays Node-free for all
    consumers, no npm supply chain enters this repo — while gaining typed, unit-tested,
    fearless runtime development.
- **Carried over into Phase 5 unchanged:** the three v1 gates — harness-first changes
  (RT1), capsule-size tripwire (R2/RT1), written-down structure & conventions (now the
  rwire-runtime repo's own docs; CONTRIBUTING links out).

### P3 — Launch framing
- **Status:** `[ ]`
- **Fix:** Announce as **experimental 0.1**, aimed at self-hosted tools, dashboards,
  internal and personal software. State known limitations up front (T3–T6, latency
  model, scale ceiling) — the AUDIT_ROADMAP/this-file style of public honesty *is*
  the credibility strategy. Lead with the two ideas nobody else has: lazy
  per-connection tree-shaking over the wire, and compile-time dependency bitmasks.
- **Acceptance:** README states scope, stability, and limitations before the feature
  list does.

---

## Phase 5 — Runtime extraction (`../rwire-runtime`)

Supersedes P2 v1 (see P2 for the decision history). The runtime moves from a
hand-minified string in `capsule_gen.rs` to a sibling **TypeScript repo** that releases
a minified artifact; rwire vendors the artifact. What this preserves: the rwire repo
stays 100% Rust, `cargo build` needs no Node anywhere (including crates.io consumers),
and there is still **no protocol version skew** — the server ships its own vendored
runtime, so encoder and decoder always travel together. What this gains: typed modules,
per-function unit tests, machine minification, fearless surgery for T1 and Phase 6.

**Sequencing:** RT1 → RT2 → RT3 land before T1 and before any Phase 6 runtime work.

### RT1 — Create the rwire-runtime TypeScript repo
- **Status:** `[ ]`
- **Location:** new sibling repo `../rwire-runtime`; source is today's `RUNTIME_JS`
  (`capsule_gen.rs:35`).
- **Fix direction:** de-minify into typed TS modules mirroring the existing structure —
  opcode executor (`x`), morph (`me`/`mk`), event send (`se`/`sep`/`gp`), client
  actions, router glue, reconnect/PWA. Unit tests per module (`node:test` or vitest);
  esbuild bundle + minify with mangling (safe: the one-letter globals are internal to
  the capsule; nothing on the wire depends on JS identifier names). In-repo size-budget
  test (fail above budget). Release = versioned `runtime.min.js` with a version header.
  The artifact stays **fully static**: dynamic config (`BASE`, theme CSS) is already
  injected by the capsule outside the runtime constant — keep config flowing through
  injected globals, never through artifact templating.
- **Port strategy:** module-by-module de-minification with rwire's existing Node
  harness (`wire_roundtrip.mjs`, `morph_test.mjs`) run against every intermediate
  build — behavior-identical (DOM-identical output), not byte-identical, is the bar.
- **Acceptance:** minified artifact passes rwire's full harness unchanged; unit tests
  cover every opcode branch; minified+gzipped size within tolerance of today's ~13KB.
- **Effort:** ~2–4 days (the port is mechanical; the tests are the real work).

### RT2 — Vendor the artifact into rwire
- **Status:** `[ ]`
- **Location:** `capsule_gen.rs` (string constant → `include_str!`), new
  `libs/rwire/assets/runtime.min.js`.
- **Fix direction:** embed via `include_str!` so the asset ships in the crate package
  (crates.io consumers stay Node-free). `RUNTIME_VERSION` const surfaced in rwire and
  stamped in the artifact header. Sync procedure: a small script (or documented agent
  workflow) copies the released artifact in and bumps the version in one commit. rwire
  CI keeps running the round-trip harness **against the vendored file** — that is the
  drift gate; a stale or hand-edited artifact fails there.
- **Acceptance:** `cargo build`/`cargo publish --dry-run` need no Node; harness targets
  the vendored artifact; versions visible in both repos; CLAUDE.md's description of
  `capsule_gen.rs` updated.
- **Effort:** ~half a day.

### RT3 — Opcode contract between the repos
- **Status:** `[ ]`
- **Problem:** the opcode table exists twice — Rust (`protocol/opcodes.rs`, source of
  truth) and the TS `O` map — and with two repos that duplication becomes silent
  cross-repo drift.
- **Fix direction:** emit a machine-readable opcode manifest (JSON) from the Rust
  constants (a tiny generator or a test that writes/validates it), checked into rwire;
  rwire-runtime vendors a copy and its CI validates the TS constants against it. The
  round-trip harness remains the end-to-end gate; the manifest catches drift at the
  repo that moved first.
- **Acceptance:** changing an opcode in either repo without the other fails CI on
  whichever side is behind.
- **Effort:** ~half a day.

---

## Phase 6 — Content & editing features

Target: large-content viewing and (optional) editing — file explorer, streamed content,
code coloration, editor affordances with dirty tracking — plus a generic chat surface.
Drafted 2026-07-06; refine per-item before implementation.

**Design principle:** server-first, per the architecture. Highlighting, diffing, and
document state live on the server; the runtime gains only *small generic primitives*
(scroll sentinel, scroll-sync, local line count, pointer-drag) — never
feature-specific logic. That keeps the runtime small and makes every primitive
reusable.

**Provenance & bar:** the file explorer (F2) and chat (F7) originate in claw-rwire's
needs (`src/ui/files.rs`; `src/ui/chat.rs` + `ui/thread.rs`) — the same
upstream-promotion pattern that produced `ChatScroll`. But the existing surfaces are
**imperfect drafts, not specs**: mine them for requirements and encoded lessons (the
pinned-composer layout, the justify-on-scroll-container regression), don't replicate
their flaws. Design each component fresh to a **first-class bar** — where the draft
and good UX disagree, UX wins (e.g. seamless history scroll replacing the drafted
load-older button). Design against claw-rwire's roadmap too (P4a: one unified `Thread`
component; P5c: a read-only per-branch file tree). Each component lands with a
claw-rwire migration that deletes the local implementation — the framework gains a
component, claw-rwire gets a strictly better surface than it drafted, and the
migration pressure-tests genericity.

**Component architecture (atomic, decided 2026-07-06):** chat and the explorer/editor
are **macro components** composed from the library — reuse before creation; macros
live in `rwire-components` beside the atoms (split into a separate crate only if the
module caps demand it). Every **new** component gets a design-system catalog entry (F8).

| Layer | Chat (F7) | Explorer/editor (F2/F3/F5/F6) |
|---|---|---|
| Reused | `ChatScroll` + `Composer` (both shipped), `Avatar`, `Chip`/`Tag`/`Badge`, `Alert`, `StatusDot`, `Skeleton`, `EmptyState`, `Divider`, `Text`, `Stack`, rwire-markdown | `Breadcrumb`, `Input`, `DropdownMenu`, `Modal`, `Code`, `CopyButton`, `Tabs`/`Chip`, `Toast`, `Skeleton`, `EmptyState`, `Tooltip`, `Grid`/`Stack`, `rwire::icons` |
| New atoms | `TypingIndicator` (animated writing dots) | `Gutter` (line numbers + per-line marks) |
| New molecules | `ChatEntry` (avatar/icon slot · author · time · phase tag · accent rail · body slot · collapsible detail) | `TreeView`/`TreeItem` (generic collapsible tree: client-action expand/collapse, selection, per-node slots), `SplitPane` (resizable panes via the pointer-drag primitive) |
| New organisms | `ChatTranscript` (windowed entries, seamless history, day dividers, empty state), `StreamedContent` (F1, shared with editor track) | `FileTree` (`TreeView` × file icons/kinds/actions), `CodeEditor` (textarea + `Gutter` + scroll-sync + dirty marks; supersedes claw's `md_editor`) |
| Macro | `Chat` (transcript + composer + error slot + writing state) | `DocumentView` (view/edit shell), `FileExplorer` (`SplitPane`: `FileTree` beside `DocumentView`, breadcrumb, actions) |

**Ordering:** F4 is independent and small — do first. F2 → F3 → F5 → F6 build on each
other; F1 is independent of F2/F3 and a **prerequisite of F7** (seamless history
scroll). F8 integrates continuously and finishes last. All runtime-touching items
(F1, F5, and F2's `SplitPane` drag) require Phase 5 done.

### F1 — Content streaming / infinite scroll for large content
- **Status:** `[ ]`
- **Problem:** a large file or page rendered as one synced update is one giant frame —
  slow first paint, big morph. Needs progressive delivery.
- **Fix direction:** a **scroll-sentinel primitive** in the runtime: new bind opcode
  (client-action space) attaching an IntersectionObserver to a sentinel element that
  fires a remote event when it nears the viewport. Server responds by appending the
  next rendered chunk (existing `APPEND`) and moving/removing the sentinel.
  One-request-in-flight backpressure server-side. Component: `StreamedContent` over a
  chunked-source trait (file lines, markdown sections, list pages). Windowing/pruning
  of off-screen chunks is explicitly deferred — append-only first.
- **Acceptance:** a multi-MB markdown/code file loads progressively with smooth
  scrolling; chunk requests never stack; sentinel survives morphs and route swaps
  (rebind covered by a harness case).
- **Effort:** ~2–3 days (runtime primitive is small; the chunked-source design is the
  real work).

### F2 — File explorer component (sandboxed FS source, from claw-rwire)
- **Status:** `[ ]`
- **Provenance:** claw-rwire's `ui/files.rs` — a working but imperfect two-pane
  manager (breadcrumb header, inline create prompt, entry rows with hover/active
  states, rename/delete, editor pane); treat it as the requirements list, not the
  spec. Its roadmap (P5c) adds a second consumer: a **read-only** file tree of a
  selected branch on the project page. Both modes are therefore requirements, not
  speculation.
- **Fix direction:** server-side directory tree over a configured root. **Path safety
  is the security item**: canonicalize, reject `..` and symlink escapes, tested. Two
  modes: **read-only** (docs site, branch trees) and **managed** (create/rename/delete
  via handler hooks the app provides — the component renders affordances only when
  hooks are present). Composition per the architecture table: generic `TreeView`
  molecule (client-action expand/collapse, selection, per-node slots) specialized into
  `FileTree` (icons/kinds/action slots), inside a **resizable `SplitPane`** (decided
  2026-07-06: resizable from v1 — needs the pointer-drag runtime primitive, local-only
  with an optional debounced remote event to persist the split). `Breadcrumb` header;
  selection → `DocumentView` (F3). Markdown via rwire-markdown, code via the
  highlighter, binary → info row.
- **Acceptance:** explorer over a sample tree in the examples app; traversal-escape
  attempts rejected with test coverage; large directories paginate or lazy-expand;
  claw-rwire migrates `ui/files.rs` onto it, deleting the local browser scaffolding.
- **Effort:** ~2–3 days.

### F3 — Document view/edit shell (edit optional)
- **Status:** `[ ]`
- **Fix direction:** `DocumentView` component: **view** = rendered markdown /
  highlighted code (F4); **edit** = optional mode behind a toggle — a plain
  `<textarea>` plus gutter column, document state server-owned (baseline + working
  copy) fed by existing `BIND_DEBOUNCED` input events. The highlighted-overlay editor
  (transparent textarea over a `<pre>`) is a later enhancement gated on F5's
  scroll-sync — start plain.
- **Acceptance:** view/edit toggle on md and code files; typing, paste, and IME input
  work; working copy survives route swap within a session.
- **Effort:** ~2–3 days.

### F4 — Extend code coloration
- **Status:** `[ ]`
- **Location:** `rwire-markdown/src/highlight.rs` (table-driven lexer; Rust, JSON,
  shell, SQL today; graceful raw-text fallback).
- **Fix direction:** add the languages the dogfood needs — JS/TS, HTML/CSS, TOML,
  Markdown — keeping the table-driven approach (one `LangSpec` per language, no regex
  engines, no external deps). Reused by markdown fences and F3 view mode.
- **Acceptance:** docs-site code blocks colored across all languages they actually
  use; unknown languages still fall back to raw text.
- **Effort:** ~1–2 days.

### F5 — Editor gutter: line numbers + scroll sync
- **Status:** `[ ]`
- **Fix direction:** gutter rendered server-side from the working copy; a small runtime
  helper recomputes line count locally on input (count newlines → patch gutter) for
  zero-latency numbering, server reconciles on the debounced event. A generic
  **scroll-sync primitive** (one bind opcode: mirror `scrollTop` between two refs)
  keeps gutter — and later the highlight overlay — aligned with the textarea.
- **Acceptance:** line numbers stay correct during fast typing; gutter never visibly
  desyncs on scroll; both primitives have harness cases.
- **Effort:** ~1–2 days (requires Phase 5).

### F6 — Dirty-line tracking + gated save
- **Status:** `[ ]`
- **Fix direction:** pure server-side — the flagship demo that the architecture
  handles "rich editor feel" with zero new runtime surface. On each debounced input:
  diff working copy vs baseline → per-line dirty set → gutter marks (St class per
  dirty line) and a document-level dirty flag gating the save button (existing attr
  system). Save writes through the F2 sandbox, resets baseline, clears marks. Content
  diff, not keystroke tracking: hand-reverting a line clears its mark. Conflict
  safety: check mtime on save; external change → surface a conflict, never silently
  overwrite.
- **Acceptance:** marks appear within the debounce interval; save enables/disables
  correctly through edit → revert → edit cycles; external-change conflict test.
- **Effort:** ~2–3 days.

### F7 — Generic Chat component (from claw-rwire)
- **Status:** `[ ]`
- **Provenance:** claw-rwire's chat surface (`ui/chat.rs` — transcript over pinned
  composer, error banner, `sending_row`, collapsible transcript items) and the shared
  thread widgets (`ui/thread.rs` — authored entry: author rail + phase tag + time
  header, markdown body, `ChatScroll`) supply the **requirements — as imperfect
  drafts, not a spec to replicate**. claw-rwire's own roadmap **P4a** wants exactly
  this: one `Thread` component unifying concierge chat, agent threads, and the task
  Thread tab, with windowed history. Build the first-class version *here*; P4a adopts
  it and gets a better surface than it drafted.
- **Fix direction — composable parts under one `Chat` family** (usable whole or à la
  carte):
  - **Shell:** transcript over a pinned composer that reserves its own height (never
    covers the last turn — claw's encoded layout lesson); bottom-pinned via
    `ChatScroll`.
  - **Entry row:** author + timestamp header, optional phase/role tag, accent rail,
    icon slot, markdown or plain body, optional collapsible detail (claw's toggled
    transcript items).
  - **Writing state:** pending/sending row (claw's `sending_row`) and a **streaming
    entry** variant — the server appends into the entry's synced region as content
    arrives; zero client logic, it's just reactive updates.
  - **Composer panel:** the shipped `Composer` component (auto-grow, Enter-submit /
    Shift+Enter newline via the existing `data-enter-submit` runtime behavior, pill +
    compact forms) — extended with a disabled-while-sending gate and leading/trailing
    action slots (attachments etc. come later without API breakage).
  - **History:** windowed — render the last N turns; older turns arrive via **seamless
    infinite scroll** (F1's sentinel in reverse, one page in flight). claw-rwire's P4a
    drafts an explicit "↑ load older" button — the component supersedes it: with the
    sentinel there is no button, history just appears as you scroll up. Better UX, one
    less affordance to style. `column-reverse` means older content appends at the DOM
    *end*, so loading history is scroll-anchor-stable by construction.
- **Depends:** `ChatScroll` (shipped); **F1's sentinel primitive** (seamless history —
  a real dependency, not optional); benefits from T1 keyed diffing for chat-switch
  swaps (claw P4a: "keep the subtree keyed").
- **Acceptance:** docs page, design-system entry, and `examples/chat` (see F8) land
  with the component; claw-rwire's P4a lands *on* it — all three of its surfaces
  render through the component, the local scaffolding in `ui/chat.rs`/`ui/thread.rs`
  is deleted, and the drafted load-older button never ships.
- **Effort:** ~3–4 days.

### F8 — Dogfood integration: docs, website, examples, design system
- **Status:** `[ ]`
- **Fix direction:**
  - **Docs site:** sidebar becomes a `FileTree` over `docs/`; long pages stream via
    F1; code blocks colored via F4.
  - **Website:** feature section + live demo embeds (view/edit, chat).
  - **Examples:** two new **fully functional** apps, the E2E debugging targets for
    the whole track. `examples/chat` — a simple chatroom: shared in-memory state
    (`#[storage(shared)]` + the broadcast registry, which currently has no public
    demo) so multiple browser tabs converse live; writing state while others type;
    seamless history scroll; non-persistent by design (restart clears the room).
    `examples/editor` — file/code explorer + editor: browse a sandboxed tree, view
    with coloration, edit with line numbers and dirty marks, gated save — wired
    end-to-end.
  - **Design system:** a catalog entry for **every new component** in the
    architecture table: `TypingIndicator`, `ChatEntry`, `ChatTranscript`, `Chat`,
    `StreamedContent`, `TreeView`, `SplitPane`, `Gutter`, `FileTree`, `CodeEditor`,
    `DocumentView`, `FileExplorer`.
- **Acceptance:** all four apps exercise the features; README + docs updated
  (component counts, feature list, this roadmap's status).
- **Effort:** ~2–4 days spread across the track.
