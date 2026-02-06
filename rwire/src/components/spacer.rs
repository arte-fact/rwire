//! Spacer component.
//!
//! Creates space between elements.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{Spacer, SpacingSize};
//!
//! Spacer::md().build()  // Medium vertical space
//! Spacer::lg().build()  // Large vertical space
//! Spacer::new(SpacingSize::Xl).horizontal().build()  // Horizontal space
//! ```

use super::divider::SpacingSize;
use crate::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Spacer component builder.
#[derive(Clone, Debug, Default)]
pub struct Spacer {
    size: SpacingSize,
    horizontal: bool,
    extra_class: Option<Cow<'static, str>>,
}

impl Spacer {
    /// Create a new spacer with the given size.
    pub fn new(size: SpacingSize) -> Self {
        Self {
            size,
            ..Self::default()
        }
    }

    /// Create an extra small spacer.
    pub fn xs() -> Self {
        Self::new(SpacingSize::Xs)
    }

    /// Create a small spacer.
    pub fn sm() -> Self {
        Self::new(SpacingSize::Sm)
    }

    /// Create a medium spacer.
    pub fn md() -> Self {
        Self::new(SpacingSize::Md)
    }

    /// Create a large spacer.
    pub fn lg() -> Self {
        Self::new(SpacingSize::Lg)
    }

    /// Create an extra large spacer.
    pub fn xl() -> Self {
        Self::new(SpacingSize::Xl)
    }

    /// Set the spacer size.
    pub fn size(mut self, size: SpacingSize) -> Self {
        self.size = size;
        self
    }

    /// Make this a horizontal spacer (width instead of height).
    pub fn horizontal(mut self) -> Self {
        self.horizontal = true;
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(32);
        classes.push_str("rw-spacer");

        let size_suffix = match self.size {
            SpacingSize::None => "-0",
            SpacingSize::Xs => "-xs",
            SpacingSize::Sm => "-sm",
            SpacingSize::Md => "-md",
            SpacingSize::Lg => "-lg",
            SpacingSize::Xl => "-xl",
        };

        if self.horizontal {
            classes.push_str("-h");
        }
        classes.push_str(size_suffix);

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the spacer into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        super::registry::mark_component_used(super::registry::ComponentType::Spacer);

        let class = self.compute_class();
        el(El::Div).class(&class)
    }
}

/// Spacer CSS.
pub const SPACER_CSS: &str = "\
.rw-spacer-0{height:0}.rw-spacer-xs{height:var(--rw-space-1)}.rw-spacer-sm{height:var(--rw-space-2)}\
.rw-spacer-md{height:var(--rw-space-4)}.rw-spacer-lg{height:var(--rw-space-6)}.rw-spacer-xl{height:var(--rw-space-8)}\
.rw-spacer-h-0{width:0}.rw-spacer-h-xs{width:var(--rw-space-1)}.rw-spacer-h-sm{width:var(--rw-space-2)}\
.rw-spacer-h-md{width:var(--rw-space-4)}.rw-spacer-h-lg{width:var(--rw-space-6)}.rw-spacer-h-xl{width:var(--rw-space-8)}\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spacer_defaults() {
        let spacer = Spacer::md();
        assert_eq!(spacer.size, SpacingSize::Md);
        assert!(!spacer.horizontal);
    }

    #[test]
    fn test_spacer_class_vertical() {
        let spacer = Spacer::lg();
        assert_eq!(spacer.compute_class(), "rw-spacer-lg");
    }

    #[test]
    fn test_spacer_class_horizontal() {
        let spacer = Spacer::md().horizontal();
        assert_eq!(spacer.compute_class(), "rw-spacer-h-md");
    }

    #[test]
    fn test_spacer_css_size() {
        assert!(SPACER_CSS.len() < 500, "Spacer CSS too large: {} bytes", SPACER_CSS.len());
        println!("Spacer CSS size: {} bytes", SPACER_CSS.len());
    }
}
