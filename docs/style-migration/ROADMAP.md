# Style Migration Roadmap: Class-Based Tokens + Pseudo-Class Support

## Executive Summary

Migrate component styling from CSS class strings (~15KB per connection) to a unified class-based style token system with pseudo-class support. Eliminates all component CSS constants, reduces STYLE_INJECT payload by ~80%, and removes ~2,400 bytes of symbol table overhead per connection.

## The Problem

### Current Architecture

```
Components (.class("rw-btn rw-btn-secondary"))
    │
    ├─► Symbol Table: class strings in WS payload (~2,400B per connection)
    │
    └─► STYLE_INJECT: full CSS rules via WS (~15,300B per connection)
            .rw-btn{display:inline-flex;align-items:center;...}
            .rw-btn:hover{background:var(--rw-accent-10)}
            ...28 components × ~550B avg
```

### Three Redundancies

1. **CSS selectors duplicate class strings** — `.rw-btn{...}` in CSS and `"rw-btn"` in symbol table
2. **CSS properties duplicated across components** — `display:flex` appears in Stack, Badge, Alert, Card...
3. **No pseudo-class support in style tokens** — St tokens are inline styles that can never be overridden by `:hover`/`:focus` CSS

### Why St Tokens Can't Replace Components Today

```js
// Current STYLE_UTIL handler (capsule_gen.rs line 181):
r[f].style.cssText += ';' + U[u]   // → inline style
```

Inline styles have the highest CSS specificity. Any `:hover` CSS rule targeting the element will be ignored:

```css
/* This NEVER works because inline background wins: */
.my-hover:hover { background: red }  /* specificity: 0,1,1 */
/* vs element.style.background = "blue"  ← always wins */
```

## The Solution

### Two Architectural Changes

**Change 1**: St tokens become **class-based** instead of inline

```
Before: STYLE_UTIL → element.style.cssText += "display:flex"  (inline)
After:  STYLE_UTIL → element.classList.add("u2")               (class)
                      CSS rule: .u2{display:flex}               (pre-injected)
```

**Change 2**: New **Ps (pseudo) token system** for interactive states

```
New: STYLE_PS → element.classList.add("p0")
                CSS rule: .p0:hover{background:var(--rw-accent-10)}
```

Both operate at class-level specificity, so pseudo-classes naturally override base styles via CSS cascade.

### End State Architecture

```
Components (.st([tokens]).ps([pseudo_tokens]))
    │
    ├─► STYLE_UTIL opcodes: token IDs on wire (3 bytes each, composited to 3 bytes)
    ├─► STYLE_PS opcodes: pseudo token IDs on wire (3 bytes each)
    │
    └─► STYLE_INJECT: compact generated CSS (~3.7KB per connection)
            .u2{display:flex}.u5{display:inline-flex}...     (utility rules)
            .p0:hover{background:var(--rw-accent-10)}...     (pseudo rules)
            .c256{display:flex;flex-direction:column;...}...  (composite rules)
```

## Payload Comparison

### Per-Connection WS Cost

| Layer | Current | After | Reduction |
|-------|---------|-------|-----------|
| Component CSS (STYLE_INJECT) | 15,300B | 0B | -15,300B |
| Token utility CSS rules | 0B | ~2,500B | +2,500B |
| Pseudo token CSS rules | 0B | ~750B | +750B |
| Composite CSS rules | 0B | ~400B | +400B |
| @keyframes | (in component CSS) | ~80B | ~0B |
| Symbol table (class strings) | ~2,400B | 0B | -2,400B |
| **STYLE_INJECT total** | **15,300B** | **~3,730B** | **-76%** |
| **Total WS savings** | | | **~14,270B (80%)** |

### One-Time JS Capsule (HTTP)

| Layer | Current | After | Reduction |
|-------|---------|-------|-----------|
| U lookup table | ~1,500B | 0B | -1,500B |
| STYLE_UTIL handler | ~60B | ~40B | -20B |
| STYLE_PS handler | 0B | ~40B | +40B |
| COMPOSITE_TABLE handler | ~100B | 0B | -100B |
| **Net JS change** | | | **~1,580B smaller** |

## Phase Overview

| Phase | Description | Files | CSS Reduction |
|-------|-------------|-------|---------------|
| **[Phase 1](phase-1-core-infrastructure.md)** | Class-based tokens + Ps system + opcodes + JS runtime | 6 core files | Foundation |
| **[Phase 2](phase-2-category-a-migration.md)** | Migrate 9 layout/visual components (no pseudo-classes) | 9 component files | -3,400B |
| **[Phase 3](phase-3-category-b-migration.md)** | Migrate 19 interactive components (use Ps tokens) | 19 component files | -11,900B |
| **[Phase 4](phase-4-cleanup.md)** | Remove ComponentRegistry, CSS constants, update docs | 5 files | Final cleanup |

## Dependency Graph

```
Phase 1: Core Infrastructure
    ├── style_tokens.rs (St additions + Ps enum)
    ├── opcodes.rs (STYLE_PS, STYLE_PS_MULTI)
    ├── encoder.rs (style_ps methods)
    ├── builder.rs (.ps() API + emit logic)
    ├── capsule_gen.rs (class-based JS runtime)
    └── style_groups.rs (class-based composites)
         │
         ├──────────────────────┐
         ▼                      ▼
Phase 2: Category A          Phase 3: Category B
(can proceed in parallel     (depends on Phase 1 fully)
 after capsule_gen.rs done)
    ├── stack.rs                ├── button.rs
    ├── spacer.rs               ├── input.rs
    ├── divider.rs              ├── textarea.rs
    ├── card.rs                 ├── checkbox.rs
    ├── badge.rs                ├── radio.rs
    ├── text.rs                 ├── switch.rs
    ├── alert.rs                ├── select.rs
    ├── container.rs            ├── link.rs
    └── avatar.rs               ├── modal.rs
         │                      ├── tabs.rs
         │                      ├── pagination.rs
         │                      ├── breadcrumb.rs
         │                      ├── theme_toggle.rs
         │                      ├── spinner.rs
         │                      ├── progress.rs
         │                      ├── label.rs
         │                      ├── list.rs
         │                      └── table.rs
         │                           │
         └───────────┬───────────────┘
                     ▼
              Phase 4: Cleanup
                  ├── registry.rs
                  ├── mod.rs
                  ├── capsule_gen.rs
                  └── CLAUDE.md
```

## Key Design Decisions

### 1. Class-based over inline styles

All style tokens generate CSS class rules (`.u{code}`) instead of inline styles. This enables pseudo-class overrides via normal CSS cascade and deduplicates CSS declarations across components.

### 2. Separate Ps enum (not merged into St)

Pseudo tokens are a distinct type (`Ps`) with their own opcode (`STYLE_PS`). This provides:
- Type safety: can't accidentally mix base and pseudo tokens
- Clear wire encoding: JS knows how to handle each opcode
- Separate tree-shaking: `used_pseudo_tokens: HashSet<u16>`

### 3. Server-generated CSS (not client-generated)

All CSS (utility + pseudo + composite rules) is generated server-side and sent via STYLE_INJECT. The JS runtime only needs to add class names — no CSS generation in the browser.

### 4. Composite CSS as single classes

Composites generate a single CSS class (`.c256{...}`) with combined declarations, not multiple individual classes. This is more efficient at the browser rendering level.

### 5. Tree-shaking preserved

The existing `HashSet<u16>` tracking mechanism extends to pseudo tokens. Only tokens that actually appear in the element tree generate CSS rules. A simple counter app would generate ~10 utility rules instead of 500+.

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| CSS specificity edge cases | Class + pseudo always beats plain class; verified by CSS spec |
| JS runtime size increase | Net decrease: removing U lookup table saves more than PS handler adds |
| Component visual regressions | Playwright E2E verification after each phase; examples as canaries |
| Breaking existing `.st()` users | Class-based is transparent to the Rust API; only wire encoding changes |
| Performance of classList vs style.cssText | classList.add is O(1) native; benchmarks show it's faster than string manipulation |
