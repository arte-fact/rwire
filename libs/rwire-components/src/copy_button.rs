//! Copy-to-clipboard button component.
//!
//! Renders a button that copies text to the clipboard using `data-copy`.
//! The client JS handles the copy; visual feedback uses a timed toggle
//! to swap between clipboard and check icons for 2 seconds.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::CopyButton;
//!
//! CopyButton::new("npm install rwire").build()
//! ```

use rwire::icons::{icon_sized, Icon};
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder, Ev, Target};

/// Client-side target for copy feedback state.
#[derive(Target)]
struct CopyFeedback;

/// Copy button builder.
pub struct CopyButton {
    text: String,
}

#[rwire::component]
impl CopyButton {
    /// Create a new copy button for the given text.
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }

    /// Build the copy button into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        el(El::Button)
            .data("copy", &self.text)
            .st([
                St::DisplayInlineFlex,
                St::ItemsCenter,
                St::JustifyCenter,
                St::PxSm,
                St::PySm,
                St::RoundedMd,
                St::BorderNone,
                St::BgTransparent,
                St::TextMuted,
                St::CursorPointer,
                St::TransitionColors,
            ])
            .hover([St::TextDefault, St::BgMuted])
            .active([St::Scale95])
            .toggle_timed::<CopyFeedback>(Ev::Click, 2000)
            .append([
                // Default icon (clipboard) — hidden when CopyFeedback is active
                el(El::Span)
                    .when::<CopyFeedback>(St::DisplayNone)
                    .append([icon_sized(Icon::Clipboard, 16)]),
                // Copied icon (check) — hidden by default, shown when CopyFeedback is active
                el(El::Span)
                    .unless::<CopyFeedback>(St::DisplayNone)
                    .append([icon_sized(Icon::ClipboardCheck, 16)]),
            ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_button_builds() {
        let btn = CopyButton::new("hello world").build();
        drop(btn);
    }
}
