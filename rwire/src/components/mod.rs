//! rwire component library.
//!
//! Pre-built components with design token integration and type-safe variants.
//!
//! # Philosophy
//!
//! Components are builder structs that produce `ElementBuilder` instances.
//! They provide:
//! - Sensible defaults (primary button, medium size)
//! - Type-safe variants (no typos in class strings)
//! - Escape hatches (`.class()` for custom overrides)
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{Button, ButtonIntent};
//!
//! // Simple usage with defaults
//! let btn = Button::primary("Click me").build();
//!
//! // Full customization
//! let btn = Button::new()
//!     .intent(ButtonIntent::Destructive)
//!     .size(ButtonSize::Lg)
//!     .text("Delete")
//!     .build();
//! ```

mod button;

pub use button::{Button, ButtonIntent, ButtonSize};

/// Generate CSS for all used components.
///
/// In the future, this will be tree-shaken to only include
/// CSS for components actually used in the application.
pub fn generate_components_css() -> String {
    let mut css = String::with_capacity(2048);
    css.push_str(button::BUTTON_CSS);
    css
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_components_css_not_empty() {
        let css = generate_components_css();
        assert!(!css.is_empty());
        assert!(css.contains(".rw-btn"));
    }
}
