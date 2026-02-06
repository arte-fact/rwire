//! Divider component.
//!
//! Horizontal or vertical separator line.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{Divider, SpacingSize};
//!
//! Divider::horizontal().build()
//! Divider::vertical().build()
//! Divider::horizontal().margin(SpacingSize::Lg).build()
//! ```

use crate::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Spacing size for margins.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpacingSize {
    /// No spacing
    None,
    /// Extra small
    Xs,
    /// Small
    Sm,
    /// Medium (default)
    #[default]
    Md,
    /// Large
    Lg,
    /// Extra large
    Xl,
}

/// Divider component builder.
#[derive(Clone, Debug, Default)]
pub struct Divider {
    vertical: bool,
    margin: SpacingSize,
    extra_class: Option<Cow<'static, str>>,
}

impl Divider {
    /// Create a new horizontal divider.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a horizontal divider.
    pub fn horizontal() -> Self {
        Self::default()
    }

    /// Create a vertical divider.
    pub fn vertical() -> Self {
        Self {
            vertical: true,
            ..Self::default()
        }
    }

    /// Set whether the divider is vertical.
    pub fn is_vertical(mut self, vertical: bool) -> Self {
        self.vertical = vertical;
        self
    }

    /// Set the margin around the divider.
    pub fn margin(mut self, margin: SpacingSize) -> Self {
        self.margin = margin;
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(48);
        classes.push_str("rw-divider");

        if self.vertical {
            classes.push_str(" rw-divider-v");
        }

        match self.margin {
            SpacingSize::None => classes.push_str(" rw-divider-m0"),
            SpacingSize::Xs => classes.push_str(" rw-divider-mxs"),
            SpacingSize::Sm => classes.push_str(" rw-divider-msm"),
            SpacingSize::Md => {}
            SpacingSize::Lg => classes.push_str(" rw-divider-mlg"),
            SpacingSize::Xl => classes.push_str(" rw-divider-mxl"),
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the divider into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        super::registry::mark_component_used(super::registry::ComponentType::Divider);

        let class = self.compute_class();
        el(El::Hr).class(&class)
    }
}

/// Divider CSS.
pub const DIVIDER_CSS: &str = "\
.rw-divider{border:none;border-top:1px solid var(--rw-border-subtle);margin:var(--rw-space-4) 0}\
.rw-divider-v{border-top:none;border-left:1px solid var(--rw-border-subtle);height:100%;margin:0 var(--rw-space-4)}\
.rw-divider-m0{margin:0}.rw-divider-mxs{margin:var(--rw-space-1) 0}\
.rw-divider-msm{margin:var(--rw-space-2) 0}.rw-divider-mlg{margin:var(--rw-space-6) 0}\
.rw-divider-mxl{margin:var(--rw-space-8) 0}\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_divider_defaults() {
        let divider = Divider::new();
        assert!(!divider.vertical);
        assert_eq!(divider.margin, SpacingSize::Md);
    }

    #[test]
    fn test_divider_class_default() {
        let divider = Divider::new();
        assert_eq!(divider.compute_class(), "rw-divider");
    }

    #[test]
    fn test_divider_class_vertical() {
        let divider = Divider::vertical().margin(SpacingSize::Lg);
        assert_eq!(divider.compute_class(), "rw-divider rw-divider-v rw-divider-mlg");
    }

    #[test]
    fn test_divider_css_size() {
        assert!(DIVIDER_CSS.len() < 500, "Divider CSS too large: {} bytes", DIVIDER_CSS.len());
        println!("Divider CSS size: {} bytes", DIVIDER_CSS.len());
    }
}
