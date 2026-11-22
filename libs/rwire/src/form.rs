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

use crate::actions::reflect_action;
use crate::builder::ElementBuilder;
use crate::protocol::{El, Ev};
use crate::state::HandlerSpec;
use crate::style_tokens::St;

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

/// Field type: an `<input type=…>`, or a multi-line `<textarea>`.
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
    /// A `<textarea>` (multi-line text), not an `<input>`.
    Textarea,
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
            FieldType::Textarea => "textarea",
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
    id: Option<String>,
    rows: Option<u32>,
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

    /// Create a multi-line text field (`<textarea>`). Submitted with the form
    /// under `name`, like any field.
    ///
    /// It gets a stable `id` (`field-<name>` unless [`Field::id`] says
    /// otherwise): a re-render matches the live element by that id, so what the
    /// user typed stays in it, even when fields before it appear or disappear.
    /// To empty it after a submit, give it a new id (e.g. with a counter).
    /// With a [`Field::label`], the wrapping `<label>` gets `<id>-label`, so the
    /// wrapper is matched by id as well.
    ///
    /// Validation: `required`, `min_length` and `max_length` become the native
    /// constraints; a pattern does not apply to a textarea.
    pub fn textarea(name: &str) -> Self {
        Self::new(name, FieldType::Textarea)
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
            id: None,
            rows: None,
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
            id: None,
            rows: None,
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

    /// Set the element's `id`. A stable id keeps the live element (and what
    /// the user typed in it) across re-renders.
    pub fn id(mut self, id: &str) -> Self {
        self.id = Some(id.to_string());
        self
    }

    /// Visible lines of a textarea (default 4). Ignored on other fields.
    pub fn rows(mut self, rows: u32) -> Self {
        self.rows = Some(rows);
        self
    }

    /// The native constraint attributes of the validation rules that HTML
    /// knows for this kind of control (the browser checks them on submit).
    fn constraints(&self) -> Vec<(&'static str, String)> {
        let textarea = matches!(self.field_type, FieldType::Textarea);
        // `Min`/`Max` hold a number: emitted only where the native constraint takes one.
        // A date or time `min` wants a date string, not a float, so they are left out.
        let numeric = matches!(self.field_type, FieldType::Number | FieldType::Range);
        let mut out = Vec::new();
        for rule in &self.validation_rules {
            match rule {
                ValidationRule::Required => out.push(("required", String::new())),
                ValidationRule::MinLength(n) => out.push(("minlength", n.to_string())),
                ValidationRule::MaxLength(n) => out.push(("maxlength", n.to_string())),
                ValidationRule::Pattern(p) if !textarea => out.push(("pattern", p.clone())),
                ValidationRule::Min(n) if numeric => out.push(("min", n.to_string())),
                ValidationRule::Max(n) if numeric => out.push(("max", n.to_string())),
                _ => {}
            }
        }
        out
    }

    /// Build the field into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        use crate::builder::el;

        if self.is_submit {
            let text = self.default_value.unwrap_or_else(|| "Submit".to_string());
            return submit_button(&text, self.class.as_deref(), SubmitPhase::Idle);
        }

        let constraints = self.constraints();
        // The field's effective id: a textarea always has one, an input only when asked.
        let field_id = match self.field_type {
            FieldType::Textarea => Some(
                self.id
                    .clone()
                    .unwrap_or_else(|| format!("field-{}", self.name)),
            ),
            _ => self.id.clone(),
        };
        let mut input = if let (FieldType::Textarea, Some(id)) = (&self.field_type, &field_id) {
            let rows = self.rows.unwrap_or(4).to_string();
            let mut area = el(El::Textarea)
                .attr("name", &self.name)
                .attr("id", id)
                .attr("rows", &rows)
                .st([St::ResizeY]);
            // A textarea's initial value is its text, not a `value` attribute.
            if let Some(value) = &self.default_value {
                area = area.text(value);
            }
            area
        } else {
            let mut input = el(El::Input)
                .attr("type", self.field_type.as_str())
                .attr("name", &self.name);
            if let Some(id) = &self.id {
                input = input.attr("id", id);
            }
            if let Some(value) = &self.default_value {
                input = input.attr("value", value);
            }
            input
        }
        // Sensible themed defaults so a form looks at home in the app without per-field
        // styling; `.class()` still layers on top as an escape hatch.
        .st([
            St::WFull,
            St::PySm,
            St::PxMd,
            St::BgApp,
            St::BorderDefault,
            St::RoundedMd,
            St::TextHigh,
            St::TextSm,
        ])
        .focus([St::BorderAccent]);

        if let Some(placeholder) = self.placeholder {
            input = input.attr("placeholder", &placeholder);
        }

        if let Some(class) = self.class {
            input = input.class(&class);
        }

        for (name, value) in &constraints {
            input = input.attr(name, value);
        }

        // Wrap with label if provided. The label gets a stable id derived from the
        // field's (`<id>-label`): a re-render matches the wrapper by id rather than by
        // position, so the field inside keeps what the user typed even when a field
        // before it appears or disappears.
        if let Some(label_text) = self.label_text {
            let mut label = el(El::Label);
            if let Some(id) = &field_id {
                label = label.attr("id", &format!("{id}-label"));
            }
            label.st([St::DisplayFlex, St::FlexCol, St::GapXs]).append([
                el(El::Span)
                    .st([St::TextSm, St::TextMuted])
                    .text(&label_text),
                input,
            ])
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
    /// When set, the submit button skips action reflection. Inverted so the derived `Default`
    /// (false) keeps reflection **on** — like `Button`/`Composer`.
    reflect_off: bool,
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
            reflect_off: false,
        }
    }

    /// Opt the submit button out of automatic action reflection (on by default): when the
    /// `action` handler defers, the submit button spins in place while the work runs and turns
    /// destructive with the message on a native tooltip if it fails, staying a submit so it
    /// retries. Same contract as `Button`.
    pub fn reflect(mut self, enabled: bool) -> Self {
        self.reflect_off = !enabled;
        self
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

        // Base layout so stacked fields breathe (a bare <form> block-stacks them flush).
        let mut form = el(El::Form).st([St::DisplayFlex, St::FlexCol, St::GapMd]);

        if let Some(class) = self.class {
            form = form.class(&class);
        }

        // The submit action's identity, captured before the handler is moved onto the form, so
        // the submit button can reflect it (unless reflection is off or there is no handler).
        let submit_reflect: Option<(u32, Vec<u8>)> = if self.reflect_off {
            None
        } else {
            self.action_handler
                .as_ref()
                .map(|h| (h.handler_id, h.param_bytes.clone().unwrap_or_default()))
        };

        // Add submit handler
        if let Some(handler) = self.action_handler {
            form = form.on(Ev::Submit, handler);
        }

        // Add field elements — a submit field becomes action-aware when there's an action to bind.
        let field_elements: Vec<ElementBuilder> = self
            .fields
            .into_iter()
            .map(|f| match (&submit_reflect, f.is_submit) {
                (Some((handler_id, param)), true) => {
                    let (handler_id, param) = (*handler_id, param.clone());
                    let text = f
                        .default_value
                        .clone()
                        .unwrap_or_else(|| "Submit".to_string());
                    let class = f.class.clone();
                    let (t_idle, t_load, t_err) = (text.clone(), text.clone(), text);
                    let (c_idle, c_load, c_err) = (class.clone(), class.clone(), class);
                    reflect_action(
                        handler_id,
                        param,
                        move || submit_button(&t_idle, c_idle.as_deref(), SubmitPhase::Idle),
                        move || submit_button(&t_load, c_load.as_deref(), SubmitPhase::Loading),
                        move |message| {
                            submit_button(&t_err, c_err.as_deref(), SubmitPhase::Errored)
                                .attr("title", message)
                        },
                    )
                }
                _ => f.build(),
            })
            .collect();

        form = form.append(field_elements);
        form = form.append(self.children);

        form
    }
}

/// The reflected phases of a form submit button.
enum SubmitPhase {
    Idle,
    Loading,
    Errored,
}

/// Render a submit button in one phase (rwire primitives only — `Form` sits below the component
/// library). `Loading` hides the label behind an in-place spinner ring and blocks pointer events;
/// `Errored` recolours destructive and stays a submit, so a click retries (the caller adds the
/// message as a native `title` tooltip).
fn submit_button(text: &str, class: Option<&str>, phase: SubmitPhase) -> ElementBuilder {
    use crate::builder::el;
    let mut btn = el(El::Button).attr("type", "submit").st([
        // Themed defaults so a bare form submit looks like a real button, matching `Button`. A
        // fixed height + flex centring (not text-driven padding) keeps the footprint constant
        // across phases and centres the label / spinner.
        St::PxLg,
        St::Py0,
        St::H2_25rem,
        St::DisplayInlineFlex,
        St::ItemsCenter,
        St::JustifyCenter,
        St::RoundedMd,
        St::BorderNone,
        St::CursorPointer,
        St::FontMedium,
        St::TextSm,
    ]);
    if let Some(class) = class {
        btn = btn.class(class);
    }
    // Each phase sets its own background, so the phases never fight over the cascade.
    match phase {
        SubmitPhase::Idle => btn
            .st([St::BgPrimary, St::TextOnPrimary])
            .hover([St::BgPrimaryHover])
            .text(text),
        SubmitPhase::Loading => btn
            .st([
                St::BgPrimary,
                St::TextOnPrimary,
                St::PositionRelative,
                St::PointerEventsNone,
                St::Opacity50,
            ])
            .after([
                St::ContentEmpty,
                St::PositionAbsolute,
                // inset:0 + margin:auto centres the fixed-size ring on both axes.
                St::Inset0,
                St::MAuto,
                St::W1rem,
                St::H1rem,
                // Ring tinted from the button's foreground (currentColor stays live because the
                // label is only hidden, not recoloured); colour + spin gap bundled in one token.
                St::Border2,
                St::SpinnerRingCurrent,
                St::RoundedFull,
                St::AnimateSpinFast,
            ])
            // Keep the label's box (constant footprint) but invisible behind the ring.
            .append([el(El::Span).st([St::Opacity0]).text(text)]),
        SubmitPhase::Errored => btn
            .st([St::BgDestructive, St::TextOnDestructive])
            .text(text),
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
    #[test]
    fn textarea_renders_a_named_textarea_with_a_stable_id() {
        let html = Field::textarea("bio")
            .placeholder("About you")
            .value("Hello <world>")
            .build()
            .to_static_html();
        assert!(html.starts_with("<textarea"), "{html}");
        assert!(html.contains("name=\"bio\""), "{html}");
        assert!(html.contains("id=\"field-bio\""), "{html}");
        assert!(html.contains("rows=\"4\""), "{html}");
        assert!(html.contains("placeholder=\"About you\""), "{html}");
        // The initial value is the text, escaped; no `type`, no `value` attribute.
        assert!(html.contains(">Hello &lt;world&gt;</textarea>"), "{html}");
        assert!(!html.contains("type="), "{html}");
        assert!(!html.contains("value="), "{html}");
        // The same field renders the same id every time: the morph keys on it.
        assert_eq!(
            html,
            Field::textarea("bio")
                .placeholder("About you")
                .value("Hello <world>")
                .build()
                .to_static_html()
        );
    }

    #[test]
    fn textarea_id_and_rows_can_be_set() {
        let html = Field::textarea("note")
            .id("note-3")
            .rows(8)
            .build()
            .to_static_html();
        assert!(html.contains("id=\"note-3\""), "{html}");
        assert!(html.contains("rows=\"8\""), "{html}");
        assert!(!html.contains("field-note"), "{html}");
    }

    #[test]
    fn textarea_validation_becomes_native_constraints() {
        let html = Field::textarea("bio")
            .required()
            .min_length(10)
            .max_length(500)
            .pattern("[a-z]+")
            .build()
            .to_static_html();
        assert!(html.contains("required"), "{html}");
        assert!(html.contains("minlength=\"10\""), "{html}");
        assert!(html.contains("maxlength=\"500\""), "{html}");
        assert!(
            !html.contains("pattern="),
            "no pattern on a textarea: {html}"
        );
    }

    #[test]
    fn input_validation_rules_reach_the_markup() {
        let html = Field::password("pass")
            .min_length(8)
            .max_length(100)
            .pattern("[A-Za-z0-9]+")
            .build()
            .to_static_html();
        assert!(html.contains("minlength=\"8\""), "{html}");
        assert!(html.contains("maxlength=\"100\""), "{html}");
        assert!(html.contains("pattern=\"[A-Za-z0-9]+\""), "{html}");
        let html = Field::number("age")
            .validate(ValidationRule::Min(0.0))
            .validate(ValidationRule::Max(130.0))
            .build()
            .to_static_html();
        assert!(html.contains("min=\"0\""), "{html}");
        assert!(html.contains("max=\"130\""), "{html}");
        // Min/max mean nothing on a text input: not emitted.
        let html = Field::text("x")
            .validate(ValidationRule::Min(1.0))
            .build()
            .to_static_html();
        assert!(!html.contains("min="), "{html}");
        // Nor on a date: its `min` is a date string, not a number.
        let html = Field::new("day", FieldType::Date)
            .validate(ValidationRule::Min(1.0))
            .validate(ValidationRule::Max(2.0))
            .build()
            .to_static_html();
        assert!(!html.contains("min=") && !html.contains("max="), "{html}");
        let html = Field::new("vol", FieldType::Range)
            .validate(ValidationRule::Max(11.0))
            .build()
            .to_static_html();
        assert!(html.contains("max=\"11\""), "{html}");
        // Inputs get an id only when asked (unchanged default).
        assert!(!html.contains("id="), "{html}");
        assert!(Field::text("x")
            .id("x-1")
            .build()
            .to_static_html()
            .contains("id=\"x-1\""));
    }

    #[test]
    fn a_form_submits_its_textarea_with_a_label() {
        let html = Form::new()
            .fields([
                Field::text("title").label("Title"),
                Field::textarea("body").label("Body").required(),
                Field::submit("Post"),
            ])
            .build()
            .to_static_html();
        let label = html.find("<span").unwrap();
        let area = html.find("<textarea").unwrap();
        assert!(label < area, "{html}");
        assert!(html.contains("name=\"body\""), "{html}");
        assert!(html.contains("<button"), "{html}");
    }

    /// A labelled field's wrapper carries a stable id too: the morph matches the
    /// `<label>` by id, so a textarea inside it keeps what the user typed when a
    /// field before it disappears (without one, the label is matched by position
    /// and the typed text lands in the wrong field or is lost).
    #[test]
    fn a_labelled_field_with_an_id_gets_a_stable_label_id() {
        let render = |with_title: bool| {
            let mut fields = Vec::new();
            if with_title {
                fields.push(Field::text("title").label("Title"));
            }
            fields.push(Field::textarea("body").label("Body"));
            fields.push(Field::email("mail").id("mail-1").label("Mail"));
            Form::new().fields(fields).build().to_static_html()
        };
        // The opening tag of the element carrying `id="<id>"`.
        let tag_of = |html: &str, id: &str| {
            let at = html.find(&format!("id=\"{id}\"")).expect(html);
            html[..at].rsplit('<').next().unwrap().to_string()
        };
        for html in [render(true), render(false)] {
            assert!(
                tag_of(&html, "field-body-label").starts_with("label "),
                "{html}"
            );
            assert!(
                tag_of(&html, "mail-1-label").starts_with("label "),
                "{html}"
            );
            let label = html.find("id=\"field-body-label\"").unwrap();
            assert!(label < html.find("<textarea").unwrap(), "{html}");
        }
        // A field without an id keeps a plain label (inputs have no id by default).
        assert_eq!(
            render(true).matches("-label\"").count(),
            2,
            "{}",
            render(true)
        );
    }
}
