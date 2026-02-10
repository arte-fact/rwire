//! Stat component.
//!
//! Metric display with value, label, and optional trend indicator.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Stat, StatTrend};
//!
//! Stat::new("1,234")
//!     .label("Active Users")
//!     .trend(StatTrend::Up, "+12%")
//!     .build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Trend direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatTrend {
    /// Positive trend (green, arrow up).
    Up,
    /// Negative trend (red, arrow down).
    Down,
    /// Neutral/flat (muted).
    Neutral,
}

/// Stat builder.
#[derive(Clone, Default)]
pub struct Stat {
    value: Cow<'static, str>,
    label: Option<Cow<'static, str>>,
    description: Option<Cow<'static, str>>,
    trend_dir: Option<StatTrend>,
    trend_text: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
}

impl Stat {
    /// Create a new stat with a value.
    pub fn new(value: impl Into<Cow<'static, str>>) -> Self {
        Self {
            value: value.into(),
            ..Self::default()
        }
    }

    /// Set the label.
    pub fn label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set a description shown below the value.
    pub fn description(mut self, description: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the trend indicator.
    pub fn trend(mut self, direction: StatTrend, text: impl Into<Cow<'static, str>>) -> Self {
        self.trend_dir = Some(direction);
        self.trend_text = Some(text.into());
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the container.
    pub fn compute_tokens() -> Vec<St> {
        vec![St::DisplayFlex, St::FlexCol, St::GapXs]
    }

    /// Build the stat into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut container = el(El::Div).st(Self::compute_tokens());

        if let Some(ref extra) = self.extra_class {
            container = container.class(extra.as_ref());
        }

        // Label (above value)
        if let Some(label) = self.label {
            container = container.append([
                el(El::Div)
                    .st([St::TextSm, St::TextMuted, St::FontMedium])
                    .text(&label),
            ]);
        }

        // Value row with optional trend
        let value_el = el(El::Div)
            .st([St::Text4xl, St::FontBold, St::TextDefault])
            .text(&self.value);

        if let (Some(dir), Some(trend_text)) = (self.trend_dir, self.trend_text) {
            let trend_tokens = match dir {
                StatTrend::Up => vec![St::TextSm, St::FontMedium, St::TextGreen12],
                StatTrend::Down => vec![St::TextSm, St::FontMedium, St::TextRed12],
                StatTrend::Neutral => vec![St::TextSm, St::FontMedium, St::TextMuted],
            };

            let arrow = match dir {
                StatTrend::Up => "\u{2191} ", // ↑
                StatTrend::Down => "\u{2193} ", // ↓
                StatTrend::Neutral => "",
            };

            let trend_el = el(El::Span)
                .st(trend_tokens)
                .text(&format!("{arrow}{trend_text}"));

            container = container.append([
                el(El::Div)
                    .st([St::DisplayFlex, St::ItemsBaseline, St::GapSm])
                    .append([value_el, trend_el]),
            ]);
        } else {
            container = container.append([value_el]);
        }

        // Description
        if let Some(description) = self.description {
            container = container.append([
                el(El::Div)
                    .st([St::TextSm, St::TextMuted])
                    .text(&description),
            ]);
        }

        container
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_defaults() {
        let stat = Stat::new("42");
        assert_eq!(stat.value.as_ref(), "42");
        assert!(stat.label.is_none());
        assert!(stat.trend_dir.is_none());
    }

    #[test]
    fn test_stat_tokens() {
        let tokens = Stat::compute_tokens();
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::FlexCol));
        assert!(tokens.contains(&St::GapXs));
    }

    #[test]
    fn test_stat_with_trend() {
        let stat = Stat::new("1,234")
            .label("Users")
            .trend(StatTrend::Up, "+12%");
        assert_eq!(stat.trend_dir, Some(StatTrend::Up));
        assert_eq!(stat.trend_text.as_deref(), Some("+12%"));
    }

    #[test]
    fn test_stat_with_description() {
        let stat = Stat::new("99.9%")
            .label("Uptime")
            .description("Last 30 days");
        assert_eq!(stat.description.as_deref(), Some("Last 30 days"));
    }
}
