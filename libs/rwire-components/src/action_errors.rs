//! Page-level error slot for the async-action lifecycle.
//!
//! Action-aware controls ([`crate::Button`], [`crate::Composer`]) surface their own errors inline.
//! `ActionErrors` is the opt-in companion for errors whose triggering control may be off-screen: a
//! synced region that renders an [`Alert`](crate::Alert) for every action currently failed, and
//! empties itself the moment each is retried or resolved.
//!
//! ```ignore
//! use rwire_components::ActionErrors;
//! // Near the top of the page:
//! ActionErrors::new().build()
//! ```

use crate::{Alert, Gap, Stack};
use rwire::{el, reflect_action_errors, El, ElementBuilder};

/// A page-level slot that lists every current action error.
#[derive(Default)]
pub struct ActionErrors {}

impl ActionErrors {
    /// Create the error slot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the slot into a synced region. It renders nothing while there are no errors.
    pub fn build(self) -> ElementBuilder {
        reflect_action_errors(|messages| {
            if messages.is_empty() {
                return el(El::Div);
            }
            let alerts: Vec<ElementBuilder> = messages
                .iter()
                .map(|message| Alert::error().message(message.clone()).build())
                .collect();
            Stack::column().gap(Gap::Sm).children(alerts).build()
        })
    }
}
