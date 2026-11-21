# rwire — status

Status maintained by the rwire group coordinator; the project index lives in the knowledge base (`pilotage/INDEX-PROJETS.md`).

## Roadmap step

Step 1 (empty the PR queue) is done. rwire is feature-frozen: it only takes changes the mcp-agents rwire front (MR !20) needs now and that block a page, through the roadmap lead — one queue, one open PR at a time.

## Main

Green CI. Latest merges: #15 (delegated auth hardening) and #16 (lighter capsule HTTP).

## Open PRs

None.

## Waiting branches (no PR; rebased on main by merge)

- `feat/components-styling` — component styling gaps 6, 7, 11, 12, 13 (each has a workaround in the front).
- `feat/on-connect` — run `on_route`/first-message handling on connect.
- `fix/login-next-base-path` — `next` redirect combined with `base_path`.
- `feat/outlet-guard-opt-out` — explicit opt-out for the router outlet guard.
- `feat/review-minors` — minor review follow-ups.
- `feat/markdown-frontmatter-extra` — extra Markdown frontmatter support.
- `docs/multi-state-renderers` — design study for multi-state renderers.

## mcp-agents front gaps

From §5 of the rwire trial notes (MR !20):

| Gap | Subject | Status |
|---|---|---|
| 1 | optional SQLite persistence (`persist` feature) | fixed in #12 |
| 2 | multi-state renderers | design study `docs/multi-state-renderers` |
| 3 | on-connect behavior | waiting branch `feat/on-connect` |
| 4, 5, 8 | short 404, favicon, capsule `<title>`/gzip | fixed in #16 |
| 6, 7, 11, 12, 13 | component styling | waiting branch `feat/components-styling` |
| 9 | delegated auth hardening, `/metrics` behind the gate | fixed in #15 |
| 10 | `login` + `next` with `base_path` | waiting branch `fix/login-next-base-path` |

## Process

Changelog fragments in `changelog.d/`; never push to `main`; merges by the owner only.

## claw-rwire (provisional)

On tokio since claw-rwire #1; no open PR.
