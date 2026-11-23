# rwire — the client runtime (TypeScript)

The browser half is TypeScript compiled to a single minified asset that is **embedded in the
crate**. You rarely touch it; when you do, there is exactly one write path.

## Layout

```
runtime/
├── src/            TypeScript sources (connect.ts, executor.ts, morph.ts, router.ts, …)
├── test/           node --test suites
├── build.mjs       esbuild bundle → dist/runtime.min.js (+ dist/ext/<name>.min.js)
├── sync.mjs        copies dist/ into the crates' assets
└── package.json
```

## The four scripts

| Command | What it does |
|---|---|
| `npm run build` | bundles `src/` into `dist/runtime.min.js` and the extensions |
| `npm test` | builds, then runs `node --test test/*.test.ts` |
| `npm run check` | `tsc --noEmit` — types only, no build |
| `npm run sync` | copies `dist/` into `libs/rwire/assets/runtime.min.js`, `libs/rwire/assets/ext/vim.min.js`, `libs/rwire-map/assets/map.min.js`, `libs/rwire-push/assets/push.min.js` |

## The rule

**`libs/rwire/assets/runtime.min.js` and the `ext/*.min.js` files are build output. Edit
`runtime/src/`, then `npm run build && npm run sync`.** `sync.mjs` states it: it is *the only write
path* for the embedded artifact, and CI rebuilds and fails on `git diff`. A hand-edited asset is
therefore a red pipeline, not a shortcut.

So a runtime change is at least three files in one commit: the `src/*.ts` you changed, and the
regenerated `assets/runtime.min.js` the server will actually ship. Forgetting the second gives you
a change that compiles and does nothing.

## When to go there

The runtime owns the socket lifecycle, opcode execution, DOM morphing, client-side actions
(Target/Selector), overlays and route intercepts. Reach into it only for those; everything else
belongs on the server. Adding an extension means adding it to `EXTS` in `build.mjs` **and** to
`EXT_DEST` in `sync.mjs` (which decides whether it lands in `rwire`, `rwire-map` or `rwire-push`),
then serving it from the app — `serve_map()` (`libs/rwire-map`) and `serve_push()`
(`libs/rwire-push`) install their extension; the vim one is served by core rwire at
`/_rw/ext/vim.js`.

The capsule ships the runtime from the same binary, so there is no compatibility matrix between
an old client and a new server — that is the design (`README.md`, "Status & scope"). It is also
why a stale checked-in asset is so damaging: it silently pairs an old runtime with a new protocol.
