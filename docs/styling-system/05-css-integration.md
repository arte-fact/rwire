# Phase 5: CSS Integration

## Goal

Integrate the styling system into rwire's capsule generation with tree-shaking to include only used component CSS.

## rwire Philosophy Alignment

| Principle | How This Phase Aligns |
|-----------|----------------------|
| Zero runtime | CSS generated once at server startup |
| Minimal bandwidth | Tree-shaking removes unused component CSS |
| Minimal capsule | Only primitive tokens + used components |

## Implementation

### Step 5.1: Component Registry

**File: `rwire/src/components/registry.rs`**

```rust
//! Component registry for CSS tree-shaking.
//!
//! Tracks which components are used in the app to generate minimal CSS.

use std::collections::HashSet;

/// Component types that have associated CSS.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ComponentType {
    Button,
    Input,
    Stack,
    Card,
    Badge,
    // Future components...
}

/// Registry of used components.
#[derive(Clone, Debug, Default)]
pub struct ComponentRegistry {
    used: HashSet<ComponentType>,
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a component type as used.
    pub fn mark_used(&mut self, component: ComponentType) {
        self.used.insert(component);
    }

    /// Check if a component is used.
    pub fn is_used(&self, component: ComponentType) -> bool {
        self.used.contains(&component)
    }

    /// Get all used component types.
    pub fn used_components(&self) -> impl Iterator<Item = ComponentType> + '_ {
        self.used.iter().copied()
    }

    /// Generate CSS for all used components.
    pub fn generate_css(&self) -> String {
        use super::{button_css, input_css, stack_css, card_css, badge_css};

        let mut css = String::with_capacity(4096);

        for component in &self.used {
            css.push_str(match component {
                ComponentType::Button => button_css::button_css(),
                ComponentType::Input => input_css::input_css(),
                ComponentType::Stack => stack_css::stack_css(),
                ComponentType::Card => card_css::card_css(),
                ComponentType::Badge => badge_css::badge_css(),
            });
        }

        css
    }
}

/// Thread-local component tracking during build.
/// This allows components to register themselves when `.build()` is called.
thread_local! {
    static REGISTRY: std::cell::RefCell<Option<ComponentRegistry>> =
        std::cell::RefCell::new(None);
}

/// Begin tracking component usage.
pub fn begin_tracking() {
    REGISTRY.with(|r| {
        *r.borrow_mut() = Some(ComponentRegistry::new());
    });
}

/// Stop tracking and return the registry.
pub fn end_tracking() -> ComponentRegistry {
    REGISTRY.with(|r| {
        r.borrow_mut().take().unwrap_or_default()
    })
}

/// Mark a component as used (called by component builders).
pub fn mark_component_used(component: ComponentType) {
    REGISTRY.with(|r| {
        if let Some(ref mut registry) = *r.borrow_mut() {
            registry.mark_used(component);
        }
    });
}
```

### Step 5.2: Update Component Builders

Each component's `build()` method registers itself:

```rust
// In button.rs
impl Button {
    pub fn build(self) -> ElementBuilder {
        // Register for CSS tree-shaking
        crate::components::registry::mark_component_used(
            crate::components::registry::ComponentType::Button
        );

        // ... existing build logic
    }
}
```

### Step 5.3: Update Capsule Generator

**File: `rwire/src/capsule_gen.rs` (modifications)**

```rust
use crate::theme::{Theme, generate_base_css};
use crate::tokens::css::generate_primitive_css;
use crate::components::registry::ComponentRegistry;

pub struct CapsuleGenerator {
    theme: Theme,
    component_registry: ComponentRegistry,
    // ... existing fields
}

impl CapsuleGenerator {
    /// Generate the complete CSS for the capsule.
    pub fn generate_css(&self) -> String {
        let mut css = String::with_capacity(8192);

        // 1. Base reset (minimal)
        css.push_str(&generate_base_css());

        // 2. Primitive tokens
        css.push_str(&generate_primitive_css());

        // 3. Semantic tokens (theme)
        css.push_str(&Theme::generate_semantic_css());

        // 4. Theme overrides (if non-default)
        if let Some(accent_css) = Theme::generate_accent_override(self.theme.accent) {
            css.push_str(&accent_css);
        }
        if let Some(radius_css) = Theme::generate_radius_override(self.theme.radius) {
            css.push_str(&radius_css);
        }

        // 5. Component CSS (tree-shaken)
        css.push_str(&self.component_registry.generate_css());

        css
    }

    /// Generate the capsule HTML.
    pub fn generate_html(&self, js: &str) -> String {
        let css = self.generate_css();
        let theme_attrs = self.generate_theme_root(&self.theme);

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>rwire</title>
<style>{css}</style>
</head>
<body>
<div id="rw" {theme_attrs}></div>
<script>{js}</script>
</body>
</html>"#,
            css = css,
            theme_attrs = theme_attrs,
            js = js
        )
    }
}
```

### Step 5.4: Build Context Integration

**File: `rwire/src/builder.rs` (modifications)**

```rust
use crate::components::registry::{begin_tracking, end_tracking, ComponentRegistry};

impl BuildContext {
    /// Build the app root and collect all used components.
    pub fn build_with_tracking<F, R>(&mut self, f: F) -> (R, ComponentRegistry)
    where
        F: FnOnce(&mut Self) -> R,
    {
        begin_tracking();
        let result = f(self);
        let registry = end_tracking();
        (result, registry)
    }
}
```

### Step 5.5: Server Integration

**File: `rwire/src/server.rs` (modifications)**

```rust
impl Server {
    pub fn new<F>(app: F) -> Self
    where
        F: Fn() -> ElementBuilder + Send + Sync + 'static,
    {
        // Build once to collect used components
        let mut ctx = BuildContext::new();
        let (_, component_registry) = ctx.build_with_tracking(|_| {
            app()
        });

        // Generate capsule with tree-shaken CSS
        let generator = CapsuleGenerator {
            theme: Theme::default(),
            component_registry,
            // ...
        };

        Self {
            capsule: generator.generate_html(&generate_js()),
            // ...
        }
    }
}
```

### Step 5.6: CSS Minification (Optional)

Simple minification for further size reduction:

```rust
/// Minify CSS by removing unnecessary whitespace.
/// Note: This is a simple implementation. For production,
/// consider using a proper minifier at build time.
pub fn minify_css(css: &str) -> String {
    css.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("")
        // Remove spaces around special characters
        .replace(" {", "{")
        .replace("{ ", "{")
        .replace(" }", "}")
        .replace("} ", "}")
        .replace(": ", ":")
        .replace("; ", ";")
        .replace(", ", ",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minify_css() {
        let input = r#"
            .foo {
                color: red;
                background: blue;
            }
        "#;
        let output = minify_css(input);
        assert_eq!(output, ".foo{color:red;background:blue;}");
    }
}
```

## CSS Budget Tracking

**File: `rwire/src/components/registry.rs` (addition)**

```rust
impl ComponentRegistry {
    /// Get total CSS size for debugging/metrics.
    pub fn css_size(&self) -> usize {
        self.generate_css().len()
    }

    /// Print CSS budget report.
    pub fn print_budget_report(&self) {
        use super::{button_css, input_css, stack_css, card_css, badge_css};

        println!("CSS Budget Report:");
        println!("------------------");

        let mut total = 0;

        if self.is_used(ComponentType::Button) {
            let size = button_css::button_css().len();
            println!("  Button: {} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Input) {
            let size = input_css::input_css().len();
            println!("  Input:  {} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Stack) {
            let size = stack_css::stack_css().len();
            println!("  Stack:  {} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Card) {
            let size = card_css::card_css().len();
            println!("  Card:   {} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Badge) {
            let size = badge_css::badge_css().len();
            println!("  Badge:  {} bytes", size);
            total += size;
        }

        println!("------------------");
        println!("  Total:  {} bytes", total);
    }
}
```

## Deliverables

- [ ] `rwire/src/components/registry.rs` — Component tracking
- [ ] Update component `build()` methods to register usage
- [ ] Update `capsule_gen.rs` with CSS generation
- [ ] Update `server.rs` to collect components at startup
- [ ] Add CSS minification
- [ ] Add budget tracking utilities

## Size Budget

| Category | Max Size |
|----------|----------|
| Base reset | 200 bytes |
| Primitive tokens | 3KB |
| Semantic tokens | 1.5KB |
| **Minimum capsule CSS** | ~4.7KB |
| Per component (avg) | ~500 bytes |
| Full components (all) | ~3KB |
| **Maximum capsule CSS** | ~8KB |
| After gzip | ~2KB |

## Verification

```rust
#[test]
fn test_tree_shaking() {
    // App that only uses Button
    begin_tracking();
    let _ = Button::primary("Click").build();
    let registry = end_tracking();

    // Only button CSS should be included
    assert!(registry.is_used(ComponentType::Button));
    assert!(!registry.is_used(ComponentType::Input));
    assert!(!registry.is_used(ComponentType::Stack));

    let css = registry.generate_css();
    assert!(css.contains(".rw-btn"));
    assert!(!css.contains(".rw-input"));
}
```

## Next Phase

[Phase 6: Extended Components](./06-extended-components.md) — Additional components.
