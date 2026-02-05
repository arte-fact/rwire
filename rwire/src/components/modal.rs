//! Modal/Dialog component for overlays and confirmations.
//!
//! This component creates a modal dialog with backdrop, focus management,
//! and keyboard handling. The modal is server-controlled via state.
//!
//! # Example
//!
//! ```ignore
//! use rwire::{State, handler, renderer};
//! use rwire::components::Modal;
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

use crate::{
    el, El, ElementBuilder, HandlerSpec,
    components::utils::{Z_MODAL, Z_MODAL_BACKDROP, class_if},
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
    /// Base CSS class.
    pub const BASE_CLASS: &'static str = "rw-modal";

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

    /// Build the modal component.
    pub fn build(self) -> ElementBuilder {
        // Register component for CSS tree-shaking
        super::registry::mark_component_used(
            super::registry::ComponentType::Modal
        );

        if !self.open {
            // Modal is closed - return empty container
            return el(El::Div).class("rw-modal-container rw-hidden");
        }

        let modal_class = self.compute_modal_class();
        let backdrop_class = self.compute_backdrop_class();

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
                    .class("rw-modal-content")
                    .append([content])
            );
        }

        // Footer
        if let Some(footer) = self.footer {
            modal_children.push(
                el(El::Div)
                    .class("rw-modal-footer")
                    .append([footer])
            );
        }

        let modal_inner = el(El::Div)
            .class(&modal_class)
            .attr("role", "dialog")
            .attr("aria-modal", "true")
            .attr("tabindex", "-1")
            .append(modal_children);

        // Backdrop
        let mut backdrop_el = el(El::Div)
            .class(&backdrop_class)
            .attr("style", &format!("z-index: {}", Z_MODAL_BACKDROP));

        if self.close_on_backdrop_click {
            if let Some(handler) = self.on_close.clone() {
                backdrop_el = backdrop_el.on(crate::Ev::Click, handler);
            }
        }

        // Container with backdrop and modal
        el(El::Div)
            .class("rw-modal-container")
            .attr("style", &format!("z-index: {}", Z_MODAL))
            .append([
                backdrop_el,
                el(El::Div)
                    .class("rw-modal-positioner")
                    .append([modal_inner])
            ])
    }

    fn build_header(&self) -> ElementBuilder {
        let mut header_children = Vec::new();

        // Title
        if let Some(ref title) = self.title {
            header_children.push(
                el(El::H2)
                    .class("rw-modal-title")
                    .text(title)
            );
        }

        // Close button
        if let Some(handler) = self.on_close.clone() {
            use crate::icons::{icon, Icon};
            header_children.push(
                el(El::Button)
                    .class("rw-modal-close")
                    .attr("type", "button")
                    .attr("aria-label", "Close")
                    .on(crate::Ev::Click, handler)
                    .append([icon(Icon::Close)])
            );
        }

        el(El::Div)
            .class("rw-modal-header")
            .append(header_children)
    }

    fn compute_modal_class(&self) -> String {
        let mut classes = String::with_capacity(64);
        classes.push_str(Self::BASE_CLASS);

        match self.size {
            ModalSize::Sm => classes.push_str(" rw-modal-sm"),
            ModalSize::Md => {},
            ModalSize::Lg => classes.push_str(" rw-modal-lg"),
            ModalSize::Xl => classes.push_str(" rw-modal-xl"),
            ModalSize::Full => classes.push_str(" rw-modal-full"),
        }

        classes
    }

    fn compute_backdrop_class(&self) -> String {
        format!("rw-modal-backdrop{}", class_if(" rw-modal-backdrop-visible", self.open))
    }
}

/// Modal CSS.
pub const MODAL_CSS: &str = "\
.rw-modal-container{position:fixed;top:0;left:0;right:0;bottom:0;pointer-events:none}\
.rw-modal-container:not(.rw-hidden){pointer-events:auto}\
.rw-modal-backdrop{position:fixed;top:0;left:0;right:0;bottom:0;background:rgba(0,0,0,0.5);\
opacity:0;transition:opacity 200ms}\
.rw-modal-backdrop-visible{opacity:1}\
.rw-modal-positioner{position:fixed;top:0;left:0;right:0;bottom:0;display:flex;\
align-items:center;justify-content:center;padding:var(--rw-space-4);pointer-events:none}\
.rw-modal{background:var(--rw-bg-default);border-radius:var(--rw-radius-lg);\
box-shadow:var(--rw-shadow-lg);max-height:90vh;display:flex;flex-direction:column;\
pointer-events:auto;transform:scale(0.95);opacity:0;transition:transform 200ms,opacity 200ms}\
.rw-modal-container:not(.rw-hidden) .rw-modal{transform:scale(1);opacity:1}\
.rw-modal-sm{width:400px;max-width:100%}\
.rw-modal,.rw-modal-md{width:600px;max-width:100%}\
.rw-modal-lg{width:800px;max-width:100%}\
.rw-modal-xl{width:1000px;max-width:100%}\
.rw-modal-full{width:100%;height:100%;max-width:100%;max-height:100%;border-radius:0}\
.rw-modal-header{display:flex;align-items:center;justify-content:space-between;\
padding:var(--rw-space-4);border-bottom:1px solid var(--rw-border-default)}\
.rw-modal-title{margin:0;font-size:var(--rw-font-size-lg);font-weight:var(--rw-font-weight-semibold);\
color:var(--rw-text-strong)}\
.rw-modal-close{display:flex;align-items:center;justify-content:center;width:32px;\
height:32px;padding:0;background:transparent;border:none;border-radius:var(--rw-radius-sm);\
color:var(--rw-text-subtle);cursor:pointer;transition:var(--rw-transition-fast)}\
.rw-modal-close:hover{background:var(--rw-bg-hover);color:var(--rw-text-default)}\
.rw-modal-close .rw-icon{width:20px;height:20px}\
.rw-modal-content{flex:1;overflow-y:auto;padding:var(--rw-space-4);\
color:var(--rw-text-default)}\
.rw-modal-footer{display:flex;align-items:center;justify-content:flex-end;gap:var(--rw-space-2);\
padding:var(--rw-space-4);border-top:1px solid var(--rw-border-default)}\
";

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
    fn test_modal_class_default() {
        let modal = Modal::new();
        let class = modal.compute_modal_class();
        assert_eq!(class, "rw-modal");
    }

    #[test]
    fn test_modal_class_with_size() {
        let modal = Modal::new().size(ModalSize::Sm);
        let class = modal.compute_modal_class();
        assert!(class.contains("rw-modal-sm"));

        let modal = Modal::new().size(ModalSize::Lg);
        let class = modal.compute_modal_class();
        assert!(class.contains("rw-modal-lg"));

        let modal = Modal::new().size(ModalSize::Full);
        let class = modal.compute_modal_class();
        assert!(class.contains("rw-modal-full"));
    }

    #[test]
    fn test_modal_backdrop_class() {
        let modal = Modal::new().open(false);
        let class = modal.compute_backdrop_class();
        assert_eq!(class, "rw-modal-backdrop");

        let modal = Modal::new().open(true);
        let class = modal.compute_backdrop_class();
        assert!(class.contains("rw-modal-backdrop-visible"));
    }

    #[test]
    fn test_modal_css_size() {
        assert!(
            MODAL_CSS.len() < 2000,
            "Modal CSS too large: {} bytes",
            MODAL_CSS.len()
        );
        println!("Modal CSS size: {} bytes", MODAL_CSS.len());
    }

    #[test]
    fn test_modal_css_structure() {
        assert!(MODAL_CSS.contains(".rw-modal"));
        assert!(MODAL_CSS.contains(".rw-modal-backdrop"));
        assert!(MODAL_CSS.contains(".rw-modal-header"));
        assert!(MODAL_CSS.contains(".rw-modal-content"));
        assert!(MODAL_CSS.contains(".rw-modal-footer"));
        assert!(MODAL_CSS.contains(".rw-modal-close"));
    }

    #[test]
    fn test_modal_open_state() {
        let modal = Modal::new().open(true);
        assert!(modal.open);

        let modal = Modal::new().open(false);
        assert!(!modal.open);
    }
}
