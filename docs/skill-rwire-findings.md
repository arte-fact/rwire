# rwire — field findings from real agent usage

Date: 2026-11-23. Prepared for the rwire agent skill (PR #27, branch `feat/rwire-skill`,
`skills/rwire/references/pitfalls.md`): raw material on drifts, pitfalls and patterns
observed in actual agent sessions. Everything below is a summarized finding with a
reference (session file or commit + date); no secrets and no raw transcript content
are reproduced.

## Method and coverage (read-only)

- Run on `threadripper` (hostname `threadreaper`); `home`, `workstation`, `office` and
  `raptors` (prod — light greps only) over SSH. `threadripper`'s own transcripts were
  **not** scanned: the run tried to reach it over SSH as if it were another machine,
  failed, and did not fall back to a local scan. Its repos were read through clones on
  the other machines.
- Transcripts: `~/.claude/projects/*/*.jsonl`, pi sessions `~/.pi/agent/sessions`,
  Vibe journals `~/.vibe/logs/session` — ~925 files, OTB projects excluded.
  The first draft carried aggregate keyword-grep counts (e.g. thousands of hits for
  "timeout"); they measured how often a word was discussed, not how often a defect
  occurred, and were removed. Every finding below rests on a commit, a review note or a
  dated document.
- Git history of the rwire-consuming repos: `rwire`, `claw-rwire`, `mcp-agents` (+ the
  `-parite` and `-pi-suivi` clones), `voix-proto`, `llama-modnitor-rwire`, `news`.
- Review comments: GitLab `games/mcp-agents` MR discussions !20, !24, !26, !29
  (agents-bot), GitHub `arte-fact/rwire` PRs #21–#27, and the MR billets in
  `news-contenu` (raptors, 2026-11-21 → 2026-11-23).

## Part 1 — Drifts

| # | Drift | Evidence | References |
|---|-------|----------------------|------------|
| D1 | Free CSS / hard-coded colors despite the typed-CSS rule | Recurring drift. In git: repeated "without free CSS" fix commits in mcp-agents-parite (e.g. `beff3e0`), and on MR !29 the `no_free_css` guard budget had to be ratcheted down: colors 53→0, `.class(` 42→1 | office sessions `-home-benoit-Workspace-mcp-agents/*.jsonl` (2026-11-18→23); home sessions `-home-artefact-Workspace-rwire-opus2/26a8dad2…jsonl` (2026-11-22); mcp-agents-parite `beff3e0`; MR !29 review notes 599/600/603 (2026-11-23) |
| D2 | Hand-reimplemented components instead of rwire's | Recurring, then corrected: web-rwire started with `LAYOUT_CSS` + "neutral renderer wrappers" (commit `cfe7525`, MR !29 title); claw-rwire history shows the fix pattern repeated per screen: `c2e7b13` "adopt rwire's FileEditor kit — retire the hand-rolled explorer", `70aa254`/`5cbcd36` "render through ChatTranscript + ChatItem; delete thread.rs", `ad44049` "migrate chat/files/widgets off rwire escape hatches", `89beb94` "ride the components promoted into rwire (Chip, ChatScroll, Badge axes)" | mcp-agents `cfe7525`; claw-rwire `c2e7b13`, `70aa254`, `5cbcd36`, `ad44049`, `89beb94` (claw-rwire, 2026-06 → 2026-07); `cfe7525` (mcp-agents, 2026-11-22) |
| D3 | rwire pins left on an unmerged branch, or bumped many times without a plan | ≥8 distinct rev-pin commits in 3 repos over 3 days: mcp-agents `main` pins rev `7ef2d35…`; mcp-agents-parite pins `67c24bc` (`810bb62`); mcp-agents-pi-suivi went through 6 intermediate pins (`84ea65d`, `5d9b4c7`, `bf6b6a6`, `06b7a71`, `681e938` → `4fb9b3b`, 2026-11-21); voix-proto pins `7ef2d35` ("pin the reviewed rwire-audio revision", `3de4834`); MR !29 branch pins `d813148` = tip of **open** PR #26 `feat/chat-authors`. | Cargo.toml of mcp-agents / mcp-agents-parite / mcp-agents-pi-suivi / voix-proto (all machines, 2026-11); MR !29 review note 603 (2026-11-23) |
| D4 | Process-global state shared between connections | Global `WIDTHS`/`LAYOUT`/`SENT` statics in web-rwire, shared across visitors; moved into a per-connection `Ui` state only after review (commit `a3e5a7b`, MR !29). | MR !29 review notes 599/603 + fix commit `a3e5a7b` (2026-11-23) |
| D5 | Outbound network calls without timeout | `post_raw`/`post` on prompt/interrupt/permission had no timeout in web-rwire; fixed with `HUB_POST_TIMEOUT` (40 s) in `a3e5a7b`. | MR !29 review note 600 + commit `a3e5a7b` (2026-11-23) |
| D6 | Stores/tables never pruned | Unbounded `outbox`/`infos` in web-rwire (pruning added in `a3e5a7b`); inside rwire the disconnected-session cache has a TTL and a cap ("keep disconnected sessions for a day", `7b62fa9`) — the model to copy | MR !29 commit `a3e5a7b` (2026-11-23); rwire commit `7b62fa9` (2026-08-31) |
| D7 | Recurring PR conflicts from parallel rwire branches | rwire PRs #15, #16, #17 and #26 each have an "en conflit" billet; decision billet 2026-11-23: #26 conflicts with `feat/chat-ui` on 12 files (`changelog.d/*`, `app_shell.rs`, `catalog.rs`, `chat_calls.rs`, `chat_entry.rs`, `chat_item.rs`, `lib.rs`, `capsule.rs`, `delegated_auth.rs`, `server.rs`, `tests/delegated_login.rs`). PR #25 (`feat/chat-ui`) was closed unmerged. The merge-order billet lists mcp-agents !29 sharing 6 files with other open MRs. | `news-contenu/billets/2026/11/23/decision-rwire-conflit-rwire-26.md`, `…/ordre-de-fusion.md`, billets `mr-rwire-15/16/17 en conflit` (2026-11-21→23) |

## Part 2 — Pitfalls (symptom → cause → best practice)

1. **Compile errors right after a rwire bump.** API renames between PRs:
   `MobilePane::List` → `Sidebar` (`ef4b1a6`, PR #23), `StatusDotIntent::Hollow` added.
   *Best practice:* after every re-pin, grep the app for renamed components/types;
   expect breakage whenever the pin is an unmerged PR tip (see D3).
2. **`#[renderer]`/`#[handler]` take exactly one state parameter.** Wanting two
   states produces type errors.
   *Best practice:* read a second state through a nested
   `ElementBuilder::synced_with_storage::<S,_>(.., rwire::RendererDeps::always())`
   (pattern used in web-rwire, MR !29).
3. **Caret lost / stale value on re-keyed synced fields.** rwire itself had to fix
   "stale value/caret stamped onto re-keyed editor fields" (`e4fcc73`), caret restore
   on undo/redo (`3ee6270`) and explicit multiline textarea geometry (`ba42f64`).
   *Best practice:* do not re-key synced inputs on every render; the runtime now
   snapshots focus — rely on it, don't fight it.
4. **Synced wrappers are `display: contents` — no box to style.** Causes width
   collapse and "invisible" containers; claw-rwire had to restore uniform tab panels
   "now that rwire nested removal is fixed" (`0b5c722`) — i.e. a workaround for a
   framework bug that later got fixed.
   *Best practice:* measure in the browser at 1280 and 375 px before assuming a
   wrapper has geometry; prefer putting layout on real components.
5. **Pane/drawer bugs on mobile and during navigation.** Preview feedback on MR !29
   (Benoît, 2026-11-23): right panel not openable on mobile, left panel close-button
   style broken, session↔`/` navigation broken in the PR preview (fixed in `a3e5a7b`
   with a routed brand link + a "← Sessions" head link), header activity pill
   redundant, right-panel close button only useful on mobile. PR #23 open points:
   drawer has no backdrop/Escape/focus handling.
   *Best practice:* every pane gets a 375 px + keyboard pass (Escape, focus trap)
   before merge; keep one activity indicator, at the chat bottom.
6. **Foldable/collapsed regions re-close when unrelated state updates.** Tool-call
   collapse in the chat re-closed whenever other state attributes re-rendered
   (Benoît, 2026-11-23) — needs fine-grained state, keyed per item, not per screen.
   *Best practice:* collapse state keyed by item id in its own state region; never
   derive it from the whole conversation state.
7. **`<select>` change values are not read uniformly.** claw-rwire fix `6d20b19`:
   read every `<select>` change via `ctx.text()`.
8. **A renderer must own its deps.** claw-rwire `661e148`: "the CI logs drawer
   opens again — it owns its deps as a `#[renderer]`" — a drawer wired outside a
   renderer silently lost its state.
9. **Markdown/URL assumptions.** Agents assumed raw HTML/remote images were safe;
   rwire hardened this (no remote images by default `3c6f616`; URL scheme filter,
   PRs #14/#19). *Best practice:* never inject remote URLs into rendered markdown.
10. **Docs drift.** Numbers and API names in docs lag the code (already tracked as
    pitfall in the skill draft). *Best practice:* verify against the pinned rev's
    source before citing doc numbers.
11. **A "fix" can be reverted.** rwire `04d8bad` reverted a synced-region fix —
    fixes interact. *Best practice:* keep a regression test on both sides of any
    revert.
12. **Risk cases ship untested until review.** On MR !29, the no-timeout POSTs,
    shared global state and unbounded stores were found only at review time; the
    fix commit `a3e5a7b` added tests for pruning and per-connection UI state, none for
    the timeout. *Best practice:* tests for network
    timeout, per-connection isolation, mobile pane open/close, and collapse
    re-render belong in the first commit, not after the review.

## Part 3 — Recommended patterns (observed working)

- **Typed CSS only**: theme tokens + component props, with a CI guard (`no_free_css`)
  whose budget shrinks over time (53→0 colors on MR !29). Zero free CSS is reachable.
- **One state per `#[renderer]`/`#[handler]`**; extra states via nested
  `synced_with_storage` + `RendererDeps::always()`.
- **Per-connection state**: a `Ui` struct per WebSocket connection, never statics;
  prune stores (`outbox`, `infos`) with caps; put a timeout on every outbound call.
- **Prefer rwire components over hand-rolled markup**, and delete the hand-rolled
  file in the same commit (`ChatTranscript`/`ChatItem`, `FileEditor`, `Sidebar`,
  `Badge`/`Chip`) — claw-rwire's migration commits are the reference sequence.
- **Some UI state belongs server-side** (Benoît, 2026-11-23): moving it server-side
  removes whole classes of client state-granularity bugs (see pitfall 6).
- **Pin rwire by full SHA, merged `main` by default**; during an iteration that needs an
  rwire PR, pin that PR's commit, then re-pin to the merged `main` commit before the MR
  merges (mcp-agents roadmap). The 6 intermediate pins in mcp-agents-pi-suivi show the
  cost of bumping without that plan.
- **Verify before pushing**: `cargo fmt --check`, `clippy -D warnings`, unit tests,
  plus a 1280/375 px smoke pass and the send-guard/`/ready` checks — all green on
  MR !29 at `a3e5a7b`.
- **Respect merge order**: follow the merge-order note, keep one rwire PR open at a
  time (`ETAT.md`), and merge `main` into a feature branch early rather than stacking
  branches on another open one (`feat/chat-authors` on `feat/chat-ui`, closed unmerged,
  is the D7 example).

## Coverage notes

- `threadripper`'s own transcripts were not scanned (see Method); its rwire repos were
  analyzed through clones on `home`/`workstation`/`office`.
- OTB project transcripts were never opened, per the rules.
- Commits, review notes and dates were re-checked against the repositories during
  review on 2026-11-23.
