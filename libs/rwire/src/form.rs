//! Form handling for rwire.
//!
//! Provides declarative form building with validation support.
//!
//! # Example
//!
//! ```ignore
//! use rwire::form::{Form, Field};
//!
//! fn login_form() -> ElementBuilder {
//!     Form::new()
//!         .action(handle_login())
//!         .append([
//!             Field::text("username").label("Username").required(),
//!             Field::password("password").label("Password").required(),
//!             Field::submit("Log In"),
//!         ])
//!         .build()
//! }
//! ```

use crate::builder::ElementBuilder;
use crate::protocol::{El, Ev};
use crate::state::HandlerSpec;

/// Validation rule for form fields.
#[derive(Clone, Debug)]
pub enum ValidationRule {
    /// Field is required (non-empty).
    Required,
    /// Minimum length for text fields.
    MinLength(usize),
    /// Maximum length for text fields.
    MaxLength(usize),
    /// Must match a regex pattern.
    Pattern(String),
    /// Must be a valid email address.
    Email,
    /// Must be a valid URL.
    Url,
    /// Numeric minimum value.
    Min(f64),
    /// Numeric maximum value.
    Max(f64),
    /// Custom validation with error message.
    Custom(String),
}

impl ValidationRule {
    /// Convert to a JSON-serializable format for client-side validation.
    pub fn to_json_value(&self) -> String {
        match self {
            ValidationRule::Required => r#"{"type":"required"}"#.to_string(),
            ValidationRule::MinLength(n) => format!(r#"{{"type":"minLength","value":{}}}"#, n),
            ValidationRule::MaxLength(n) => format!(r#"{{"type":"maxLength","value":{}}}"#, n),
            ValidationRule::Pattern(p) => format!(r#"{{"type":"pattern","value":"{}"}}"#, p),
            ValidationRule::Email => r#"{"type":"email"}"#.to_string(),
            ValidationRule::Url => r#"{"type":"url"}"#.to_string(),
            ValidationRule::Min(n) => format!(r#"{{"type":"min","value":{}}}"#, n),
            ValidationRule::Max(n) => format!(r#"{{"type":"max","value":{}}}"#, n),
            ValidationRule::Custom(msg) => format!(r#"{{"type":"custom","message":"{}"}}"#, msg),
        }
    }
}

/// Input field type.
#[derive(Clone, Copy, Debug, Default)]
pub enum FieldType {
    #[default]
    Text,
    Password,
    Email,
    Number,
    Tel,
    Url,
    Search,
    Hidden,
    Checkbox,
    Radio,
    File,
    Date,
    Time,
    DateTime,
    Color,
    Range,
}

impl FieldType {
    /// Get the HTML input type attribute value.
    pub fn as_str(&self) -> &'static str {
        match self {
            FieldType::Text => "text",
            FieldType::Password => "password",
            FieldType::Email => "email",
            FieldType::Number => "number",
            FieldType::Tel => "tel",
            FieldType::Url => "url",
            FieldType::Search => "search",
            FieldType::Hidden => "hidden",
            FieldType::Checkbox => "checkbox",
            FieldType::Radio => "radio",
            FieldType::File => "file",
            FieldType::Date => "date",
            FieldType::Time => "time",
            FieldType::DateTime => "datetime-local",
            FieldType::Color => "color",
            FieldType::Range => "range",
        }
    }
}

/// Builder for form fields.
#[derive(Clone, Debug)]
pub struct Field {
    name: String,
    field_type: FieldType,
    label_text: Option<String>,
    placeholder: Option<String>,
    default_value: Option<String>,
    validation_rules: Vec<ValidationRule>,
    class: Option<String>,
    is_submit: bool,
}

impl Field {
    /// Create a new text input field.
    pub fn text(name: &str) -> Self {
        Self::new(name, FieldType::Text)
    }

    /// Create a new password input field.
    pub fn password(name: &str) -> Self {
        Self::new(name, FieldType::Password)
    }

    /// Create a new email input field.
    pub fn email(name: &str) -> Self {
        Self::new(name, FieldType::Email)
    }

    /// Create a new number input field.
    pub fn number(name: &str) -> Self {
        Self::new(name, FieldType::Number)
    }

    /// Create a new hidden input field.
    pub fn hidden(name: &str, value: &str) -> Self {
        Self::new(name, FieldType::Hidden).value(value)
    }

    /// Create a new checkbox input field.
    pub fn checkbox(name: &str) -> Self {
        Self::new(name, FieldType::Checkbox)
    }

    /// Create a submit button.
    pub fn submit(text: &str) -> Self {
        Self {
            name: String::new(),
            field_type: FieldType::Text,
            label_text: None,
            placeholder: None,
            default_value: Some(text.to_string()),
            validation_rules: Vec::new(),
            class: None,
            is_submit: true,
        }
    }

    /// Create a new field with a specific type.
    pub fn new(name: &str, field_type: FieldType) -> Self {
        Self {
            name: name.to_string(),
            field_type,
            label_text: None,
            placeholder: None,
            default_value: None,
            validation_rules: Vec::new(),
            class: None,
            is_submit: false,
        }
    }

    /// Set the field label.
    pub fn label(mut self, text: &str) -> Self {
        self.label_text = Some(text.to_string());
        self
    }

    /// Set the placeholder text.
    pub fn placeholder(mut self, text: &str) -> Self {
        self.placeholder = Some(text.to_string());
        self
    }

    /// Set the default value.
    pub fn value(mut self, value: &str) -> Self {
        self.default_value = Some(value.to_string());
        self
    }

    /// Mark the field as required.
    pub fn required(mut self) -> Self {
        self.validation_rules.push(ValidationRule::Required);
        self
    }

    /// Set minimum length validation.
    pub fn min_length(mut self, len: usize) -> Self {
        self.validation_rules.push(ValidationRule::MinLength(len));
        self
    }

    /// Set maximum length validation.
    pub fn max_length(mut self, len: usize) -> Self {
        self.validation_rules.push(ValidationRule::MaxLength(len));
        self
    }

    /// Set pattern validation (regex).
    pub fn pattern(mut self, regex: &str) -> Self {
        self.validation_rules
            .push(ValidationRule::Pattern(regex.to_string()));
        self
    }

    /// Add a custom validation rule.
    pub fn validate(mut self, rule: ValidationRule) -> Self {
        self.validation_rules.push(rule);
        self
    }

    /// Set CSS class.
    pub fn class(mut self, class: &str) -> Self {
        self.class = Some(class.to_string());
        self
    }

    /// Build the field into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        use crate::builder::el;

        if self.is_submit {
            let text = self.default_value.unwrap_or_else(|| "Submit".to_string());
            let mut btn = el(El::Button).attr("type", "submit").text(&text);
            if let Some(class) = self.class {
                btn = btn.class(&class);
            }
            return btn;
        }

        let mut input = el(El::Input)
            .attr("type", self.field_type.as_str())
            .attr("name", &self.name);

        if let Some(placeholder) = self.placeholder {
            input = input.attr("placeholder", &placeholder);
        }

        if let Some(value) = self.default_value {
            input = input.attr("value", &value);
        }

        if let Some(class) = self.class {
            input = input.class(&class);
        }

        // Add required attribute if present
        if self
            .validation_rules
            .iter()
            .any(|r| matches!(r, ValidationRule::Required))
        {
            input = input.attr("required", "");
        }

        // Wrap with label if provided
        if let Some(label_text) = self.label_text {
            el(El::Label).append([el(El::Span).text(&label_text), input])
        } else {
            input
        }
    }
}

/// Builder for forms.
#[derive(Clone, Default)]
pub struct Form {
    action_handler: Option<HandlerSpec>,
    method: String,
    class: Option<String>,
    fields: Vec<Field>,
    children: Vec<ElementBuilder>,
}

impl Form {
    /// Create a new form builder.
    pub fn new() -> Self {
        Self {
            action_handler: None,
            method: "POST".to_string(),
            class: None,
            fields: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Set the form submission handler.
    pub fn action(mut self, handler: HandlerSpec) -> Self {
        self.action_handler = Some(handler);
        self
    }

    /// Set the form method (GET or POST).
    pub fn method(mut self, method: &str) -> Self {
        self.method = method.to_uppercase();
        self
    }

    /// Set CSS class.
    pub fn class(mut self, class: &str) -> Self {
        self.class = Some(class.to_string());
        self
    }

    /// Add fields to the form.
    pub fn fields<I>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = Field>,
    {
        self.fields.extend(fields);
        self
    }

    /// Add child elements to the form.
    pub fn append<I>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = ElementBuilder>,
    {
        self.children.extend(children);
        self
    }

    /// Build the form into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        use crate::builder::el;

        let mut form = el(El::Form);

        if let Some(class) = self.class {
            form = form.class(&class);
        }

        // Add submit handler
        if let Some(handler) = self.action_handler {
            form = form.on(Ev::Submit, handler);
        }

        // Add field elements
        let field_elements: Vec<ElementBuilder> =
            self.fields.into_iter().map(|f| f.build()).collect();

        form = form.append(field_elements);
        form = form.append(self.children);

        form
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_text() {
        let field = Field::text("username")
            .label("Username")
            .placeholder("Enter username")
            .required();

        assert_eq!(field.name, "username");
        assert!(matches!(field.field_type, FieldType::Text));
        assert_eq!(field.label_text, Some("Username".to_string()));
        assert!(field
            .validation_rules
            .iter()
            .any(|r| matches!(r, ValidationRule::Required)));
    }

    #[test]
    fn test_field_password() {
        let field = Field::password("pass").min_length(8).max_length(100);

        assert_eq!(field.name, "pass");
        assert!(matches!(field.field_type, FieldType::Password));
        assert_eq!(field.validation_rules.len(), 2);
    }

    #[test]
    fn test_field_submit() {
        let field = Field::submit("Log In");
        assert!(field.is_submit);
        assert_eq!(field.default_value, Some("Log In".to_string()));
    }

    #[test]
    fn test_validation_rule_json() {
        assert_eq!(
            ValidationRule::Required.to_json_value(),
            r#"{"type":"required"}"#
        );
        assert_eq!(
            ValidationRule::MinLength(5).to_json_value(),
            r#"{"type":"minLength","value":5}"#
        );
        assert_eq!(ValidationRule::Email.to_json_value(), r#"{"type":"email"}"#);
    }

    #[test]
    fn test_form_builder() {
        let form = Form::new().class("login-form").fields([
            Field::text("username").required(),
            Field::password("password").required(),
            Field::submit("Log In"),
        ]);

        assert_eq!(form.class, Some("login-form".to_string()));
        assert_eq!(form.fields.len(), 3);
    }
}
