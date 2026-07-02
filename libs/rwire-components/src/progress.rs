//! Progress component.
//!
//! Progress bar for showing completion state.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::Progress;
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

use rwire::attr_tokens::{At, Av};
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder, Style};
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

#[rwire::component]
impl Progress {
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

    /// Compute style tokens for the progress container.
    ///
    /// `WFull` makes the bar span its container — without it the track is auto-width and
    /// collapses to nothing in a shrink-to-fit (e.g. flex-centered) context.
    pub fn compute_tokens(&self) -> Vec<St> {
        vec![St::WFull, St::BgMuted, St::RoundedFull, St::OverflowHidden]
    }

    /// Build the progress bar into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Calculate percentage
        let percentage = if self.max > 0 {
            ((self.value as f64 / self.max as f64) * 100.0).min(100.0)
        } else {
            0.0
        };

        let mut tokens = self.compute_tokens();
        tokens.push(St::H05rem);
        let mut container = el(El::Div)
            .st(tokens)
            .at(At::Role, Av::RoleProgressbar)
            .at_str(At::AriaValuenow, &self.value.to_string())
            .at(At::AriaValuemin, Av::Zero)
            .at_str(At::AriaValuemax, &self.max.to_string());

        if let Some(ref extra) = self.extra_class {
            container = container.class(extra.as_ref());
        }

        if let Some(label_text) = self.label {
            container = container.at_str(At::AriaLabel, &label_text);
        }

        // Progress bar inner element. The width is data-driven so it uses the
        // typed `Style` value builder (not a raw `style` attr); a min-width token
        // keeps a sliver visible at 0%. Rounded to one decimal so sub-percent
        // jitter does not churn the bytes pushed on every re-render — identical
        // rounded values hash-dedup to no update at all.
        let bar = el(El::Div)
            .st([
                St::HFull,
                St::BgPrimary,
                St::RoundedFull,
                St::TransitionAll,
                St::MinW05rem,
            ])
            .style(Style::new().width(&format!("{percentage:.1}%")));

        container.append([bar])
    }
}

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
    fn test_progress_tokens() {
        let progress = Progress::new();
        let tokens = progress.compute_tokens();
        assert!(tokens.contains(&St::BgMuted));
        assert!(tokens.contains(&St::RoundedFull));
        assert!(tokens.contains(&St::OverflowHidden));
    }

    #[test]
    fn test_progress_with_values() {
        let progress = Progress::new().value(50).max(100).label("Loading");
        assert_eq!(progress.value, 50);
        assert_eq!(progress.max, 100);
        assert_eq!(progress.label.as_deref(), Some("Loading"));
    }
}
