//! Chip component.
//!
//! A selectable chip for filters, view toggles, and inline tab pickers — the middle ground
//! between a `Button` (an action) and `Tabs` (a full navigation bar with content panes).
//! Accent-filled when active; muted with a hover lift otherwise.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::Chip;
//!
//! Chip::new("All").active(true).build()
//! Chip::new("Failed").on_click(set_filter()).build().data("filter", "failed")
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder, Ev, HandlerSpec};
use std::borrow::Cow;

/// Chip builder.
#[derive(Debug, Default)]
pub struct Chip {
    label: Cow<'static, str>,
    active: bool,
    on_click: Option<HandlerSpec>,
    extra_class: Option<Cow<'static, str>>,
}

impl Chip {
    /// Create a chip with a label.
    pub fn new(label: impl Into<Cow<'static, str>>) -> Self {
        Self {
            label: label.into(),
            ..Self::default()
        }
    }

    /// Mark the chip as the active selection (accent-filled).
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Attach the click handler. Chain `.data(...)` on the built element for the
    /// handler's payload.
    pub fn on_click(mut self, handler: HandlerSpec) -> Self {
        self.on_click = Some(handler);
        self
    }

    /// Add extra CSS classes.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute the style tokens for the current state.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![
            St::FontInheritAll,
            St::TextXs,
            St::PxSm,
            St::PyXs,
            St::Rounded0,
            St::BorderDefault,
            St::CursorPointer,
        ];
        if self.active {
            tokens.extend([St::BgAccent, St::TextOnAccent, St::BorderAccent]);
        } else {
            tokens.extend([St::BgApp, St::TextMuted]);
        }
        tokens
    }

    /// Build the chip into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let active = self.active;
        let mut builder = el(El::Button)
            .at(rwire::At::Type, rwire::Av::Button)
            .st(self.compute_tokens());
        if let Some(handler) = self.on_click {
            builder = builder.on(Ev::Click, handler);
        }
        if let Some(ref extra) = self.extra_class {
            builder = builder.class(extra.as_ref());
        }
        if !active {
            builder = builder.hover([St::TextHigh]);
        }
        builder.text(self.label.as_ref())
    }
}
