//! Slider component.
//!
//! Touch-friendly range input with a visual track, a fill bar and a live value
//! readout. The fill and readout follow the thumb client-side (no round-trip);
//! give it a `name` to submit it as a form field, or `on_change` to react on
//! release.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::Slider;
//!
//! Slider::new()
//!     .name("volume")
//!     .label("Volume")
//!     .unit("%")
//!     .min(0)
//!     .max(100)
//!     .value(42)
//!     .build()
//! ```
use std::borrow::Cow;

use rwire::attr_tokens::{At, Av};
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
    label: Option<Cow<'static, str>>,
    unit: Option<Cow<'static, str>>,
    name: Option<Cow<'static, str>>,
    id: Option<Cow<'static, str>>,
}

#[rwire::component]
impl Slider {
    /// Create a new slider (0..=100, value 0).
    pub fn new() -> Self {
        Self {
            max: 100,
            ..Default::default()
        }
    }

    pub fn min(mut self, min: i32) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: i32) -> Self {
        self.max = max;
        self
    }

    pub fn value(mut self, value: i32) -> Self {
        self.value = value;
        self
    }

    pub fn step(mut self, step: i32) -> Self {
        self.step = Some(step);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Server handler fired on release (`change`), not on every drag tick.
    pub fn on_change(mut self, handler: HandlerSpec) -> Self {
        self.on_change = Some(handler);
        self
    }

    /// Visible label (also the accessible name).
    pub fn label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Suffix shown after the live value (e.g. `"%"`).
    pub fn unit(mut self, unit: impl Into<Cow<'static, str>>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// Form field name; the value is submitted with the enclosing `<form>`.
    pub fn name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Stable element id (keeps focus/value across re-renders).
    pub fn id(mut self, id: impl Into<Cow<'static, str>>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn compute_tokens() -> Vec<St> {
        vec![St::DisplayFlex, St::FlexCol, St::GapXs, St::WFull]
    }

    /// Build the slider into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let channel = rwire::builder::next_live_channel();
        let value = self
            .value
            .clamp(self.min.min(self.max), self.max.max(self.min));
        let range = self.max - self.min;
        let fill_pct = if range > 0 {
            ((value - self.min) as f64 / range as f64 * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

        // Label row: label on the left, live value (+ unit) on the right.
        let mut readout = el(El::Div)
            .st([St::DisplayFlex, St::ItemsBaseline, St::GapXs])
            .append([el(El::Strong)
                .st([St::TextLg])
                .text(&value.to_string())
                .live_text(channel)]);
        if let Some(ref unit) = self.unit {
            readout = readout.append([el(El::Span).st([St::TextSm, St::TextMuted]).text(unit)]);
        }
        let mut header = el(El::Div).st([
            St::DisplayFlex,
            St::JustifyBetween,
            St::ItemsBaseline,
            St::GapSm,
        ]);
        if let Some(ref label) = self.label {
            header = header.append([el(El::Span).st([St::TextSm, St::FontMedium]).text(label)]);
        } else {
            header = header.append([el(El::Span)]);
        }
        header = header.append([readout]);

        // Visual track with fill, vertically centered in the touch-sized control box.
        let track = el(El::Div)
            .st([St::SliderTrack, St::PositionAbsolute, St::RoundedSm])
            .attr("style", "top:50%;left:0;right:0;transform:translateY(-50%)")
            .append([el(El::Div)
                .st([St::SliderFill])
                .attr("style", &format!("width:{fill_pct:.1}%"))
                .live_fill(channel)]);

        // Native range input overlaid on the track: transparent, only the thumb is painted.
        let mut input = el(El::Input)
            .st([
                St::SliderInput,
                St::SliderThumb,
                St::PositionAbsolute,
                St::Inset0,
            ])
            .at(At::Type, Av::Range)
            .at_str(At::Min, &self.min.to_string())
            .at_str(At::Max, &self.max.to_string())
            .at_str(At::Value, &value.to_string())
            .live_source(channel);

        if let Some(step) = self.step {
            input = input.at_str(At::Step, &step.to_string());
        }
        if let Some(ref name) = self.name {
            input = input.at_str(At::Name, name);
        }
        if let Some(ref id) = self.id {
            input = input.at_str(At::Id, id);
        }
        if self.disabled {
            input = input.bool_attr(At::Disabled);
        }
        if let Some(ref label) = self.label {
            input = input.at_str(At::AriaLabel, label);
        }
        // Bind `change` (fires on release), not `input`: a round-trip per drag tick
        // would re-render and interrupt the native drag.
        if let Some(handler) = self.on_change {
            input = input.on(Ev::Change, handler);
        }

        el(El::Div).st(Self::compute_tokens()).append([
            header,
            el(El::Div)
                .st([St::PositionRelative, St::WFull, St::H3rem])
                .append([track, input]),
        ])
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
        assert!(slider.name.is_none());
    }

    #[test]
    fn test_slider_tokens() {
        let tokens = Slider::compute_tokens();
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::WFull));
    }

    #[test]
    fn test_slider_with_values() {
        let slider = Slider::new().min(10).max(50).value(30).name("n").unit("%");
        assert_eq!(slider.min, 10);
        assert_eq!(slider.max, 50);
        assert_eq!(slider.value, 30);
        assert_eq!(slider.name.as_deref(), Some("n"));
        assert_eq!(slider.unit.as_deref(), Some("%"));
    }

    #[test]
    fn build_clamps_value_into_range() {
        // Out-of-range values must not panic and must clamp.
        let _ = Slider::new().min(0).max(10).value(99).build();
        let _ = Slider::new().min(5).max(5).value(1).build();
    }
}
