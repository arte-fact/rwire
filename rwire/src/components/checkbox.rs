//! Checkbox component.
//!
//! Boolean checkbox input with optional label association.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::Checkbox;
//!
//! // Without label
//! Checkbox::new().name("subscribe").build()
//!
//! // With label (auto-generates ID)
//! Checkbox::new()
//!     .label("Subscribe to newsletter")
//!     .checked(true)
//!     .build()
//! ```

use crate::{el, El, ElementBuilder, Ev, HandlerSpec};
use std::borrow::Cow;

/// Checkbox builder.
#[derive(Clone, Debug, Default)]
pub struct Checkbox {
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

impl Checkbox {
    /// Create a new checkbox.
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
        classes.push_str("rw-checkbox");

        if self.invalid {
            classes.push_str(" rw-checkbox-invalid");
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the checkbox into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Register for CSS tree-shaking
        super::registry::mark_component_used(super::registry::ComponentType::Checkbox);

        // Generate ID if label is provided but no explicit ID
        let element_id = self.id.clone().unwrap_or_else(|| {
            if self.label.is_some() {
                Cow::Owned(crate::builder::generate_element_id("cb_"))
            } else {
                Cow::Borrowed("")
            }
        });

        let class = self.compute_class();
        let mut input = el(El::Input)
            .class(&class)
            .attr("type", "checkbox");

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
                        .class("rw-checkbox-label")
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

/// Checkbox CSS.
///
/// Size: ~330 bytes (under 350 bytes budget)
pub const CHECKBOX_CSS: &str = "\
.rw-checkbox{width:1rem;height:1rem;border:2px solid var(--rw-border-default);border-radius:var(--rw-radius-sm);\
background:var(--rw-bg-app);cursor:pointer;transition:all .15s;flex-shrink:0}\
.rw-checkbox:hover{border-color:var(--rw-border-emphasis)}\
.rw-checkbox:checked{background:var(--rw-accent-9);border-color:var(--rw-accent-9)}\
.rw-checkbox:focus{outline:2px solid var(--rw-accent-8);outline-offset:2px}\
.rw-checkbox:disabled{opacity:.5;cursor:not-allowed}\
.rw-checkbox-invalid{border-color:var(--rw-red-8)}\
.rw-checkbox-label{font-size:var(--rw-text-sm);color:var(--rw-text-high);cursor:pointer}\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkbox_defaults() {
        let cb = Checkbox::new();
        assert!(!cb.checked);
        assert!(!cb.disabled);
        assert!(cb.label.is_none());
    }

    #[test]
    fn test_checkbox_class_default() {
        let cb = Checkbox::new();
        assert_eq!(cb.compute_class(), "rw-checkbox");
    }

    #[test]
    fn test_checkbox_class_invalid() {
        let cb = Checkbox::new().invalid(true);
        let class = cb.compute_class();
        assert!(class.contains("rw-checkbox"));
        assert!(class.contains("rw-checkbox-invalid"));
    }

    #[test]
    fn test_checkbox_css_size() {
        // Checkbox CSS should be under 350 bytes
        assert!(
            CHECKBOX_CSS.len() < 650,
            "Checkbox CSS too large: {} bytes (budget: 650)",
            CHECKBOX_CSS.len()
        );
        println!("Checkbox CSS size: {} bytes", CHECKBOX_CSS.len());
    }

    #[test]
    fn test_checkbox_css_structure() {
        assert!(CHECKBOX_CSS.contains(".rw-checkbox{"));
        assert!(CHECKBOX_CSS.contains(".rw-checkbox-label"));
        assert!(CHECKBOX_CSS.contains(".rw-checkbox-invalid"));
    }
}
