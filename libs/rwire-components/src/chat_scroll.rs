//! ChatScroll component.
//!
//! A bottom-pinned autoscroll container for chat logs, threads, and live feeds — pure CSS,
//! no JS. `column-reverse` makes the bottom the scroll origin, so the view stays pinned to
//! the newest entry as content grows, yet a reader who scrolls up keeps their place (new
//! entries land below, off screen).
//!
//! The inner column carries `margin-bottom: auto` — in a reversed column an auto bottom
//! margin pushes content to the *visual top*, so a short log reads from the top instead of
//! floating at the bottom of a void. It must be an auto margin and **not**
//! `justify-content`: justify on a scroll container makes overflowing content unreachable
//! (a real regression this component encodes), while an auto margin collapses to zero once
//! content overflows, handing the bottom pin back untouched.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::ChatScroll;
//!
//! ChatScroll::new(el(El::Div).append(entries)).build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};

/// ChatScroll builder.
pub struct ChatScroll {
    inner: ElementBuilder,
}

impl ChatScroll {
    /// Wrap the log's inner column (the element holding the entries, oldest first).
    pub fn new(inner: ElementBuilder) -> Self {
        Self { inner }
    }

    /// Compute the scroller's style tokens.
    pub fn compute_tokens() -> Vec<St> {
        vec![
            St::MinH0,
            St::OverflowAuto,
            St::Flex1,
            St::DisplayFlex,
            St::FlexColReverse,
        ]
    }

    /// Build the scroller into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        el(El::Div)
            .st(Self::compute_tokens())
            .append([self.inner.st([St::MbAuto])])
    }
}
