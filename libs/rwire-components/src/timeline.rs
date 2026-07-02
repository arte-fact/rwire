//! Timeline component.
//!
//! Vertical event timeline with timestamps and status dots.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Timeline, TimelineItem};
//!
//! Timeline::new()
//!     .item(TimelineItem::new("Deployed to production").time("2m ago").active(true))
//!     .item(TimelineItem::new("Tests passed").time("5m ago"))
//!     .item(TimelineItem::new("PR merged").time("10m ago"))
//!     .build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// A single timeline event.
#[derive(Clone)]
pub struct TimelineItem {
    title: Cow<'static, str>,
    description: Option<Cow<'static, str>>,
    time: Option<Cow<'static, str>>,
    active: bool,
}

impl TimelineItem {
    /// Create a new timeline item.
    pub fn new(title: impl Into<Cow<'static, str>>) -> Self {
        Self {
            title: title.into(),
            description: None,
            time: None,
            active: false,
        }
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the timestamp text.
    pub fn time(mut self, time: impl Into<Cow<'static, str>>) -> Self {
        self.time = Some(time.into());
        self
    }

    /// Mark this item as active/current.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

/// Timeline builder.
#[derive(Clone, Default)]
pub struct Timeline {
    items: Vec<TimelineItem>,
    extra_class: Option<Cow<'static, str>>,
}

#[rwire::component]
impl Timeline {
    /// Create a new timeline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a timeline item.
    pub fn item(mut self, item: TimelineItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the timeline container.
    pub fn compute_tokens() -> Vec<St> {
        vec![St::DisplayFlex, St::FlexCol]
    }

    /// Compute style tokens for a timeline row.
    pub fn compute_row_tokens() -> Vec<St> {
        vec![St::DisplayFlex, St::GapMd]
    }

    /// Build the timeline into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut container = el(El::Div).st(Self::compute_tokens());

        if let Some(ref extra) = self.extra_class {
            container = container.class(extra.as_ref());
        }

        for (i, item) in self.items.iter().enumerate() {
            let is_last = i == self.items.len() - 1;

            // Left column: dot + line
            let dot = el(El::Div).st(if item.active {
                vec![St::TimelineDotActive, St::FlexShrink0]
            } else {
                vec![St::TimelineDot, St::FlexShrink0]
            });

            let mut left = el(El::Div)
                .st([St::DisplayFlex, St::FlexCol, St::ItemsCenter])
                .append([dot]);

            if !is_last {
                left = left.append([el(El::Div).st([St::Flex1, St::TimelineLine])]);
            }

            // Right column: content
            let mut content = el(El::Div).st([St::Flex1, St::PbMd]).append([el(El::Div)
                .st([St::FontMedium, St::TextDefault])
                .text(&item.title)]);

            if let Some(ref desc) = item.description {
                content = content.append([el(El::P)
                    .st([St::TextSm, St::TextMuted, St::MtXs])
                    .text(desc)]);
            }

            if let Some(ref time) = item.time {
                content = content.append([el(El::Span).st([St::TextXs, St::TextMuted]).text(time)]);
            }

            let row = el(El::Div)
                .st(Self::compute_row_tokens())
                .append([left, content]);

            container = container.append([row]);
        }

        container
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_item_new() {
        let item = TimelineItem::new("Deployed");
        assert_eq!(item.title.as_ref(), "Deployed");
        assert!(!item.active);
        assert!(item.time.is_none());
    }

    #[test]
    fn test_timeline_defaults() {
        let tl = Timeline::new();
        assert!(tl.items.is_empty());
    }

    #[test]
    fn test_timeline_container_tokens() {
        let tokens = Timeline::compute_tokens();
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::FlexCol));
    }

    #[test]
    fn test_timeline_with_items() {
        let tl = Timeline::new()
            .item(TimelineItem::new("First").time("1m ago").active(true))
            .item(TimelineItem::new("Second").time("5m ago"));
        assert_eq!(tl.items.len(), 2);
        assert!(tl.items[0].active);
        assert!(!tl.items[1].active);
    }
}
