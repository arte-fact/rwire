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
    channel: Option<u16>,
    marks: Vec<(i32, Cow<'static, str>)>,
    readout_suffix: Option<ElementBuilder>,
    above_track: Vec<ElementBuilder>,
    readout: Readout,
}

/// How the slider's figure is printed: `fmt` renders the initial figure
/// server-side, the live readout follows the page's language.
#[derive(Clone, Copy, Default)]
enum Readout {
    #[default]
    Plain,
    Grouped(fn(i32) -> String),
    /// The value is in `10^digits`ths of the unit shown.
    Decimal(u8, fn(i32) -> String),
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

    /// Use a caller-allocated live channel (from `rwire::builder::next_live_channel`)
    /// so other elements can follow the thumb with `live_text`/`live_scaled`/…
    pub fn channel(mut self, channel: u16) -> Self {
        self.channel = Some(channel);
        self
    }

    /// A tick on the track at `value`, with a small label under it. Marks
    /// outside `min..=max` are ignored.
    pub fn mark(mut self, value: i32, label: impl Into<Cow<'static, str>>) -> Self {
        self.marks.push((value, label.into()));
        self
    }

    /// Show the value with thousands separators: `fmt` renders the initial
    /// figure server-side and the live readout is grouped in the page's language
    /// (`CapsuleConfig::lang`) — pick a `fmt` that prints the same separator.
    pub fn grouped(mut self, fmt: fn(i32) -> String) -> Self {
        self.readout = Readout::Grouped(fmt);
        self
    }

    /// The value counts in `10^digits`ths of the unit shown (tenths for a
    /// slider stepping by `0,1`): `fmt` renders the initial figure server-side
    /// and the live readout prints `digits` decimals in the page's language.
    pub fn decimals(mut self, digits: u8, fmt: fn(i32) -> String) -> Self {
        self.readout = Readout::Decimal(digits, fmt);
        self
    }

    /// Extra content after the live value and unit (e.g. a live percentage).
    pub fn readout_suffix(mut self, suffix: ElementBuilder) -> Self {
        self.readout_suffix = Some(suffix);
        self
    }

    /// Content between the readout and the track, as wide as the track (e.g. a
    /// chart of what the value produces, so the thumb lines up with its x).
    pub fn above_track(mut self, content: ElementBuilder) -> Self {
        self.above_track.push(content);
        self
    }

    pub fn compute_tokens() -> Vec<St> {
        vec![St::DisplayFlex, St::FlexCol, St::GapXs, St::WFull]
    }

    /// Build the slider into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let channel = self
            .channel
            .unwrap_or_else(rwire::builder::next_live_channel);
        let value = self
            .value
            .clamp(self.min.min(self.max), self.max.max(self.min));
        let range = self.max - self.min;
        let pct = |v: i32| {
            if range > 0 {
                ((v - self.min) as f64 / range as f64 * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            }
        };
        let fill_pct = pct(value);

        // Label row: label on the left, live value (+ unit) on the right.
        let figure = el(El::Strong).st([St::TextLg]);
        let figure = match self.readout {
            Readout::Plain => figure.text(&value.to_string()).live_text(channel),
            Readout::Grouped(fmt) => figure.text(&fmt(value)).live_text_grouped(channel),
            Readout::Decimal(digits, fmt) => figure.text(&fmt(value)).live_decimal(channel, digits),
        };
        let mut readout = el(El::Div)
            .st([St::DisplayFlex, St::ItemsBaseline, St::GapXs])
            .append([figure]);
        if let Some(ref unit) = self.unit {
            readout = readout.append([el(El::Span).st([St::TextSm, St::TextMuted]).text(unit)]);
        }
        if let Some(suffix) = self.readout_suffix {
            readout = readout.append([suffix]);
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

        // Ticks over the track, labels in a row under the control box.
        let marks: Vec<(f64, Cow<'static, str>)> = self
            .marks
            .into_iter()
            .filter(|&(v, _)| v >= self.min && v <= self.max)
            .map(|(v, label)| (pct(v), label))
            .collect();
        let ticks = marks.iter().map(|(p, _)| {
            el(El::Div)
                .st([St::SliderMark])
                .attr("style", &format!("left:{p:.1}%"))
        });
        let labels = (!marks.is_empty()).then(|| {
            el(El::Div)
                .st([St::PositionRelative, St::H1rem, St::TextXs, St::TextMuted])
                .append(marks.iter().map(|(p, label)| {
                    // Keep edge labels inside the track.
                    let shift = if *p < 5.0 {
                        None
                    } else if *p > 95.0 {
                        Some(St::TranslateXNegFull)
                    } else {
                        Some(St::TransformCenterX)
                    };
                    el(El::Span)
                        .st(std::iter::once(St::SliderMarkLabel).chain(shift))
                        .attr("style", &format!("left:{p:.1}%"))
                        .text(label)
                }))
        });

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

        el(El::Div)
            .st(Self::compute_tokens())
            .append([header])
            .append(self.above_track)
            .append([el(El::Div)
                .st([St::PositionRelative, St::WFull, St::H3rem])
                .append([track])
                .append(ticks)
                .append([input])])
            .append(labels)
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
