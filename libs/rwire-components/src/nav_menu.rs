//! NavigationMenu component.
//!
//! Top-level navigation bar with links and optional active state.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{NavMenu, NavItem};
//!
//! NavMenu::new()
//!     .item(NavItem::new("Home", "/"))
//!     .item(NavItem::new("Docs", "/docs"))
//!     .item(NavItem::new("API", "/api"))
//!     .active_path("/docs")
//!     .build()
//! ```

use rwire::attr_tokens::At;
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// A navigation menu item.
#[derive(Clone, Debug)]
pub struct NavItem {
    label: Cow<'static, str>,
    href: Cow<'static, str>,
}

impl NavItem {
    /// Create a new navigation item.
    pub fn new(
        label: impl Into<Cow<'static, str>>,
        href: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            label: label.into(),
            href: href.into(),
        }
    }
}

/// NavigationMenu builder.
#[derive(Clone, Default)]
pub struct NavMenu {
    items: Vec<NavItem>,
    active_path: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
}

#[rwire::component]
impl NavMenu {
    /// Create a new navigation menu.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a navigation item.
    pub fn item(mut self, item: NavItem) -> Self {
        self.items.push(item);
        self
    }

    /// Set the active path for highlighting.
    pub fn active_path(mut self, path: impl Into<Cow<'static, str>>) -> Self {
        self.active_path = Some(path.into());
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the nav container.
    pub fn compute_tokens() -> Vec<St> {
        vec![St::DisplayFlex, St::ItemsCenter, St::GapXs]
    }

    /// Compute style tokens for a nav link.
    pub fn compute_link_tokens() -> Vec<St> {
        vec![
            St::PxSm,
            St::PySm,
            St::TextSm,
            St::FontMedium,
            St::NoDecoration,
            St::TextMuted,
            St::RoundedMd,
            St::TransitionColors,
        ]
    }

    /// Compute style tokens for the active nav link.
    pub fn compute_active_link_tokens() -> Vec<St> {
        vec![
            St::PxSm,
            St::PySm,
            St::TextSm,
            St::FontMedium,
            St::NoDecoration,
            St::TextDefault,
            St::RoundedMd,
            St::BgEmphasis,
        ]
    }

    /// Build the navigation menu into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut nav = el(El::Nav).st(Self::compute_tokens());

        if let Some(ref extra) = self.extra_class {
            nav = nav.class(extra.as_ref());
        }

        for item in &self.items {
            let is_active = self
                .active_path
                .as_deref()
                .is_some_and(|p| p == item.href.as_ref());

            let tokens = if is_active {
                Self::compute_active_link_tokens()
            } else {
                Self::compute_link_tokens()
            };

            let mut link = el(El::A)
                .st(tokens)
                .at_str(At::Href, &item.href)
                .text(&item.label);

            if !is_active {
                link = link.hover([St::TextDefault, St::BgHover]);
            }

            nav = nav.append([link]);
        }

        nav
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nav_item_new() {
        let item = NavItem::new("Home", "/");
        assert_eq!(item.label.as_ref(), "Home");
        assert_eq!(item.href.as_ref(), "/");
    }

    #[test]
    fn test_nav_menu_defaults() {
        let menu = NavMenu::new();
        assert!(menu.items.is_empty());
        assert!(menu.active_path.is_none());
    }

    #[test]
    fn test_nav_menu_container_tokens() {
        let tokens = NavMenu::compute_tokens();
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::ItemsCenter));
    }

    #[test]
    fn test_nav_menu_link_tokens() {
        let tokens = NavMenu::compute_link_tokens();
        assert!(tokens.contains(&St::TextMuted));
        assert!(tokens.contains(&St::NoDecoration));
        assert!(tokens.contains(&St::RoundedMd));
    }

    #[test]
    fn test_nav_menu_active_link_tokens() {
        let tokens = NavMenu::compute_active_link_tokens();
        assert!(tokens.contains(&St::TextDefault));
        assert!(tokens.contains(&St::BgEmphasis));
    }

    #[test]
    fn test_nav_menu_with_items() {
        let menu = NavMenu::new()
            .item(NavItem::new("Home", "/"))
            .item(NavItem::new("Docs", "/docs"))
            .active_path("/docs");
        assert_eq!(menu.items.len(), 2);
        assert_eq!(menu.active_path.as_deref(), Some("/docs"));
    }
}
