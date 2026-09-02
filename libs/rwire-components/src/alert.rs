//! Alert component.
//!
//! Alert messages with intent-based styling.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Alert, AlertIntent};
//!
//! Alert::info()
//!     .title("Note")
//!     .message("Your changes have been saved")
//!     .build()
//!
//! Alert::new()
//!     .intent(AlertIntent::Error)
//!     .title("Error")
//!     .message("Failed to connect to server")
//!     .build()
//! ```

use rwire::attr_tokens::{At, Av};
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Alert intent (visual style).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlertIntent {
    /// Informational message (blue)
    #[default]
    Info,
    /// Success message (green)
    Success,
    /// Warning message (yellow)
    Warning,
    /// Error message (red)
    Error,
}

/// Alert builder.
#[derive(Clone, Debug, Default)]
pub struct Alert {
    intent: AlertIntent,
    title: Option<Cow<'static, str>>,
    message: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
}

#[rwire::component]
impl Alert {
    /// Create a new alert.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an info alert.
    pub fn info() -> Self {
        Self::new().intent(AlertIntent::Info)
    }

    /// Create a success alert.
    pub fn success() -> Self {
        Self::new().intent(AlertIntent::Success)
    }

    /// Create a warning alert.
    pub fn warning() -> Self {
        Self::new().intent(AlertIntent::Warning)
    }

    /// Create an error alert.
    pub fn error() -> Self {
        Self::new().intent(AlertIntent::Error)
    }

    /// Set the alert intent.
    pub fn intent(mut self, intent: AlertIntent) -> Self {
        self.intent = intent;
        self
    }

    /// Set the alert title.
    pub fn title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the alert message.
    pub fn message(mut self, message: impl Into<Cow<'static, str>>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the alert container.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![
            St::DisplayFlex,
            St::FlexCol,
            St::GapSm,
            St::PMd,
            St::RoundedMd,
            St::BorderLTheme,
            St::TextSm,
        ];

        match self.intent {
            AlertIntent::Info => {
                tokens.push(St::BgInfoSubtle);
                tokens.push(St::BorderBlue8);
            }
            AlertIntent::Success => {
                tokens.push(St::BgSuccessSubtle);
                tokens.push(St::BorderGreen8);
            }
            AlertIntent::Warning => {
                tokens.push(St::BgWarningSubtle);
                tokens.push(St::BorderAmber8);
            }
            AlertIntent::Error => {
                tokens.push(St::BgErrorSubtle);
                tokens.push(St::BorderRed8);
            }
        }

        tokens
    }

    /// Build the alert into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut alert = el(El::Div)
            .st(self.compute_tokens())
            .at(At::Role, Av::RoleAlert);

        if let Some(ref extra) = self.extra_class {
            alert = alert.class(extra.as_ref());
        }

        if let Some(title_text) = self.title {
            alert = alert.append([el(El::Span)
                .st([St::FontMedium, St::TextHigh])
                .text(&title_text)]);
        }

        if let Some(message_text) = self.message {
            alert = alert.append([el(El::P).st([St::M0, St::TextDefault]).text(&message_text)]);
        }

        alert
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_defaults() {
        let alert = Alert::new();
        assert_eq!(alert.intent, AlertIntent::Info);
        assert!(alert.title.is_none());
        assert!(alert.message.is_none());
    }

    #[test]
    fn test_alert_info_tokens() {
        let alert = Alert::new();
        let tokens = alert.compute_tokens();
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::FlexCol));
        assert!(tokens.contains(&St::BgInfoSubtle));
        assert!(tokens.contains(&St::BorderBlue8));
    }

    #[test]
    fn test_alert_error_tokens() {
        let alert = Alert::error();
        let tokens = alert.compute_tokens();
        assert!(tokens.contains(&St::BgErrorSubtle));
        assert!(tokens.contains(&St::BorderRed8));
    }
}
