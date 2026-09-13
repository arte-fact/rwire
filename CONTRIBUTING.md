# Contributing to rwire

rwire is experimental (0.x): breaking changes are normal, the protocol is
unstable by design, and the API is still finding its shape. Issues and small,
focused PRs are welcome; for anything large, open an issue first.

## Building and testing

```bash
cargo test --workspace            # everything Rust (748+ tests)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Node ≥ 22 is a **dev** dependency (never needed to build or consume the
crates): the wire round-trip harness runs the real browser runtime against
Rust-encoded fixtures, and skips silently when Node is absent — so install it,
or you are not running the whole suite.

## The JS runtime

The browser runtime is TypeScript at `runtime/` and is embedded into the
`rwire` crate as a checked-in artifact. Read `runtime/README.md` before
touching it. The workflow:

```bash
cd runtime
npm install
npm test          # build + unit suite (every opcode branch) + size budget
npm run sync      # the ONLY write path for libs/rwire/assets/runtime.min.js
```

Commit source and artifact together — CI rebuilds from source and fails on
drift. `runtime/src/opcodes.ts` is generated from `protocol/opcodes.rs`; add
opcodes on the Rust side.

## Code rules (the short version)

- Zero clippy warnings; fix, don't suppress. Remove dead code immediately.
- Every wire-format change lands with a round-trip fixture; every runtime
  change with a unit test; every component with catalog entry + tests.
- Comments state constraints the code can't; names over comments.
- See `CLAUDE.md` for the full working conventions.

## License

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as MIT OR Apache-2.0, without any additional
terms or conditions.
