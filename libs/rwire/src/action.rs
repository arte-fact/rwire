//! Client-side action primitives: Targets (bool toggle) and Selectors (exclusive enum).
//!
//! These provide zero-latency DOM reactivity entirely in the browser,
//! without server round-trips. Both use the existing St token CSS class system.
//!
//! # Targets
//!
//! A Target is a named boolean that can be toggled via events, with bound
//! elements automatically gaining/losing CSS classes.
//!
//! ```ignore
//! #[derive(Target)]
//! pub struct ModalOpen;
//!
//! el(El::Div).when::<ModalOpen>(St::DisplayFlex).st([St::DisplayNone])
//! el(El::Button).toggle::<ModalOpen>(Ev::Click)
//! ```
//!
//! # Selectors
//!
//! A Selector is a named enum with one active variant at a time.
//! Elements can bind CSS classes to specific variants.
//!
//! ```ignore
//! #[derive(Selector)]
//! pub enum ActiveTab {
//!     #[default]
//!     Home,
//!     Settings,
//! }
//!
//! el(El::Div).when_eq(ActiveTab::Home, St::DisplayBlock)
//! el(El::Button).select(ActiveTab::Home, Ev::Click)
//! ```

/// Marker trait for boolean toggle targets.
///
/// Derive with `#[derive(Target)]` on a unit struct.
/// Each distinct type gets a unique u8 index at build time.
pub trait Target: 'static {
    /// The default value when the target is first initialized.
    fn default_value() -> bool {
        false
    }
}

/// Trait for exclusive enum selectors.
///
/// Derive with `#[derive(Selector)]` on a unit-variant enum.
/// Each distinct enum type gets a unique u8 index at build time.
pub trait Selector: 'static {
    /// The default variant's u8 value.
    fn default_value() -> u8;

    /// This variant's u8 value.
    fn variant_value(&self) -> u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestTarget;
    impl Target for TestTarget {}

    #[test]
    fn test_target_default_is_false() {
        assert!(!TestTarget::default_value());
    }

    enum TestSelector {
        A,
        B,
        C,
    }

    impl Selector for TestSelector {
        fn default_value() -> u8 { 1 }
        fn variant_value(&self) -> u8 {
            match self {
                TestSelector::A => 0,
                TestSelector::B => 1,
                TestSelector::C => 2,
            }
        }
    }

    #[test]
    fn test_selector_values() {
        assert_eq!(TestSelector::default_value(), 1);
        assert_eq!(TestSelector::A.variant_value(), 0);
        assert_eq!(TestSelector::B.variant_value(), 1);
        assert_eq!(TestSelector::C.variant_value(), 2);
    }
}
