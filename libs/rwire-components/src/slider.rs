//! Slider component.
//!
//! Range input control with visual track and fill.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::Slider;
//!
//! Slider::new()
//!     .min(0)
//!     .max(100)
//!     .value(42)
//!     .on_change(update_volume())
//!     .build()
//! ```

use rwire::attr_tokens::At;
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder, Ev, HandlerSpec};

/// Slider builder.
#[derive(Clone, Default)]
pub struct Slider {
    min: i32,
    max: i32,
    value: i32,
    step: Option<i32>,
    disabled: bool,
    on_change: Option<HandlerSpec>,
    label: Option<String>,
}

#[rwire::component]
impl Slider {
    /// Create a new slider (default 0-100).
    pub fn new() -> Self {
        Self {
            max: 100,
            ..Self::default()
        }
    }

    /// Set the minimum value.
    pub fn min(mut self, min: i32) -> Self {
        self.min = min;
        self
    }

    /// Set the maximum value.
    pub fn max(mut self, max: i32) -> Self {
        self.max = max;
        self
    }

    /// Set the current value.
    pub fn value(mut self, value: i32) -> Self {
        self.value = value;
        self
    }

    /// Set the step increment.
    pub fn step(mut self, step: i32) -> Self {
        self.step = Some(step);
        self
    }

    /// Set disabled state.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the change handler.
    pub fn on_change(mut self, handler: HandlerSpec) -> Self {
        self.on_change = Some(handler);
        self
    }

    /// Set an accessible label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Compute style tokens for the slider container.
    pub fn compute_tokens() -> Vec<St> {
        vec![St::DisplayFlex, St::FlexCol, St::GapSm, St::WFull]
    }

    /// Build the slider into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Visual track with fill, vertically centered in a fixed-height control box.
        let range = self.max - self.min;
        let fill_pct = if range > 0 {
            (((self.value - self.min).max(0) as f64 / range as f64) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

        let track = el(El::Div)
            .st([St::SliderTrack, St::PositionAbsolute, St::RoundedSm])
            .attr("style", "top:50%;left:0;right:0;transform:translateY(-50%)")
            .append([
                el(El::Div)
                    .st([St::SliderFill])
                    .attr("style", &format!("width:{fill_pct:.1}%")),
            ]);

        // Native range input overlaid on top of the track so the thumb sits ON the bar.
        // Transparent track (the visual track shows through); only the thumb is painted.
        let mut input = el(El::Input)
            .st([St::SliderInput, St::SliderThumb, St::PositionAbsolute])
            .attr("style", "top:0;left:0;width:100%;height:100%")
            .attr("type", "range")
            .attr("min", &self.min.to_string())
            .attr("max", &self.max.to_string())
            .attr("value", &self.value.to_string());

        if let Some(step) = self.step {
            input = input.attr("step", &step.to_string());
        }

        if self.disabled {
            input = input.bool_attr(At::Disabled);
        }

        if let Some(ref label) = self.label {
            input = input.at_str(At::AriaLabel, label);
        }

        // Bind the `change` event (fires on release), not `input` — a server round-trip on
        // every drag tick would re-render and interrupt the native drag.
        if let Some(handler) = self.on_change {
            input = input.on(Ev::Change, handler);
        }

        el(El::Div)
            .st([St::PositionRelative, St::WFull])
            .attr("style", "height:20px")
            .append([track, input])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slider_defaults() {
        let slider = Slider::new();
        assert_eq!(slider.min, 0);
        assert_eq!(slider.max, 100);
        assert_eq!(slider.value, 0);
        assert!(!slider.disabled);
    }

    #[test]
    fn test_slider_tokens() {
        let tokens = Slider::compute_tokens();
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::WFull));
    }

    #[test]
    fn test_slider_with_values() {
        let slider = Slider::new().min(10).max(50).value(30).step(5);
        assert_eq!(slider.min, 10);
        assert_eq!(slider.max, 50);
        assert_eq!(slider.value, 30);
        assert_eq!(slider.step, Some(5));
    }

    #[test]
    fn test_slider_disabled() {
        let slider = Slider::new().disabled(true);
        assert!(slider.disabled);
    }
}
