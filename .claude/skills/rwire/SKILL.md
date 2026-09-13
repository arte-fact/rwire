---
name: rwire
description: Building UIs on the rwire framework — components-first workflow, St/At/Av tokens, theming, and the traps a real app hit so you don't have to. Read before writing or reviewing rwire UI code.
---

# Building UIs on rwire

rwire renders synchronous Rust (`#[handler]` / `#[renderer]` / `el(...)` builders) over a
binary wire protocol — no JS/CSS source, no build step. This skill is the practical order of
operations for UI work, distilled from the first big production app on the framework
(claw-rwire). Follow the ladder: **theme → component → token → inline style**, top first.

## 1. The theme owns the aesthetic

Set the look once in `main()` and never re-assert it per element:

```rust
Theme::dark()
    .palette(palettes::nord())
    .style(ThemeStyle::flat())      // terminal look: mono UI font, hairline borders, no shadows
    .radius(RadiusScale::None)      // sharp corners — themed components follow automatically
```

- `ThemeStyle::soft()` is the default (tinted, rounded, shadow-free); `ThemeStyle::flat()`
  adds the monospace UI font via `--Qft`. Custom presets: implement `IntoStyle`.
- `RadiusScale` scales every `Rounded*` token except `RoundedFull` (circles stay circles).
- Because radius/shadow/font are CSS-var based, **components inherit the theme** — never
  sprinkle `FontMono` / `Rounded0` / `FontInheritAll` to enforce a look (the base reset
  already applies `font:inherit` to form controls).
- Colours only via semantic tokens (`St::BgApp`, `St::TextMuted`, `St::BorderDefault`, …),
  never hex. The palette maps them.

## 2. Components before markup

`rwire-components` has ~55 components, all `St`-token based, catalogued in
`libs/rwire-components/src/catalog.rs` (which auto-generates the design-system app —
`apps/rwire-design-system`). Reach for them before hand-rolling:

| You're about to build… | Use instead |
|---|---|
| a button of any kind | `Button::{primary,secondary,ghost,outline,destructive}(…)` — sizes `Xs` (24px dense chrome) to `Lg`; you get the focus-visible ring, hover/disabled/loading states, icon handling free |
| an icon-only button | `Button::icon_only(icon, "Aria label")` — squares itself, carries the aria-label |
| a status chip / label | `Badge::new().intent(…)` + `.shape(BadgeShape::Square)` + `.fill(BadgeFill::Outline)` for the sharp outlined look |
| a selectable filter / tab / view toggle | `Chip::new(label).active(bool).on_click(h)` — chain `.data(…)` for the payload |
| a presence / activity dot | `StatusDot::new().intent(…).pulse(bool).label("running")` |
| a chat input bar | `Composer::new().id(key).placeholder(…).on_submit(h)` — Enter submits, Shift+Enter newline; `.compact(true)` for inline bars |
| a chat / log scroll area | `ChatScroll::new(inner)` — see the trap in §5 |
| an avatar with initials | `Avatar::new().name("Ada Lovelace")` — derives "AL" |
| form fields | `Input::text()/email()/number()`, `Textarea`, `Select`, `Checkbox`, `Switch` — they take `name/placeholder/autocomplete/spellcheck/rows` as methods |
| empty screens | `EmptyState::new().title(…).description(…).action(…)` |
| overlays | `Modal`, `Drawer` (position, backdrop, on_close) |

**Clearing an input after submit**: key the field's `id` by something that changes when the
submit lands (e.g. `format!("composer-{}", messages.len())`) — a fresh element is an empty
element. No client opcode needed. `Composer` documents this pattern.

**When a pattern repeats** (≥2 sites, or it encodes a hard-won fix): promote it into
`rwire-components` instead of copying — add the module, `pub use` it in `lib.rs`, register a
`ComponentEntry` in `catalog.rs` (name/slug/category/variants/bools/`build_demo`), and it
appears in the design-system app automatically. Gate with
`cargo fmt && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.

## 3. Tokens before strings

- **Styles**: `St::*` covers every constant (`grep style_tokens.rs` first). Inline
  `.style(Style::new().set(…))` is *only* for genuinely dynamic values — `calc()`, `vh`
  offsets, depth-based padding, CSS custom properties. If a constant lacks a token, **add the
  token** to `style_tokens.rs` (next free hex index) rather than inlining.
- **Attributes**: `.at(At::Type, Av::Submit)`, `.at_str(At::Name, v)`, `.bool_attr(…)` —
  never string `.attr("k","v")`. Missing key/value? Add it to `attr_tokens.rs`.
- Full-height is `St::HDvh` / `St::MinHDvh` (dynamic viewport) — never raw `100vh`.

## 4. Reactivity + routing rules

- Wrap changing UI in `#[renderer]` (a synced region); static scaffolding stays plain `el()`.
  Keep hot regions small — a token stream should repaint one entry, not a board.
- Lists: `ItemRef` + `iter_with_ref()` + `on_ref()` — no `data-*` index strings for identity.
  (Plain `.data(…)` payloads on buttons are fine for handler args.)
- Navigate with `Link::route(…)`, never raw `<a href>` (that's a full reload).
- One routing model per page tree: `on_route` **or** `routes` + `outlet()` — a `Router`
  without an `outlet()` freezes the page.
- Enter-to-submit on a textarea: set `data-enter-submit` (the runtime handles
  preventDefault + `requestSubmit`; Shift+Enter falls through to a newline). Don't bind
  keydown yourself — rwire `preventDefault`s bound keydowns, which blocks typing.

## 5. Traps a real app hit (each cost a debugging session)

- **Bottom-pinned chat scroll**: `column-reverse` + an **auto bottom margin** on the inner
  column (`St::MbAuto`) — **never** `justify-content` on the scroll container: justify makes
  overflowing content unreachable, while an auto margin collapses to 0 on overflow. This is
  exactly what `ChatScroll` encodes; use it.
- **Width collapse**: a width-spanning child of a flex-*centered* parent needs `St::WFull`
  or it shrinks to content (~0 for empty divs).
- **Class order ≠ specificity**: `u`-token classes apply in *stylesheet* (token-index) order,
  not `.st()` call order — you cannot "override" one token with a later conflicting one.
  Pick the right token instead of stacking.
- **Shared-state renderers inside memory-state renderers** don't receive the shared cache —
  keep shared (`App`) renderers nested under shared parents.
- **`#[derive(State)]` field ceiling is 64** per struct — split view models out (re-export
  flat so callers don't churn).
- **Keyed swaps**: when switching what an entire region shows (e.g. another agent's thread),
  key the container `id` by the subject so the client swaps clean instead of diffing across
  subjects.

## 6. Reference

- Component source + catalog: `libs/rwire-components/src/`
- Tokens: `libs/rwire/src/style_tokens.rs`, `attr_tokens.rs`
- Theme system: `libs/rwire/src/theme.rs` (palettes, styles, radius, base CSS)
- Live playground: run `apps/rwire-design-system`
- Reference apps: `../claw-rwire` (large, flat-theme), `apps/rwire-website`, examples/
