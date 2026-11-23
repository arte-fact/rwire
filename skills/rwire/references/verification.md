# rwire — verification and version pinning

## 1. Before you claim it works

These are the CI gates (`.github/workflows/ci.yml`, job `rust`) — stricter than the
`CLAUDE.md` "Before Committing" list, which has no `--check` / `-D warnings` — plus the
out-of-tree smoke check from `CLAUDE.md`. Run from the repository root:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
# rwire without its default `persist` feature (no rusqlite) must build clean too:
cargo clippy -p rwire --no-default-features --all-targets -- -D warnings
cargo test --workspace
# after a framework change, the out-of-tree consumer must still build:
[ -d ../claw-rwire ] && cargo check --manifest-path ../claw-rwire/Cargo.toml
```

`claw-rwire` depends on rwire **by path** (`../rwire/libs/…` in its `Cargo.toml`): the check
builds against the checkout at `../rwire`, not against a worktree you may be editing elsewhere.

"Zero warnings" is the rule in `CLAUDE.md`: a clippy warning is fixed, not `#[allow]`ed away.
`-- -D warnings` makes clippy fail instead of printing, which is what you want in a gate.

If you touched `runtime/src/`:

```bash
cd runtime && npm run check && npm test && npm run build && npm run sync
```

`npm run sync` regenerates `libs/rwire/assets/runtime.min.js` and the extension assets —
commit them, or CI fails on `git diff` (for `runtime.min.js` and `src/opcodes.ts`; see
`runtime.md` for the extension assets CI does not check).

The claw-rwire check is the real smoke test: it is the large out-of-tree app that consumes these
crates, so a breaking signature change shows up there and nowhere else.

In a consuming front (mcp-agents `crates/web-rwire`, its own workspace, excluded from the
mcp-agents root one):

```bash
cargo test --manifest-path crates/web-rwire/Cargo.toml   # includes the no_free_css guard
```

The `no_free_css` guard is a normal test: it reads `src/`, counts the bypass markers per file and
compares them to the `BUDGET` table in `crates/web-rwire/src/garde.rs`. It fails **above** budget
(free CSS added) and **below** budget (free CSS removed without lowering the budget). So:

- lowering the budget belongs in the same commit as the removal;
- proving the guard still bites: add one `.class("x")` on a scratch commit, run `cargo test`, see
  it fail, drop the commit. Do this when you change the guard itself.

Visual parity against the reference app (mcp-agents, `tools/essai-rwire/`):

```bash
node tools/essai-rwire/parity.mjs <pile> <screen> <app|rwire> <app|rwire>
node tools/essai-rwire/capture.mjs <pile> <screen|all> [--ref]
```

`parity.mjs` measures each mapped element at 1280/375 × light/dark and compares width, height,
padding, gap, radius (±1 px), font size/weight/family, and background/text/border colours (equal
in sRGB8, or ΔE2000 < 2). At 375 px it also checks there is no horizontal scroll and that every
visible interactive target is **≥ 44 × 44 px** (a smaller target that is no smaller than its
`/app` equivalent is reported separately, not failed). Exit code 0 means zero gaps and zero failed
checks — that is the only acceptable "done".

`capture.mjs` renders the side-by-side images (or `/app` alone with `--ref`) with
`playwright-core` + `ImageMagick montage`.

Do not run these on a production host: they drive a headless browser. Both the parity script and
the gallery are heavy; run them where there is RAM to spare.

## 2. Pinning rwire

There is **no useful version to pin**: the workspace version is `0.1.0`, no tag is published, and
the wire protocol is explicitly unstable (`README.md`, "Status & scope" — breaking changes are
normal, and the runtime ships from the same binary). Pinning by semver therefore pins nothing.

Pin by **commit rev**:

```toml
rwire           = { git = "https://github.com/arte-fact/rwire", rev = "<full-sha>" }
rwire-components = { git = "https://github.com/arte-fact/rwire", rev = "<full-sha>" }
rwire-markdown  = { git = "https://github.com/arte-fact/rwire", rev = "<full-sha>" }
rwire-themes    = { git = "https://github.com/arte-fact/rwire", rev = "<full-sha>" }
```

The reference front (`mcp-agents`, `crates/web-rwire/Cargo.toml`) does exactly this: one `rev`
for all four crates, identical across them.

Rules:

- **full 40-character SHA**, never a short one, never `branch = "main"` — the branch moves and
  the protocol moves with it;
- **one rev for every rwire crate** in a workspace; a mixed set is a wire mismatch waiting for a
  busy afternoon;
- bump the rev in its own commit, and say which rwire commit came in, so a bisect has somewhere
  to land;
- after a bump, re-run the front's tests (`cargo test --manifest-path
  crates/web-rwire/Cargo.toml`: it compiles against the new rev; the `no_free_css` guard only
  counts the front's own `src/`, so it does not see the bump itself) and `parity.mjs` on at
  least one screen.

Check what a tree actually resolved to:

```bash
cargo tree -p rwire --depth 0        # sha of the checked-out rwire
cargo metadata --format-version 1 | grep -o 'rwire?rev=[0-9a-f]*' | sort -u
```

## 3. Where this skill comes from

Written against the rwire tree at the merge commit on `main` this skill ships with — see
`git log -1 --format=%H` for the exact SHA, and `ETAT.md` for the current status and process.
rwire is feature-frozen: it takes only changes a downstream page needs now, through the roadmap
lead, one open PR at a time, and `main` is merged by the owner only.
