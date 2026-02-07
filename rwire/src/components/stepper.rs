//! Stepper component.
//!
//! Multi-step progress indicator with numbered circles and connecting lines.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::Stepper;
//!
//! Stepper::new()
//!     .step("Cart")
//!     .step("Shipping")
//!     .step("Payment")
//!     .step("Confirm")
//!     .current(1) // 0-indexed, "Shipping" is active
//!     .build()
//! ```

use crate::style_tokens::St;
use crate::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Stepper builder.
#[derive(Clone, Default)]
pub struct Stepper {
    steps: Vec<Cow<'static, str>>,
    current: usize,
    extra_class: Option<Cow<'static, str>>,
}

impl Stepper {
    /// Create a new stepper.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a step label.
    pub fn step(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.steps.push(label.into());
        self
    }

    /// Set the current step index (0-based).
    pub fn current(mut self, index: usize) -> Self {
        self.current = index;
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the stepper container.
    pub fn compute_tokens() -> Vec<St> {
        vec![St::DisplayFlex, St::ItemsCenter, St::WFull]
    }

    /// Build the stepper into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut container = el(El::Div).st(Self::compute_tokens());

        if let Some(ref extra) = self.extra_class {
            container = container.class(extra.as_ref());
        }

        for (i, label) in self.steps.iter().enumerate() {
            // Circle
            let circle_token = if i < self.current {
                St::StepCircleDone
            } else if i == self.current {
                St::StepCircleActive
            } else {
                St::StepCircle
            };

            let circle_text = if i < self.current {
                "\u{2713}".to_string() // ✓
            } else {
                (i + 1).to_string()
            };

            let circle = el(El::Div)
                .st([circle_token, St::FlexShrink0])
                .text(&circle_text);

            // Step column: circle + label
            let step = el(El::Div)
                .st([St::DisplayFlex, St::FlexCol, St::ItemsCenter, St::GapXs])
                .append([
                    circle,
                    el(El::Span)
                        .st([
                            St::TextXs,
                            if i <= self.current { St::TextDefault } else { St::TextMuted },
                            if i == self.current { St::FontMedium } else { St::FontNormal },
                        ])
                        .text(label),
                ]);

            container = container.append([step]);

            // Connecting line (between steps)
            if i < self.steps.len() - 1 {
                let line_token = if i < self.current {
                    St::StepLineActive
                } else {
                    St::StepLine
                };
                container = container.append([el(El::Div).st([line_token])]);
            }
        }

        container
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stepper_defaults() {
        let stepper = Stepper::new();
        assert!(stepper.steps.is_empty());
        assert_eq!(stepper.current, 0);
    }

    #[test]
    fn test_stepper_tokens() {
        let tokens = Stepper::compute_tokens();
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::ItemsCenter));
        assert!(tokens.contains(&St::WFull));
    }

    #[test]
    fn test_stepper_with_steps() {
        let stepper = Stepper::new()
            .step("Cart")
            .step("Shipping")
            .step("Payment")
            .current(1);
        assert_eq!(stepper.steps.len(), 3);
        assert_eq!(stepper.current, 1);
    }

    #[test]
    fn test_stepper_labels() {
        let stepper = Stepper::new()
            .step("A")
            .step("B")
            .step("C");
        assert_eq!(stepper.steps[0].as_ref(), "A");
        assert_eq!(stepper.steps[2].as_ref(), "C");
    }
}
