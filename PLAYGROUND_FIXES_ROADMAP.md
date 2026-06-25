# Design-System Playground — Fix Roadmap

Tracks the work to make the design-system playground deliver its goal: **let a user adjust a
component's params via controls and get a live preview + a copy-pasteable, compiling Rust snippet
that matches.**

- **Audit date:** 2026-06-25
- **Scope:** `apps/rwire-design-system/src/main.rs` (playground + codegen) and
  `libs/rwire-components/src/catalog.rs` (per-component metadata + demos); component sources under
  `libs/rwire-components/src/`.
- Convention: `- [ ]` open · `- [x]` done.

## Root cause

The playground has only **two control types** — variant selectors (enums) and bool checkboxes —
and a generic code generator (`build_code_example`, main.rs:678):
`{entry.name}::new()` + `.{axis.name}({rust_expr})` (non-default only) + `.{prop.name}({val})`
(non-default only) + `.build()`. Consequences:

1. **No text / numeric / range / list control** → params like `value`, `max`, `label`, `title`,
   page counts, lines, options, items can't be adjusted *or* shown in the snippet (e.g. `Progress`
   → snippet is always `Progress::new().build()` with no way to set `value`).
2. **`{Name}::new()` ignores the real constructor** → required ctor args, named constructors,
   multi-arg setters, and bool-prop/setter name mismatches produce snippets that **don't compile**.

## Progress overview

| Phase | Theme | Items | Done |
|-------|-------|-------|------|
| P0 | Quick wins (catalog one-liners, no new infra) | 9 | 9 / 9 |
| P1 | New **Text + Number(slider)** param controls (infra + per-component) | 5 + 25 | 30 / 30 |
| P2 | Constructor-aware codegen (fix non-compiling snippets) | 6 | 6 / 6 |
| P3 | Composite children in snippets | 3 + 18 | 21 / 21 |
| P4 | Verification | 4 | 4 / 4 |
| — | **Total** | **65** | **65 / 65 ✅** |

> Suggested order: **P0 → P1 → P2 → P3 → P4**. P0 is cheap and unblocks correctness; P1 fixes the
> headline complaint (adjustable numeric/text params, incl. `progress`); P2 makes snippets compile;
> P3 makes composite snippets faithful.

---

## Phase 0 — Quick wins (one-line catalog/codegen edits)

- [x] **P0-1 · Fix the placeholder `SM_MD_LG_SIZE` const** (catalog.rs). It has blank
  `rust_type`/`rust_expr`, so selecting Sm/Lg emits `.size()` (no arg → won't compile) and the
  props table shows an empty "Type". Give avatar / spinner / theme-toggle their real size axes
  (`AvatarSize::Sm/Md/Lg`, `SpinnerSize::…`, `ToggleSize::…`) with proper `rust_expr` + `rust_type`.
- [x] **P0-2 · Fix the `Dropdown` entry name** → the type is `DropdownMenu`, so the snippet emits
  `Dropdown::new()` (won't compile). Either rename `entry.name` to `"DropdownMenu"` or special-case.
- [x] **P0-3 · Rename mismatched bool props to the real setter names:** link `external` →
  `is_external`; list `ordered` → `is_ordered` (the bare names are constructors, not `(bool)` setters).
- [x] **P0-4 · Handle no-arg toggles:** spacer `horizontal` and divider `vertical` map to no-arg
  methods (`.horizontal()` / `Divider::vertical()`), so `.horizontal(true)` won't compile. Model
  them so codegen emits the no-arg/named-ctor form (overlaps P2).
- [x] **P0-5 · Fix form-field `error` bool** → the setter is `.error(impl Into<Cow<str>>)` (a
  message), not `.error(bool)`. Make it a text param (P1) or special-case; current `.error(true)`
  won't compile.
- [x] **P0-6 · theme-toggle `mode` → variant axis** (`ThemeToggleMode::Light/Dark`). It's an enum,
  so the existing variant-selector control covers it; currently hardcoded in the demo only.
- [x] **P0-7 · Add `close_on_backdrop_click` bool to modal** (real setter, default true) — currently
  unexposed.
- [x] **P0-8 · Add `show_label` bool to theme-toggle**; add `required` + `invalid` bools to radio
  and switch (all real setters, currently unexposed).
- [x] **P0-9 · Add an `aspect` variant axis to image** (`ImageAspect::Auto/Square/Video`) — real
  named methods `aspect_square()/aspect_video()`, currently unexposed.

---

## Phase 1 — Add **Text** and **Number** param controls

### Infrastructure

- [x] **P1-A · Param model** — extend `ComponentEntry` (catalog.rs) with text + number params.
  Recommended: a richer demo context instead of growing positional args. Replace
  `build_demo: fn(&[usize], &[bool]) -> ElementBuilder` with `fn(&DemoParams) -> ElementBuilder`
  where `DemoParams { variants: &[usize], bools: &[bool], texts: &[&str], numbers: &[i64] }`, and add
  `text_props: &[TextProp]` / `num_props: &[NumProp]` (each: name, default, + for numbers min/max/step).
- [x] **P1-B · State** — add `text_states: Vec<String>` and `num_states: Vec<i64>` to
  `DesignSystemState` (main.rs); seed from defaults on navigation (next to `bool_states`).
- [x] **P1-C · Controls** — in `build_playground`, wired to new handlers (`set_text_prop` /
  `set_num_prop`) that update the state vectors by index:
  - **Number params → `Slider` (range) controls by default** (use `NumProp.min/max/step`), so they're
    easy to drag. Show the **live numeric value beside the slider**. Only fall back to a number field
    when a slider doesn't fit (truly unbounded values).
  - **Text params → `Input::text().on_input_debounced(...)`** with the param name as the label.
  - **Slider component reworked** (`slider.rs` + `SliderThumb` CSS): overlay the native range input on
    the track (thumb now sits *on* the bar), bind `Ev::Change` not `Ev::Input` (a re-render per drag
    tick was interrupting the drag — sliders now drag smoothly), plus hover/active thumb states.
  - **`NumProp.slider` flag**: sliders for visual magnitudes (progress value, page numbers), **number
    inputs** for discrete counts (max_visible, rows, step) — "number input where more suited".
- [x] **P1-D · Codegen** — extend `build_code_example` to emit `.{name}("{text}")` and
  `.{name}({number})` for non-default text/number params.
- [x] **P1-E · UX polish (make the playground inviting & easy to use)** — a labelled, well-aligned
  controls panel: each control row has a clear name label and consistent spacing; sliders show their
  live value; group variants / bools / sliders / text sensibly; keep the live preview and the
  copy-able snippet prominent; ensure controls are keyboard-accessible. The playground should *feel*
  like a polished interactive doc, not a form dump.

### Per-component param additions (after infra)

- [x] **P1-1 · progress** — number `value`, number `max`, text `label` _(the canonical fix)_
- [x] **P1-2 · slider** — number `min`, `max`, `value`, `step`; text `label`
- [x] **P1-3 · pagination** — number `current_page`, `total_pages`, `max_visible`
- [x] **P1-4 · textarea** — text `placeholder`, number `rows`
- [x] **P1-5 · tabs** — number `active` (composite tabs → P3)
- [x] **P1-6 · stepper** — number `current` (composite steps → P3)
- [x] **P1-7 · skeleton** — number `lines` (named ctor → P2)
- [x] **P1-8 · app-shell** — number `sidebar_width`, `header_height` (slots → P3)
- [x] **P1-9 · avatar-group** — number `max_visible` (composite avatars → P3)
- [x] **P1-10 · alert** — text `title`, `message`
- [x] **P1-11 · badge** — text `text`
- [x] **P1-12 · button** — text `text`
- [x] **P1-13 · checkbox** — text `label`
- [x] **P1-14 · switch** — text `label`
- [x] **P1-15 · drawer** — text `title`
- [x] **P1-16 · modal** — text `title`
- [x] **P1-17 · empty-state** — text `title`, `description`
- [x] **P1-18 · input** — text `placeholder`, `value`
- [x] **P1-19 · text** — text `content`
- [x] **P1-20 · footer** — text `tagline`, `copyright`
- [x] **P1-21 · nav-menu** — text `active_path`
- [x] **P1-22 · select** — text `aria_label`
- [x] **P1-23 · spinner** — text `label`
- [x] **P1-24 · avatar** — text `fallback` (and `src`/`alt`)
- [x] **P1-25 · stat / tag / toast / tooltip / label / kbd / link / image / blockquote / copy-button**
  — text params for their constructor/string args (the **required-arg** ones are wired in P2's
  constructor-aware codegen; this item adds the text controls that feed them).

---

## Phase 2 — Constructor-aware codegen (fix snippets that don't compile)

Replace the hardcoded `{Name}::new()` with a per-entry constructor description so the snippet
reflects the real API. Recommended: add a `ctor` spec (or an optional per-entry code closure for the
gnarly ones). Then fix each class:

- [x] **P2-1 · Required-arg constructors** — emit `Name::new("{text}")` / `Name::new({num})`:
  blockquote (content), copy-button (text), image (src), kbd (key), label (text), tag (text),
  toast (message), tooltip (text), stat (value), spacer (size).
- [x] **P2-2 · Named constructors** — emit the named form from a variant:
  skeleton (`Skeleton::{shape}()`), code (`Code::{mode}("{content}")`). These have **no `::new()`**.
- [x] **P2-3 · Multi-arg setters** — stat `.trend({dir}, "{text}")` (axis currently emits one arg).
- [x] **P2-4 · No-arg toggles** — spacer `.horizontal()` / divider `Divider::vertical()` (from P0-4).
- [x] **P2-5 · Type-name / bool-name** — dropdown `DropdownMenu` (P0-2), link `is_external`,
  list `is_ordered` (P0-3), form-field `error(&str)` (P0-5) — verify codegen now compiles.
- [x] **P2-6 · Compile-check the generated snippets** — add a test or script that feeds every
  entry's generated snippet (at defaults + a couple of selections) into a throwaway compile to catch
  regressions (see P4-2).

---

## Phase 3 — Composite children in snippets

These components are built from repeated children the variant/bool model can't represent. Have
codegen emit the demo's representative children (e.g. `.item(...)`, `.children([...])`,
`.row(...)`), so the snippet reproduces the preview. Add a per-entry "children snippet" string or
closure.

- [x] **P3-A · Mechanism** — add an optional `children_code: Option<&str>` (or closure) to
  `ComponentEntry`, inserted into the generated chain before `.build()`.
- [x] **P3-B · Document the limitation** in the playground UI for components where children are
  inherently dynamic (a short "snippet shows representative children" note).
- [x] **P3-C · Apply to each composite component:**
  - [x] table (headers + rows)
  - [x] tabs (tabs + `active`)
  - [x] timeline (items)
  - [x] select (options)
  - [x] stepper (steps)
  - [x] nav-menu (items)
  - [x] footer (logo, columns)
  - [x] breadcrumb (items)
  - [x] accordion (items)
  - [x] dropdown (trigger + items)
  - [x] grid (children)
  - [x] list (items)
  - [x] modal (content)
  - [x] drawer (content)
  - [x] form-field (input)
  - [x] app-shell (header/sidebar/main slots)
  - [x] avatar-group (avatars)
  - [x] tooltip (child) + radio (group) + card/container (child)

---

## Phase 4 — Verification

- [x] **P4-1 · Interaction sweep** — re-run the all-50 variant×bool×(new text/number) sweep
  (Playwright) for console errors + render collapse.
- [x] **P4-2 · Snippet-compiles check** — verify each generated snippet compiles (a `trybuild`-style
  test, or generate a temp crate that pastes each snippet with the right `use`s).
- [x] **P4-3 · Visual pass** — spot-check that preview == snippet for progress, slider, pagination,
  stat, toast, table, tabs, and the overlay components.
- [x] **P4-4 · Props table** — confirm the props table reflects the new text/number params and that
  no "Type" column is empty (the `SM_MD_LG_SIZE` symptom).

---

## Per-component gap matrix

Legend: **C1** = snippet won't compile · **C2** = snippet ≠ preview (text/number dropped) ·
**Comp** = composite children dropped · **Hidden** = real params not exposed.

| Component | Gaps | Fixed by | Done |
|-----------|------|----------|------|
| progress | C2, Hidden (value/max/label) | P1-1 | ✅ |
| slider | C2, Hidden (min/max/value/step/label) | P1-2 | ✅ |
| pagination | C2, Hidden (current/total/max) | P1-3 | ✅ |
| textarea | C2 (placeholder/rows) | P1-4 | ✅ |
| tabs | Comp, Hidden (active) | P1-5, P3 | ✅ |
| stepper | Comp, Hidden (current) | P1-6, P3 | ✅ |
| skeleton | **C1** (named ctor), Hidden (lines) | P2-2, P1-7 | ✅ |
| app-shell | Comp, Hidden (sidebar_width/header_height) | P1-8, P3 | ✅ |
| avatar-group | Comp, Hidden (max_visible) | P1-9, P3 | ✅ |
| alert | C2 (title/message) | P1-10 | ✅ |
| badge | C2 (text) | P1-11 | ✅ |
| button | C2 (text, minor) | P1-12 | ✅ |
| checkbox | C2 (label) | P1-13 | ✅ |
| switch | C2 (label), Hidden (required/invalid) | P1-14, P0-8 | ✅ |
| drawer | C2 (title), Comp (content) | P1-15, P3 | ✅ |
| modal | C2 (title), Comp (content), Hidden (close_on_backdrop) | P1-16, P3, P0-7 | ✅ |
| empty-state | C2 (title/description), Comp (icon/action) | P1-17, P3 | ✅ |
| input | C2 (placeholder/value) | P1-18 | ✅ |
| text | C2 (content) | P1-19 | ✅ |
| footer | C2 (tagline/copyright), Comp (logo/columns) | P1-20, P3 | ✅ |
| nav-menu | C2 (active_path), Comp (items) | P1-21, P3 | ✅ |
| select | C2 (aria_label), Comp (options) | P1-22, P3 | ✅ |
| spinner | **C1** (empty rust_expr), C2 (label) | P0-1, P1-23 | ✅ |
| avatar | **C1** (empty rust_expr), C2 (fallback/src/alt) | P0-1, P1-24 | ✅ |
| theme-toggle | **C1** (empty rust_expr), C2 (mode→variant), Hidden (show_label) | P0-1/6/8 | ✅ |
| stat | **C1** (req ctor + multi-arg trend), C2 (label/desc) | P2-1/3, P1-25 | ✅ |
| tag | **C1** (req ctor text) | P2-1, P1-25 | ✅ |
| toast | **C1** (req ctor message) | P2-1, P1-25 | ✅ |
| tooltip | **C1** (req ctor text), Comp (child) | P2-1, P1-25, P3 | ✅ |
| label | **C1** (req ctor text) | P2-1, P1-25 | ✅ |
| kbd | **C1** (req ctor key) | P2-1, P1-25 | ✅ |
| link | **C1** (req ctor href + `is_external`), C2 (text) | P2-1/5, P1-25, P0-3 | ✅ |
| image | **C1** (req ctor src), C2 (alt), Hidden (aspect) | P2-1, P1-25, P0-9 | ✅ |
| blockquote | **C1** (req ctor content), C2 (cite) | P2-1, P1-25 | ✅ |
| copy-button | **C1** (req ctor text) | P2-1, P1-25 | ✅ |
| spacer | **C1** (req ctor size + no-arg horizontal) | P2-1/4, P0-4 | ✅ |
| dropdown | **C1** (`Dropdown`→`DropdownMenu`), Comp (trigger/items) | P0-2, P3 | ✅ |
| code | **C1** (named ctor + content/language) | P2-2 | ✅ |
| divider | **C1** (`vertical`→named/no-arg) | P0-4, P2-4 | ✅ |
| list | **C1** (`ordered`→`is_ordered`), Comp (items) | P0-3, P3 | ✅ |
| form-field | **C1** (`error(&str)`), C2 (label/help), Comp (input) | P0-5, P1, P3 | ✅ |
| radio | C2 (label/name/value), Comp (group), Hidden (required/invalid) | P1, P3, P0-8 | ✅ |
| table | Comp (headers/rows) | P3 | ✅ |
| timeline | Comp (items) | P3 | ✅ |
| breadcrumb | Comp (items) | P3 | ✅ |
| accordion | Comp (items) | P3 | ✅ |
| grid | Comp (children) | P3 | ✅ |
| card | Comp (child, minor) | P3 | ✅ |
| container | Comp (child, minor) | P3 | ✅ |
| stack | clean (children are scaffolding) | — | ✅ |

> Delete this file once all items are checked.
