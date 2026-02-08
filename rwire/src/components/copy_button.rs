//! Copy-to-clipboard button component.
//!
//! Renders a button that copies text to the clipboard using `data-copy`.
//! The client JS handles the copy and shows a brief "copied" feedback state.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::CopyButton;
//!
//! CopyButton::new("npm install rwire").build()
//! ```

use crate::icons::{icon_sized, Icon};
use crate::style_tokens::St;
use crate::{el, El, ElementBuilder};

/// Copy button builder.
pub struct CopyButton {
    text: String,
}

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
            .append([
                // Default icon (clipboard)
                el(El::Span)
                    .class("copy-icon")
                    .append([icon_sized(Icon::Clipboard, 16)]),
                // Copied icon (shown via CSS when .copied is on parent)
                el(El::Span)
                    .class("copied-icon")
                    .st([St::DisplayNone])
                    .append([icon_sized(Icon::ClipboardCheck, 16)]),
            ])
    }
}

/// CSS rules for the copy button feedback state.
///
/// When JS adds `.copied` to the button:
/// - `.copy-icon` is hidden
/// - `.copied-icon` is shown
pub const COPY_BUTTON_CSS: &str = ".copied .copy-icon{display:none}.copied .copied-icon{display:inline-flex}";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_button_builds() {
        let btn = CopyButton::new("hello world").build();
        drop(btn);
    }
}
