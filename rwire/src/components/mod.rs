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
//! # Available Components
//!
//! - **Button**: Primary, Secondary, Ghost, Destructive buttons
//! - **Input**: Text, password, email, number inputs
//! - **Stack**: Flexbox layout (row/column)
//! - **Card**: Surface container with padding/shadow
//! - **Badge**: Status indicators
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::*;
//!
//! Stack::column()
//!     .gap(Gap::Lg)
//!     .children([
//!         Card::new()
//!             .child(
//!                 Stack::row()
//!                     .gap(Gap::Sm)
//!                     .children([
//!                         Input::text().placeholder("Search...").build(),
//!                         Button::primary("Search").build(),
//!                     ])
//!             )
//!             .build(),
//!         Stack::row()
//!             .gap(Gap::Sm)
//!             .children([
//!                 Badge::success("Active").build(),
//!                 Badge::warning("Pending").build(),
//!             ])
//!             .build(),
//!     ])
//!     .build()
//! ```

mod badge;
mod button;
mod card;
mod input;
pub mod registry;
mod stack;

pub use badge::{Badge, BadgeIntent, BADGE_CSS};
pub use button::{Button, ButtonIntent, ButtonSize, BUTTON_CSS};
pub use card::{Card, CardPadding, CardShadow, CARD_CSS};
pub use input::{Input, InputSize, InputType, INPUT_CSS};
pub use registry::{begin_tracking, end_tracking, ComponentRegistry, ComponentType};
pub use stack::{Gap, Stack, StackAlign, StackDirection, StackJustify, STACK_CSS};

/// Generate CSS for all components.
///
/// In the future, this will be tree-shaken to only include
/// CSS for components actually used in the application.
pub fn generate_components_css() -> String {
    let mut css = String::with_capacity(4096);
    css.push_str(BUTTON_CSS);
    css.push_str(INPUT_CSS);
    css.push_str(STACK_CSS);
    css.push_str(CARD_CSS);
    css.push_str(BADGE_CSS);
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
        assert!(css.contains(".rw-input"));
        assert!(css.contains(".rw-stack"));
        assert!(css.contains(".rw-card"));
        assert!(css.contains(".rw-badge"));
    }

    #[test]
    fn test_total_components_css_size() {
        let css = generate_components_css();
        // Total component CSS should be under 4KB
        assert!(
            css.len() < 4096,
            "Total component CSS too large: {} bytes",
            css.len()
        );
        println!("Total component CSS size: {} bytes", css.len());
    }
}
