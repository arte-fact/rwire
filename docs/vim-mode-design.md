# Vim mode for FileEditor — evaluation & plan (draft 2026-07-07)

## The architectural question: where does vim state live?

Modal editing is keystroke-latency-sensitive: `j` must move the caret *now*.
Four options were evaluated:

| Option | Verdict |
|---|---|
| **A. Full server-side engine** (every normal-mode key is a round trip) | **Recommended** — see below |
| B. Client-side vim interpreter in the runtime | Rejected: a usable core is 3–10KB min (CodeMirror's vim is ~60KB); blows the 15KB budget and puts app logic in the browser — against the framework's soul |
| C. Hybrid (client motions, server mutations) | Rejected for v1: duplicates motion semantics in two languages; can be layered onto A later without changing the wire contract if internet latency ever matters |
| D. Embed CodeMirror/Monaco | Already rejected in the syntax-coloring survey; doubly so here |

**Why A wins here:** rwire's real consumers (claw, llama-modnitor, the examples)
are localhost/LAN apps — RTT ~1ms, so server-side vim feels native. The engine
becomes **pure Rust** (`fn key(&mut VimState, text, caret, key) -> VimEffect`),
unit-testable without a browser. And every mutation flows through the existing
working-copy path, so **autosave, dirty tracking, undo/redo, and conflict
gating compose for free** — vim `u` literally maps to `Action::Undo`.
Trade-off: over high-latency links motions feel heavy; vim mode is opt-in and
the modeless editor remains the default.

## Framework gaps to fill (the reusable part)

Verified against the current tree:

1. **`Ev::Keydown` + `EventPayload::Key`** — no keydown event type exists;
   payloads are Text/Data/Form/Empty. New payload carries
   `{ key, mods, sel_start, sel_end }` — **caret position rides on every
   keystroke**, so the server never tracks caret between keys (no drift;
   clicking around in normal mode just works).
2. **`SET_SELECTION` opcode** (server → client caret/selection control on an
   element ref). The runtime already calls `setSelectionRange` in the
   BATCH_END focus-restore path — this promotes it to a first-class opcode.
   Also useful beyond vim: caret restore after undo/redo re-keying.
3. **Conditional preventDefault** — client-side, synchronous, attribute-driven:
   when the textarea carries `data-vim` (server-rendered mode flag), the
   keydown binding preventDefaults unmodified printable keys; insert mode has
   no keydown binding at all (today's editor IS insert mode). The `data-kbd`
   global hook must skip targets with `data-vim` (Esc means "leave insert",
   not "cancel prompt", while the editor has vim focus).
4. Runtime budget: ~+300–400B → next bump justified.

## Engine scope

**v1** — modes: normal / insert / visual(char). Motions: `h j k l 0 $ ^ w b e
gg G` (+counts, e.g. `3j`, `d2w`; wrap=off means logical lines == visual
lines — `j`/`k` are exact). Operators: `d c y` + motion, `x dd yy cc D C p P`,
`o O a A i I`, `u` → existing undo stack, `Ctrl-R` → redo. Unnamed register
only. Mode chip in the status bar (NORMAL/INSERT/VISUAL, distinct tones);
visual mode renders through the native textarea selection via SET_SELECTION —
free highlighting.

**v1.5 backlog:** `V` line-visual, `f/t/F/T`, dot-repeat (needs change
recording), named registers, `:w :q` ex-line, block-cursor rendering (overlay
tint on the caret's char cell).

**Out of scope:** macros, marks, plugins, `:%s` — this is a modal-editing
affordance, not a vim clone.

## Plan (phases, ~2.5 days)

| Phase | Work | Size |
|---|---|---|
| **V1 transport** | `Ev::Keydown`, `EventPayload::Key` (key+mods+caret), `SET_SELECTION` opcode (next free El-op slot), `data-vim` preventDefault rules, `data-kbd` scoping, runtime tests | 0.5d |
| **V2 engine** | `VimState` + `key()` reducer in rwire-editor, pure Rust, ~40 unit tests (motions, operators, counts, register, mode transitions) | 1d |
| **V3 kit** | `FileEditorState.vim` + status-bar toggle (like autosave), mode chip, insert-mode passthrough, undo/redo mapping, generation interplay | 0.5d |
| **V4 proof** | Enable in examples/editor (off by default), E2E: `dw` hits the disk via autosave, `i…Esc,u` round-trips, visual-`d`; docs page | 0.5d |

## Risks

- **IME / dead keys** in normal mode (`e.key`-based): documented limitation;
  standard for web vim implementations.
- **preventDefault correctness**: only unmodified keys — never eat
  browser-level chords (⌘L, ⌘T are uninterceptable anyway).
- **Key-event volume**: ~15 keys/s × ~30B is negligible on the wire.
- **Latency over internet**: opt-in feature; hybrid motion-echo (option C) is
  the future escape hatch, layerable without wire changes.

## Open decisions

1. Default state of vim mode in the example/kit (`off` recommended — discoverable via status-bar toggle).
2. Is visual-char enough for v1, or is `V` (line) a must-have?
3. `:w`/`:q` ex-line in v1 or v1.5? (v1.5 recommended; ⌘S already saves.)
4. Persist the vim preference per session (memory state) or per app default only?
