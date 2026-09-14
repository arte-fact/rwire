//! Drawer component.
//!
//! Slide-in panel from edge of screen with backdrop overlay.
//!
//! The full DOM structure is always rendered. Visibility is toggled via
//! transform (slide) and opacity (backdrop), enabling smooth CSS transitions.
//!
//! # Example
//!
//! ```ignore
//! use rwire::{Ev, handler, renderer, State};
//! use rwire_components::{Drawer, DrawerPosition};
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

use rwire::attr_tokens::{At, Av};
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder, Ev, HandlerSpec};
use std::borrow::Cow;

/// Drawer slide-in position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DrawerPosition {
    #[default]
    Left,
    Right,
    /// Bottom sheet: full width, slides up, capped at 85dvh.
    Bottom,
}

/// Drawer builder.
#[derive(Clone, Default)]
pub struct Drawer {
    title: Option<Cow<'static, str>>,
    position: DrawerPosition,
    open: bool,
    on_close: Option<HandlerSpec>,
    content: Option<ElementBuilder>,
    /// Pinned under the content, outside its scroll: a wide close button,
    /// the sheet's action.
    footer: Option<ElementBuilder>,
    extra_class: Option<Cow<'static, str>>,
}

#[rwire::component]
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
    /// Set what stays pinned at the bottom while the content scrolls.
    pub fn footer(mut self, footer: ElementBuilder) -> Self {
        self.footer = Some(footer);
        self
    }

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
            St::Bottom0,
            St::MaxWFull,
            St::BgApp,
            St::ShadowXl,
            St::DisplayFlex,
            St::FlexCol,
            St::TransitionTransformMd,
            St::Z1400,
        ]
    }

    /// Side panels are 320px tall columns; the bottom sheet spans the width.
    pub fn compute_shape_tokens(position: DrawerPosition) -> Vec<St> {
        match position {
            DrawerPosition::Left | DrawerPosition::Right => vec![St::Top0, St::W320px],
            DrawerPosition::Bottom => vec![
                St::Left0,
                St::Right0,
                St::WFull,
                St::MaxH85Dvh,
                St::RoundedTLg,
                St::PbSafe,
            ],
        }
    }

    /// Build the drawer into an ElementBuilder.
    ///
    /// Always renders the full DOM structure. Open/close state is toggled
    /// via transform (slide) and opacity (backdrop), enabling CSS transitions.
    pub fn build(self) -> ElementBuilder {
        let mut panel_tokens = Self::compute_panel_tokens();
        panel_tokens.extend(Self::compute_shape_tokens(self.position));

        // Position the panel and set slide transform
        match self.position {
            DrawerPosition::Left => {
                panel_tokens.push(St::Left0);
                if self.open {
                    panel_tokens.push(St::TransformNone);
                } else {
                    panel_tokens.push(St::TranslateXNegFull);
                }
            }
            DrawerPosition::Right => {
                panel_tokens.push(St::Right0);
                if self.open {
                    panel_tokens.push(St::TransformNone);
                } else {
                    panel_tokens.push(St::TranslateXFull);
                }
            }
            DrawerPosition::Bottom => {
                if self.open {
                    panel_tokens.push(St::TransformNone);
                } else {
                    panel_tokens.push(St::TranslateYPosFull);
                }
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
                use rwire::icons::{icon, Icon};
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
                        St::FlexShrink0,
                    ])
                    .append(header_children),
            );
        }

        // Content: the one part that scrolls — the header above and the
        // footer below stay where the thumb expects them.
        if let Some(content) = self.content {
            panel_children.push(
                el(El::Div)
                    .st([St::Flex1, St::MinH0, St::OverflowYScroll, St::PMd])
                    .append([content]),
            );
        }
        if let Some(footer) = self.footer {
            panel_children.push(
                el(El::Div)
                    .st([St::FlexShrink0, St::PxMd, St::PySm, St::BorderT])
                    .append([footer]),
            );
        }

        let mut panel = el(El::Div)
            .st(panel_tokens)
            .at(At::Role, Av::RoleDialog)
            .at(At::AriaModal, Av::True)
            .append(panel_children);

        if let Some(ref extra) = self.extra_class {
            panel = panel.class(extra.as_ref());
        }

        // Backdrop — always present, visibility toggled
        let mut backdrop = el(El::Div).st([
            St::PositionFixed,
            St::Inset0,
            St::Z1300,
            St::BgOverlay50,
            St::TransitionOpacity,
        ]);

        if !self.open {
            backdrop = backdrop.st([St::Opacity0, St::PointerEventsNone]);
        }

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
    fn the_content_scrolls_between_a_pinned_header_and_footer() {
        // The panel is a fixed-height column: it must not scroll as a whole,
        // or the title's cross and the footer's button leave with the content.
        assert!(!Drawer::compute_panel_tokens().contains(&St::OverflowYScroll));
        let built = Drawer::new()
            .title("T")
            .content(el(El::P).text("body"))
            .footer(el(El::Button).text("Fermer"))
            .build();
        let panel = &built.children()[1];
        let parts = panel.children();
        assert_eq!(parts.len(), 3);
        let utils = |i: usize| parts[i].get_style_utils().to_vec();
        assert!(utils(0).contains(&(St::FlexShrink0 as u16)));
        assert!(utils(1).contains(&(St::OverflowYScroll as u16)));
        assert!(utils(1).contains(&(St::MinH0 as u16)));
        assert!(utils(2).contains(&(St::FlexShrink0 as u16)));
    }

    #[test]
    fn test_drawer_panel_tokens() {
        let tokens = Drawer::compute_panel_tokens();
        assert!(tokens.contains(&St::PositionFixed));
        assert!(tokens.contains(&St::BgApp));
        let side = Drawer::compute_shape_tokens(DrawerPosition::Left);
        assert!(side.contains(&St::W320px) && side.contains(&St::Top0));
        let sheet = Drawer::compute_shape_tokens(DrawerPosition::Bottom);
        assert!(sheet.contains(&St::WFull) && sheet.contains(&St::MaxH85Dvh));
        assert!(!sheet.contains(&St::W320px));
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

    #[test]
    fn test_drawer_always_renders_structure() {
        // Both open and closed should build successfully (always-render)
        let _open = Drawer::new().open(true).build();
        let _closed = Drawer::new().open(false).build();
    }
}
