# Design Token-Driven Styling System for rwire

> Research Report & System Design Proposal

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [State of the Art Research](#state-of-the-art-research)
3. [UI Library Pattern Analysis](#ui-library-pattern-analysis)
4. [rwire Styling System Design](#rwire-styling-system-design)
5. [Implementation Roadmap](#implementation-roadmap)
6. [Sources](#sources)

---

## Executive Summary

This document presents research on modern design token systems and proposes an optimal styling architecture for rwire. Given rwire's unique server-side rendering model with a ~1.5KB browser runtime, the styling system must:

- **Zero browser runtime** - All styling decisions made at compile/build time
- **Minimal bandwidth** - Leverage CSS custom properties + atomic classes
- **Full theming support** - Light/dark modes, brand customization via tokens
- **Type-safe variants** - CVA-inspired variant system in Rust
- **Built-in components** - Ready-to-use components with sensible defaults

---

## State of the Art Research

### W3C Design Tokens Specification (2025.10)

The [Design Tokens Community Group](https://www.w3.org/community/design-tokens/) released the first stable specification in October 2025. Key features:

| Feature | Description |
|---------|-------------|
| **Theming/Multi-brand** | `$extends` property and group inheritance for light/dark modes |
| **Modern Colors** | Full support for Display P3, Oklch, CSS Color Module 4 |
| **Token Relationships** | Aliases via `{token.name}` syntax |
| **Cross-platform** | Single token file generates iOS, Android, Web, Flutter code |

**Key Insight**: Design tokens are abstract data that compile to platform-specific code. CSS custom properties are the runtime representation on web.

### Token Hierarchy: Primitive → Semantic → Component

Modern design systems use a three-tier token architecture:

```
┌─────────────────────────────────────────────────────────────────┐
│  PRIMITIVE TOKENS (Global)                                      │
│  Raw values without context                                     │
│  --color-blue-500: #3b82f6                                      │
│  --space-4: 1rem                                                │
│  --radius-md: 0.375rem                                          │
├─────────────────────────────────────────────────────────────────┤
│  SEMANTIC TOKENS (Alias)                                        │
│  Context-aware names referencing primitives                     │
│  --color-primary: var(--color-blue-500)                         │
│  --color-background-surface: var(--color-neutral-50)            │
│  --spacing-component-gap: var(--space-4)                        │
├─────────────────────────────────────────────────────────────────┤
│  COMPONENT TOKENS (Specific)                                    │
│  Component-level customization                                  │
│  --button-bg: var(--color-primary)                              │
│  --button-padding: var(--spacing-component-gap)                 │
│  --button-radius: var(--radius-md)                              │
└─────────────────────────────────────────────────────────────────┘
```

**Benefits**:
- Theme switching only requires redefining primitive values
- Semantic layer provides intent/guidance for usage
- Component layer enables surgical customization

### 12-Step Color Scales

[Radix Colors](https://www.radix-ui.com/themes/docs/theme/color) pioneered the 12-step color scale system:

| Steps | Purpose |
|-------|---------|
| 1-2 | App/page backgrounds |
| 3-4 | Component backgrounds |
| 5 | Hovered component backgrounds |
| 6 | Active/selected backgrounds |
| 7-8 | Borders and separators (8 = interactive minimum) |
| 9 | Solid backgrounds (primary action) |
| 10 | Hovered solid backgrounds |
| 11 | Low-contrast text |
| 12 | High-contrast text |

**Accessibility**: Steps 11-12 guarantee 4.5:1 contrast against steps 1-2.

---

## UI Library Pattern Analysis

### Tailwind CSS v4 - CSS-First Configuration

[Tailwind v4](https://tailwindcss.com/blog/tailwindcss-v4) represents a paradigm shift:

```css
@theme {
  --color-primary: oklch(0.7 0.15 250);
  --spacing-lg: 1.5rem;
  --radius-xl: 1rem;
}
```

**Key Changes**:
- `@theme` directive defines tokens directly in CSS
- All tokens become CSS variables automatically
- Rust-based Oxide engine: 10x faster compilation
- Zero runtime - all processing at build time
- Uses `@property` for registered custom properties

**Implications for rwire**: The CSS-first approach aligns perfectly with server-side generation.

### shadcn/ui - Open Code Distribution

[shadcn/ui](https://ui.shadcn.com/) introduced a revolutionary model:

```
┌────────────────────────────────────────────────────┐
│  Architecture: Two-Layer Components               │
├────────────────────────────────────────────────────┤
│  Layer 1: Headless Primitives (Radix)             │
│  - Behavior & accessibility                       │
│  - State management                               │
│  - Keyboard navigation                            │
│                                                   │
│  Layer 2: Styled Layer (Tailwind + CVA)           │
│  - Visual appearance                              │
│  - Variant definitions                            │
│  - Token-based styling                            │
└────────────────────────────────────────────────────┘
```

**Key Patterns**:
1. **Copy-paste philosophy**: Source code added to your project, not npm dependency
2. **CSS variables for theming**: All colors/sizes in `globals.css`
3. **CVA for variants**: Type-safe variant management
4. **Radix primitives**: Behavior separated from styling

### Class Variance Authority (CVA)

[CVA](https://cva.style/docs) provides a structured approach to component variants:

```typescript
const button = cva("base-classes", {
  variants: {
    intent: {
      primary: "bg-primary text-white",
      secondary: "bg-secondary text-primary",
      destructive: "bg-red-500 text-white",
    },
    size: {
      sm: "px-2 py-1 text-sm",
      md: "px-4 py-2",
      lg: "px-6 py-3 text-lg",
    },
  },
  compoundVariants: [
    { intent: "primary", size: "lg", class: "uppercase" },
  ],
  defaultVariants: {
    intent: "primary",
    size: "md",
  },
});
```

**Enterprise Benefits**:
- IDE autocomplete eliminates guesswork
- Impossible to ship invalid states
- Breaking changes caught at compile time

### Bootstrap 5 - CSS Custom Properties

[Bootstrap 5.3](https://getbootstrap.com/docs/5.3/customize/css-variables/) embraces CSS variables:

- `--bs-` prefixed variables for namespacing
- Component-level variables (e.g., `.navbar` defines its own)
- Sass compilation for full token customization
- Dark mode via `[data-bs-theme="dark"]`

**Lesson**: Namespace prefixes prevent conflicts with user CSS.

### Open Props - Pure CSS Tokens

[Open Props](https://open-props.style/) provides tokens without a framework:

- JIT compiler delivers only used properties
- Multiple formats: CSS, JSON, Style Dictionary
- Normalize.css with light/dark mode built-in
- No build pipeline required

**Lesson**: CSS custom properties can be a complete design system alone.

### Panda CSS - Zero Runtime CSS-in-JS

[Panda CSS](https://panda-css.com/) by Chakra UI's creator:

- Static analysis extracts styles at build time
- W3C token specification support
- Atomic CSS generation via CVA-like `cva` function
- React Server Components compatible

**Lesson**: Server-side frameworks can use CSS-in-JS patterns with zero runtime cost.

### Headless UI Patterns

Common across Radix, Headless UI, React Aria, Ariakit:

| Aspect | Implementation |
|--------|----------------|
| **State** | Exposed via `data-state` attributes |
| **Styling** | User provides all CSS |
| **Accessibility** | WAI-ARIA built-in |
| **Composition** | Compound component pattern |

---

## rwire Styling System Design

### Design Goals

1. **Zero browser runtime** - All CSS generated server-side
2. **Minimal wire format** - Class names in symbol table, atomic CSS
3. **Full theming** - CSS custom properties for runtime themes
4. **Rust-native** - Type-safe tokens and variants
5. **Tree-shakeable** - Only include used component styles

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        rwire Styling                            │
├─────────────────────────────────────────────────────────────────┤
│  Build Time (Rust)                                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │   Tokens    │  │  Variants   │  │ Components  │             │
│  │  (structs)  │  │   (CVA)     │  │  (builders) │             │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘             │
│         │                │                │                     │
│         └────────────────┼────────────────┘                     │
│                          ▼                                      │
│                ┌─────────────────┐                              │
│                │  CSS Generator  │                              │
│                └────────┬────────┘                              │
│                         │                                       │
├─────────────────────────┼───────────────────────────────────────┤
│  Runtime (Browser)      │                                       │
│                         ▼                                       │
│  ┌─────────────────────────────────────────────────┐           │
│  │  Static CSS (tokens + atomic + components)      │           │
│  │  + CSS Custom Properties for theming            │           │
│  └─────────────────────────────────────────────────┘           │
└─────────────────────────────────────────────────────────────────┘
```

### Token System

#### Primitive Tokens (tokens.rs)

```rust
/// Primitive color tokens - raw values
pub mod color {
    pub const BLUE_1: &str = "oklch(0.99 0.01 250)";
    pub const BLUE_2: &str = "oklch(0.97 0.02 250)";
    // ... steps 3-12
    pub const BLUE_9: &str = "oklch(0.55 0.20 250)";
    pub const BLUE_10: &str = "oklch(0.50 0.22 250)";
    pub const BLUE_11: &str = "oklch(0.40 0.18 250)";
    pub const BLUE_12: &str = "oklch(0.25 0.12 250)";

    // Neutral scale
    pub const NEUTRAL_1: &str = "oklch(0.99 0 0)";
    // ...
}

/// Primitive spacing tokens
pub mod space {
    pub const _0: &str = "0";
    pub const _1: &str = "0.25rem";   // 4px
    pub const _2: &str = "0.5rem";    // 8px
    pub const _3: &str = "0.75rem";   // 12px
    pub const _4: &str = "1rem";      // 16px
    pub const _5: &str = "1.25rem";   // 20px
    pub const _6: &str = "1.5rem";    // 24px
    pub const _8: &str = "2rem";      // 32px
    pub const _10: &str = "2.5rem";   // 40px
    pub const _12: &str = "3rem";     // 48px
}

/// Primitive radius tokens
pub mod radius {
    pub const NONE: &str = "0";
    pub const SM: &str = "0.125rem";
    pub const MD: &str = "0.375rem";
    pub const LG: &str = "0.5rem";
    pub const XL: &str = "0.75rem";
    pub const FULL: &str = "9999px";
}

/// Primitive typography tokens
pub mod font {
    pub const SIZE_XS: &str = "0.75rem";
    pub const SIZE_SM: &str = "0.875rem";
    pub const SIZE_BASE: &str = "1rem";
    pub const SIZE_LG: &str = "1.125rem";
    pub const SIZE_XL: &str = "1.25rem";
    pub const SIZE_2XL: &str = "1.5rem";

    pub const WEIGHT_NORMAL: &str = "400";
    pub const WEIGHT_MEDIUM: &str = "500";
    pub const WEIGHT_SEMIBOLD: &str = "600";
    pub const WEIGHT_BOLD: &str = "700";
}
```

#### Semantic Tokens (Theme Definition)

```rust
/// Theme definition using semantic tokens
#[derive(Clone)]
pub struct Theme {
    // Accent color (configurable)
    pub accent: AccentColor,

    // Semantic colors (reference primitives)
    pub bg_app: &'static str,
    pub bg_subtle: &'static str,
    pub bg_component: &'static str,
    pub bg_component_hover: &'static str,
    pub bg_component_active: &'static str,

    pub border_subtle: &'static str,
    pub border_default: &'static str,
    pub border_interactive: &'static str,

    pub text_high_contrast: &'static str,
    pub text_low_contrast: &'static str,
    pub text_muted: &'static str,

    // Component spacing
    pub gap_xs: &'static str,
    pub gap_sm: &'static str,
    pub gap_md: &'static str,
    pub gap_lg: &'static str,

    // Border radius
    pub radius_sm: &'static str,
    pub radius_md: &'static str,
    pub radius_lg: &'static str,
}

pub enum AccentColor {
    Blue, Violet, Purple, Pink, Red, Orange,
    Yellow, Green, Teal, Cyan, Gray,
}

impl Theme {
    pub fn light() -> Self {
        Self {
            accent: AccentColor::Blue,
            bg_app: color::NEUTRAL_1,
            bg_subtle: color::NEUTRAL_2,
            bg_component: color::NEUTRAL_3,
            bg_component_hover: color::NEUTRAL_4,
            bg_component_active: color::NEUTRAL_5,
            border_subtle: color::NEUTRAL_6,
            border_default: color::NEUTRAL_7,
            border_interactive: color::NEUTRAL_8,
            text_high_contrast: color::NEUTRAL_12,
            text_low_contrast: color::NEUTRAL_11,
            text_muted: color::NEUTRAL_9,
            gap_xs: space::_1,
            gap_sm: space::_2,
            gap_md: space::_4,
            gap_lg: space::_6,
            radius_sm: radius::SM,
            radius_md: radius::MD,
            radius_lg: radius::LG,
        }
    }

    pub fn dark() -> Self {
        Self {
            accent: AccentColor::Blue,
            bg_app: color::NEUTRAL_12,      // Inverted
            bg_subtle: color::NEUTRAL_11,
            // ... inverted scale
            text_high_contrast: color::NEUTRAL_1,
            text_low_contrast: color::NEUTRAL_2,
            ..Self::light()
        }
    }

    /// Generate CSS custom properties for this theme
    pub fn to_css_vars(&self) -> String {
        format!(r#"
:root {{
  --rw-bg-app: {bg_app};
  --rw-bg-subtle: {bg_subtle};
  --rw-bg-component: {bg_component};
  --rw-bg-component-hover: {bg_component_hover};
  --rw-bg-component-active: {bg_component_active};
  --rw-border-subtle: {border_subtle};
  --rw-border-default: {border_default};
  --rw-border-interactive: {border_interactive};
  --rw-text-high: {text_high};
  --rw-text-low: {text_low};
  --rw-text-muted: {text_muted};
  --rw-gap-xs: {gap_xs};
  --rw-gap-sm: {gap_sm};
  --rw-gap-md: {gap_md};
  --rw-gap-lg: {gap_lg};
  --rw-radius-sm: {radius_sm};
  --rw-radius-md: {radius_md};
  --rw-radius-lg: {radius_lg};
}}
"#,
            bg_app = self.bg_app,
            bg_subtle = self.bg_subtle,
            bg_component = self.bg_component,
            bg_component_hover = self.bg_component_hover,
            bg_component_active = self.bg_component_active,
            border_subtle = self.border_subtle,
            border_default = self.border_default,
            border_interactive = self.border_interactive,
            text_high = self.text_high_contrast,
            text_low = self.text_low_contrast,
            text_muted = self.text_muted,
            gap_xs = self.gap_xs,
            gap_sm = self.gap_sm,
            gap_md = self.gap_md,
            gap_lg = self.gap_lg,
            radius_sm = self.radius_sm,
            radius_md = self.radius_md,
            radius_lg = self.radius_lg,
        )
    }
}
```

### Variant System (CVA-Inspired)

```rust
use std::collections::HashMap;

/// Component variant definition (CVA-like)
pub struct Variants<V: VariantKey> {
    base: &'static str,
    variants: HashMap<V, HashMap<&'static str, &'static str>>,
    compound_variants: Vec<CompoundVariant<V>>,
    default_variants: HashMap<V, &'static str>,
}

pub trait VariantKey: Eq + std::hash::Hash + Copy {}

/// Example: Button variants
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonVariant {
    Intent,
    Size,
}

impl VariantKey for ButtonVariant {}

#[derive(Clone, Copy, Default)]
pub enum ButtonIntent {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Destructive,
}

#[derive(Clone, Copy, Default)]
pub enum ButtonSize {
    Sm,
    #[default]
    Md,
    Lg,
}

/// Button component with variants
pub fn button_variants() -> Variants<ButtonVariant> {
    Variants {
        base: "rw-btn",  // Base class
        variants: HashMap::from([
            (ButtonVariant::Intent, HashMap::from([
                ("primary", "rw-btn-primary"),
                ("secondary", "rw-btn-secondary"),
                ("ghost", "rw-btn-ghost"),
                ("destructive", "rw-btn-destructive"),
            ])),
            (ButtonVariant::Size, HashMap::from([
                ("sm", "rw-btn-sm"),
                ("md", "rw-btn-md"),
                ("lg", "rw-btn-lg"),
            ])),
        ]),
        compound_variants: vec![],
        default_variants: HashMap::from([
            (ButtonVariant::Intent, "primary"),
            (ButtonVariant::Size, "md"),
        ]),
    }
}

/// Fluent builder for component variants
pub struct Button {
    intent: ButtonIntent,
    size: ButtonSize,
    disabled: bool,
}

impl Button {
    pub fn new() -> Self {
        Self {
            intent: ButtonIntent::default(),
            size: ButtonSize::default(),
            disabled: false,
        }
    }

    pub fn intent(mut self, intent: ButtonIntent) -> Self {
        self.intent = intent;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Returns the computed class string
    pub fn class(&self) -> String {
        let mut classes = vec!["rw-btn"];

        classes.push(match self.intent {
            ButtonIntent::Primary => "rw-btn-primary",
            ButtonIntent::Secondary => "rw-btn-secondary",
            ButtonIntent::Ghost => "rw-btn-ghost",
            ButtonIntent::Destructive => "rw-btn-destructive",
        });

        classes.push(match self.size {
            ButtonSize::Sm => "rw-btn-sm",
            ButtonSize::Md => "rw-btn-md",
            ButtonSize::Lg => "rw-btn-lg",
        });

        if self.disabled {
            classes.push("rw-btn-disabled");
        }

        classes.join(" ")
    }

    /// Build as ElementBuilder
    pub fn build(self) -> ElementBuilder {
        el(El::Button)
            .class(&self.class())
            .attr("disabled", if self.disabled { "true" } else { "" })
    }
}
```

### Component CSS Generation

```rust
/// Generated CSS for button component
pub fn button_css() -> &'static str {
    r#"
/* Base button styles */
.rw-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-weight: var(--rw-font-weight-medium, 500);
    border-radius: var(--rw-radius-md);
    transition: background-color 0.15s, border-color 0.15s;
    cursor: pointer;
    border: 1px solid transparent;
}

/* Intent variants */
.rw-btn-primary {
    background-color: var(--rw-accent-9);
    color: var(--rw-accent-contrast);
}
.rw-btn-primary:hover {
    background-color: var(--rw-accent-10);
}

.rw-btn-secondary {
    background-color: var(--rw-bg-component);
    color: var(--rw-text-high);
    border-color: var(--rw-border-default);
}
.rw-btn-secondary:hover {
    background-color: var(--rw-bg-component-hover);
    border-color: var(--rw-border-interactive);
}

.rw-btn-ghost {
    background-color: transparent;
    color: var(--rw-text-high);
}
.rw-btn-ghost:hover {
    background-color: var(--rw-bg-component);
}

.rw-btn-destructive {
    background-color: var(--rw-red-9);
    color: white;
}
.rw-btn-destructive:hover {
    background-color: var(--rw-red-10);
}

/* Size variants */
.rw-btn-sm {
    height: 1.75rem;
    padding: 0 var(--rw-gap-sm);
    font-size: var(--rw-font-size-sm);
    gap: var(--rw-gap-xs);
}

.rw-btn-md {
    height: 2.25rem;
    padding: 0 var(--rw-gap-md);
    font-size: var(--rw-font-size-base);
    gap: var(--rw-gap-sm);
}

.rw-btn-lg {
    height: 2.75rem;
    padding: 0 var(--rw-gap-lg);
    font-size: var(--rw-font-size-lg);
    gap: var(--rw-gap-sm);
}

/* States */
.rw-btn-disabled {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
}

.rw-btn:focus-visible {
    outline: 2px solid var(--rw-accent-8);
    outline-offset: 2px;
}
"#
}
```

### Built-in Components

```rust
/// Core UI components with sensible defaults
pub mod components {
    // Layout
    pub mod stack;      // Flexbox column/row with gap
    pub mod grid;       // CSS Grid wrapper
    pub mod container;  // Max-width container
    pub mod card;       // Surface with border/shadow

    // Forms
    pub mod button;     // Primary, secondary, ghost, destructive
    pub mod input;      // Text, email, password, number
    pub mod textarea;   // Multi-line text
    pub mod select;     // Dropdown selection
    pub mod checkbox;   // Boolean toggle
    pub mod radio;      // Single selection from group
    pub mod switch;     // Toggle switch

    // Data Display
    pub mod badge;      // Status indicators
    pub mod avatar;     // User images with fallback
    pub mod table;      // Data tables

    // Feedback
    pub mod alert;      // Info, success, warning, error
    pub mod progress;   // Progress bars
    pub mod spinner;    // Loading indicator

    // Navigation
    pub mod tabs;       // Tab navigation
    pub mod breadcrumb; // Path navigation
    pub mod pagination; // Page navigation
}
```

### Usage Example

```rust
use rwire::{el, El, Ev, handler, renderer, State};
use rwire::components::{Button, ButtonIntent, ButtonSize, Stack, StackDirection};

#[derive(State, Default)]
#[storage(memory)]
struct AppState {
    count: i32,
}

fn app() -> ElementBuilder {
    Stack::new()
        .direction(StackDirection::Column)
        .gap("md")
        .append([
            // Primary button (default)
            Button::new()
                .text("Increment")
                .on(Ev::Click, increment())
                .build(),

            // Secondary small button
            Button::new()
                .intent(ButtonIntent::Secondary)
                .size(ButtonSize::Sm)
                .text("Reset")
                .on(Ev::Click, reset())
                .build(),

            // Destructive button
            Button::new()
                .intent(ButtonIntent::Destructive)
                .text("Delete")
                .on(Ev::Click, delete())
                .build(),

            render_count(),
        ])
        .build()
}

#[renderer]
fn render_count(state: &AppState) -> ElementBuilder {
    el(El::Span)
        .class("rw-text-lg rw-text-high")
        .text(&format!("Count: {}", state.count))
}
```

### Wire Format Optimization

Class names are added to the symbol table and referenced by index:

```
Symbol Table (sent once per session):
  0x80: "rw-btn"
  0x81: "rw-btn-primary"
  0x82: "rw-btn-md"
  0x83: "rw-stack"
  ...

Opcode stream:
  SET_CLASS [ref] [0x80 0x81 0x82]  // "rw-btn rw-btn-primary rw-btn-md"
```

**Bandwidth analysis**:
- Without optimization: `class="rw-btn rw-btn-primary rw-btn-md"` = 32 bytes
- With symbol table: 3 bytes (indices) + amortized table cost
- Savings: ~90% on repeated usage

### Theme Switching at Runtime

```rust
/// Theme provider component
fn theme_provider(theme: &Theme) -> ElementBuilder {
    el(El::Div)
        .attr("data-theme", theme.name())
        .style(&format!("{}; {}",
            theme.to_css_vars(),
            "--rw-accent-9: var(--rw-{}-9)".replace("{}", theme.accent.name())
        ))
}

// In capsule HTML:
// <style>
//   :root { /* light theme vars */ }
//   [data-theme="dark"] { /* dark theme vars */ }
// </style>
```

### Tree-Shaking Components

```rust
impl BuildContext {
    /// Track which components are used
    fn track_component(&mut self, component: ComponentType) {
        self.used_components.insert(component);
    }

    /// Generate CSS only for used components
    pub fn generate_component_css(&self) -> String {
        let mut css = String::new();

        if self.used_components.contains(&ComponentType::Button) {
            css.push_str(button_css());
        }
        if self.used_components.contains(&ComponentType::Input) {
            css.push_str(input_css());
        }
        // ...

        css
    }
}
```

---

## Implementation Roadmap

### Phase 1: Token Foundation
- [ ] Define primitive tokens (colors, spacing, typography, radius)
- [ ] Create 12-step color scales for accent colors
- [ ] Implement `Theme` struct with CSS variable generation
- [ ] Add theme CSS to capsule generation

### Phase 2: Variant System
- [ ] Implement CVA-like variant macro/builder
- [ ] Create `VariantKey` trait for type-safe variants
- [ ] Build compound variant support
- [ ] Add default variant fallbacks

### Phase 3: Core Components
- [ ] Button (primary, secondary, ghost, destructive × sm, md, lg)
- [ ] Input (text, password, email, number)
- [ ] Stack (vertical, horizontal layouts)
- [ ] Card (surface container)

### Phase 4: Form Components
- [ ] Checkbox, Radio, Switch
- [ ] Select dropdown
- [ ] Textarea
- [ ] Form validation styling

### Phase 5: Data & Feedback Components
- [ ] Table (with sorting indicators)
- [ ] Badge, Alert
- [ ] Progress, Spinner
- [ ] Avatar

### Phase 6: Navigation Components
- [ ] Tabs
- [ ] Breadcrumb
- [ ] Pagination

### Phase 7: Optimization
- [ ] Component tree-shaking
- [ ] Symbol table optimization for class names
- [ ] CSS minification in capsule
- [ ] Dark mode support

---

## Sources

### W3C & Standards
- [Design Tokens Specification - W3C Community Group](https://www.w3.org/community/design-tokens/2025/10/28/design-tokens-specification-reaches-first-stable-version/)
- [Design Tokens Community Group](https://www.designtokens.org/)
- [Understanding W3C Design Token Types](https://designtokens.substack.com/p/understanding-w3c-design-token-types)

### Tailwind CSS
- [Tailwind CSS v4.0 Announcement](https://tailwindcss.com/blog/tailwindcss-v4)
- [Theme Variables - Tailwind CSS](https://tailwindcss.com/docs/theme)
- [Tailwind CSS Best Practices 2025-2026](https://www.frontendtools.tech/blog/tailwind-css-best-practices-design-system-patterns)

### shadcn/ui
- [shadcn/ui Official Site](https://ui.shadcn.com/)
- [The Anatomy of shadcn/ui](https://manupa.dev/blog/anatomy-of-shadcn-ui)
- [Theming - shadcn/ui](https://ui.shadcn.com/docs/theming)
- [Building a Scalable Design System with Shadcn/UI](https://shadisbaih.medium.com/building-a-scalable-design-system-with-shadcn-ui-tailwind-css-and-design-tokens-031474b03690)

### Class Variance Authority
- [CVA Documentation](https://cva.style/docs)
- [Enterprise Component Architecture with CVA](https://www.thedanielmark.com/blog/enterprise-component-architecture-type-safe-design-systems-with-class-variance-authority)

### Radix UI
- [Radix Themes - Styling](https://www.radix-ui.com/themes/docs/overview/styling)
- [Radix Colors - 12-Step System](https://www.radix-ui.com/themes/docs/theme/color)

### Bootstrap
- [Bootstrap 5.3 CSS Variables](https://getbootstrap.com/docs/5.3/customize/css-variables/)
- [Use Design Tokens to Customize Bootstrap](https://smth.uk/use-design-tokens-to-customise-bootstrap/)

### Open Props
- [Open Props Official Site](https://open-props.style/)
- [Open Props - CSS Custom Properties as a System](https://css-tricks.com/open-props-and-custom-properties-as-a-system/)

### Panda CSS
- [Panda CSS Official Site](https://panda-css.com/)
- [Why Panda - Zero Runtime](https://panda-css.com/docs/overview/why-panda)
- [Panda CSS Tokens](https://panda-css.com/docs/theming/tokens)

### Design Token Best Practices
- [Naming Tokens in Design Systems - EightShapes](https://medium.com/eightshapes-llc/naming-tokens-in-design-systems-9e86c7444676)
- [Best Practices For Naming Design Tokens - Smashing Magazine](https://www.smashingmagazine.com/2024/05/naming-best-practices/)
- [The Developer's Guide to Design Tokens and CSS Variables](https://penpot.app/blog/the-developers-guide-to-design-tokens-and-css-variables/)

### Atomic CSS
- [Reimagine Atomic CSS - Anthony Fu](https://antfu.me/posts/reimagine-atomic-css)
- [The Case for Atomic CSS](https://johnpolacek.github.io/the-case-for-atomic-css/)
- [Atomic CSS-in-JS](https://sebastienlorber.com/atomic-css-in-js)

### Headless Components
- [Headless UI](https://headlessui.com/)
- [Headless Component Pattern - Martin Fowler](https://martinfowler.com/articles/headless-component.html)
- [Top 6 Headless UI Libraries for React 2025](https://medium.com/@letscodefuture/top-6-headless-ui-libraries-for-react-for-2025-d8af257dd0fb)

### Rust CSS Tools
- [CSS-in-Rust](https://github.com/lukidoescode/css-in-rust)
- [Next-Yak - Zero Runtime CSS-in-JS](https://yak.js.org/)

### Color Systems
- [Accessible Color Tokens for Enterprise Design Systems](https://www.aufaitux.com/blog/color-tokens-enterprise-design-systems-best-practices/)
- [Designing a Scalable Color System](https://uxdesign.cc/designing-a-scalable-and-accessible-color-system-for-your-design-system-f98207eda166)
