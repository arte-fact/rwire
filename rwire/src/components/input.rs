//! Input component.
//!
//! Text input with size variants and validation states.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{Input, InputSize};
//!
//! Input::text()
//!     .placeholder("Enter your name")
//!     .name("username")
//!     .build()
//!
//! Input::password()
//!     .placeholder("Password")
//!     .required(true)
//!     .build()
//! ```

use crate::{el, El, ElementBuilder, Ev, HandlerSpec};
use std::borrow::Cow;

/// Input type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputType {
    #[default]
    Text,
    Password,
    Email,
    Number,
    Search,
    Tel,
    Url,
}

impl InputType {
    fn as_str(&self) -> &'static str {
        match self {
            InputType::Text => "text",
            InputType::Password => "password",
            InputType::Email => "email",
            InputType::Number => "number",
            InputType::Search => "search",
            InputType::Tel => "tel",
            InputType::Url => "url",
        }
    }
}

/// Input size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputSize {
    /// Small: 28px height
    Sm,
    /// Medium: 36px height (default)
    #[default]
    Md,
    /// Large: 44px height
    Lg,
}

/// Input builder.
#[derive(Clone, Debug, Default)]
pub struct Input {
    input_type: InputType,
    size: InputSize,
    placeholder: Option<Cow<'static, str>>,
    value: Option<Cow<'static, str>>,
    name: Option<Cow<'static, str>>,
    id: Option<Cow<'static, str>>,
    disabled: bool,
    readonly: bool,
    required: bool,
    invalid: bool,
    extra_class: Option<Cow<'static, str>>,
}

impl Input {
    /// Create a new text input.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a text input.
    pub fn text() -> Self {
        Self::new()
    }

    /// Create a password input.
    pub fn password() -> Self {
        Self::new().input_type(InputType::Password)
    }

    /// Create an email input.
    pub fn email() -> Self {
        Self::new().input_type(InputType::Email)
    }

    /// Create a number input.
    pub fn number() -> Self {
        Self::new().input_type(InputType::Number)
    }

    /// Create a search input.
    pub fn search() -> Self {
        Self::new().input_type(InputType::Search)
    }

    /// Set the input type.
    pub fn input_type(mut self, input_type: InputType) -> Self {
        self.input_type = input_type;
        self
    }

    /// Set the input size.
    pub fn size(mut self, size: InputSize) -> Self {
        self.size = size;
        self
    }

    /// Set placeholder text.
    pub fn placeholder(mut self, placeholder: impl Into<Cow<'static, str>>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set initial value.
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

    /// Set readonly state.
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
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
        classes.push_str("rw-input");

        match self.size {
            InputSize::Sm => classes.push_str(" rw-input-sm"),
            InputSize::Md => {}
            InputSize::Lg => classes.push_str(" rw-input-lg"),
        }

        if self.invalid {
            classes.push_str(" rw-input-invalid");
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the input into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Register for CSS tree-shaking
        super::registry::mark_component_used(super::registry::ComponentType::Input);

        let class = self.compute_class();
        let mut builder = el(El::Input)
            .class(&class)
            .attr("type", self.input_type.as_str());

        if let Some(ref placeholder) = self.placeholder {
            builder = builder.attr("placeholder", placeholder);
        }
        if let Some(ref value) = self.value {
            builder = builder.attr("value", value);
        }
        if let Some(ref name) = self.name {
            builder = builder.attr("name", name);
        }
        if let Some(ref id) = self.id {
            builder = builder.attr("id", id);
        }
        if self.disabled {
            builder = builder.attr("disabled", "");
        }
        if self.readonly {
            builder = builder.attr("readonly", "");
        }
        if self.required {
            builder = builder.attr("required", "");
        }
        if self.invalid {
            builder = builder.attr("aria-invalid", "true");
        }

        builder
    }

    /// Build with input event handler.
    pub fn on_input(self, handler: HandlerSpec) -> ElementBuilder {
        self.build().on(Ev::Input, handler)
    }

    /// Build with change event handler.
    pub fn on_change(self, handler: HandlerSpec) -> ElementBuilder {
        self.build().on(Ev::Change, handler)
    }
}

/// Input CSS.
pub const INPUT_CSS: &str = "\
.rw-input{display:block;width:100%;height:2.25rem;padding:0 var(--rw-space-3);\
font-size:var(--rw-text-sm);line-height:var(--rw-leading-normal);color:var(--rw-text-high);\
background:var(--rw-bg-app);border:1px solid var(--rw-border-default);border-radius:var(--rw-radius-md);\
transition:border-color .15s,box-shadow .15s}\
.rw-input::placeholder{color:var(--rw-text-muted)}\
.rw-input:hover{border-color:var(--rw-border-emphasis)}\
.rw-input:focus{outline:none;border-color:var(--rw-accent-8);box-shadow:0 0 0 3px var(--rw-accent-4)}\
.rw-input:disabled{opacity:.5;cursor:not-allowed;background:var(--rw-bg-muted)}\
.rw-input-sm{height:1.75rem;padding:0 var(--rw-space-2);font-size:var(--rw-text-xs)}\
.rw-input-lg{height:2.75rem;padding:0 var(--rw-space-4);font-size:var(--rw-text-base)}\
.rw-input-invalid{border-color:var(--rw-red-8)}\
.rw-input-invalid:focus{border-color:var(--rw-red-8);box-shadow:0 0 0 3px var(--rw-red-4)}\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_defaults() {
        let input = Input::new();
        assert_eq!(input.input_type, InputType::Text);
        assert_eq!(input.size, InputSize::Md);
    }

    #[test]
    fn test_input_class_default() {
        let input = Input::new();
        assert_eq!(input.compute_class(), "rw-input");
    }

    #[test]
    fn test_input_class_small_invalid() {
        let input = Input::new().size(InputSize::Sm).invalid(true);
        let class = input.compute_class();
        assert!(class.contains("rw-input"));
        assert!(class.contains("rw-input-sm"));
        assert!(class.contains("rw-input-invalid"));
    }

    #[test]
    fn test_input_css_size() {
        assert!(INPUT_CSS.len() < 1000, "Input CSS too large: {} bytes", INPUT_CSS.len());
        println!("Input CSS size: {} bytes", INPUT_CSS.len());
    }
}
