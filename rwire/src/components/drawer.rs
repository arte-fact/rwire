//! Drawer component.
//!
//! Slide-in panel from edge of screen with backdrop overlay.
//!
//! # Example
//!
//! ```ignore
//! use rwire::{Ev, handler, renderer, State};
//! use rwire::components::{Drawer, DrawerPosition};
//!
//! #[derive(State, Default)]
//! #[storage(memory)]
//! struct AppState {
//!     drawer_open: bool,
//! }
//!
//! #[renderer]
//! fn render_drawer(state: &AppState) -> ElementBuilder {
//!     Drawer::new()
//!         .title("Navigation")
//!         .open(state.drawer_open)
//!         .on_close(close_drawer())
//!         .content(sidebar_content())
//!         .build()
//! }
//! ```

use crate::attr_tokens::{At, Av};
use crate::style_tokens::St;
use crate::{el, El, ElementBuilder, Ev, HandlerSpec};
use std::borrow::Cow;

/// Drawer slide-in position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DrawerPosition {
    #[default]
    Left,
    Right,
}

/// Drawer builder.
#[derive(Clone, Default)]
pub struct Drawer {
    title: Option<Cow<'static, str>>,
    position: DrawerPosition,
    open: bool,
    on_close: Option<HandlerSpec>,
    content: Option<ElementBuilder>,
    extra_class: Option<Cow<'static, str>>,
}

impl Drawer {
    /// Create a new drawer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the drawer title.
    pub fn title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the drawer position.
    pub fn position(mut self, position: DrawerPosition) -> Self {
        self.position = position;
        self
    }

    /// Set whether the drawer is open.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Set the close handler.
    pub fn on_close(mut self, handler: HandlerSpec) -> Self {
        self.on_close = Some(handler);
        self
    }

    /// Set the drawer content.
    pub fn content(mut self, content: ElementBuilder) -> Self {
        self.content = Some(content);
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the drawer panel.
    pub fn compute_panel_tokens() -> Vec<St> {
        vec![
            St::PositionFixed,
            St::Top0,
            St::Bottom0,
            St::W320px,
            St::MaxWFull,
            St::BgApp,
            St::ShadowXl,
            St::DisplayFlex,
            St::FlexCol,
            St::OverflowYScroll,
            St::TransitionTransformMd,
            St::Z1400,
        ]
    }

    /// Build the drawer into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        if !self.open {
            return el(El::Div).st([St::DisplayNone]);
        }

        let mut panel_tokens = Self::compute_panel_tokens();

        // Position the panel
        let position_style = match self.position {
            DrawerPosition::Left => {
                panel_tokens.push(St::Left0);
                "left:0"
            }
            DrawerPosition::Right => {
                panel_tokens.push(St::Right0);
                "right:0"
            }
        };

        // Panel content
        let mut panel_children = Vec::new();

        // Header
        if self.title.is_some() || self.on_close.is_some() {
            let mut header_children = Vec::new();

            if let Some(ref title) = self.title {
                header_children.push(
                    el(El::H2)
                        .st([St::M0, St::FontMedium, St::TextLg])
                        .text(title),
                );
            }

            if let Some(handler) = self.on_close.clone() {
                use crate::icons::{icon, Icon};
                header_children.push(
                    el(El::Button)
                        .st([
                            St::DisplayFlex,
                            St::ItemsCenter,
                            St::JustifyCenter,
                            St::BgTransparent,
                            St::BorderNone,
                            St::RoundedSm,
                            St::TextMuted,
                            St::CursorPointer,
                            St::TransitionColors,
                            St::W2rem,
                            St::H2rem,
                            St::P0,
                        ])
                        .hover([St::BgHover])
                        .at(At::Type, Av::Button)
                        .at_str(At::AriaLabel, "Close drawer")
                        .on(Ev::Click, handler)
                        .append([icon(Icon::Close)]),
                );
            }

            panel_children.push(
                el(El::Div)
                    .st([
                        St::DisplayFlex,
                        St::ItemsCenter,
                        St::JustifyBetween,
                        St::PMd,
                        St::BorderBDefault,
                    ])
                    .append(header_children),
            );
        }

        // Content
        if let Some(content) = self.content {
            panel_children.push(el(El::Div).st([St::Flex1, St::PMd]).append([content]));
        }

        let mut panel = el(El::Div)
            .st(panel_tokens)
            .attr("style", position_style)
            .at(At::Role, Av::RoleDialog)
            .at(At::AriaModal, Av::True)
            .append(panel_children);

        if let Some(ref extra) = self.extra_class {
            panel = panel.class(extra.as_ref());
        }

        // Backdrop
        let mut backdrop = el(El::Div)
            .st([St::PositionFixed, St::Inset0, St::Z1300, St::BgOverlay50]);

        if let Some(handler) = self.on_close {
            backdrop = backdrop.on(Ev::Click, handler);
        }

        // Wrapper
        el(El::Div).append([backdrop, panel])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drawer_defaults() {
        let drawer = Drawer::new();
        assert_eq!(drawer.position, DrawerPosition::Left);
        assert!(!drawer.open);
        assert!(drawer.title.is_none());
    }

    #[test]
    fn test_drawer_panel_tokens() {
        let tokens = Drawer::compute_panel_tokens();
        assert!(tokens.contains(&St::PositionFixed));
        assert!(tokens.contains(&St::W320px));
        assert!(tokens.contains(&St::BgApp));
        assert!(tokens.contains(&St::ShadowXl));
        assert!(tokens.contains(&St::TransitionTransformMd));
        assert!(tokens.contains(&St::Z1400));
    }

    #[test]
    fn test_drawer_position() {
        let drawer = Drawer::new().position(DrawerPosition::Right);
        assert_eq!(drawer.position, DrawerPosition::Right);
    }

    #[test]
    fn test_drawer_open_state() {
        let drawer = Drawer::new().open(true);
        assert!(drawer.open);
    }
}
