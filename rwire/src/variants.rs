//! CVA-inspired variant system for rwire components.
//!
//! Provides type-safe component variants that resolve to CSS classes
//! at build time with zero runtime cost.
//!
//! # Philosophy
//!
//! Variants are Rust enums. The mapping from variant to CSS class is
//! determined at compile time. No runtime class string concatenation.
//!
//! # Example
//!
//! ```ignore
//! use rwire::variants::Variant;
//!
//! #[derive(Clone, Copy, Default)]
//! pub enum ButtonIntent {
//!     #[default]
//!     Primary,
//!     Secondary,
//! }
//!
//! impl Variant for ButtonIntent {
//!     fn class(&self) -> Option<&'static str> {
//!         match self {
//!             ButtonIntent::Primary => None, // Default, no extra class
//!             ButtonIntent::Secondary => Some("rw-btn-secondary"),
//!         }
//!     }
//! }
//! ```

/// Trait for variant enums that map to CSS class names.
///
/// Each variant value maps to an optional CSS class.
/// The default variant typically returns `None` (no extra class needed).
pub trait Variant: Copy + Default {
    /// CSS class for this variant value.
    ///
    /// Returns `None` for the default variant (no class needed).
    /// Returns `Some("class-name")` for non-default variants.
    fn class(&self) -> Option<&'static str>;
}

/// Trait for components that support variants.
pub trait VariantComponent: Sized {
    /// Base CSS class for this component.
    const BASE_CLASS: &'static str;

    /// Build the component into an ElementBuilder.
    fn build(self) -> crate::ElementBuilder;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Default, PartialEq)]
    enum TestIntent {
        #[default]
        Default,
        Alt,
    }

    impl Variant for TestIntent {
        fn class(&self) -> Option<&'static str> {
            match self {
                TestIntent::Default => None,
                TestIntent::Alt => Some("test-alt"),
            }
        }
    }

    #[test]
    fn test_variant_default() {
        let intent = TestIntent::default();
        assert!(intent.class().is_none());
    }

    #[test]
    fn test_variant_non_default() {
        let intent = TestIntent::Alt;
        assert_eq!(intent.class(), Some("test-alt"));
    }
}
