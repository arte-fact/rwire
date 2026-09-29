//! Error toast notifications for the async-action lifecycle.
//!
//! Drop `ActionToasts::new().build()` once near the root of the page: whenever any action fails, a
//! toast slides in (fixed, bottom-right) carrying the message, and it disappears the moment that
//! action is retried or resolved. It reads the same live error list as [`crate::ActionErrors`] —
//! use whichever surface fits: an inline slot, floating toasts, or both.
//!
//! ```ignore
//! use rwire_components::ActionToasts;
//! ActionToasts::new().build()
//! ```

use crate::{Toast, ToastContainer};
use rwire::{reflect_action_errors, ElementBuilder};

/// Floating error notifications, one per failed action.
#[derive(Default)]
pub struct ActionToasts {}

impl ActionToasts {
    /// Create the notification surface.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build it into a synced region. It renders an empty (hidden) container while there are no
    /// errors, so it's safe to mount unconditionally.
    pub fn build(self) -> ElementBuilder {
        reflect_action_errors(|messages| {
            let mut container = ToastContainer::new();
            for message in messages {
                container = container.toast(Toast::error(message.clone()));
            }
            container.build()
        })
    }
}
