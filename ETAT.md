# rwire — status

Status maintained by the rwire group coordinator; the project index lives in the knowledge base (`pilotage/INDEX-PROJETS.md`).

## Roadmap step

Step 1 (empty the PR queue) is done. rwire is feature-frozen: it only takes changes the mcp-agents rwire front (MR !20) needs now and that block a page, through the roadmap lead — one queue, one open PR at a time.

## Main

Green CI. Latest merges: #15 (delegated auth hardening) and #16 (lighter capsule HTTP).

## Open PRs

None.

## Waiting branches (no PR; rebased on main by merge)

- `feat/components-styling` — gaps 6, 7, 11, 12, 13: `Field::textarea` in `Form`, last `st()` call wins, `Composer` disabled state with its reason, `Button::title` and tooltips on disabled buttons, Nord dark contrast with a WCAG test. Each has a workaround in the front.
- `feat/on-connect` — an `on_connect(ctx)` hook with `ctx.identity()`, to start a per-user task.
- `fix/login-next-base-path` — the login `next` includes `base_path`; option to turn `/metrics` off.
- `feat/outlet-guard-opt-out` — explicit opt-out for the router outlet guard.
- `fix/review-minors` — minor follow-ups from an earlier review.
- `feat/markdown-frontmatter-extra` — free extra fields in the Markdown front matter (asked by news).
- `docs/multi-state-renderers` — design study for multi-state renderers.

## mcp-agents front gaps

From §5 of the rwire trial notes (MR !20):

| Gap | Subject | Status |
|---|---|---|
| 1 | optional SQLite persistence (`persist` feature) | fixed in #12 |
| 2 | multi-state renderers | design study `docs/multi-state-renderers` |
| 3 | connection hook with the identity (`on_connect`) | waiting branch `feat/on-connect` |
| 4, 5, 8 | short 404, favicon, capsule `<title>`/gzip | fixed in #16 |
| 6, 7, 11, 12, 13 | textarea field, style order, disabled reasons, button tooltip, Nord dark contrast | waiting branch `feat/components-styling` |
| 9 | delegated auth hardening, `/metrics` behind the gate | fixed in #15 |
| 10 | login `next` ignores `base_path` | waiting branch `fix/login-next-base-path` |

## Process

Changelog fragments in `changelog.d/`; never push to `main`; merges by the owner only.

## claw-rwire (provisional)

On tokio since claw-rwire #1; no open PR.

## Handoff (coordination of the rwire group)

Written for the agent who takes over coordination of the rwire group (the rwire-fable role: coordinator on `home`, reporting to the mcp-agents roadmap lead). The team sessions were stopped to save tokens; no agent is running.

### Role and rules
- The roadmap lead (raptors, `rapports` session) dispatches, reviews and merges, with the owner's OK on gallery screenshots.
- Never push or merge to `main`, never force-push, no agent signature, code and PRs in English.
- One open PR at a time; one heavy cargo build at a time (`pgrep -x cargo` empty).
- Shared `CARGO_TARGET_DIR=$HOME/.cache/rwire-shared-target`; touch sources before checking.
- Changelog: fragments in `changelog.d/`. Branches are updated by merging `main`, never by rebasing.
- Every PR gets a full description (why, changes, evidence, behaviour changes, open points), an independent review before opening, screenshots on a `pr-assets/*` branch (images only), and an MR post (French) in news-contenu, branch `redaction`.
- A gap found by the mcp-agents front goes into the current rwire lot, never into local CSS. Rule for theme colours: the old front's exact value when it passes contrast (4.5:1 text, 3:1 dots), else the nearest value that passes, listed as a named gap.

### PR queue (in this order)
1. **#23 open**: `feat/shell-navigation` @ `bbb4efa` (lot R2). CI green, code approved by the roadmap lead. **Waiting for the owner's OK on the gallery**, then the roadmap lead merges.
2. `feat/shell-parity` @ `67c24bc`: stacked on `bbb4efa`, no PR yet. Fixes 10 parity gaps on screen 1, including dark link contrast 2.28:1 → 6.24:1. Open its PR after #23 is merged:
   - merge `main` into it first;
   - the PR text is in `rwire-groupe/notes/opus-33-rapport.md`;
   - before/after screenshots are on `pr-assets/r2-parity`; screenshots 7-10 are local only, in `rwire-groupe/medias-r2-parity/`.
3. `feat/audio` @ `0fce62b`: the `rwire-audio` extension (RW1-RW4) for the voice prototype.
   - Already up to date with `main` 4fb9b3b, reviewed, fixes done; the voice design session pins `4b21d14`/`0fce62b`.
   - The PR text is in `rwire-groupe/notes/opus2-26-rapport.md` (updated by opus2-29).
4. `feat/conversation` @ `5c28725`: lot R3, screens 2 and 4. **Work in progress**.
   - Iteration 1 (avatar layout, bubbles, reading prose) is committed unfinished: `cargo check` passes; tests, clippy and measurements have not run.
   - The plan is in `rwire-groupe/consignes/opus-34-r3-conversation.md`.
   - The Vibe part (PageHeader, Badge::status, DangerGhost, ChatDivider/ChatNote, TypingIndicator::tone; `rwire-groupe/consignes/vibe-18-r3-header-bits.md`) was not started.
   - The screen 2 spec and parity tables are in games/mcp-agents, branch `feat/visual-parity-e4`: `docs/parite-ecran2-spec.md` and `tools/essai-rwire/parite/{conversation,validation}.json`.

### Next steps
- After #23 merges: set its MR post to `etat: fusionnée` (news-contenu, `billets/2026/11/22/mr-rwire-23.md`), then open the `feat/shell-parity` PR, then `feat/audio`, then R3.
- Resume R3 at iteration 1: run tests, clippy and the gallery measurements, then continue with Markdown::face, delivery, ToolCall, DecisionCard, the composer (`reset_on_ack` with a Node E2E test) and ComposerDock.
- Send every pushed SHA to the mcp-agents coordinator, who pins it on `feat/visual-parity-e4`.
- HORLOGE: `set_coordinator` was not available in this session, so no teammate is linked to the coordinator yet.

### Decisions waiting for the owner
- Named gaps with the old front (dark inactive dot at 3:1, light accent fill, ink-faint, dot colours, light link `#466389`, search placeholder `TextFaint`): each can be reversed to the old front's value.
- Drawer below 1100 px without `role=dialog`/`inert`: accepted provisionally by the roadmap lead (open point in #23).
- OK on the #23 gallery screenshots, needed for the merge.
