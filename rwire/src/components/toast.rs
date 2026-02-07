//! Toast component.
//!
//! Transient notification messages shown at screen edge.
//! Server pushes toasts via state; client displays them.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{Toast, ToastIntent, ToastContainer};
//!
//! // In a renderer, show active toasts:
//! ToastContainer::new()
//!     .toast(Toast::success("Changes saved"))
//!     .toast(Toast::error("Failed to delete"))
//!     .build()
//! ```

use crate::attr_tokens::{At, Av};
use crate::style_tokens::St;
use crate::{el, El, ElementBuilder, Ev, HandlerSpec};
use std::borrow::Cow;

/// Toast intent (visual style).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToastIntent {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

/// A single toast notification.
#[derive(Clone, Default)]
pub struct Toast {
    intent: ToastIntent,
    message: Cow<'static, str>,
    on_dismiss: Option<HandlerSpec>,
    extra_class: Option<Cow<'static, str>>,
}

impl Toast {
    /// Create a new toast with a message.
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
            ..Self::default()
        }
    }

    /// Create an info toast.
    pub fn info(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(message).intent(ToastIntent::Info)
    }

    /// Create a success toast.
    pub fn success(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(message).intent(ToastIntent::Success)
    }

    /// Create a warning toast.
    pub fn warning(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(message).intent(ToastIntent::Warning)
    }

    /// Create an error toast.
    pub fn error(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(message).intent(ToastIntent::Error)
    }

    /// Set the toast intent.
    pub fn intent(mut self, intent: ToastIntent) -> Self {
        self.intent = intent;
        self
    }

    /// Set the dismiss handler.
    pub fn on_dismiss(mut self, handler: HandlerSpec) -> Self {
        self.on_dismiss = Some(handler);
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the toast.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![
            St::DisplayFlex,
            St::ItemsCenter,
            St::GapSm,
            St::PMd,
            St::RoundedMd,
            St::ShadowLg,
            St::TextSm,
            St::AnimateSlideIn,
            St::MaxW360px,
            St::WFull,
        ];

        match self.intent {
            ToastIntent::Info => {
                tokens.push(St::BgApp);
                tokens.push(St::BorderSubtle);
                tokens.push(St::TextDefault);
            }
            ToastIntent::Success => {
                tokens.push(St::BgGreen4);
                tokens.push(St::BorderGreen8);
                tokens.push(St::TextGreen11);
            }
            ToastIntent::Warning => {
                tokens.push(St::BgYellow2);
                tokens.push(St::BorderYellow8);
                tokens.push(St::TextAmber11);
            }
            ToastIntent::Error => {
                tokens.push(St::BgRed4);
                tokens.push(St::BorderRed8);
                tokens.push(St::TextRed11);
            }
        }

        tokens
    }

    /// Build the toast into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut toast = el(El::Div)
            .st(self.compute_tokens())
            .at(At::Role, Av::RoleAlert);

        if let Some(ref extra) = self.extra_class {
            toast = toast.class(extra.as_ref());
        }

        toast = toast.append([
            el(El::Span).st([St::Flex1]).text(&self.message),
        ]);

        if let Some(handler) = self.on_dismiss {
            use crate::icons::{icon, Icon};
            toast = toast.append([
                el(El::Button)
                    .st([
                        St::DisplayFlex,
                        St::ItemsCenter,
                        St::BgTransparent,
                        St::BorderNone,
                        St::TextMuted,
                        St::CursorPointer,
                        St::P0,
                        St::FlexShrink0,
                    ])
                    .hover([St::TextDefault])
                    .at(At::Type, Av::Button)
                    .at_str(At::AriaLabel, "Dismiss")
                    .on(Ev::Click, handler)
                    .append([icon(Icon::Close)]),
            ]);
        }

        toast
    }
}

/// Container for positioning toasts on screen.
#[derive(Clone, Default)]
pub struct ToastContainer {
    toasts: Vec<Toast>,
    extra_class: Option<Cow<'static, str>>,
}

impl ToastContainer {
    /// Create a new toast container.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a toast to the container.
    pub fn toast(mut self, toast: Toast) -> Self {
        self.toasts.push(toast);
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the container.
    pub fn compute_tokens() -> Vec<St> {
        vec![
            St::FixedBottomRight,
            St::Z9999,
            St::DisplayFlex,
            St::FlexCol,
            St::GapSm,
            St::PointerEventsAuto,
        ]
    }

    /// Build the toast container.
    pub fn build(self) -> ElementBuilder {
        if self.toasts.is_empty() {
            return el(El::Div).st([St::DisplayNone]);
        }

        let mut container = el(El::Div).st(Self::compute_tokens());

        if let Some(ref extra) = self.extra_class {
            container = container.class(extra.as_ref());
        }

        for toast in self.toasts {
            container = container.append([toast.build()]);
        }

        container
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toast_info() {
        let toast = Toast::info("Hello");
        assert_eq!(toast.intent, ToastIntent::Info);
        assert_eq!(toast.message.as_ref(), "Hello");
    }

    #[test]
    fn test_toast_success() {
        let toast = Toast::success("Saved");
        assert_eq!(toast.intent, ToastIntent::Success);
    }

    #[test]
    fn test_toast_error() {
        let toast = Toast::error("Failed");
        assert_eq!(toast.intent, ToastIntent::Error);
    }

    #[test]
    fn test_toast_info_tokens() {
        let toast = Toast::info("test");
        let tokens = toast.compute_tokens();
        assert!(tokens.contains(&St::BgApp));
        assert!(tokens.contains(&St::TextDefault));
        assert!(tokens.contains(&St::AnimateSlideIn));
    }

    #[test]
    fn test_toast_success_tokens() {
        let toast = Toast::success("test");
        let tokens = toast.compute_tokens();
        assert!(tokens.contains(&St::BgGreen4));
        assert!(tokens.contains(&St::TextGreen11));
    }

    #[test]
    fn test_toast_error_tokens() {
        let toast = Toast::error("test");
        let tokens = toast.compute_tokens();
        assert!(tokens.contains(&St::BgRed4));
        assert!(tokens.contains(&St::TextRed11));
    }

    #[test]
    fn test_toast_container_tokens() {
        let tokens = ToastContainer::compute_tokens();
        assert!(tokens.contains(&St::FixedBottomRight));
        assert!(tokens.contains(&St::Z9999));
        assert!(tokens.contains(&St::DisplayFlex));
    }

    #[test]
    fn test_toast_container_empty() {
        let container = ToastContainer::new();
        assert!(container.toasts.is_empty());
    }

    #[test]
    fn test_toast_container_with_toasts() {
        let container = ToastContainer::new()
            .toast(Toast::success("Saved"))
            .toast(Toast::error("Failed"));
        assert_eq!(container.toasts.len(), 2);
    }
}
