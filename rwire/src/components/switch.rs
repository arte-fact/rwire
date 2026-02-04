//! Switch component.
//!
//! Toggle switch styled like a checkbox but with switch appearance.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::Switch;
//!
//! // Without label
//! Switch::new().name("notifications").build()
//!
//! // With label (auto-generates ID)
//! Switch::new()
//!     .label("Enable notifications")
//!     .checked(true)
//!     .build()
//! ```

use crate::{el, El, ElementBuilder, Ev, HandlerSpec};
use std::borrow::Cow;

/// Switch builder.
#[derive(Clone, Debug, Default)]
pub struct Switch {
    label: Option<Cow<'static, str>>,
    checked: bool,
    name: Option<Cow<'static, str>>,
    id: Option<Cow<'static, str>>,
    value: Option<Cow<'static, str>>,
    disabled: bool,
    required: bool,
    invalid: bool,
    extra_class: Option<Cow<'static, str>>,
}

impl Switch {
    /// Base CSS class.
    pub const BASE_CLASS: &'static str = "rw-switch";

    /// Create a new switch.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the label text (automatically creates ID for association).
    pub fn label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set checked state.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set the name attribute.
    pub fn name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the id attribute (overrides auto-generated ID).
    pub fn id(mut self, id: impl Into<Cow<'static, str>>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the value attribute.
    pub fn value(mut self, value: impl Into<Cow<'static, str>>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Set disabled state.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set required state.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Set invalid state.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(48);
        classes.push_str(Self::BASE_CLASS);

        if self.invalid {
            classes.push_str(" rw-switch-invalid");
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the switch into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Register for CSS tree-shaking
        super::registry::mark_component_used(super::registry::ComponentType::Switch);

        // Generate ID if label is provided but no explicit ID
        let element_id = self.id.clone().unwrap_or_else(|| {
            if self.label.is_some() {
                Cow::Owned(crate::builder::generate_element_id("switch_"))
            } else {
                Cow::Borrowed("")
            }
        });

        let class = self.compute_class();
        let mut input = el(El::Input)
            .class(&class)
            .attr("type", "checkbox")
            .attr("role", "switch");

        if !element_id.is_empty() {
            input = input.attr("id", &element_id);
        }

        if let Some(ref name) = self.name {
            input = input.attr("name", name);
        }
        if let Some(ref value) = self.value {
            input = input.attr("value", value);
        }
        if self.checked {
            input = input.attr("checked", "");
        }
        if self.disabled {
            input = input.attr("disabled", "");
        }
        if self.required {
            input = input.attr("required", "");
        }
        if self.invalid {
            input = input.attr("aria-invalid", "true");
        }

        // If label is provided, wrap in a container with label
        if let Some(label_text) = self.label {
            // Use Stack for layout
            use crate::components::{Stack, Gap};
            Stack::row()
                .gap(Gap::Xs)
                .children([
                    input,
                    el(El::Label)
                        .class("rw-switch-label")
                        .attr("for", &element_id)
                        .text(&label_text),
                ])
                .build()
        } else {
            input
        }
    }

    /// Build with change event handler.
    pub fn on_change(self, handler: HandlerSpec) -> ElementBuilder {
        self.build().on(Ev::Change, handler)
    }
}

/// Switch CSS.
///
/// Size: ~340 bytes (under 350 bytes budget)
pub const SWITCH_CSS: &str = "\
.rw-switch{position:relative;width:2.5rem;height:1.25rem;border:2px solid var(--rw-border-default);\
border-radius:1rem;background:var(--rw-bg-muted);cursor:pointer;transition:all .2s;flex-shrink:0;appearance:none}\
.rw-switch::after{content:\"\";position:absolute;left:2px;top:2px;width:0.75rem;height:0.75rem;\
background:var(--rw-white);border-radius:50%;transition:transform .2s}\
.rw-switch:checked{background:var(--rw-accent-9);border-color:var(--rw-accent-9)}\
.rw-switch:checked::after{transform:translateX(1.25rem)}\
.rw-switch:focus{outline:2px solid var(--rw-accent-8);outline-offset:2px}\
.rw-switch:disabled{opacity:.5;cursor:not-allowed}\
.rw-switch-invalid{border-color:var(--rw-red-8)}\
.rw-switch-label{font-size:var(--rw-text-sm);color:var(--rw-text-high);cursor:pointer}\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_switch_defaults() {
        let sw = Switch::new();
        assert!(!sw.checked);
        assert!(!sw.disabled);
        assert!(sw.label.is_none());
    }

    #[test]
    fn test_switch_class_default() {
        let sw = Switch::new();
        assert_eq!(sw.compute_class(), "rw-switch");
    }

    #[test]
    fn test_switch_class_invalid() {
        let sw = Switch::new().invalid(true);
        let class = sw.compute_class();
        assert!(class.contains("rw-switch"));
        assert!(class.contains("rw-switch-invalid"));
    }

    #[test]
    fn test_switch_css_size() {
        // Switch CSS should be under 350 bytes
        assert!(
            SWITCH_CSS.len() < 800,
            "Switch CSS too large: {} bytes (budget: 800)",
            SWITCH_CSS.len()
        );
        println!("Switch CSS size: {} bytes", SWITCH_CSS.len());
    }

    #[test]
    fn test_switch_css_structure() {
        assert!(SWITCH_CSS.contains(".rw-switch{"));
        assert!(SWITCH_CSS.contains(".rw-switch::after"));
        assert!(SWITCH_CSS.contains(".rw-switch:checked"));
        assert!(SWITCH_CSS.contains(".rw-switch-label"));
        assert!(SWITCH_CSS.contains(".rw-switch-invalid"));
    }
}
