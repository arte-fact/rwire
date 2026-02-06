//! Accordion component.
//!
//! Collapsible content sections controlled by server state.
//!
//! # Example
//!
//! ```ignore
//! use rwire::{Ev, handler, renderer, State};
//! use rwire::components::{Accordion, AccordionItem};
//!
//! #[derive(State, Default)]
//! #[storage(memory)]
//! struct DocsState {
//!     open_sections: Vec<bool>,
//! }
//!
//! #[renderer]
//! fn render_faq(state: &DocsState) -> ElementBuilder {
//!     Accordion::new()
//!         .item(AccordionItem::new("Getting Started")
//!             .open(state.open_sections.get(0).copied().unwrap_or(false))
//!             .on_toggle(toggle_section_0())
//!             .content(el(El::P).text("Welcome to rwire!")))
//!         .item(AccordionItem::new("API Reference")
//!             .open(state.open_sections.get(1).copied().unwrap_or(false))
//!             .on_toggle(toggle_section_1())
//!             .content(el(El::P).text("See the API docs.")))
//!         .build()
//! }
//! ```

use crate::attr_tokens::{At, Av};
use crate::style_tokens::St;
use crate::{el, El, ElementBuilder, Ev, HandlerSpec};
use std::borrow::Cow;

/// A single accordion item with trigger and content.
#[derive(Clone)]
pub struct AccordionItem {
    title: Cow<'static, str>,
    open: bool,
    on_toggle: Option<HandlerSpec>,
    content: Option<ElementBuilder>,
}

impl AccordionItem {
    /// Create a new accordion item with a title.
    pub fn new(title: impl Into<Cow<'static, str>>) -> Self {
        Self {
            title: title.into(),
            open: false,
            on_toggle: None,
            content: None,
        }
    }

    /// Set whether this item is open.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Set the toggle handler (called when trigger is clicked).
    pub fn on_toggle(mut self, handler: HandlerSpec) -> Self {
        self.on_toggle = Some(handler);
        self
    }

    /// Set the content to show when open.
    pub fn content(mut self, content: ElementBuilder) -> Self {
        self.content = Some(content);
        self
    }
}

/// Accordion builder.
#[derive(Clone, Default)]
pub struct Accordion {
    items: Vec<AccordionItem>,
    extra_class: Option<Cow<'static, str>>,
}

impl Accordion {
    /// Create a new accordion.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an item to the accordion.
    pub fn item(mut self, item: AccordionItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the accordion container.
    pub fn compute_tokens() -> Vec<St> {
        vec![St::DisplayFlex, St::FlexCol, St::BorderSubtle, St::RoundedMd]
    }

    /// Compute style tokens for the trigger button.
    pub fn compute_trigger_tokens() -> Vec<St> {
        vec![
            St::WFull,
            St::DisplayFlex,
            St::ItemsCenter,
            St::JustifyBetween,
            St::PxMd,
            St::PyMd,
            St::BgTransparent,
            St::BorderNone,
            St::CursorPointer,
            St::TextLeft,
            St::FontMedium,
            St::TextDefault,
            St::TransitionColors,
        ]
    }

    /// Compute style tokens for the content area when open.
    pub fn compute_content_open_tokens() -> Vec<St> {
        vec![
            St::OverflowHidden,
            St::PxMd,
            St::PbMd,
            St::TransitionAll,
        ]
    }

    /// Compute style tokens for the content area when closed.
    pub fn compute_content_closed_tokens() -> Vec<St> {
        vec![
            St::OverflowHidden,
            St::MaxH0,
            St::P0,
            St::DisplayNone,
        ]
    }

    /// Build the accordion into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut container = el(El::Div).st(Self::compute_tokens());

        if let Some(ref extra) = self.extra_class {
            container = container.class(extra.as_ref());
        }

        let item_count = self.items.len();
        for (i, item) in self.items.into_iter().enumerate() {
            let mut trigger = el(El::Button)
                .st(Self::compute_trigger_tokens())
                .hover([St::BgHover])
                .at(At::Type, Av::Button)
                .at_str(At::AriaExpanded, if item.open { "true" } else { "false" })
                .text(&item.title);

            if let Some(handler) = item.on_toggle {
                trigger = trigger.on(Ev::Click, handler);
            }

            let content_tokens = if item.open {
                Self::compute_content_open_tokens()
            } else {
                Self::compute_content_closed_tokens()
            };

            let mut content_el = el(El::Div).st(content_tokens);

            if let Some(content) = item.content {
                content_el = content_el.append([content]);
            }

            let mut item_el = el(El::Div).append([trigger, content_el]);

            // Add border between items (not on the last one)
            if i < item_count - 1 {
                item_el = item_el.st([St::BorderBSubtle]);
            }

            container = container.append([item_el]);
        }

        container
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accordion_item_new() {
        let item = AccordionItem::new("Title");
        assert_eq!(item.title.as_ref(), "Title");
        assert!(!item.open);
        assert!(item.on_toggle.is_none());
        assert!(item.content.is_none());
    }

    #[test]
    fn test_accordion_item_open() {
        let item = AccordionItem::new("Title").open(true);
        assert!(item.open);
    }

    #[test]
    fn test_accordion_container_tokens() {
        let tokens = Accordion::compute_tokens();
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::FlexCol));
        assert!(tokens.contains(&St::BorderSubtle));
        assert!(tokens.contains(&St::RoundedMd));
    }

    #[test]
    fn test_accordion_trigger_tokens() {
        let tokens = Accordion::compute_trigger_tokens();
        assert!(tokens.contains(&St::WFull));
        assert!(tokens.contains(&St::CursorPointer));
        assert!(tokens.contains(&St::JustifyBetween));
        assert!(tokens.contains(&St::FontMedium));
    }

    #[test]
    fn test_accordion_content_open_tokens() {
        let tokens = Accordion::compute_content_open_tokens();
        assert!(tokens.contains(&St::OverflowHidden));
        assert!(tokens.contains(&St::PxMd));
        assert!(tokens.contains(&St::TransitionAll));
    }

    #[test]
    fn test_accordion_content_closed_tokens() {
        let tokens = Accordion::compute_content_closed_tokens();
        assert!(tokens.contains(&St::MaxH0));
        assert!(tokens.contains(&St::DisplayNone));
    }

    #[test]
    fn test_accordion_empty() {
        let acc = Accordion::new();
        assert!(acc.items.is_empty());
    }

    #[test]
    fn test_accordion_with_items() {
        let acc = Accordion::new()
            .item(AccordionItem::new("First"))
            .item(AccordionItem::new("Second"));
        assert_eq!(acc.items.len(), 2);
    }
}
