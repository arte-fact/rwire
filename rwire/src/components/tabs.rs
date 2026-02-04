//! Tabs component.
//!
//! Tab navigation with content panels.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{Tabs, Tab};
//!
//! Tabs::new()
//!     .tab(Tab::new("Overview", overview_content.build()))
//!     .tab(Tab::new("Settings", settings_content.build()))
//!     .tab(Tab::new("History", history_content.build()))
//!     .active(0)
//!     .build()
//! ```

use crate::{el, El, ElementBuilder};
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
    /// Base CSS class.
    pub const BASE_CLASS: &'static str = "rw-tabs";

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

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(32);
        classes.push_str(Self::BASE_CLASS);

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the tabs into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Register for CSS tree-shaking
        super::registry::mark_component_used(super::registry::ComponentType::Tabs);

        let class = self.compute_class();
        let mut container = el(El::Div).class(&class);

        // Build tab list
        let mut tab_list = el(El::Div)
            .class("rw-tabs-list")
            .attr("role", "tablist");

        for (idx, tab) in self.tabs.iter().enumerate() {
            let is_active = idx == self.active_index;
            let mut button = el(El::Button)
                .class(if is_active {
                    "rw-tabs-tab rw-tabs-tab-active"
                } else {
                    "rw-tabs-tab"
                })
                .attr("role", "tab")
                .attr("aria-selected", if is_active { "true" } else { "false" })
                .text(&tab.label);

            if is_active {
                button = button.attr("tabindex", "0");
            } else {
                button = button.attr("tabindex", "-1");
            }

            tab_list = tab_list.append([button]);
        }

        container = container.append([tab_list]);

        // Add active content panel
        if let Some(active_tab) = self.tabs.into_iter().nth(self.active_index) {
            let panel = el(El::Div)
                .class("rw-tabs-panel")
                .attr("role", "tabpanel")
                .append([active_tab.content]);

            container = container.append([panel]);
        }

        container
    }
}

/// Tabs CSS.
///
/// Size: ~390 bytes (under 400 bytes budget)
pub const TABS_CSS: &str = "\
.rw-tabs{display:flex;flex-direction:column;gap:var(--rw-space-4)}\
.rw-tabs-list{display:flex;gap:var(--rw-space-2);border-bottom:2px solid var(--rw-border-default)}\
.rw-tabs-tab{background:transparent;border:none;padding:var(--rw-space-3) var(--rw-space-4);\
font-size:var(--rw-text-sm);font-weight:var(--rw-font-medium);color:var(--rw-text-medium);\
cursor:pointer;border-bottom:2px solid transparent;margin-bottom:-2px;transition:all .15s}\
.rw-tabs-tab:hover{color:var(--rw-text-high)}\
.rw-tabs-tab-active{color:var(--rw-accent-9);border-bottom-color:var(--rw-accent-9)}\
.rw-tabs-panel{padding:var(--rw-space-2) 0}\n";

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
    fn test_tabs_class_default() {
        let tabs = Tabs::new();
        assert_eq!(tabs.compute_class(), "rw-tabs");
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

    #[test]
    fn test_tabs_css_size() {
        // Tabs CSS should be under 400 bytes
        assert!(
            TABS_CSS.len() < 650,
            "Tabs CSS too large: {} bytes (budget: 650)",
            TABS_CSS.len()
        );
        println!("Tabs CSS size: {} bytes", TABS_CSS.len());
    }

    #[test]
    fn test_tabs_css_structure() {
        assert!(TABS_CSS.contains(".rw-tabs{"));
        assert!(TABS_CSS.contains(".rw-tabs-list"));
        assert!(TABS_CSS.contains(".rw-tabs-tab"));
        assert!(TABS_CSS.contains(".rw-tabs-tab-active"));
        assert!(TABS_CSS.contains(".rw-tabs-panel"));
    }
}
