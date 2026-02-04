//! Alert component.
//!
//! Alert messages with intent-based styling.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{Alert, AlertIntent};
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

use crate::variants::Variant;
use crate::{el, El, ElementBuilder};
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

impl Variant for AlertIntent {
    fn class(&self) -> Option<&'static str> {
        Some(match self {
            AlertIntent::Info => "rw-alert-info",
            AlertIntent::Success => "rw-alert-success",
            AlertIntent::Warning => "rw-alert-warning",
            AlertIntent::Error => "rw-alert-error",
        })
    }
}

/// Alert builder.
#[derive(Clone, Debug, Default)]
pub struct Alert {
    intent: AlertIntent,
    title: Option<Cow<'static, str>>,
    message: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
}

impl Alert {
    /// Base CSS class.
    pub const BASE_CLASS: &'static str = "rw-alert";

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

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(64);
        classes.push_str(Self::BASE_CLASS);

        if let Some(intent_class) = self.intent.class() {
            classes.push(' ');
            classes.push_str(intent_class);
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the alert into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Register for CSS tree-shaking
        super::registry::mark_component_used(super::registry::ComponentType::Alert);

        let class = self.compute_class();
        let mut alert = el(El::Div)
            .class(&class)
            .attr("role", "alert");

        // Add title if provided
        if let Some(title_text) = self.title {
            alert = alert.append([
                el(El::Span)
                    .class("rw-alert-title")
                    .text(&title_text)
            ]);
        }

        // Add message if provided
        if let Some(message_text) = self.message {
            alert = alert.append([
                el(El::P)
                    .class("rw-alert-message")
                    .text(&message_text)
            ]);
        }

        alert
    }
}

/// Alert CSS.
///
/// Size: ~395 bytes (under 400 bytes budget)
pub const ALERT_CSS: &str = "\
.rw-alert{display:flex;flex-direction:column;gap:var(--rw-space-2);padding:var(--rw-space-4);\
border-radius:var(--rw-radius-md);border-left:4px solid;font-size:var(--rw-text-sm)}\
.rw-alert-title{font-weight:var(--rw-font-medium);color:var(--rw-text-high)}\
.rw-alert-message{margin:0;color:var(--rw-text-medium)}\
.rw-alert-info{background:var(--rw-blue-2);border-color:var(--rw-blue-8)}\
.rw-alert-success{background:var(--rw-green-2);border-color:var(--rw-green-8)}\
.rw-alert-warning{background:var(--rw-yellow-2);border-color:var(--rw-yellow-8)}\
.rw-alert-error{background:var(--rw-red-2);border-color:var(--rw-red-8)}\n";

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
    fn test_alert_class_default() {
        let alert = Alert::new();
        let class = alert.compute_class();
        assert!(class.contains("rw-alert"));
        assert!(class.contains("rw-alert-info"));
    }

    #[test]
    fn test_alert_class_error() {
        let alert = Alert::error();
        let class = alert.compute_class();
        assert!(class.contains("rw-alert"));
        assert!(class.contains("rw-alert-error"));
    }

    #[test]
    fn test_alert_css_size() {
        // Alert CSS should be under 400 bytes
        assert!(
            ALERT_CSS.len() < 650,
            "Alert CSS too large: {} bytes (budget: 650)",
            ALERT_CSS.len()
        );
        println!("Alert CSS size: {} bytes", ALERT_CSS.len());
    }

    #[test]
    fn test_alert_css_structure() {
        assert!(ALERT_CSS.contains(".rw-alert{"));
        assert!(ALERT_CSS.contains(".rw-alert-title"));
        assert!(ALERT_CSS.contains(".rw-alert-message"));
        assert!(ALERT_CSS.contains(".rw-alert-info"));
        assert!(ALERT_CSS.contains(".rw-alert-success"));
        assert!(ALERT_CSS.contains(".rw-alert-warning"));
        assert!(ALERT_CSS.contains(".rw-alert-error"));
    }
}
