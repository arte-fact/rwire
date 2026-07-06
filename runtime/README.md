# rwire-runtime

TypeScript source of the rwire browser runtime — the ~13KB script that executes
binary DOM opcodes over a WebSocket and sends events back. De-minified 1:1 from
the hand-minified `RUNTIME_JS` string in `libs/rwire/src/capsule_gen.rs`
(RELEASE_ROADMAP.md **RT1**); the built artifact replaces that string in **RT2**.

## Commands

```bash
npm install        # devDependencies: esbuild, typescript
npm run check      # tsc --noEmit (strict)
npm run build      # dist/runtime.js (readable) + dist/runtime.min.js
npm test           # build + node:test unit suite (every opcode branch) + size budget
npm run sync       # build + copy into libs/rwire/assets/runtime.min.js (the ONLY write path)
```

Full-stack sanity (manual): `cargo run -p counter`, then `node e2e/counter.mjs` —
boots the shipped artifact over a real WebSocket and asserts live round-trips.

Cross-validation against the Rust encoder (the real fixture set):

```bash
RWIRE_RUNTIME=$PWD/dist/runtime.min.js cargo test -p rwire --test wire_roundtrip
```

## Artifact contract

- **Injected by the capsule, before the bundle:** `const BASE='…'` (mount path;
  the only dynamic config). Everything else — name maps, opcode table, state —
  lives inside the bundle.
- **Exposed by the bundle:** `globalThis.__rwx` = the opcode executor `x()`
  (debug/testing handle; the wire harness drives it).
- **Fully static:** config flows through injected globals, never artifact
  templating. Client actions are always included (~250B; the old
  `has_client_actions` conditional collapses at RT2).
- **Minification:** identifiers only. Property names are never mangled —
  `__hk` (morph binding keys), `__t` (debounce timers), and the DOM API are
  load-bearing.

## Module map

| Module | Contents (original names kept) |
|---|---|
| `opcodes.ts` | GENERATED from `protocol/opcodes.rs` by `build.mjs` — do not edit |
| `state.ts` | shared mutable state `st` (symbols, word table, socket, composites, morph staging) + name maps `E/V/P/Y/AT/AV/SE`, `A` seed |
| `varint.ts` | `rv`/`wv` — mirrors `protocol/varint.rs` |
| `sanitize.ts` | `sa` — refuses `on*` attrs and `javascript:` URLs |
| `events.ts` | `gp` payload collection, `se`/`sep` wire messages, `snd` input debounce |
| `morph.ts` | `me`/`mk` node-reuse reconciliation, `fm` staged flush |
| `actions.ts` | client actions: target/selector stores + `uf2`/`us2` |
| `bind.ts` | `BL` local bind, `xi` WASM-opcode stub (see quirk note in file) |
| `executor.ts` | `x()` — the opcode loop; PARSE ERROR / Unknown-opcode formats are harness-load-bearing |
| `overlay.ts` | `ov` reconnect/offline overlay |
| `hash.ts` | `sh` scroll-to-hash with MutationObserver wait |
| `connect.ts` | WebSocket lifecycle, backoff, `bx`/`bj` base-path mapping |
| `router.ts` | data-route clicks, data-copy, Enter-submit, popstate |
| `index.ts` | bootstrap side effects (capsule order preserved) |

## Rules

- **Behavior-identical is the bar.** This is a port, not a rewrite; quirks are
  preserved and commented in place. Fix behavior in its own commit with a
  harness case, never silently during refactors.
- Every runtime change lands with a test — unit case here, plus a fixture in
  `libs/rwire/tests/wire_roundtrip.rs` when the wire format is involved.
- The size budget test (`test/size.test.ts`) fails the suite on bloat; raising
  the budget is a deliberate, justified act.
