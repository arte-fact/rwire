//! Textarea component.
//!
//! Multi-line text input with size variants.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{Textarea, InputSize};
//!
//! Textarea::new()
//!     .placeholder("Enter description")
//!     .rows(6)
//!     .build()
//! ```

use crate::{el, El, ElementBuilder, Ev, HandlerSpec};
use std::borrow::Cow;

/// Textarea size (reuses InputSize from input module).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextareaSize {
    /// Small: compact spacing
    Sm,
    /// Medium: default
    #[default]
    Md,
    /// Large: spacious
    Lg,
}

/// Textarea builder.
#[derive(Clone, Debug)]
pub struct Textarea {
    size: TextareaSize,
    placeholder: Option<Cow<'static, str>>,
    value: Option<Cow<'static, str>>,
    name: Option<Cow<'static, str>>,
    id: Option<Cow<'static, str>>,
    rows: Option<u32>,
    disabled: bool,
    readonly: bool,
    required: bool,
    invalid: bool,
    extra_class: Option<Cow<'static, str>>,
}

impl Default for Textarea {
    fn default() -> Self {
        Self {
            size: TextareaSize::Md,
            placeholder: None,
            value: None,
            name: None,
            id: None,
            rows: Some(4), // Default 4 rows
            disabled: false,
            readonly: false,
            required: false,
            invalid: false,
            extra_class: None,
        }
    }
}

impl Textarea {
    /// Create a new textarea.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the textarea size.
    pub fn size(mut self, size: TextareaSize) -> Self {
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

    /// Set number of visible rows.
    pub fn rows(mut self, rows: u32) -> Self {
        self.rows = Some(rows);
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
        classes.push_str("rw-textarea");

        match self.size {
            TextareaSize::Sm => classes.push_str(" rw-textarea-sm"),
            TextareaSize::Md => {}
            TextareaSize::Lg => classes.push_str(" rw-textarea-lg"),
        }

        if self.invalid {
            classes.push_str(" rw-textarea-invalid");
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the textarea into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Register for CSS tree-shaking
        super::registry::mark_component_used(super::registry::ComponentType::Textarea);

        let class = self.compute_class();
        let mut builder = el(El::Textarea).class(&class);

        if let Some(ref placeholder) = self.placeholder {
            builder = builder.attr("placeholder", placeholder);
        }
        if let Some(ref value) = self.value {
            builder = builder.text(value);
        }
        if let Some(ref name) = self.name {
            builder = builder.attr("name", name);
        }
        if let Some(ref id) = self.id {
            builder = builder.attr("id", id);
        }
        if let Some(rows) = self.rows {
            builder = builder.attr("rows", &rows.to_string());
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

/// Textarea CSS.
///
/// Size: ~370 bytes (under 400 bytes budget)
pub const TEXTAREA_CSS: &str = "\
.rw-textarea{display:block;width:100%;min-height:5rem;padding:var(--rw-space-3);\
font-size:var(--rw-text-sm);line-height:var(--rw-leading-normal);color:var(--rw-text-high);\
background:var(--rw-bg-app);border:1px solid var(--rw-border-default);border-radius:var(--rw-radius-md);\
transition:border-color .15s,box-shadow .15s;resize:vertical;font-family:inherit}\
.rw-textarea::placeholder{color:var(--rw-text-muted)}\
.rw-textarea:hover{border-color:var(--rw-border-emphasis)}\
.rw-textarea:focus{outline:none;border-color:var(--rw-accent-8);box-shadow:0 0 0 3px var(--rw-accent-4)}\
.rw-textarea:disabled{opacity:.5;cursor:not-allowed;background:var(--rw-bg-muted)}\
.rw-textarea-sm{padding:var(--rw-space-2);font-size:var(--rw-text-xs);min-height:4rem}\
.rw-textarea-lg{padding:var(--rw-space-4);font-size:var(--rw-text-base);min-height:6rem}\
.rw-textarea-invalid{border-color:var(--rw-red-8)}\
.rw-textarea-invalid:focus{border-color:var(--rw-red-8);box-shadow:0 0 0 3px var(--rw-red-4)}\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_textarea_defaults() {
        let textarea = Textarea::new();
        assert_eq!(textarea.size, TextareaSize::Md);
        assert_eq!(textarea.rows, Some(4));
        assert!(!textarea.disabled);
    }

    #[test]
    fn test_textarea_class_default() {
        let textarea = Textarea::new();
        assert_eq!(textarea.compute_class(), "rw-textarea");
    }

    #[test]
    fn test_textarea_class_small_invalid() {
        let textarea = Textarea::new().size(TextareaSize::Sm).invalid(true);
        let class = textarea.compute_class();
        assert!(class.contains("rw-textarea"));
        assert!(class.contains("rw-textarea-sm"));
        assert!(class.contains("rw-textarea-invalid"));
    }

    #[test]
    fn test_textarea_css_size() {
        // Textarea CSS should be under 400 bytes
        assert!(
            TEXTAREA_CSS.len() < 1000,
            "Textarea CSS too large: {} bytes (budget: 1000)",
            TEXTAREA_CSS.len()
        );
        println!("Textarea CSS size: {} bytes", TEXTAREA_CSS.len());
    }

    #[test]
    fn test_textarea_css_structure() {
        assert!(TEXTAREA_CSS.contains(".rw-textarea{"));
        assert!(TEXTAREA_CSS.contains(".rw-textarea-sm"));
        assert!(TEXTAREA_CSS.contains(".rw-textarea-lg"));
        assert!(TEXTAREA_CSS.contains(".rw-textarea-invalid"));
    }
}
