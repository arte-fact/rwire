# Documentation Update Roadmap

Tracks all doc/copy updates needed to reconcile the rwire docs site and app marketing
copy with the current codebase.

- **Audit date:** 2026-06-25
- **Codebase ref at audit:** `4c85275` (latest on `main`)
- **Docs written:** 2026-02-08 .. 02-11 (frozen ~4 months; only `router.md` + `tree-shaking.md` touched 2026-06-18)
- **Ground truth:** current code in `libs/`, not the docs.

> Convention: `- [ ]` open · `- [x]` done. Severity: **P0** compile-breaking / false headline ·
> **P1** architecturally wrong (rewrite) · **P2** stale numbers / inventory.
> Line numbers are audit-time hints and may drift — match on the quoted text.

---

## Why this is needed (context)

The docs were written in the project's first two weeks. The framework overhaul of
2026-06-16..24 (plus earlier token/theme refactors) invalidated many assumptions:

- Capsule strategy: the capsule is just the runtime. Element/event/attribute **name maps**
  (`MAP_DEF 0x88`) and utility **CSS** (`STYLE_DEF 0x87`) both stream lazily over the WebSocket
  (the name-map half landed in Phase Pre). No per-token tree-shaking into the capsule.
- `ThemeStyle` became a **struct** (was an enum); `AccentColor` / `ThemeColors` /
  `CapsuleConfig::accent|palette|colors` deleted; palettes moved to the `rwire-themes` crate.
- Theming no longer uses `data-theme` / `data-style` attributes — it's a server round-trip
  mutating `&mut Theme`.
- New router model: `outlet()` + `CurrentRoute` + `EventContext::navigate` (undocumented).
- CSS variables migrated to short names (`--a`, `--S4`, `--n1`…); `--rw-*` is dead.

---

## Progress overview

| Phase | Theme | Items | Done |
|-------|-------|-------|------|
| **Pre** | **Code: lazy-load JS name maps over the wire (like CSS) + measure** | **2** | **✅ 2 / 2** |
| P0 | Compile-breaking & false headline numbers | 9 | ✅ 9 / 9 |
| P1 | Architecture rewrites | 7 | ✅ 7 / 7 |
| P2 | Inventory & stale numbers | 7 | ✅ 7 / 7 |
| P3 | Verification & coverage gaps (links, README) | 2 | ✅ 2 / 2 |
| — | **Total** | **27** | **✅ 27 / 27** |

> **Phase Pre gates the size numbers.** Several doc/copy claims are about runtime/capsule
> size (P0-2, P2-7). We do not want to document the *current* number and then immediately change
> it. So: lazy-load the JS name maps over the wire (like CSS) → measure the real number → then
> write that measured number into the docs.

---

## Phase Pre — Lazy-load the JS name maps over the wire (like CSS), then measure

Engineering, not docs. Lands **before** P0-2 / P2-7 so the size numbers we write are real and stable.

**Approach (per the "do it like CSS, not tree-shaking" decision):** instead of statically
tree-shaking the element/event/attr/value maps into the capsule — which re-introduces the
structural-failure risk the `6ffcf40` removal was avoiding — deliver the name mappings **lazily
over the WebSocket**, exactly like utility CSS (`STYLE_DEF` + per-connection `sent_css`). The
static capsule ships the runtime with **empty** maps; the server sends each `(kind, code) → name`
definition the first time that code is referenced on a connection, deduped per connection.
Definitions arrive exactly when used → nothing can be missing (same guarantee as lazy CSS), and
dynamic content / navigation are covered for free with no startup static walk.

### Measured: counter = minimal app (before → after, 2026-06-25)

| Part | Before | After (lazy maps) | Notes |
|------|------:|------:|-------|
| **Capsule total** | **20,794** | **17,400** | **−3,394 B (−16%)** |
| `<style>` CSS globals | 4,428 | 4,428 | unchanged (already lazy) |
| `<script>` JS runtime | 16,168 | 12,774 | −3,394 B — name-map literals removed |
|  ↳ name-map literals | ~3,623 | 0 | now streamed via `MAP_DEF` |
|  ↳ runtime logic | ~12,500 | ~12,800 | unchanged + the small `MAP_DEF` handler |
| shell | 198 | 198 | |

**Realistic app (`rwire-docs`):** capsule **20,893 → 17,499 B**. Over the wire, the first full
render streams the names it actually uses once per connection (53 entries ≈ ~0.5 KB in one
`MAP_DEF` block at the front of the initial DOM); a subsequent navigation re-sent **0** names
(per-connection dedup via `sent_maps`, verified). So the ~3.4 KB that used to sit in *every*
capsule is replaced by a one-time ~0.5 KB per-connection payload of only the used names.

**Verified end-to-end** (WS probe): initial DOM begins with `MAP_DEF (0x88)`; all names decode
correctly across categories; SVG elements carry wire `kind` 6 (set both `E[code]` and `SE[code]`).
`cargo test --workspace` green, `cargo clippy --workspace` clean.

**Expected impact:** removes the ~3.6KB of map literals from the static capsule (counter
**20.3KB → ~17KB**; the lazy defs it then streams are tiny — ~tens of bytes, since counter uses
~3 elements + 1 event). Real apps pay-as-they-go, deduped per connection. It does **not** restore
the legacy "~1.5KB" — that figure predates the runtime-logic growth (morphing/client-actions/
router/highlighting added since Feb). The doc number (P0-2) is **whatever P-Pre-2 measures**, not 1.5KB.

- [x] **P-Pre-1 · Lazy map delivery over the wire (the CSS model)** ✅ done
  Source: `libs/rwire/src/protocol/opcodes.rs` (new opcode), `libs/rwire/src/protocol/encoder.rs`
  (emit defs), `libs/rwire/src/server.rs` (per-connection `sent_maps`, threaded through the
  synced-update build like `sent_css`), `libs/rwire/src/capsule_gen.rs` (client runtime + empty maps).
  - Add a `MAP_DEF` opcode (next free byte near `STYLE_DEF 0x87`), format
    `[MAP_DEF, kind, count, (code, len, name-bytes)...]`, `kind` ∈ {E, V, AT, AV, P, Y, SE}.
    Batch by kind; reuse the varint helpers.
  - Ship the capsule with **empty** maps: `E={}`, `V={}`, `AT={}`, `AV={}`, `P={}`, `Y={}`,
    `SE={}` (keep `A={4:'id'}`). Client handles `MAP_DEF` → populates the matching object. Keep
    the existing `||'div'` / `||'click'` fallbacks as a belt-and-suspenders safety net.
  - Per-connection `sent_maps: HashSet<(u8 kind, u8 code)>` (mirror `sent_css`). Before encoding
    any opcode that references a map code (`CREATE`/element, `BIND_*`/event, `SET_ATTR_ENUM`/at+av,
    `STYLE_PROP`/prop+value, SVG set), emit the missing `MAP_DEF`(s) first. Centralize so both the
    initial DOM and synced updates go through it.
  - **No structural-failure risk** — defs are sent exactly when the code is first referenced; this
    is the whole reason for choosing lazy over tree-shaking. **Keep lazy CSS as-is.**
  - Gate: `cargo clippy --workspace` clean, `cargo test --workspace` green, and manually verify
    all 4 apps + the `examples/*` render and stay interactive after navigation, theme toggle, and
    (design-system) component-page demos — i.e. no missing element/event name regressions.

- [x] **P-Pre-2 · Measure final JS runtime size** ✅ done (numbers above)
  After P-Pre-1, re-measure on a free port (9000 = SonarQube locally):
  - `counter` (minimal) and `rwire-docs` (realistic): capsule total, `<style>` CSS, `<script>` JS,
    plus the over-the-wire `MAP_DEF` bytes sent for a first full render.
  - Record the numbers here (add an "after" column to the baseline table).
  - These measured values are the **single source of truth** for P0-2 and P2-7. Do not edit the
    size copy until this is filled in.

> Note: lazy maps make the "small u8 maps shipped whole" statement in `tree-shaking.md` (and the
> capsule narratives in install.md / project-structure.md) **wrong** — after Phase Pre they must say
> "name maps are delivered lazily over the wire, like CSS." Folded into P1-5. Unlike tree-shaking,
> lazy delivery carries **no** structural-failure risk, so there's no trade-off to weigh.

---

## Phase 0 — Compile-breaking & false headline numbers (do first)

Anything here either fails to compile when copy-pasted or is the landing page's headline
number being wrong.

- [x] **P0-1 · website `.run_blocking()` doesn't exist**
  `apps/rwire-website/src/main.rs` (code snippet, ~line 344)
  `.root(root).run_blocking();` → `Server` only has `async fn run()`.
  **Fix:** `.root(root).run().await` inside an `async` main (as every real example does).

- [x] **P0-2 · website "~1.5KB tree-shaken JS runtime" is false** — _P-Pre-2 done; numbers ready_
  `apps/rwire-website/src/main.rs` (stats ~282, features ~415, comparison ~493)
  **Measured (post lazy-maps, minimal counter app):** full capsule **~17 KB**, of which JS runtime
  **~12.8 KB** and embedded CSS ~4.4 KB. **Fix:** replace "~1.5KB"/"tree-shaken" with these real
  numbers; distinguish "JS runtime (~13 KB)" from "full capsule (~17 KB, CSS embedded)". **Also
  recompute or remove** the dependent "20× / 200× / 60× smaller" and competitor multipliers — their
  1.5KB premise is gone.

- [x] **P0-3 · `ctx.form()` does not exist**
  `02-core-concepts/handlers.md` (~57, ~92-95), `02-core-concepts/events.md` (~92-94)
  No map accessor. **Fix:** `ctx.field("name") -> Option<&str>` (per-field).

- [x] **P0-4 · `elements.md` St-token names are wrong (two `.st([...])` examples won't compile)**
  `02-core-concepts/elements.md` (~56-62, ~73-76)
  **Fix:** `St::Flex`→`St::DisplayFlex`, `St::Gap4`→`St::GapMd`, `St::Rounded`→`St::RoundedMd`,
  `St::Px4`/`St::Py2`→`St::PxMd`/`St::PySm`, `St::RingPrimary`→`St::RingFocus`,
  `St::Outline0`→`St::OutlineNone`, `St::BgPrimaryActive`→(no equivalent; remove).

- [x] **P0-5 · `ThemeStyle::{Soft,Default,Brutalist,Minimal}` enum variants removed** — _custom.md + config.md done; theme-styles.md folded into P1-2 (full rewrite)_
  `04-theming/theme-styles.md` (throughout), `04-theming/custom.md` (~12,19), `05-advanced/config.md` (~107)
  `ThemeStyle` is now a struct. **Fix:** built-in `ThemeStyle::soft()`; other presets are
  `rwire_themes::styles::{solid,brutalist,minimal,glass,neon}`. Default style is **Soft**;
  the old "Default" look is `styles::solid()`.

- [x] **P0-6 · Nonexistent palette/theme constructors**
  `04-theming/palettes.md` (~24,~83), `05-advanced/config.md` (~101,~103), `01-getting-started/project-structure.md` (~47)
  `CapsuleConfig::dark_nord()`, `Theme::dark_nord()`/`light_nord()`, `ColorPalette::nord()` — none exist.
  **Fix:** `Theme::dark().palette(rwire_themes::palettes::nord())`. Named palettes are free
  functions in the `rwire-themes` crate, not methods on `ColorPalette`.

- [x] **P0-7 · `install.md` `features = ["docs"]` is invalid**
  `01-getting-started/install.md` (~82-86)
  `rwire` has no `[features]`. **Fix:** markdown is the separate `rwire-markdown` crate
  (`rwire-markdown = { path = ... }`). Also drop the unverified "Rust 1.75+" (no MSRV declared).

- [x] **P0-8 · `project-structure.md` `main()` chain won't compile**
  `01-getting-started/project-structure.md` (~43-51)
  `.persist_interval(...)` is a pre-`.root()` `ServerBuilder` method (can't chain after `.root()`),
  and `CapsuleConfig::dark_nord()` doesn't exist.
  **Fix:** call `.persist_interval(...)` before `.root(...)`; use `CapsuleConfig::new()` + `.theme(app_theme())`.

- [x] **P0-9 · examples "local" storage type doesn't exist**
  `apps/rwire-examples/src/main.rs` (~130, Multi-State TodoMVC description)
  Only `memory` and `persisted` exist. **Fix:** "memory and persisted storage types"
  (todo-combined uses two, not three).

---

## Phase 1 — Architecture rewrites (docs actively mislead)

These need a rewrite of the explanation, not a token swap.

- [x] **P1-1 · `dark-mode.md` core mechanism inverted (most stale doc)**
  `04-theming/dark-mode.md`
  Remove all `data-theme` attribute / `[data-theme="dark"]` selectors / "ships both light and
  dark CSS" / "no server round-trip" claims. **Correct model:** one `:root{}` block for the
  *current* mode; toggling is a server round-trip — a handler mutates `&mut Theme`
  (`theme.mode = theme.mode.toggle()`) and the synced `<style>` re-renders. Replace the
  `AppState { dark_mode: bool }` example with `fn toggle_theme(theme: &mut Theme)`.

- [x] **P1-2 · `theme-styles.md` `data-style` mechanism + preset set wrong**
  `04-theming/theme-styles.md`
  No `data-style` attribute / `[data-style="…"]` selectors; style overrides are written inline
  into the single `:root{}`. Rewrite around struct-based `ThemeStyle` + `rwire-themes` presets
  (5: solid, brutalist, minimal, **glass**, **neon**; built-in soft). Fix runtime style cycling
  to mutate `theme.style` / `theme.set_style(...)`, not an enum array indexed in app state.

- [x] **P1-3 · `router.md` conflates the two routing models**
  `05-advanced/router.md`
  Lead example shows `.routes(Router)` **and** `.on_route(handler)` together; a Router with no
  `outlet()` **freezes the page** (the bug just fixed in docs/design-system). Split into:
  **(a)** Router + `outlet()` + `CurrentRoute` (`CurrentRoute::param`), the SPA view-swap model
  (`examples/router`); **(b)** `on_route` + root re-render, no router/outlet
  (`apps/rwire-docs`, `apps/rwire-design-system`). Document `outlet()`, `CurrentRoute`, and
  `EventContext::navigate()` / `replace_route()` (server-initiated nav). Keep the (correct)
  `Link::to`, route-pattern, and history sections.

- [x] **P1-4 · `protocol.md` missing current opcodes + varint encoding**
  `02-core-concepts/protocol.md`
  Add `STYLE_DEF 0x87` (lazy CSS), the new `MAP_DEF` opcode (lazy name maps, from Phase Pre), and
  `SYMBOLS_EXTEND 0xF1`; note `ref`/`sym`/`handler` and symbol lengths are **varint**-encoded
  (1-3 bytes), not fixed bytes; mention `SET_TEXT_INT 0x15` / `SET_TEXT_WORDS 0x13`. Soften
  "interned per message" and the flat "3-byte text update" claim.

- [x] **P1-5 · Capsule narrative → "capsule = runtime; content-specific tables stream lazily"** _(reflects Phase Pre)_
  `01-getting-started/project-structure.md` (~134-140), `01-getting-started/install.md` (~tree-shaking mention),
  **and `05-advanced/tree-shaking.md`** (was accurate for "maps shipped whole", but Phase Pre changes that).
  Old docs say the capsule embeds per-token CSS / excludes unused `El::Textarea`/`Ev::Scroll`;
  `tree-shaking.md` says maps are "shipped whole" + CSS lazy. **After Phase Pre the unified, correct
  story is:** the static capsule is just the runtime; **both** utility CSS (`STYLE_DEF`) **and** the
  element/event/attr/value name maps (`MAP_DEF`) are delivered **lazily over the WebSocket**, deduped
  per connection. Update all three. `.routes()` does not drive capsule tree-shaking.

- [x] **P1-6 · `state.md` persisted storage = JSON files, not SQLite**
  `02-core-concepts/state.md` (~33-35, ~41)
  Persisted state uses `JsonFileStore` (`{key}.json` via serde_json), not SQLite.
  **Fix:** "survives restart via JSON files on disk (`JsonFileStore`)". Also verify the
  `#[storage(persisted, …, key = "…")]` form — key is set via a `#[key]` **field** attribute,
  not a `key = "…"` arg. Optionally mention the third storage type `Shared`.

- [x] **P1-7 · `project-structure.md` / docstring framing of `.capsule_config()` + `.routes()`**
  `01-getting-started/project-structure.md` (~57), and `Server::routes` rustdoc in `libs/rwire/src/server.rs`
  "Configures theme, colors, and component CSS" / `.routes()` "for view tree-shaking" are stale —
  theme/colors moved to `.theme()`; `.routes()` drives the `outlet()` runtime, not tree-shaking.

---

## Phase 2 — Inventory & stale numbers

- [x] **P2-1 · Unify the component count (inconsistent across 4 places)**
  Real: **50 modules / 51 catalog entries**. Today: website "51", design-system hero hardcodes
  "50", examples "25+", `03-components/overview.md` "50+".
  **Fix:** pick the catalog count (**51**) everywhere; prefer deriving it dynamically
  (`catalog::catalog().len()`) where rendered.
  Files: `apps/rwire-website/src/main.rs` (~285), `apps/rwire-design-system/src/main.rs` (~401),
  `apps/rwire-examples/src/main.rs` (~139), `03-components/overview.md` (~110).

- [x] **P2-2 · Remove fabricated `Prose` section**
  `03-components/data-display.md` (~144-165), `03-components/overview.md` (Data Display list)
  `Prose`/`ProseSize` don't exist. **Fix:** delete the section; rewrite around the real `Text`
  component (`Text::body()`, `Text::heading1()`, `.variant()`, `.color()`) if typography docs are wanted.

- [x] **P2-3 · Remove fabricated `TableOfContents`**
  `03-components/overview.md` (Utilities list, ~117)
  No such component. **Fix:** remove (no replacement).

- [x] **P2-4 · Document the 4 missing real components**
  `03-components/overview.md` (+ optionally dedicated sections)
  Add `AvatarGroup`, `DropdownMenu` (+`DropdownItem`), `Footer` (+`FooterColumn`), `Label`.
  After P2-2/P2-3/P2-4 the list reconciles to the real component set.

- [x] **P2-5 · Style-token count understated**
  `04-theming/tokens.md` (~129) "590+", `apps/rwire-website/src/main.rs` (~430) "580+"
  Actual: **716** `St` variants (max code `0x342`). **Fix:** "~700+".

- [x] **P2-6 · Replace placeholder links** ✅ done
  Set GitHub → `https://github.com/arte-fact/rwire` and **removed Discord** in all three app
  footers (website, examples, design-system), per the user.

- [x] **P2-7 · Re-validate perf numbers (benchmarked)** ✅ done
  `apps/rwire-website/src/main.rs` (comparison/stats). Benchmarked the two rwire-specific claims:
  - **Memory:** held **5,000** concurrent WS connections (server-side ESTABLISHED = 5000, 0 closed)
    in **3.85 MB total RSS** — sub-KB/connection. The old "200K+/GB" / "2–5KB/conn" were
    conservative. → stat kept "200K+/GB"; feature card now cites the measured anchor ("5,000
    connections in under 4 MB").
  - **Per-update wire size:** a real counter increment round-trip measures **31 bytes** (consistent),
    not 4. → "4 bytes" replaced with "~30 bytes" / "tens of bytes" in the stat, feature, and
    comparison "Update cost" row (rwire only; competitor cells left at their original approximate
    values under the softened "ballpark" subtitle).
  - Earlier (P0-2): removed the 1.5KB-derived 20×/200×/60× multipliers; rwire runtime row → ~17KB.
  - **Note:** the ~30 B figure is for the component counter (re-renders a styled `Text::heading1`
    + interns the new number string); a bare text patch is ~6 B. Competitor KB figures (LiveView
    30KB, Blazor 200KB, htmx 14KB) remain external/approximate — not independently benchmarked.

---

## Phase 3 — Verification & coverage gaps (beyond the original 25-item audit)

The original audit was claim-by-claim within the docs/app copy. This phase covers cross-cutting
and out-of-scope content the audit didn't reach.

- [x] **P3-1 · Fix broken internal doc cross-links** ✅ done
  Four links pointed at non-existent pages:
  - `state.md`, `project-structure.md`: `/docs/advanced/client-actions` → `/docs/05-advanced/client-actions`
  - `config.md`: `/docs/advanced/router` → `/docs/05-advanced/router` (+ dropped the stale "automatic
    tree shaking" wording)
  - `quick-start.md`: `/docs/02-core-concepts/concepts` (no such page) → `/docs/02-core-concepts/state`
  Re-scanned: every `/docs/…` link now resolves to a real page.

- [x] **P3-2 · Reconcile `README.md`** ✅ done (was not in the original audit)
  - "~1.5KB runtime" (×3: intro, ASCII diagram, benefits) → "~13KB runtime; names + CSS stream lazily"
  - "Tree Shaking" section → "Capsule Size" (empty maps + lazy `MAP_DEF`/`STYLE_DEF`, ~17KB capsule)
  - "tree-shaken per app" / "720+ tokens" → lazy-over-wire / "700+ tokens"
  - "52 components" (×2) → 51; todo-combined "SQLite persistence" → "JSON file persistence"
  - Quick-start snippet already used `#[async_std::main]` + `.run().await` (no change needed)

---

## Reference — verified clean (no action)

These verified accurate against current code:

- `01-getting-started/quick-start.md` — compiles as-is.
- `05-advanced/item-ref.md` — accurate (negligible 16383-vs-16511 boundary note only).
- `05-advanced/client-actions.md` — accurate (only the "~250 bytes" figure is unverified).
- ~~`05-advanced/tree-shaking.md`~~ — *was* accurate for "maps shipped whole", but **Phase Pre
  makes the maps lazy too** → now needs P1-5 (see Phase 1).
- `04-theming/breakpoints.md` — fully accurate.
- `03-components/forms.md`, `layout.md`, `feedback.md`, `navigation.md` — verified accurate.

---

## Per-file status matrix

| File | Phase items | Status |
|------|-------------|--------|
| `libs/rwire/src/protocol/opcodes.rs` (`MAP_DEF`) | P-Pre-1 | ✅ |
| `libs/rwire/src/protocol/encoder.rs` (referenced_names) | P-Pre-1 | ✅ |
| `libs/rwire/src/builder.rs` (`map_def_prefix`, threading) | P-Pre-1 | ✅ |
| `libs/rwire/src/server.rs` (`sent_maps`, build path) | P-Pre-1 | ✅ |
| `libs/rwire/src/capsule_gen.rs` (client JS + empty maps) | P-Pre-1, P-Pre-2 | ✅ |
| `apps/rwire-website/src/main.rs` | P0-1, P0-2, P2-1, P2-5, P2-6, P2-7 | ◐ P0 |
| `apps/rwire-examples/src/main.rs` | P0-9, P2-1, P2-6 | ◐ P0 |
| `apps/rwire-design-system/src/main.rs` | P2-1, P2-6 | ☐ |
| `docs/01-getting-started/install.md` | P0-7, P1-5 | ☐ |
| `docs/01-getting-started/quick-start.md` | — clean | ✅ |
| `docs/01-getting-started/project-structure.md` | P0-6, P0-8, P1-5, P1-7 | ☐ |
| `docs/02-core-concepts/state.md` | P1-6 | ☐ |
| `docs/02-core-concepts/handlers.md` | P0-3 | ✅ |
| `docs/02-core-concepts/renderers.md` | — clean | ✅ |
| `docs/02-core-concepts/elements.md` | P0-4 | ✅ |
| `docs/02-core-concepts/events.md` | P0-3 | ✅ |
| `docs/02-core-concepts/protocol.md` | P1-4 | ☐ |
| `docs/03-components/overview.md` | P2-1, P2-2, P2-3, P2-4 | ☐ |
| `docs/03-components/data-display.md` | P2-2 | ☐ |
| `docs/03-components/forms.md` | — clean | ✅ |
| `docs/03-components/feedback.md` | — clean | ✅ |
| `docs/03-components/layout.md` | — clean | ✅ |
| `docs/03-components/navigation.md` | — clean | ✅ |
| `docs/04-theming/tokens.md` | P2-5 | ☐ |
| `docs/04-theming/palettes.md` | P0-6 | ✅ |
| `docs/04-theming/dark-mode.md` | P1-1 | ☐ |
| `docs/04-theming/theme-styles.md` | P0-5, P1-2 | ☐ |
| `docs/04-theming/custom.md` | P0-5, P0-6 | ✅ |
| `docs/04-theming/breakpoints.md` | — clean | ✅ |
| `docs/05-advanced/router.md` | P1-3 | ☐ |
| `docs/05-advanced/item-ref.md` | — clean | ✅ |
| `docs/05-advanced/config.md` | P0-5, P0-6 | ✅ |
| `docs/05-advanced/tree-shaking.md` | P1-5 | ☐ |
| `docs/05-advanced/client-actions.md` | — clean | ✅ |

> `docs/` paths are under `apps/rwire-docs/docs/`.

---

## Suggested execution order

0. **Phase Pre** (code): lazy-load the JS name maps over the wire (`P-Pre-1`), then measure
   (`P-Pre-2`) and fill the "after" numbers into this file. Gates the size copy in P0-2 / P2-7.
1. **Phase 0** in one pass (compile/number fixes across docs + website) → show diff. The size
   rows (P0-2) use the P-Pre-2 number.
2. **Phase 1** rewrites: dark-mode → theme-styles → router → protocol → capsule narrative → state.
   (Note: Phase Pre makes the name maps lazy, so the "maps shipped whole" framing in
   `tree-shaking.md` / install / project-structure becomes "lazy over the wire" — that's P1-5,
   and `protocol.md`/P1-4 should document the new `MAP_DEF` opcode.)
3. **Phase 2** inventory cleanup (component count, Prose/TableOfContents, missing components, numbers, links).

Delete this file once all items are checked (per the repo's plan-document cleanup policy).
