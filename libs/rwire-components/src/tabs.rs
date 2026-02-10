//! Tabs component.
//!
//! Tab navigation with content panels.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Tabs, Tab};
//!
//! Tabs::new()
//!     .tab(Tab::new("Overview", overview_content.build()))
//!     .tab(Tab::new("Settings", settings_content.build()))
//!     .tab(Tab::new("History", history_content.build()))
//!     .active(0)
//!     .build()
//! ```

use rwire::attr_tokens::{At, Av};
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// A single tab with label and content.
#[derive(Clone)]
pub struct Tab {
    label: Cow<'static, str>,
    content: ElementBuilder,
}

impl Tab {
    /// Create a new tab.
    pub fn new(label: impl Into<Cow<'static, str>>, content: ElementBuilder) -> Self {
        Self {
            label: label.into(),
            content,
        }
    }
}

/// Tabs builder.
#[derive(Clone, Default)]
pub struct Tabs {
    tabs: Vec<Tab>,
    active_index: usize,
    extra_class: Option<Cow<'static, str>>,
}

impl Tabs {
    /// Create a new tabs component.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a tab.
    pub fn tab(mut self, tab: Tab) -> Self {
        self.tabs.push(tab);
        self
    }

    /// Set the active tab index.
    pub fn active(mut self, index: usize) -> Self {
        self.active_index = index;
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the tabs container.
    pub fn compute_tokens(&self) -> Vec<St> {
        vec![St::DisplayFlex, St::FlexCol, St::GapMd]
    }

    /// Build the tabs into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut container = el(El::Div).st(self.compute_tokens());

        if let Some(ref extra) = self.extra_class {
            container = container.class(extra.as_ref());
        }

        // Build tab list
        let mut tab_list = el(El::Div)
            .st([St::DisplayFlex, St::GapSm, St::BorderB2Default])
            .at(At::Role, Av::RoleTablist);

        for (idx, tab) in self.tabs.iter().enumerate() {
            let is_active = idx == self.active_index;
            let mut button_tokens = vec![
                St::BgTransparent, St::BorderNone, St::TextSm, St::FontMedium,
                St::CursorPointer, St::TransitionAll,
                St::PySp3, St::PxMd, St::MbNeg2px,
            ];

            if is_active {
                button_tokens.push(St::TextAccent);
                button_tokens.push(St::BorderB2Accent);
            } else {
                button_tokens.push(St::TextMedium);
                button_tokens.push(St::BorderB2Transparent);
            }

            let mut button = el(El::Button)
                .st(button_tokens)
                .at(At::Role, Av::RoleTab)
                .at(At::AriaSelected, if is_active { Av::True } else { Av::False })
                .text(&tab.label);

            if is_active {
                button = button.at(At::Tabindex, Av::Zero);
            } else {
                button = button.at(At::Tabindex, Av::MinusOne);
            }

            tab_list = tab_list.append([button]);
        }

        container = container.append([tab_list]);

        // Add active content panel
        if let Some(active_tab) = self.tabs.into_iter().nth(self.active_index) {
            let panel = el(El::Div)
                .st([St::PySm, St::Px0])
                .at(At::Role, Av::RoleTabpanel)
                .append([active_tab.content]);

            container = container.append([panel]);
        }

        container
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tabs_defaults() {
        let tabs = Tabs::new();
        assert!(tabs.tabs.is_empty());
        assert_eq!(tabs.active_index, 0);
    }

    #[test]
    fn test_tabs_tokens() {
        let tabs = Tabs::new();
        let tokens = tabs.compute_tokens();
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::FlexCol));
        assert!(tokens.contains(&St::GapMd));
    }

    #[test]
    fn test_tabs_with_tabs() {
        let tabs = Tabs::new()
            .tab(Tab::new("Tab 1", el(El::Div)))
            .tab(Tab::new("Tab 2", el(El::Div)))
            .active(1);
        assert_eq!(tabs.tabs.len(), 2);
        assert_eq!(tabs.active_index, 1);
    }

}
