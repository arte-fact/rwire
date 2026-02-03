# Phase 3: Variant System

## Goal

Create a CVA-inspired system for defining component variants with type safety and minimal CSS output.

## rwire Philosophy Alignment

| Principle | How This Phase Aligns |
|-----------|----------------------|
| Zero runtime | Variant resolution at build time, class strings computed in Rust |
| Minimal bandwidth | Atomic classes compose; symbol table deduplication |
| Minimal capsule | Variants share base styles; no CSS duplication |

## Design Decision: Variant Architecture

**Option A**: Runtime class concatenation (flexible but costly)
**Option B**: Compile-time variant resolution (type-safe, zero runtime) ✓

rwire uses Option B: variants are Rust enums, variant → class mapping is const/static, class string computed at element build time.

```rust
// At build time, this resolves to: "rw-btn rw-btn-primary rw-btn-md"
Button::new().intent(Primary).size(Md).build()
```

## Implementation

### Step 3.1: Variant Trait Definition

**File: `rwire/src/variants.rs`**

```rust
//! CVA-inspired variant system for rwire components.
//!
//! Provides type-safe component variants that resolve to CSS classes
//! at build time with zero runtime cost.

use std::borrow::Cow;

/// Trait for variant enums that map to CSS class names.
pub trait Variant: Copy + Default {
    /// CSS class suffix for this variant value.
    /// Returns None for default variant (no class needed).
    fn class_suffix(&self) -> Option<&'static str>;
}

/// Component with variants.
pub trait VariantComponent {
    /// Base CSS class for this component.
    const BASE_CLASS: &'static str;

    /// Compute full class string from current variant state.
    fn compute_class(&self) -> Cow<'static, str>;
}
```

### Step 3.2: Macro for Variant Definitions

**File: `rwire-macros/src/lib.rs` (addition)**

```rust
/// Define a variant enum with class mappings.
///
/// # Example
/// ```
/// define_variant! {
///     pub enum ButtonIntent {
///         #[default]
///         Primary => "primary",
///         Secondary => "secondary",
///         Ghost => "ghost",
///         Destructive => "destructive",
///     }
/// }
/// ```
#[proc_macro]
pub fn define_variant(input: TokenStream) -> TokenStream {
    // Implementation generates:
    // - Enum with Copy, Clone, Default derives
    // - Variant trait implementation
    // - class_suffix() match arms
}
```

For Phase 3, we'll use manual implementations (no macro yet):

### Step 3.3: Button Variants

**File: `rwire/src/components/button.rs`**

```rust
//! Button component with variants.

use crate::variants::Variant;
use crate::{el, El, Ev, ElementBuilder};
use std::borrow::Cow;

/// Button visual intent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonIntent {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Destructive,
}

impl Variant for ButtonIntent {
    fn class_suffix(&self) -> Option<&'static str> {
        match self {
            ButtonIntent::Primary => None, // Default, no suffix needed
            ButtonIntent::Secondary => Some("secondary"),
            ButtonIntent::Ghost => Some("ghost"),
            ButtonIntent::Destructive => Some("destructive"),
        }
    }
}

/// Button size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    Sm,
    #[default]
    Md,
    Lg,
}

impl Variant for ButtonSize {
    fn class_suffix(&self) -> Option<&'static str> {
        match self {
            ButtonSize::Sm => Some("sm"),
            ButtonSize::Md => None, // Default
            ButtonSize::Lg => Some("lg"),
        }
    }
}

/// Button builder with fluent API.
#[derive(Clone, Debug, Default)]
pub struct Button {
    intent: ButtonIntent,
    size: ButtonSize,
    disabled: bool,
    loading: bool,
    full_width: bool,
    text: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
}

impl Button {
    pub const BASE_CLASS: &'static str = "rw-btn";

    pub fn new() -> Self {
        Self::default()
    }

    /// Convenience constructor with text.
    pub fn primary(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().text(text)
    }

    pub fn secondary(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(ButtonIntent::Secondary).text(text)
    }

    pub fn ghost(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(ButtonIntent::Ghost).text(text)
    }

    pub fn destructive(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(ButtonIntent::Destructive).text(text)
    }

    // Fluent setters
    pub fn intent(mut self, intent: ButtonIntent) -> Self {
        self.intent = intent;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn text(mut self, text: impl Into<Cow<'static, str>>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn full_width(mut self, full: bool) -> Self {
        self.full_width = full;
        self
    }

    /// Add custom class (escape hatch).
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute the full class string.
    pub fn compute_class(&self) -> String {
        let mut classes = vec![Self::BASE_CLASS];

        if let Some(suffix) = self.intent.class_suffix() {
            // We use a static match instead of format! for zero allocation
            classes.push(match suffix {
                "secondary" => "rw-btn-secondary",
                "ghost" => "rw-btn-ghost",
                "destructive" => "rw-btn-destructive",
                _ => unreachable!(),
            });
        }

        if let Some(suffix) = self.size.class_suffix() {
            classes.push(match suffix {
                "sm" => "rw-btn-sm",
                "lg" => "rw-btn-lg",
                _ => unreachable!(),
            });
        }

        if self.disabled {
            classes.push("rw-btn-disabled");
        }

        if self.loading {
            classes.push("rw-btn-loading");
        }

        if self.full_width {
            classes.push("rw-btn-full");
        }

        if let Some(ref extra) = self.extra_class {
            // This allocation only happens if custom class is used
            let mut result = classes.join(" ");
            result.push(' ');
            result.push_str(extra);
            return result;
        }

        classes.join(" ")
    }

    /// Build the ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let class = self.compute_class();
        let mut builder = el(El::Button).class(&class);

        if let Some(text) = self.text {
            builder = builder.text(&text);
        }

        if self.disabled {
            builder = builder.attr("disabled", "");
        }

        if self.loading {
            builder = builder.attr("aria-busy", "true");
        }

        builder
    }

    /// Build with click handler.
    pub fn on_click(self, handler: impl Into<crate::HandlerFn>) -> ElementBuilder {
        self.build().on(Ev::Click, handler.into())
    }
}
```

### Step 3.4: Button CSS

**File: `rwire/src/components/button_css.rs`**

```rust
//! CSS for button component.

/// Generate button component CSS.
/// This is tree-shaken: only included if Button is used.
pub fn button_css() -> &'static str {
    r#"/* Button base */
.rw-btn{display:inline-flex;align-items:center;justify-content:center;gap:var(--rw-space-2);font-weight:var(--rw-font-medium);border-radius:var(--rw-radius-md);border:1px solid transparent;cursor:pointer;transition:background .15s,border-color .15s;height:2.25rem;padding:0 var(--rw-space-4);font-size:var(--rw-text-sm);background:var(--rw-accent-9);color:var(--rw-text-on-accent)}
.rw-btn:hover{background:var(--rw-accent-10)}
.rw-btn:focus-visible{outline:2px solid var(--rw-accent-8);outline-offset:2px}

/* Intent variants */
.rw-btn-secondary{background:var(--rw-bg-muted);color:var(--rw-text-high);border-color:var(--rw-border-default)}
.rw-btn-secondary:hover{background:var(--rw-bg-hover);border-color:var(--rw-border-emphasis)}
.rw-btn-ghost{background:transparent;color:var(--rw-text-high)}
.rw-btn-ghost:hover{background:var(--rw-bg-hover)}
.rw-btn-destructive{background:var(--rw-red-9);color:var(--rw-white)}
.rw-btn-destructive:hover{background:var(--rw-red-10)}

/* Size variants */
.rw-btn-sm{height:1.75rem;padding:0 var(--rw-space-3);font-size:var(--rw-text-xs);gap:var(--rw-space-1)}
.rw-btn-lg{height:2.75rem;padding:0 var(--rw-space-6);font-size:var(--rw-text-base);gap:var(--rw-space-3)}

/* State variants */
.rw-btn-disabled{opacity:.5;cursor:not-allowed;pointer-events:none}
.rw-btn-loading{position:relative;color:transparent}
.rw-btn-loading::after{content:"";position:absolute;width:1em;height:1em;border:2px solid currentColor;border-right-color:transparent;border-radius:50%;animation:rw-spin .6s linear infinite}
.rw-btn-full{width:100%}

@keyframes rw-spin{to{transform:rotate(360deg)}}
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_css_size() {
        let css = button_css();
        // Button CSS should be under 1.5KB
        assert!(css.len() < 1536, "Button CSS too large: {} bytes", css.len());
    }
}
```

### Step 3.5: Compound Variants (Optional Pattern)

For components with compound variants (combinations that need special styling):

```rust
/// Compound variant definition.
pub struct CompoundVariant<I, S> {
    pub intent: Option<I>,
    pub size: Option<S>,
    pub class: &'static str,
}

impl Button {
    /// Check compound variants and return additional class if matched.
    fn check_compound_variants(&self) -> Option<&'static str> {
        // Example: destructive + lg = uppercase
        if self.intent == ButtonIntent::Destructive && self.size == ButtonSize::Lg {
            return Some("rw-btn-destructive-lg");
        }
        None
    }
}
```

### Step 3.6: Generic Component Builder Pattern

**File: `rwire/src/components/mod.rs`**

```rust
//! rwire component library.
//!
//! Components are builder structs that produce ElementBuilder instances
//! with pre-configured classes and attributes.

pub mod button;

pub use button::{Button, ButtonIntent, ButtonSize};

/// Trait for all rwire components.
pub trait Component {
    /// Build into an ElementBuilder.
    fn build(self) -> crate::ElementBuilder;
}

impl Component for Button {
    fn build(self) -> crate::ElementBuilder {
        Button::build(self)
    }
}
```

## Deliverables

- [ ] `rwire/src/variants.rs` — Variant trait
- [ ] `rwire/src/components/mod.rs` — Component module structure
- [ ] `rwire/src/components/button.rs` — Button component
- [ ] `rwire/src/components/button_css.rs` — Button CSS
- [ ] Update `rwire/src/lib.rs` to export components

## Size Budget

| Component | Max Size |
|-----------|----------|
| Button CSS | < 1.5KB |
| Per additional variant | < 100 bytes |

## Usage Example

```rust
use rwire::components::{Button, ButtonIntent, ButtonSize};

// Simple
let btn = Button::primary("Click me").build();

// With handler
let btn = Button::primary("Submit")
    .on_click(submit_handler());

// Full customization
let btn = Button::new()
    .intent(ButtonIntent::Destructive)
    .size(ButtonSize::Lg)
    .disabled(true)
    .class("my-custom-class")
    .build();
```

## Next Phase

[Phase 4: Core Components](./04-core-components.md) — Input, Stack, Card components.
