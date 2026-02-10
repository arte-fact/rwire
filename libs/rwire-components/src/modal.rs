//! Modal/Dialog component for overlays and confirmations.
//!
//! This component creates a modal dialog with backdrop, focus management,
//! and keyboard handling. The modal is server-controlled via state.
//!
//! # Example
//!
//! ```ignore
//! use rwire::{State, handler, renderer};
//! use rwire_components::Modal;
//!
//! #[derive(State, Default)]
//! #[storage(memory)]
//! struct AppState {
//!     modal_open: bool,
//! }
//!
//! #[renderer]
//! fn render_modal(state: &AppState) -> ElementBuilder {
//!     Modal::new()
//!         .title("Confirm Action")
//!         .open(state.modal_open)
//!         .on_close(close_modal())
//!         .content(el(El::P).text("Are you sure?"))
//!         .footer(Stack::row()
//!             .gap(Gap::Sm)
//!             .children([
//!                 Button::new().text("Cancel").on_click(close_modal()),
//!                 Button::primary().text("Confirm").on_click(confirm_action()),
//!             ])
//!             .build())
//!         .build()
//! }
//! ```

use rwire::attr_tokens::{At, Av};
use rwire::style_tokens::St;
use rwire::{
    el, El, ElementBuilder, HandlerSpec,
};

/// Modal/Dialog builder.
#[derive(Clone)]
pub struct Modal {
    title: Option<String>,
    size: ModalSize,
    open: bool,
    on_close: Option<HandlerSpec>,
    content: Option<ElementBuilder>,
    footer: Option<ElementBuilder>,
    close_on_backdrop_click: bool,
}

/// Size variants for the modal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ModalSize {
    Sm,
    #[default]
    Md,
    Lg,
    Xl,
    Full,
}

impl Default for Modal {
    fn default() -> Self {
        Self {
            title: None,
            size: ModalSize::default(),
            open: false,
            on_close: None,
            content: None,
            footer: None,
            close_on_backdrop_click: true,
        }
    }
}

impl Modal {
    /// Create a new modal.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the modal title.
    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    /// Set the modal size.
    pub fn size(mut self, size: ModalSize) -> Self {
        self.size = size;
        self
    }

    /// Set whether the modal is open.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Set the close handler.
    pub fn on_close(mut self, handler: HandlerSpec) -> Self {
        self.on_close = Some(handler);
        self
    }

    /// Set the modal content.
    pub fn content(mut self, content: ElementBuilder) -> Self {
        self.content = Some(content);
        self
    }

    /// Set the modal footer.
    pub fn footer(mut self, footer: ElementBuilder) -> Self {
        self.footer = Some(footer);
        self
    }

    /// Set whether clicking the backdrop closes the modal.
    pub fn close_on_backdrop_click(mut self, close: bool) -> Self {
        self.close_on_backdrop_click = close;
        self
    }

    /// Compute style tokens for the modal inner panel.
    pub fn compute_tokens(&self) -> Vec<St> {
        vec![
            St::BgSurfaceRaised, St::RoundedLg, St::ShadowTheme, St::DisplayFlex,
            St::FlexCol, St::PointerEventsAuto, St::OverflowHidden,
            St::BackdropTheme, St::OpacityTheme,
        ]
    }

    /// Compute size-specific style tokens.
    fn size_tokens(&self) -> Vec<St> {
        match self.size {
            ModalSize::Sm => vec![St::W400px, St::MaxWFull],
            ModalSize::Md => vec![St::W600px, St::MaxWFull],
            ModalSize::Lg => vec![St::W800px, St::MaxWFull],
            ModalSize::Xl => vec![St::W1000px, St::MaxWFull],
            ModalSize::Full => vec![St::WFull, St::HFull, St::MaxWFull, St::MaxHFull, St::Rounded0],
        }
    }

    /// Build the modal component.
    pub fn build(self) -> ElementBuilder {
        if !self.open {
            // Modal is closed - return hidden container
            return el(El::Div).st([St::DisplayNone]);
        }

        let mut tokens = self.compute_tokens();
        tokens.extend(self.size_tokens());
        tokens.push(St::MaxH90vh);

        // Build modal structure
        let mut modal_children = Vec::new();

        // Header with title and close button
        if self.title.is_some() || self.on_close.is_some() {
            modal_children.push(self.build_header());
        }

        // Content
        if let Some(content) = self.content {
            modal_children.push(
                el(El::Div)
                    .st([St::OverflowYAuto, St::TextDefault, St::Flex1, St::PMd])
                    .append([content])
            );
        }

        // Footer
        if let Some(footer) = self.footer {
            modal_children.push(
                el(El::Div)
                    .st([St::DisplayFlex, St::ItemsCenter, St::JustifyEnd, St::GapSm, St::PMd, St::BorderT])
                    .append([footer])
            );
        }

        let modal_inner = el(El::Div)
            .st(tokens)
            .at(At::Role, Av::RoleDialog)
            .at(At::AriaModal, Av::True)
            .at(At::Tabindex, Av::MinusOne)
            .append(modal_children);

        // Backdrop
        let mut backdrop_el = el(El::Div)
            .st([St::PositionFixed, St::Inset0, St::Z1300, St::BgOverlay50]);

        if self.close_on_backdrop_click {
            if let Some(handler) = self.on_close.clone() {
                backdrop_el = backdrop_el.on(rwire::Ev::Click, handler);
            }
        }

        // Container with backdrop and modal
        el(El::Div)
            .st([St::PositionFixed, St::Inset0, St::PointerEventsAuto, St::Z1400])
            .append([
                backdrop_el,
                el(El::Div)
                    .st([St::PositionFixed, St::Inset0, St::DisplayFlex, St::ItemsCenter, St::JustifyCenter, St::PointerEventsNone, St::PMd])
                    .append([modal_inner])
            ])
    }

    fn build_header(&self) -> ElementBuilder {
        let mut header_children = Vec::new();

        // Title
        if let Some(ref title) = self.title {
            header_children.push(
                el(El::H2)
                    .st([St::M0, St::FontMedium, St::TextLg])
                    .text(title)
            );
        }

        // Close button
        if let Some(handler) = self.on_close.clone() {
            use rwire::icons::{icon, Icon};
            header_children.push(
                el(El::Button)
                    .st([
                        St::DisplayFlex, St::ItemsCenter, St::JustifyCenter,
                        St::BgTransparent, St::BorderNone, St::RoundedSm,
                        St::TextMuted, St::CursorPointer, St::TransitionColors,
                        St::W2rem, St::H2rem, St::P0,
                    ])
                    .hover([St::BgHover])
                    .at(At::Type, Av::Button)
                    .at_str(At::AriaLabel, "Close")
                    .on(rwire::Ev::Click, handler)
                    .append([icon(Icon::Close)])
            );
        }

        el(El::Div)
            .st([St::DisplayFlex, St::ItemsCenter, St::JustifyBetween, St::PMd, St::BorderBDefault])
            .append(header_children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modal_defaults() {
        let modal = Modal::new();
        assert_eq!(modal.size, ModalSize::Md);
        assert!(!modal.open);
        assert!(modal.title.is_none());
        assert!(modal.on_close.is_none());
        assert!(modal.content.is_none());
        assert!(modal.footer.is_none());
        assert!(modal.close_on_backdrop_click);
    }

    #[test]
    fn test_modal_tokens() {
        let modal = Modal::new();
        let tokens = modal.compute_tokens();
        assert!(tokens.contains(&St::BgSurfaceRaised));
        assert!(tokens.contains(&St::RoundedLg));
        assert!(tokens.contains(&St::ShadowTheme));
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::FlexCol));
        assert!(tokens.contains(&St::PointerEventsAuto));
        assert!(tokens.contains(&St::OverflowHidden));
    }

    #[test]
    fn test_modal_size_tokens() {
        let modal = Modal::new().size(ModalSize::Sm);
        let tokens = modal.size_tokens();
        assert_eq!(tokens, vec![St::W400px, St::MaxWFull]);

        let modal = Modal::new().size(ModalSize::Lg);
        let tokens = modal.size_tokens();
        assert_eq!(tokens, vec![St::W800px, St::MaxWFull]);

        let modal = Modal::new().size(ModalSize::Full);
        let tokens = modal.size_tokens();
        assert!(tokens.contains(&St::WFull));
        assert!(tokens.contains(&St::HFull));
        assert!(tokens.contains(&St::MaxWFull));
        assert!(tokens.contains(&St::MaxHFull));
    }

    #[test]
    fn test_modal_open_state() {
        let modal = Modal::new().open(true);
        assert!(modal.open);

        let modal = Modal::new().open(false);
        assert!(!modal.open);
    }

}
