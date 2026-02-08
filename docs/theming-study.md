# Theming Study: Unlocking Creativity While Maintaining Harmony

A comparative analysis of how popular design systems handle theming, and a
proposal for evolving rwire's token architecture.

## Current rwire Architecture

rwire has a proper 3-layer token system:

```
Layer 1: Primitives (CSS vars)       Layer 2: Semantic (CSS vars)              Layer 3: St tokens (binary)
--rw-neutral-1..12 (oklch)           --rw-bg-app: var(--rw-neutral-1)          St::BgApp    -> .u{n}{background:var(--rw-bg-app)}
--rw-blue-1..12                      --rw-bg-muted: var(--rw-neutral-3)        St::BgMuted  -> .u{n}{...}
--rw-green-1..12                     --rw-text-default: var(--rw-neutral-11)   St::TextDefault -> ...
--rw-amber-1..12                     --rw-accent-9: var(--rw-blue-9)
--rw-red-1..12                       [data-theme="dark"] { inverted }
```

Dark mode works via CSS cascade: `[data-theme="dark"]` remaps semantic
variables to inverted primitive steps. Components use `St::BgApp`,
`St::TextDefault`, etc. and are automatically theme-aware. Accent color
switching (`[data-accent="green"]`) remaps `--rw-accent-*` to a different hue.

This is architecturally identical to Radix Themes and shadcn/ui.

### What works

- Three-layer indirection (primitive, semantic, binary token).
- Dark/light mode via CSS cascade, zero JS.
- Accent hue switching via data attributes.
- Color palette presets (Nord, custom hex palettes).
- Tree-shaken CSS: only used tokens ship to the client.
- Binary protocol stays compact regardless of theme complexity.

### Where creativity is locked

1. **Components hardcode semantic tokens.** A `Button` with `Primary` intent
   always uses `[St::BgAccent, St::TextOnAccent]`. You can change the accent
   hue (blue to green), but you cannot say "primary buttons should have subtle
   backgrounds instead of solid ones" without editing `button.rs`.

2. **Incomplete palette-step tokenization.** The 12-step scale exists in CSS
   variables, but only a few steps have St tokens. You cannot access accent
   step 3 (interactive surface) or step 6 (accent border) without falling back
   to `.attr("style", ...)`.

3. **No foreground/background pairing.** `St::BgAccent` pairs with
   `St::TextOnAccent`, but there is no `TextOnMuted`, `TextOnSubtle`, etc.
   Accessibility depends on the developer choosing correct pairs manually.

4. **No color palette concept for components.** Intents are closed enums
   (`Primary`, `Secondary`, `Destructive`) with hardcoded color mappings. You
   cannot say "this button should use the green palette" without a new intent
   variant.

---

## Industry Comparison

### Systems Studied

| System | Token Layers | Scale Steps | Dark Mode Approach | Component Recipes |
|--------|-------------|-------------|-------------------|-------------------|
| Radix Themes | 3 (primitive, semantic, component props) | 12 per hue | Token remap via CSS context | `<Theme>` component props |
| shadcn/ui | 2 (semantic, component) | Flat (no scale) | `.dark` class redefines vars | Copy-paste source, `className` |
| Tailwind CSS v4 | 2-3 (user-defined) | 11 per hue (50-950) | Media query or `.dark` class | No component system |
| Open Props | 2 (primitive, normalize aliases) | 13 per hue (0-12) | `prefers-color-scheme` remap | No component system |
| Material Design 3 | 4 (source, key, tonal, roles) | 13 tones (0-100) | Deterministic tone inversion | 3 levels (scheme, type, instance) |
| Chakra UI v3 | 3 (tokens, semantic, recipes) | 11 per family (50-950) | Conditional semantic tokens | Recipes + Slot Recipes |

### Pattern 1: Primitive to Semantic (Universal)

Every system has at least two layers. The primitive layer provides raw
aesthetic building blocks. The semantic layer maps them to UI intent. This
separation is what makes dark mode, theming, and brand customization possible
without touching component code.

**Radix:**
```
Steps 1-2:   Backgrounds (app, subtle)
Steps 3-5:   Interactive surfaces (normal, hover, active)
Steps 6-8:   Borders (subtle, default, emphasis)
Steps 9-10:  Solid fills (normal, hover)
Steps 11-12: Text (low contrast, high contrast)
```

**MD3:**
```
Tone 0:    Pure black
Tone 10:   Dark mode surface
Tone 40:   Light mode foreground / solid
Tone 80:   Dark mode foreground / solid
Tone 90:   Light mode container
Tone 99:   Light mode surface
Tone 100:  Pure white
```

Both converge on ~12-13 steps per hue, each tuned for a specific purpose.

### Pattern 2: Dark Mode = Token Remapping

All six systems handle dark mode by remapping semantic tokens to different
primitive values, not by maintaining parallel styles:

- **Radix**: Same variable names, different values under dark context.
- **shadcn/ui**: `.dark` class redefines the same CSS variables.
- **Tailwind**: Variables redefined in `@media (prefers-color-scheme: dark)`.
- **Open Props**: Normalize aliases remap (`--text-1` from `gray-9` to `gray-1`).
- **MD3**: Deterministic tone inversion (primary tone 40 becomes tone 80).
- **Chakra**: Conditional tokens `{ base: "...", _dark: "..." }`.

Components never know what mode they are in. rwire already does this correctly.

### Pattern 3: Background/Foreground Pairing

The strongest accessibility pattern across these systems: every background
color has a paired foreground with guaranteed contrast.

**shadcn/ui:**
```css
--primary: oklch(0.21 0.034 264.67);
--primary-foreground: oklch(0.985 0.002 247.84);
```

**MD3:**
```
primary        / on-primary         (tone 40 / tone 100)
primary-container / on-primary-container (tone 90 / tone 10)
```

**Chakra:**
```
colorPalette.solid    / colorPalette.contrast
colorPalette.subtle   / colorPalette.fg
```

This makes inaccessible combinations structurally impossible. rwire has
`BgAccent + TextOnAccent` but only for the solid accent fill. There is no
`TextOnMuted`, `TextOnSubtle`, etc.

### Pattern 4: Constrained Choices = Harmony

Every system constrains color choices to prevent discord:

- **Radix**: 12 steps, each purpose-mapped. Step 3 is for interactive
  backgrounds, not text.
- **shadcn/ui**: ~8 semantic pairs. You work within `primary`, `secondary`,
  `muted`, `accent`, `destructive`. No room for arbitrary colors.
- **MD3**: Algorithm ensures secondary is desaturated relative to primary,
  neutral is near-achromatic. Mathematical relationships guarantee harmony.
- **Chakra**: Semantic tokens create a vocabulary boundary. Components can only
  use what the vocabulary defines.

**Harmony comes from limiting choices, not from adding more.** The system
provides a curated set of options guaranteed to work together. Creativity
operates within the constraints.

### Pattern 5: The colorPalette Concept

Chakra's most powerful pattern: a component references "the current color
palette" without naming which color:

```jsx
<Button colorPalette="red">Delete</Button>
<Button colorPalette="green">Save</Button>
```

The component recipe references `colorPalette.solid`, `colorPalette.fg`. The
actual hue is injected at usage. This separates the component shape (button
with solid fill) from the color choice.

### Pattern 6: Component Recipes

Systems with component libraries provide multiple override levels:

| Level | Radix | MD3 | Chakra | shadcn/ui |
|-------|-------|-----|--------|-----------|
| Global theme | `<Theme accentColor>` | Seed color | `defineConfig` | CSS var reassignment |
| Component type | N/A | Component Theme | Recipes | Modify copied source |
| Instance | Nested `<Theme>` | Widget props | Style props | `className` overrides |

The pattern: global changes touch one place. Per-component-type changes do not
require modifying every instance. Per-instance changes are possible but
discouraged for consistency.

### Pattern 7: Seed-to-Palette Generation

MD3 demonstrates the highest-leverage approach: one source color generates the
entire palette algorithmically via HCT (Hue, Chroma, Tone):

```
1 source color
  -> 5 key colors (primary, secondary, tertiary, neutral, neutral-variant)
    -> 5 x 13 tonal palettes
      -> 29+ color roles
        -> automatic dark mode via tone inversion
```

The ratio of inputs to outputs is extreme: 1 color produces 65+ usable tokens.

---

## Proposal for rwire

### Principle

The binary wire protocol does not need to change. St tokens already reference
CSS variables. The creativity problem lives entirely in the CSS variable layer
and the semantic token vocabulary.

**Expand the semantic vocabulary. Do not expand the escape hatches.**

### Expanded Semantic Token Vocabulary

Current rwire has ~20 semantic tokens. The proposal expands to ~35, organized
as background/foreground pairs:

```
Surface tokens (paired bg + fg for guaranteed contrast):
  --rw-surface         / --rw-on-surface          (app background + body text)
  --rw-surface-raised  / --rw-on-raised           (cards, elevated panels)
  --rw-primary         / --rw-on-primary          (accent solid fills)
  --rw-primary-subtle  / --rw-on-primary-subtle   (accent tinted backgrounds)
  --rw-secondary       / --rw-on-secondary        (neutral interactive fills)
  --rw-muted           / --rw-on-muted            (de-emphasized content)
  --rw-destructive     / --rw-on-destructive      (danger/error fills)

Border tokens:
  --rw-border-default                              (standard borders)
  --rw-border-subtle                               (de-emphasized borders)
  --rw-border-emphasis                             (strong borders)
  --rw-border-primary                              (accent-colored borders)

Interactive state tokens:
  --rw-hover                                       (generic hover overlay)
  --rw-active                                      (generic press overlay)
  --rw-focus-ring                                  (focus indicator color)

Status tokens (paired):
  --rw-success         / --rw-on-success
  --rw-warning         / --rw-on-warning
  --rw-error           / --rw-on-error

Accent scale access (all 12 steps):
  --rw-accent-1 .. --rw-accent-12                  (full scale access)
```

Each new semantic token gets a corresponding St variant. The binary protocol
remains compact: each is still a u16 sent as 1-2 bytes on the wire.

### Light/Dark Mappings

```
Semantic Token            Light Mode                    Dark Mode
--rw-surface              --rw-neutral-1                --rw-neutral-12
--rw-surface-raised       --rw-neutral-2                --rw-neutral-11
--rw-on-surface           --rw-neutral-12               --rw-neutral-1
--rw-primary              --rw-accent-9                 --rw-accent-9
--rw-on-primary           --rw-white                    --rw-white
--rw-primary-subtle       --rw-accent-3                 --rw-accent-4
--rw-on-primary-subtle    --rw-accent-11                --rw-accent-11
--rw-secondary            --rw-neutral-4                --rw-neutral-9
--rw-on-secondary         --rw-neutral-12               --rw-neutral-1
--rw-muted                --rw-neutral-3                --rw-neutral-10
--rw-on-muted             --rw-neutral-11               --rw-neutral-3
--rw-destructive          --rw-red-9                    --rw-red-9
--rw-on-destructive       --rw-white                    --rw-white
--rw-border-primary       --rw-accent-7                 --rw-accent-7
--rw-hover                --rw-neutral-4                --rw-neutral-8
--rw-active               --rw-neutral-5                --rw-neutral-7
--rw-focus-ring           --rw-accent-8                 --rw-accent-8
```

### ThemeStyle Presets

Instead of component-level recipes (complex), define theme-wide style presets
that shift the semantic-to-primitive mappings:

```rust
ThemeStyle::default()      // Radix-like: solid accents, medium radius, subtle shadows
ThemeStyle::soft()         // Subtle tinted backgrounds, large radius, no shadows
ThemeStyle::brutalist()    // Sharp corners, heavy borders, high contrast
ThemeStyle::minimal()      // Near-zero borders, large spacing, text-only hierarchy
```

Each preset redefines the semantic-to-primitive mapping. Components do not
change. The CSS variable values shift underneath them.

**Example: default vs soft**

```
                  default                          soft
--rw-primary      accent-9 (solid vibrant)         accent-3 (subtle tint)
--rw-on-primary   white                            accent-11 (dark text)
border-radius     --rw-radius-md (0.375rem)        --rw-radius-xl (1rem)
shadows           --rw-shadow-sm                   none
borders           1px solid neutral-7              1px solid accent-6
```

Same components, same St tokens, same binary protocol. Different feel.

### Component Updates

Components shift from hardcoded semantic tokens to the richer paired vocabulary:

```rust
// Before (button.rs, primary intent):
[St::BgAccent, St::TextOnAccent, St::RoundedMd, St::BorderTransparent]

// After:
[St::BgPrimary, St::TextOnPrimary, St::RoundedMd, St::BorderTransparent]
```

The difference: `St::BgAccent` always points to `--rw-accent-9` (hardcoded
step). `St::BgPrimary` points to `--rw-primary`, which the ThemeStyle preset
can remap to any step.

### Full Accent Scale Tokenization

All 12 accent steps get St tokens, following the Radix purpose-mapping:

```rust
St::Accent1   // App background tint
St::Accent2   // Subtle background tint
St::Accent3   // Interactive surface (normal)
St::Accent4   // Interactive surface (hover)
St::Accent5   // Interactive surface (active)
St::Accent6   // Border (subtle)
St::Accent7   // Border (default)
St::Accent8   // Border (emphasis) / focus ring
St::Accent9   // Solid fill (normal)
St::Accent10  // Solid fill (hover)
St::Accent11  // Text (low contrast)
St::Accent12  // Text (high contrast)
```

With these, users can build custom components that access any point on the
accent scale without escape hatches, while staying within the binary protocol.

### Backward Compatibility

Existing St tokens (`BgApp`, `BgMuted`, `TextDefault`, etc.) continue to work.
They map to the same CSS variables. New tokens are additive. The ThemeStyle
presets add new CSS variable definitions but do not remove existing ones.

Components can be migrated incrementally from direct semantic tokens
(`St::BgAccent`) to the paired vocabulary (`St::BgPrimary`) over time.

### Implementation Scope

1. **Expand semantic CSS variables** in `theme.rs` (~15 new variables with
   light/dark mappings).
2. **Add St variants** in `style_tokens.rs` for each new semantic variable
   (~15 new tokens) and the full accent scale (12 tokens).
3. **Add ThemeStyle enum** to `theme.rs` with 3-4 presets that adjust the
   semantic-to-primitive mapping.
4. **Wire ThemeStyle** into `CapsuleConfig` and `generate_capsule_css`.
5. **Migrate components** to use paired tokens (`BgPrimary`/`TextOnPrimary`
   instead of `BgAccent`/`TextOnAccent`).
6. **Update docs-site** to demonstrate ThemeStyle switching.

No changes to the binary protocol, wire format, or JS runtime.

---

## Summary

| Concern | Current | Proposed |
|---------|---------|----------|
| Semantic tokens | ~20 | ~35 (with bg/fg pairs) |
| Accent scale access | 3-4 steps tokenized | All 12 steps |
| Dark mode | Token remap (correct) | Same, more tokens |
| Theme presets | AccentColor + RadiusScale | + ThemeStyle (soft, brutalist, etc.) |
| Component colors | Hardcoded to accent-9 | Indirect via --rw-primary |
| Accessibility | Manual pairing | Structural bg/fg pairs |
| Wire protocol change | N/A | None required |
| JS runtime change | N/A | None required |

The core insight from studying Radix, shadcn, Tailwind, Open Props, MD3, and
Chakra: **harmony comes from constrained choices, not from more choices.** The
goal is not to expose more raw CSS, but to expand the semantic vocabulary so
that more creative intentions can be expressed within the token system.
