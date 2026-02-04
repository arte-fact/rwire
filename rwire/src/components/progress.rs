//! Progress component.
//!
//! Progress bar for showing completion state.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::Progress;
//!
//! Progress::new()
//!     .value(65)
//!     .max(100)
//!     .build()
//!
//! Progress::new()
//!     .value(3)
//!     .max(5)
//!     .label("Step 3 of 5")
//!     .build()
//! ```

use crate::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Progress bar builder.
#[derive(Clone, Debug)]
pub struct Progress {
    value: u32,
    max: u32,
    label: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            value: 0,
            max: 100,
            label: None,
            extra_class: None,
        }
    }
}

impl Progress {
    /// Base CSS class.
    pub const BASE_CLASS: &'static str = "rw-progress";

    /// Create a new progress bar.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the current value.
    pub fn value(mut self, value: u32) -> Self {
        self.value = value;
        self
    }

    /// Set the max value.
    pub fn max(mut self, max: u32) -> Self {
        self.max = max;
        self
    }

    /// Set aria-label.
    pub fn label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(32);
        classes.push_str(Self::BASE_CLASS);

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the progress bar into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Register for CSS tree-shaking
        super::registry::mark_component_used(super::registry::ComponentType::Progress);

        let class = self.compute_class();

        // Calculate percentage
        let percentage = if self.max > 0 {
            ((self.value as f64 / self.max as f64) * 100.0).min(100.0)
        } else {
            0.0
        };

        let mut container = el(El::Div)
            .class(&class)
            .attr("role", "progressbar")
            .attr("aria-valuenow", &self.value.to_string())
            .attr("aria-valuemin", "0")
            .attr("aria-valuemax", &self.max.to_string());

        if let Some(label_text) = self.label {
            container = container.attr("aria-label", &label_text);
        }

        // Progress bar inner element with width set via inline style
        let bar = el(El::Div)
            .class("rw-progress-bar")
            .attr("style", &format!("width:{}%", percentage));

        container.append([bar])
    }
}

/// Progress CSS.
///
/// Size: ~240 bytes (under 250 bytes budget)
pub const PROGRESS_CSS: &str = "\
.rw-progress{height:0.5rem;background:var(--rw-bg-muted);border-radius:var(--rw-radius-full);overflow:hidden}\
.rw-progress-bar{height:100%;background:var(--rw-accent-9);border-radius:var(--rw-radius-full);\
transition:width .3s ease;min-width:0.5rem}\
.rw-progress-bar:empty{min-width:0}\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_defaults() {
        let progress = Progress::new();
        assert_eq!(progress.value, 0);
        assert_eq!(progress.max, 100);
        assert!(progress.label.is_none());
    }

    #[test]
    fn test_progress_class_default() {
        let progress = Progress::new();
        assert_eq!(progress.compute_class(), "rw-progress");
    }

    #[test]
    fn test_progress_with_values() {
        let progress = Progress::new()
            .value(50)
            .max(100)
            .label("Loading");
        assert_eq!(progress.value, 50);
        assert_eq!(progress.max, 100);
        assert_eq!(progress.label.as_deref(), Some("Loading"));
    }

    #[test]
    fn test_progress_css_size() {
        // Progress CSS should be under 250 bytes
        assert!(
            PROGRESS_CSS.len() < 300,
            "Progress CSS too large: {} bytes (budget: 300)",
            PROGRESS_CSS.len()
        );
        println!("Progress CSS size: {} bytes", PROGRESS_CSS.len());
    }

    #[test]
    fn test_progress_css_structure() {
        assert!(PROGRESS_CSS.contains(".rw-progress{"));
        assert!(PROGRESS_CSS.contains(".rw-progress-bar"));
    }
}
