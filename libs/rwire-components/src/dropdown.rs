//! DropdownMenu component.
//!
//! Action menu triggered by a button. Server-controlled open state.
//!
//! The full DOM structure (trigger, menu panel, items, backdrop) is always
//! rendered. Visibility is toggled via opacity and pointer-events, enabling
//! smooth CSS transitions when opening/closing.
//!
//! # Example
//!
//! ```ignore
//! use rwire::{Ev, handler, renderer, State};
//! use rwire_components::{DropdownMenu, DropdownItem};
//!
//! #[derive(State, Default)]
//! #[storage(memory)]
//! struct AppState {
//!     menu_open: bool,
//! }
//!
//! #[renderer]
//! fn render_menu(state: &AppState) -> ElementBuilder {
//!     DropdownMenu::new()
//!         .open(state.menu_open)
//!         .on_toggle(toggle_menu())
//!         .trigger(Button::secondary("Actions").build())
//!         .item(DropdownItem::new("Edit").on_click(edit_handler()))
//!         .item(DropdownItem::new("Delete").on_click(delete_handler()))
//!         .build()
//! }
//! ```

use rwire::attr_tokens::{At, Av};
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder, Ev, HandlerSpec};
use std::borrow::Cow;

/// A single dropdown menu item.
#[derive(Clone)]
pub struct DropdownItem {
    label: Cow<'static, str>,
    on_click: Option<HandlerSpec>,
    destructive: bool,
    divider_before: bool,
}

impl DropdownItem {
    /// Create a new dropdown item.
    pub fn new(label: impl Into<Cow<'static, str>>) -> Self {
        Self {
            label: label.into(),
            on_click: None,
            destructive: false,
            divider_before: false,
        }
    }

    /// Set the click handler.
    pub fn on_click(mut self, handler: HandlerSpec) -> Self {
        self.on_click = Some(handler);
        self
    }

    /// Mark as destructive (red text).
    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    /// Show a divider before this item.
    pub fn divider(mut self) -> Self {
        self.divider_before = true;
        self
    }
}

/// DropdownMenu builder.
#[derive(Clone, Default)]
pub struct DropdownMenu {
    open: bool,
    on_toggle: Option<HandlerSpec>,
    trigger: Option<ElementBuilder>,
    items: Vec<DropdownItem>,
    extra_class: Option<Cow<'static, str>>,
}

#[rwire::component]
impl DropdownMenu {
    /// Create a new dropdown menu.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether the menu is open.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Set the toggle handler (opens/closes the menu).
    pub fn on_toggle(mut self, handler: HandlerSpec) -> Self {
        self.on_toggle = Some(handler);
        self
    }

    /// Set the trigger element.
    pub fn trigger(mut self, trigger: ElementBuilder) -> Self {
        self.trigger = Some(trigger);
        self
    }

    /// Add a menu item.
    pub fn item(mut self, item: DropdownItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the menu panel.
    pub fn compute_menu_tokens() -> Vec<St> {
        vec![
            St::PositionAbsolute,
            St::Top0,
            St::Left0,
            St::BgApp,
            St::BorderSubtle,
            St::RoundedMd,
            St::ShadowLg,
            St::PySm,
            St::Z50,
            St::MinW0,
        ]
    }

    /// Compute style tokens for a menu item.
    pub fn compute_item_tokens() -> Vec<St> {
        vec![
            St::WFull,
            St::DisplayFlex,
            St::ItemsCenter,
            St::PxMd,
            St::PySm,
            St::BgTransparent,
            St::BorderNone,
            St::TextLeft,
            St::TextSm,
            St::TextDefault,
            St::CursorPointer,
            St::TransitionColors,
        ]
    }

    /// Build the dropdown into an ElementBuilder.
    ///
    /// Always renders the full DOM structure. Menu visibility is toggled
    /// via opacity and pointer-events, enabling CSS transitions.
    pub fn build(self) -> ElementBuilder {
        let mut container = el(El::Div).st([St::PositionRelative, St::DisplayInlineBlock]);

        if let Some(ref extra) = self.extra_class {
            container = container.class(extra.as_ref());
        }

        // Trigger button
        if let Some(mut trigger) = self.trigger {
            if let Some(handler) = self.on_toggle.clone() {
                trigger = trigger.on(Ev::Click, handler);
            }
            container = container.append([trigger]);
        }

        // Invisible backdrop to close menu on outside click — always rendered
        if let Some(handler) = self.on_toggle.clone() {
            let mut backdrop = el(El::Div)
                .st([St::PositionFixed, St::Inset0, St::Z40]);
            if self.open {
                backdrop = backdrop.on(Ev::Click, handler);
            } else {
                backdrop = backdrop.st([St::PointerEventsNone]);
            }
            container = container.append([backdrop]);
        }

        // Menu panel — always rendered, visibility toggled
        let mut menu_tokens = Self::compute_menu_tokens();
        menu_tokens.push(St::TransitionOpacity);

        if !self.open {
            menu_tokens.extend([St::Opacity0, St::PointerEventsNone, St::Invisible]);
        }

        let mut menu = el(El::Div)
            .st(menu_tokens)
            .attr("style", "top:100%;margin-top:4px;min-width:160px")
            .at(At::Role, Av::RoleMenu);

        for item in &self.items {
            if item.divider_before {
                menu = menu.append([el(El::Hr).st([
                    St::MySm,
                    St::BorderNone,
                    St::BorderTSubtle,
                ])]);
            }

            let mut item_tokens = Self::compute_item_tokens();
            if item.destructive {
                item_tokens.push(St::TextError);
            }

            let mut btn = el(El::Button)
                .st(item_tokens)
                .hover([St::BgHover])
                .at(At::Type, Av::Button)
                .at(At::Role, Av::RoleMenuItem)
                .text(&item.label);

            if let Some(handler) = item.on_click.clone() {
                btn = btn.on(Ev::Click, handler);
            }

            menu = menu.append([btn]);
        }

        container = container.append([menu]);

        container
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dropdown_defaults() {
        let menu = DropdownMenu::new();
        assert!(!menu.open);
        assert!(menu.items.is_empty());
        assert!(menu.trigger.is_none());
    }

    #[test]
    fn test_dropdown_item() {
        let item = DropdownItem::new("Edit");
        assert_eq!(item.label.as_ref(), "Edit");
        assert!(!item.destructive);
        assert!(!item.divider_before);
    }

    #[test]
    fn test_dropdown_item_destructive() {
        let item = DropdownItem::new("Delete").destructive();
        assert!(item.destructive);
    }

    #[test]
    fn test_dropdown_menu_tokens() {
        let tokens = DropdownMenu::compute_menu_tokens();
        assert!(tokens.contains(&St::PositionAbsolute));
        assert!(tokens.contains(&St::BgApp));
        assert!(tokens.contains(&St::BorderSubtle));
        assert!(tokens.contains(&St::ShadowLg));
        assert!(tokens.contains(&St::RoundedMd));
    }

    #[test]
    fn test_dropdown_item_tokens() {
        let tokens = DropdownMenu::compute_item_tokens();
        assert!(tokens.contains(&St::WFull));
        assert!(tokens.contains(&St::CursorPointer));
        assert!(tokens.contains(&St::TextSm));
    }

    #[test]
    fn test_dropdown_with_items() {
        let menu = DropdownMenu::new()
            .item(DropdownItem::new("Edit"))
            .item(DropdownItem::new("Delete").destructive().divider());
        assert_eq!(menu.items.len(), 2);
    }

    #[test]
    fn test_dropdown_always_renders_structure() {
        // Both open and closed should build successfully (always-render)
        let _open = DropdownMenu::new().open(true).build();
        let _closed = DropdownMenu::new().open(false).build();
    }
}
