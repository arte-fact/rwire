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

use crate::attr_tokens::{At, Av};
use crate::style_tokens::St;
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
    fn av(self) -> Av {
        match self {
            InputType::Text => Av::Text,
            InputType::Password => Av::Password,
            InputType::Email => Av::Email,
            InputType::Number => Av::Number,
            InputType::Search => Av::Search,
            InputType::Tel => Av::Tel,
            InputType::Url => Av::Url,
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

    // ========================================================================
    // Token computation
    // ========================================================================

    /// Compute style tokens for this input configuration.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![
            St::DisplayBlock,
            St::WFull,
            St::TextSm,
            St::LeadingNormal,
            St::TextHigh,
            St::BgApp,
            St::BorderDefault,
            St::RoundedMd,
            St::TransitionColors,
        ];

        match self.size {
            InputSize::Sm => {
                tokens.retain(|t| !matches!(t, St::TextSm));
                tokens.push(St::TextXs);
            }
            InputSize::Md => {}
            InputSize::Lg => {
                tokens.retain(|t| !matches!(t, St::TextSm));
                tokens.push(St::TextBase);
            }
        }

        if self.invalid {
            tokens.push(St::BorderRed8);
        }

        tokens
    }

    /// Compute size-specific style tokens.
    fn size_tokens(&self) -> Vec<St> {
        match self.size {
            InputSize::Sm => vec![St::H1_75rem, St::Py0, St::PxSm],
            InputSize::Md => vec![St::H2_25rem, St::Py0, St::PxSp3],
            InputSize::Lg => vec![St::H2_75rem, St::Py0, St::PxMd],
        }
    }

    // ========================================================================
    // Build
    // ========================================================================

    /// Build the input into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut tokens = self.compute_tokens();
        tokens.extend(self.size_tokens());
        let mut builder = el(El::Input)
            .st(tokens)
            .placeholder_style([St::TextMuted])
            .hover([St::BorderEmphasis])
            .focus([St::BorderPrimary, St::OutlineNone])
            .at(At::Type, self.input_type.av());
        if self.disabled {
            builder = builder.disabled_style([St::Opacity50, St::CursorNotAllowed, St::PointerEventsNone]);
        }

        if let Some(ref placeholder) = self.placeholder {
            builder = builder.at_str(At::Placeholder, placeholder);
        }
        if let Some(ref value) = self.value {
            builder = builder.at_str(At::Value, value);
        }
        if let Some(ref name) = self.name {
            builder = builder.at_str(At::Name, name);
        }
        if let Some(ref id) = self.id {
            builder = builder.at_str(At::Id, id);
        }
        if self.disabled {
            builder = builder.bool_attr(At::Disabled);
        }
        if self.readonly {
            builder = builder.bool_attr(At::Readonly);
        }
        if self.required {
            builder = builder.bool_attr(At::Required);
        }
        if self.invalid {
            builder = builder.at(At::AriaInvalid, Av::True);
        }
        if let Some(ref extra) = self.extra_class {
            builder = builder.class(extra.as_ref());
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
    fn test_input_default_tokens() {
        let input = Input::new();
        let tokens = input.compute_tokens();
        assert!(tokens.contains(&St::DisplayBlock));
        assert!(tokens.contains(&St::WFull));
        assert!(tokens.contains(&St::BgApp));
        assert!(tokens.contains(&St::BorderDefault));
    }

    #[test]
    fn test_input_small_tokens() {
        let input = Input::new().size(InputSize::Sm);
        let tokens = input.compute_tokens();
        assert!(tokens.contains(&St::TextXs));
        assert!(!tokens.contains(&St::TextSm));
    }

    #[test]
    fn test_input_invalid_tokens() {
        let input = Input::new().invalid(true);
        let tokens = input.compute_tokens();
        assert!(tokens.contains(&St::BorderRed8));
    }

    #[test]
    fn test_input_pseudo_groups() {
        let input = Input::new().build();
        let groups = input.get_pseudo_groups();
        assert!(!groups.is_empty());
    }
}
