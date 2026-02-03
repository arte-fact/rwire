//! Button component with variants.
//!
//! # Variants
//!
//! - **Intent**: Primary (default), Secondary, Ghost, Destructive
//! - **Size**: Sm, Md (default), Lg
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{Button, ButtonIntent, ButtonSize};
//!
//! // Convenience constructors
//! Button::primary("Save").build()
//! Button::secondary("Cancel").build()
//! Button::destructive("Delete").build()
//!
//! // Full configuration
//! Button::new()
//!     .intent(ButtonIntent::Ghost)
//!     .size(ButtonSize::Sm)
//!     .text("More options")
//!     .disabled(true)
//!     .build()
//! ```

use crate::variants::Variant;
use crate::{el, El, ElementBuilder, Ev, HandlerSpec};
use std::borrow::Cow;

/// Button visual intent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonIntent {
    /// Primary action (solid accent color)
    #[default]
    Primary,
    /// Secondary action (subtle background, border)
    Secondary,
    /// Ghost button (transparent, text only)
    Ghost,
    /// Destructive action (red)
    Destructive,
}

impl Variant for ButtonIntent {
    fn class(&self) -> Option<&'static str> {
        match self {
            ButtonIntent::Primary => None,
            ButtonIntent::Secondary => Some("rw-btn-secondary"),
            ButtonIntent::Ghost => Some("rw-btn-ghost"),
            ButtonIntent::Destructive => Some("rw-btn-destructive"),
        }
    }
}

/// Button size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    /// Small: 28px height
    Sm,
    /// Medium: 36px height (default)
    #[default]
    Md,
    /// Large: 44px height
    Lg,
}

impl Variant for ButtonSize {
    fn class(&self) -> Option<&'static str> {
        match self {
            ButtonSize::Sm => Some("rw-btn-sm"),
            ButtonSize::Md => None,
            ButtonSize::Lg => Some("rw-btn-lg"),
        }
    }
}

/// Button component builder.
#[derive(Clone, Debug, Default)]
pub struct Button {
    intent: ButtonIntent,
    size: ButtonSize,
    text: Option<Cow<'static, str>>,
    disabled: bool,
    loading: bool,
    full_width: bool,
    extra_class: Option<Cow<'static, str>>,
}

impl Button {
    /// Base CSS class.
    pub const BASE_CLASS: &'static str = "rw-btn";

    /// Create a new button with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    // ========================================================================
    // Convenience constructors
    // ========================================================================

    /// Primary button with text.
    pub fn primary(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().text(text)
    }

    /// Secondary button with text.
    pub fn secondary(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(ButtonIntent::Secondary).text(text)
    }

    /// Ghost button with text.
    pub fn ghost(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(ButtonIntent::Ghost).text(text)
    }

    /// Destructive button with text.
    pub fn destructive(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(ButtonIntent::Destructive).text(text)
    }

    // ========================================================================
    // Fluent setters
    // ========================================================================

    /// Set the button intent.
    pub fn intent(mut self, intent: ButtonIntent) -> Self {
        self.intent = intent;
        self
    }

    /// Set the button size.
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Set the button text.
    pub fn text(mut self, text: impl Into<Cow<'static, str>>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Set disabled state.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set loading state.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Set full width.
    pub fn full_width(mut self, full: bool) -> Self {
        self.full_width = full;
        self
    }

    /// Add custom class (escape hatch).
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    // ========================================================================
    // Build
    // ========================================================================

    /// Compute the full class string.
    fn compute_class(&self) -> String {
        // Pre-allocate enough for typical class strings
        let mut classes = String::with_capacity(64);
        classes.push_str(Self::BASE_CLASS);

        if let Some(intent_class) = self.intent.class() {
            classes.push(' ');
            classes.push_str(intent_class);
        }

        if let Some(size_class) = self.size.class() {
            classes.push(' ');
            classes.push_str(size_class);
        }

        if self.disabled {
            classes.push_str(" rw-btn-disabled");
        }

        if self.loading {
            classes.push_str(" rw-btn-loading");
        }

        if self.full_width {
            classes.push_str(" rw-btn-full");
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the button into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Register for CSS tree-shaking
        super::registry::mark_component_used(super::registry::ComponentType::Button);

        let class = self.compute_class();
        let mut builder = el(El::Button).class(&class);

        if let Some(text) = self.text {
            builder = builder.text(&text);
        }

        if self.disabled {
            builder = builder.attr("disabled", "");
        }

        if self.loading {
            builder = builder.attr("aria-busy", "true");
        }

        builder
    }

    /// Build with click handler.
    pub fn on_click(self, handler: HandlerSpec) -> ElementBuilder {
        self.build().on(Ev::Click, handler)
    }
}

/// Button CSS.
///
/// Minified CSS for the button component.
/// Size: ~1.5KB
pub const BUTTON_CSS: &str = "\
.rw-btn{display:inline-flex;align-items:center;justify-content:center;gap:var(--rw-space-2);\
font-weight:var(--rw-font-medium);border-radius:var(--rw-radius-md);border:1px solid transparent;\
cursor:pointer;transition:background .15s;height:2.25rem;padding:0 var(--rw-space-4);\
font-size:var(--rw-text-sm);background:var(--rw-accent-9);color:var(--rw-text-on-accent)}\
.rw-btn:hover{background:var(--rw-accent-10)}\
.rw-btn:focus-visible{outline:2px solid var(--rw-accent-8);outline-offset:2px}\
.rw-btn-secondary{background:var(--rw-bg-muted);color:var(--rw-text-high);border-color:var(--rw-border-default)}\
.rw-btn-secondary:hover{background:var(--rw-bg-hover);border-color:var(--rw-border-emphasis)}\
.rw-btn-ghost{background:transparent;color:var(--rw-text-high)}\
.rw-btn-ghost:hover{background:var(--rw-bg-hover)}\
.rw-btn-destructive{background:var(--rw-red-9);color:var(--rw-white)}\
.rw-btn-destructive:hover{background:var(--rw-red-10)}\
.rw-btn-sm{height:1.75rem;padding:0 var(--rw-space-3);font-size:var(--rw-text-xs);gap:var(--rw-space-1)}\
.rw-btn-lg{height:2.75rem;padding:0 var(--rw-space-6);font-size:var(--rw-text-base);gap:var(--rw-space-3)}\
.rw-btn-disabled{opacity:.5;cursor:not-allowed;pointer-events:none}\
.rw-btn-loading{position:relative;color:transparent}\
.rw-btn-loading::after{content:\"\";position:absolute;width:1em;height:1em;\
border:2px solid;border-right-color:transparent;border-radius:50%;animation:rw-spin .6s linear infinite}\
.rw-btn-full{width:100%}\
@keyframes rw-spin{to{transform:rotate(360deg)}}\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_defaults() {
        let btn = Button::new();
        assert_eq!(btn.intent, ButtonIntent::Primary);
        assert_eq!(btn.size, ButtonSize::Md);
        assert!(!btn.disabled);
    }

    #[test]
    fn test_button_class_default() {
        let btn = Button::new();
        assert_eq!(btn.compute_class(), "rw-btn");
    }

    #[test]
    fn test_button_class_secondary() {
        let btn = Button::secondary("Cancel");
        assert_eq!(btn.compute_class(), "rw-btn rw-btn-secondary");
    }

    #[test]
    fn test_button_class_full() {
        let btn = Button::new()
            .intent(ButtonIntent::Destructive)
            .size(ButtonSize::Lg)
            .disabled(true)
            .loading(true)
            .full_width(true);

        let class = btn.compute_class();
        assert!(class.contains("rw-btn"));
        assert!(class.contains("rw-btn-destructive"));
        assert!(class.contains("rw-btn-lg"));
        assert!(class.contains("rw-btn-disabled"));
        assert!(class.contains("rw-btn-loading"));
        assert!(class.contains("rw-btn-full"));
    }

    #[test]
    fn test_button_class_with_extra() {
        let btn = Button::primary("Test").class("my-custom-class");
        let class = btn.compute_class();
        assert!(class.contains("rw-btn"));
        assert!(class.contains("my-custom-class"));
    }

    #[test]
    fn test_button_css_size() {
        // Button CSS should be under 1.5KB
        assert!(
            BUTTON_CSS.len() < 1536,
            "Button CSS too large: {} bytes",
            BUTTON_CSS.len()
        );
        println!("Button CSS size: {} bytes", BUTTON_CSS.len());
    }

    #[test]
    fn test_button_css_structure() {
        assert!(BUTTON_CSS.contains(".rw-btn{"));
        assert!(BUTTON_CSS.contains(".rw-btn-secondary"));
        assert!(BUTTON_CSS.contains(".rw-btn-ghost"));
        assert!(BUTTON_CSS.contains(".rw-btn-destructive"));
        assert!(BUTTON_CSS.contains(".rw-btn-sm"));
        assert!(BUTTON_CSS.contains(".rw-btn-lg"));
        assert!(BUTTON_CSS.contains("@keyframes rw-spin"));
    }
}
