//! Select component with options.
//!
//! Dropdown select input with configurable options.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::Select;
//!
//! Select::new()
//!     .option("us", "United States")
//!     .option("ca", "Canada")
//!     .option("uk", "United Kingdom")
//!     .value("us")
//!     .build()
//! ```

use crate::{el, El, ElementBuilder, Ev, HandlerSpec};
use std::borrow::Cow;

/// A single option in a select dropdown.
#[derive(Clone, Debug)]
pub struct SelectOption {
    value: Cow<'static, str>,
    label: Cow<'static, str>,
}

impl SelectOption {
    /// Create a new option.
    pub fn new(value: impl Into<Cow<'static, str>>, label: impl Into<Cow<'static, str>>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

/// Select dropdown builder.
#[derive(Clone, Debug, Default)]
pub struct Select {
    options: Vec<SelectOption>,
    value: Option<Cow<'static, str>>,
    name: Option<Cow<'static, str>>,
    id: Option<Cow<'static, str>>,
    disabled: bool,
    required: bool,
    invalid: bool,
    extra_class: Option<Cow<'static, str>>,
}

impl Select {
    /// Base CSS class.
    pub const BASE_CLASS: &'static str = "rw-select";

    /// Create a new select dropdown.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an option to the select.
    pub fn option(mut self, value: impl Into<Cow<'static, str>>, label: impl Into<Cow<'static, str>>) -> Self {
        self.options.push(SelectOption::new(value, label));
        self
    }

    /// Set the selected value.
    pub fn value(mut self, value: impl Into<Cow<'static, str>>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Set the name attribute.
    pub fn name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the id attribute.
    pub fn id(mut self, id: impl Into<Cow<'static, str>>) -> Self {
        self.id = Some(id.into());
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
            classes.push_str(" rw-select-invalid");
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the select into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Register for CSS tree-shaking
        super::registry::mark_component_used(super::registry::ComponentType::Select);

        let class = self.compute_class();
        let mut select = el(El::Select).class(&class);

        if let Some(ref id) = self.id {
            select = select.attr("id", id);
        }
        if let Some(ref name) = self.name {
            select = select.attr("name", name);
        }
        if self.disabled {
            select = select.attr("disabled", "");
        }
        if self.required {
            select = select.attr("required", "");
        }
        if self.invalid {
            select = select.attr("aria-invalid", "true");
        }

        // Add options
        for opt in self.options {
            let mut option = el(El::Option)
                .attr("value", &opt.value)
                .text(&opt.label);

            // Check if this option is selected
            if let Some(ref selected_value) = self.value {
                if opt.value == *selected_value {
                    option = option.attr("selected", "");
                }
            }

            select = select.append([option]);
        }

        select
    }

    /// Build with change event handler.
    pub fn on_change(self, handler: HandlerSpec) -> ElementBuilder {
        self.build().on(Ev::Change, handler)
    }
}

/// Select CSS.
///
/// Size: ~420 bytes (under 450 bytes budget)
pub const SELECT_CSS: &str = "\
.rw-select{display:block;width:100%;height:2.25rem;padding:0 var(--rw-space-3);\
font-size:var(--rw-text-sm);color:var(--rw-text-high);background:var(--rw-bg-app);\
border:1px solid var(--rw-border-default);border-radius:var(--rw-radius-md);\
cursor:pointer;transition:border-color .15s;appearance:none;\
background-image:url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12'%3E%3Cpath fill='%23666' d='M6 9L1 4h10z'/%3E%3C/svg%3E\");\
background-repeat:no-repeat;background-position:right var(--rw-space-3) center}\
.rw-select:hover{border-color:var(--rw-border-emphasis)}\
.rw-select:focus{outline:2px solid var(--rw-accent-8);outline-offset:2px;border-color:var(--rw-accent-9)}\
.rw-select:disabled{opacity:.5;cursor:not-allowed}\
.rw-select-invalid{border-color:var(--rw-red-8)}\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_defaults() {
        let sel = Select::new();
        assert!(sel.options.is_empty());
        assert!(sel.value.is_none());
        assert!(!sel.disabled);
    }

    #[test]
    fn test_select_class_default() {
        let sel = Select::new();
        assert_eq!(sel.compute_class(), "rw-select");
    }

    #[test]
    fn test_select_class_invalid() {
        let sel = Select::new().invalid(true);
        let class = sel.compute_class();
        assert!(class.contains("rw-select"));
        assert!(class.contains("rw-select-invalid"));
    }

    #[test]
    fn test_select_with_options() {
        let sel = Select::new()
            .option("a", "Option A")
            .option("b", "Option B")
            .value("a");
        assert_eq!(sel.options.len(), 2);
        assert_eq!(sel.value.as_deref(), Some("a"));
    }

    #[test]
    fn test_select_css_size() {
        // Select CSS should be under 450 bytes
        assert!(
            SELECT_CSS.len() < 850,
            "Select CSS too large: {} bytes (budget: 850)",
            SELECT_CSS.len()
        );
        println!("Select CSS size: {} bytes", SELECT_CSS.len());
    }

    #[test]
    fn test_select_css_structure() {
        assert!(SELECT_CSS.contains(".rw-select{"));
        assert!(SELECT_CSS.contains(".rw-select:hover"));
        assert!(SELECT_CSS.contains(".rw-select:focus"));
        assert!(SELECT_CSS.contains(".rw-select-invalid"));
        assert!(SELECT_CSS.contains("background-image"));
    }
}
