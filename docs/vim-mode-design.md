# Vim mode for FileEditor — evaluation & plan (rev. 3 — 2026-07-07)

## The architectural question: where does vim state live?

Modal editing is keystroke-latency-sensitive: `j` must move the caret *now*.

**Rev. 1 recommended a full server-side engine** (every normal-mode key a
round trip), reasoning from the localhost consumers (RTT ~1ms) and pure-Rust
testability. **User review overturned it**: a client interpreter is viable if
it is *lazy-loaded like the CSS and name maps* — pay-for-what-you-use, capsule
untouched. That dissolves the size objection, and with it the philosophical
one: **vim is an input method, not application state.** Pending operators,
counts, registers are keystroke *composition* — the same class of state as IME
composition, which the browser already owns. The server keeps owning what it
always owned: the working copy, autosave, undo, persistence.

| Option | Verdict |
|---|---|
| **B′. Client engine as a lazy runtime extension** | **Recommended** — zero-latency motions on any link; capsule unchanged; composes with the shipped editor paths (see below) |
| A. Full server-side engine | Rejected after review: adds `Ev::Keydown` flood + SET_SELECTION wire surface, feels heavy off-LAN, and misplaces input-method state on the server |
| B. Vim inside the CORE runtime | Rejected: 3–10KB against the 15KB capsule budget for a feature most apps never use |
| C. Hybrid split engine / D. CodeMirror | Rejected as before (duplication / against the framework) |

## Delivery: the runtime-extension primitive (new, reusable)

Not wire-streamed JS (that would need `eval` → CSP `unsafe-eval`). Instead:

- `runtime/src/ext/vim.ts` builds to a **separate artifact** `vim.min.js`,
  vendored via `include_str!` like `runtime.min.js`, served from memory at a
  server route (`/_rw/ext/vim.js`) — single-binary story intact.
- The core runtime gains a tiny **extension loader**: the server HINTS the
  modules a batch needs via a `MOD_DEF`-style opcode (exactly like `MAP_DEF` —
  sent once per connection, deduped), so the `import()` starts while the batch
  executes instead of after a DOM scan. Standard dynamic import: same-origin,
  HTTP-cacheable, `script-src 'self'`.
- This is a general mechanism: any future heavy interaction module
  (drag-drop kit, canvas widget) ships the same way, with its own size budget,
  outside the capsule.

## Why the composition is nearly free

The module interprets normal/visual-mode keydowns on the `[data-vim]`
textarea, mutates `textarea.value` + caret **locally**, and dispatches a
synthetic `input` event. Everything downstream is the shipped machinery,
unchanged: overlay echo → debounced `Edit` → server working copy → dirty
diff → autosave → undo history. No new opcodes. No caret sync. Specifically:

- `u` / `Ctrl-R` → module clicks the existing `[data-kbd="mod+z"]` /
  `[data-kbd="mod+shift+z"]` elements (server undo stays the one history).
- `:w` (v1.5) → clicks `[data-kbd="mod+s"]`.
- Visual-mode highlight → native textarea selection. Free.
- Mode chip (NORMAL/INSERT/VISUAL) → a server-rendered `[data-vim-chip]`
  placeholder whose text/class the module updates client-side — input-method
  status is client-owned by definition.
- `data-kbd` global hook skips events whose target has `[data-vim]` while not
  in insert mode (Esc = leave insert, not cancel-prompt).

## Engine scope (locked 2026-07-07: motions for text editing, NO commands)

Modal *text editing* only — the `: ` ex-line is out of scope permanently, not
deferred. Modes: normal / insert / visual(char). Motions: `h j k l 0 $ ^ w b e
gg G` + counts (`3j`, `d2w`; wrap=off ⇒ logical lines == visual lines).
Operators: `d c y` + motion, `x dd yy cc D C p P o O a A i I`, `u`/`Ctrl-R`
via server history (clicking the data-kbd elements). Unnamed register
(module-local). Saving stays ⌘S / autosave — no `:w`.
**v1.5:** `V` line-visual, `f/t/F/T`, dot-repeat, block cursor (overlay
char-cell tint). **Out permanently:** ex commands, macros, marks, `:%s`.

## Plan (~2.5 days)

| Phase | Work | Size |
|---|---|---|
| **M1 extension primitive** | separate esbuild artifact + own size budget; server route serving vendored module; core loader on `[data-vim]`; `data-kbd` scoping | 0.5d |
| **M2 vim module** | `ext/vim.ts`: modal engine, motions/operators/counts, synthetic-input dispatch, chip updates; node:test suite against the mock DOM (~40 tests) | 1d |
| **M3 kit** | `.vim(true)` + status-bar toggle (like autosave) rendering `data-vim` + chip placeholder; insert-mode passthrough | 0.5d |
| **M4 proof** | examples/editor toggle (off by default), E2E (`dw` → disk via autosave; `i…Esc`; visual-`d`; `u` round-trip), docs page | 0.5d |

## Risks

- **IME / dead keys** in normal mode (`e.key`-based) — documented limitation,
  standard for web vim.
- **Synthetic-input fidelity**: the module must dispatch events the delegated
  dispatcher and echo hook both see (bubbling `Event("input")`) — covered by
  M1 tests.
- **Module staleness vs core**: vendored together, synced by the same
  `npm run sync` + CI drift gate.
- **preventDefault correctness**: unmodified keys only; browser-level chords
  untouched.

## Should the BASE runtime be split into lazy modules too? (evaluated: no)

Considered as part of this design (user question, 2026-07-07). Inventory of
split candidates in the 14.9KB core: sentinel ~400B, resize ~300B, router
~800B, clipboard ~150B, kbd ~250B, echo ~100B, tooltip-escape ~350B, client
actions ~1KB ≈ **3.3KB raw / ~1.2KB gz**. Unlike CSS/`MAP_DEF` (defs ride the
same message that first references them — race-free by construction), JS
modules arrive by async `import()`, so migrating opcode-backed features
(sentinel, resize) needs a pending-bindings queue in core (~+400B). Net win
≈ **<1KB gz**, paid with a race-window bug class in features freshly
stabilized by three E2E harnesses.

**Verdict: build the primitive (vim justifies it alone), don't migrate the
existing hooks.** Instead, stop the creep with policy:

- **Core budget frozen at 15.2KB raw / 5.5KB gz.** Raises need a reason the
  feature can't be an extension.
- New interaction features >~500B, or useful to a minority of apps, ship as
  extensions.
- Documented second wave IF capsule pressure returns: router → sentinel →
  resize. A decision for evidence, not symmetry.

## Open decisions

1. Default state of vim mode in the example/kit (`off` recommended — status-bar toggle).
2. Is visual-char enough for v1, or is `V` (line) a must-have?
3. Persist the vim preference (per-session state) or per-app default only?
