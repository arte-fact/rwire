# rwire — components, theme, and the no-free-CSS rule

## 1. The gallery decides, not your memory

`libs/rwire-components/src/catalog.rs` is a catalogue of `ComponentEntry` (name, slug, category,
variants, bool props, and a `build_demo` closure). `apps/rwire-design-system` renders that
catalogue into a live gallery: sidebar grouped by category, a preview per variant combination.

Run it and look:

```bash
cargo run -p rwire-design-system     # BIND_ADDR defaults to 0.0.0.0:9002
```

Use it as the arbiter for "which component, which variant". If a need is not covered, the
answer is a new component in `rwire-components`, not a hand-rolled `el()` tree.

**Catalogue size, measured on `main`, not quoted:** 69 `ComponentEntry` records — Layout 11,
Forms 13, Data Display 20, Feedback 8, Navigation 10, Overlay 7 — of which five are guide pages
rather than components (`Three-column shell`, `Navigation`, `Tones`, `Density`, `Canvas & fonts`).
`libs/rwire-components/src/lib.rs` exports 71 names, and the directory holds 73 `.rs` files.
`README.md` advertises "90+ components", which no count in the tree supports. Count in
`catalog.rs` before writing a number anywhere.

## 2. Component by need

Layout

| Need | Use |
|---|---|
| app frame (sidebar + header + content) | `AppShell` |
| vertical / horizontal rhythm | `Stack` (`wrap`) |
| column layout | `Grid` |
| centred max-width page | `Container` (`centered`) |
| group content | `Card` (`bordered`) |
| push things apart | `Spacer` (`horizontal`) |
| separate two blocks | `Divider` (`margin`) |
| two resizable panes | `SplitPane` |
| chat/log scroll area | `ChatScroll` |

Forms

| Need | Use |
|---|---|
| any button | `Button::{primary, secondary, ghost, outline, destructive}(text)` |
| icon-only button | `Button::icon_only(icon, "Aria label")` — squares itself, carries the label |
| text box | `Input` |
| multi-line | `Textarea` |
| choice list | `Select`, `Radio`, `Checkbox`, `Switch` |
| ranged value | `Slider` |
| label above a field | `Label`, `FormField` |
| chat input bar | `Composer` (Enter submits, Shift+Enter newline) |
| code editing surface | `CodeEditor` |
| plain server-handled form | `rwire::form::Form` + `Field::…` — see `recipes.md` |

Data display

| Need | Use |
|---|---|
| prose | `Text` |
| short status word | `Badge` (`intent`, `shape`, `fill`, `tone`, `mono`, `size`) |
| presence / activity dot | `StatusDot` (`intent`, `pulse`, `ring`, `size`, `label`) |
| removable / counted label | `Tag` |
| code snippet | `Code` |
| keyboard hint | `Kbd` |
| tabular data | `Table` (`headers`, `row`, `striped`) |
| single number with a label | `Stat` |
| person | `Avatar`, `AvatarGroup` |
| picture | `Image` |
| bullet / numbered list | `List` (`is_ordered`) |
| quotation | `Blockquote` |
| history | `Timeline` |
| conversation | `Chat`, `ChatEntry`, `ChatTranscript`, `StreamedContent` |
| long document | `DocumentView` |

Feedback

| Need | Use |
|---|---|
| inline banner | `Alert` |
| transient notification | `Toast` |
| loading | `Spinner`, `Skeleton`, `Progress` |
| nothing-yet screen | `EmptyState` (`title`, `description`, `icon`, `action`) |
| hover hint | `Tooltip` |
| "someone is typing" | `TypingIndicator` |

Navigation

| Need | Use |
|---|---|
| internal link | `Link::route(…)` — never a raw `<a href>`, that is a full reload |
| menu | `NavMenu` |
| path | `Breadcrumb` |
| tabbed views | `Tabs` |
| hierarchical tree | `TreeView` |
| file tree | `FileTree` |
| filter chip | `Chip` (`active`, `count`, `href`, `on_click`) |
| pages | `Pagination` |
| page foot | `Footer` |

Overlay

| Need | Use |
|---|---|
| modal dialog | `Modal` (`title`, `size`, `open`, `on_close`, `content`, `footer`, `close_on_backdrop_click`) |
| side panel | `Drawer` (`title`, `position`, `open`, `on_close`, `footer`, `content`) |
| menu on a trigger | `DropdownMenu` |
| collapsible section | `Accordion` |
| copy-to-clipboard | `CopyButton` |
| light/dark switch | `ThemeToggle` |
| multi-step | `Stepper` |

Two orderings that bite:

- **Builder order matters.** On `Button`, set `title` / `data` / `aria_label` / `class` /
  `hover` / `attr` **before** `on_click`. Chained after, they land on the box-less reflection
  wrapper and debug builds panic there.
- **Clearing an input after submit**: key the field's `id` on something that changes when the
  submit lands (e.g. `format!("composer-{}", messages.len())`). A fresh element is an empty
  element — no client opcode needed.

When a pattern repeats (≥2 sites, or it encodes a hard-won fix), promote it into
`rwire-components`: add the module, `pub use` it in `lib.rs`, register a `ComponentEntry` in
`catalog.rs`, and it appears in the gallery automatically.

## 3. Theme owns the aesthetic

Set it once, in `main()`, and never re-assert it per element:

```rust
Theme::dark()
    .palette(palettes::nord())
    .style(ThemeStyle::flat())
    .radius(RadiusScale::None)
```

- `Theme::{new, light, dark}`, then `.palette(…)`, `.accent(…)`, `.neutral(…)`,
  `.error(…)`, `.success(…)`, `.warning(…)`, `.radius(…)`, `.style(…)`
  (`libs/rwire/src/theme.rs`).
- Palettes shipped by `rwire-themes`: `nord`, `indigo`, `catppuccin`, `dracula`, `solarized`,
  `gruvbox`, `tokyo_night`, `rose_pine`, `one_dark`.
- `ThemeStyle::soft()` (default, tinted and rounded) / `ThemeStyle::flat()` (mono UI font,
  hairline borders, no shadows). Custom presets implement `IntoStyle`.
- `RadiusScale` scales every `Rounded*` token except `RoundedFull` — circles stay circles.
- **The theme is state, not config.** Handlers take `&mut Theme` and mutate it
  (`theme.mode = theme.mode.toggle()`); a built-in renderer turns it into CSS variables, so
  mode/accent changes are reactive with no reload.
- Because radius, shadow and font are CSS variables, **components inherit the theme**. Never
  sprinkle `FontMono` / `Rounded0` / `FontInheritAll` to force a look.
- Subtle pairs are readable by construction: `TextWarning` / `TextSuccess` / `TextError`, the
  `TextOn*Subtle` tokens on their `Bg*Subtle`, and `TextDefault` / `TextHigh` on
  `BgAccentSubtle` reach 4.5:1 in every built-in palette, style and mode; the theme computes
  them (`subtle_pairs`) and `rwire-themes/tests/contrast.rs` guards them.

## 4. Tokens, then inline style — never the other way

```rust
el(El::Div)
    .st([St::BgApp, St::Px4, St::Py2])
    .hover([St::BgSubtle])
    .md([St::DisplayFlex])
```

- **Styles**: `St::*` covers every constant; grep `style_tokens.rs` first.
- **Attributes**: `.at(At::Type, Av::Submit)`, `.at_str(At::Name, v)`, `.bool_attr(…)` — never
  a string `.attr("k", "v")`. Missing key or value? Add it to `attr_tokens.rs`.
- **Full height**: `St::HDvh` / `St::MinHDvh`, never raw `100vh`.
- **Responsive**: `.sm()` 640px, `.md()` 768px, `.lg()` 1024px, `.xl()` 1280px — `Bp` in
  `style_tokens.rs`, min-width semantics.
- Inline `.style(Style::new()…)` is **only** for genuinely dynamic values: `calc()`, `vh`
  offsets, depth-based padding, custom properties. If a constant lacks a token, add the token
  (next free hex index in `style_tokens.rs`) rather than inlining.

## 5. No free CSS, no hard-coded colour

The rule: a page is styled by the theme and by `St` / `At` / `Av` tokens. `.class("…")`,
`El::Style`, `.style(…)`, `Style::new`, `At::Style`, a `*_CSS` constant of your own, or a
literal `#rgb` / `#rrggbb` / `rgb()` / `rgba()` / `oklch()` are all bypasses.

The consuming app enforces it with a test, `no_free_css`, in `crates/web-rwire/src/garde.rs`
(mcp-agents; spec in `docs/roadmap-front-rwire-composants.md` §4.4, "outil 1"). It reads the
sources of `src/`, strips line comments, and counts per file:

| Counter | Pattern |
|---|---|
| `class` | `.class(` |
| `el_style` | `El::Style` |
| `style` | `.style(` |
| `style_new` | `Style::new` |
| `at_style` | `At::Style` |
| `feuille` | `const …_CSS` declarations |
| `couleur` | `#rgb`, `#rrggbb`, `rgb(`, `rgba(`, `oklch(` (not counted in `main.rs`) |
| `filter_css` | `fn filter_css(` |

Each counter has a **budget** in the same file. The test fails when a counter goes **above** its
budget, and it also fails when a counter drops **below** it: you cannot add free CSS, and you
cannot remove some either without recording the gain by lowering the budget in the same commit.
That asymmetry is the point — the budget is a ratchet, and every step of the migration lowers it.

Named exception: `filter_css` (search/filter CSS generated by a renderer) with a budget of 1;
and the single `Theme::style(styles::studio())` call in `main.rs`, which is the theme, not free
CSS.

When you touch such a page: check the counters *before* you write, and lower the budget in the
same commit that removes a bypass. Leaving the budget high after a removal fails the test just
as loudly.
