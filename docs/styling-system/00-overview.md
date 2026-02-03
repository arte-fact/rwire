# rwire Styling System - Implementation Roadmap

## Vision

A design token-driven styling system that embodies rwire's core philosophy:
- **Zero browser runtime cost** — all styling decisions at compile time
- **Minimal bandwidth** — class names as symbol indices, atomic CSS
- **Minimal capsule** — tree-shake unused component CSS

## Phases

| Phase | Name | Description |
|-------|------|-------------|
| 1 | [Token Foundation](./01-token-foundation.md) | Primitive tokens, CSS variable generation |
| 2 | [Theme System](./02-theme-system.md) | Semantic tokens, light/dark themes |
| 3 | [Variant System](./03-variant-system.md) | CVA-inspired type-safe variants |
| 4 | [Core Components](./04-core-components.md) | Button, Input, Stack |
| 5 | [CSS Integration](./05-css-integration.md) | Capsule CSS injection, tree-shaking |
| 6 | [Extended Components](./06-extended-components.md) | Full component library |

## Design Principles

### 1. Server-Side Everything

```
WRONG: Generate CSS at runtime in browser
RIGHT: Generate CSS at build/startup, serve static
```

All CSS is known before the first client connects. The capsule includes a static `<style>` block.

### 2. Symbols Over Strings

```
WRONG: SET_CLASS [ref] "btn btn-primary btn-md"  (20 bytes)
RIGHT: SET_CLASS [ref] [0x80, 0x81, 0x82]        (3 bytes)
```

Class names enter the symbol table. Repeated use costs 1 byte per class.

### 3. Composition Over Inheritance

```
WRONG: .btn-primary-lg { /* duplicates .btn, .btn-primary, .btn-lg */ }
RIGHT: .btn { } .btn-primary { } .btn-lg { }  /* compose at use site */
```

Atomic classes that compose. Smaller CSS, better caching.

### 4. CSS Variables for Runtime, Constants for Build

```css
/* Runtime-switchable (themes) */
.btn { background: var(--rw-accent-9); }

/* Build-time constant (no runtime cost) */
.btn-sm { padding: 0.25rem 0.5rem; }  /* Not var(--spacing-sm) */
```

Use CSS variables only where runtime flexibility is needed (theming). Use constants elsewhere.

### 5. Opt-In Complexity

```rust
// Simple (most users)
Button::primary("Click me")

// Full control (power users)
Button::new()
    .intent(Destructive)
    .size(Lg)
    .class("custom-override")
    .build()
```

Sensible defaults, escape hatches when needed.

## File Structure

```
rwire/src/
├── style.rs              # Existing inline style builder
├── tokens/
│   ├── mod.rs            # Re-exports
│   ├── primitives.rs     # Raw values (colors, spacing, etc.)
│   ├── semantic.rs       # Context-aware aliases
│   └── css.rs            # CSS variable generation
├── theme.rs              # Theme struct, light/dark
├── variants.rs           # CVA-like variant system
└── components/
    ├── mod.rs            # Re-exports, component registry
    ├── button.rs         # Button component
    ├── input.rs          # Input component
    ├── stack.rs          # Layout component
    └── ...
```

## Success Metrics

| Metric | Target |
|--------|--------|
| Capsule size increase | < 2KB for full theme + base components |
| Per-component CSS | < 500 bytes minified |
| Symbol overhead | 1 byte per class after first use |
| Build time impact | < 100ms for CSS generation |
