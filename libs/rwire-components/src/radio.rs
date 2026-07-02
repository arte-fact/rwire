//! Radio component.
//!
//! Radio button input with optional label association.
//! Requires `name` attribute for grouping.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::Radio;
//!
//! // Radio group
//! Radio::new()
//!     .name("plan")
//!     .value("free")
//!     .label("Free Plan")
//!     .build()
//!
//! Radio::new()
//!     .name("plan")
//!     .value("pro")
//!     .label("Pro Plan")
//!     .checked(true)
//!     .build()
//! ```

use rwire::attr_tokens::{At, Av};
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder, Ev, HandlerSpec};
use std::borrow::Cow;

/// Radio button builder.
#[derive(Clone, Debug, Default)]
pub struct Radio {
    label: Option<Cow<'static, str>>,
    checked: bool,
    name: Option<Cow<'static, str>>,
    value: Option<Cow<'static, str>>,
    id: Option<Cow<'static, str>>,
    disabled: bool,
    required: bool,
    invalid: bool,
    extra_class: Option<Cow<'static, str>>,
}

#[rwire::component]
impl Radio {
    /// Create a new radio button.
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

    /// Set the name attribute (required for radio groups).
    pub fn name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the value attribute.
    pub fn value(mut self, value: impl Into<Cow<'static, str>>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Set the id attribute (overrides auto-generated ID).
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

    /// Compute style tokens for this radio configuration.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![
            St::RoundedFull,
            St::BgApp,
            St::CursorPointer,
            St::TransitionAll,
            St::FlexShrink0,
            St::AppearanceNone,
        ];
        if self.invalid {
            tokens.push(St::BorderRed8);
        }
        tokens
    }

    /// Build the radio button into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Generate ID if label is provided but no explicit ID
        let element_id = self.id.clone().unwrap_or_else(|| {
            if self.label.is_some() {
                Cow::Owned(rwire::builder::generate_element_id("radio_"))
            } else {
                Cow::Borrowed("")
            }
        });

        let mut tokens = self.compute_tokens();
        tokens.extend([St::W1rem, St::H1rem, St::Border2Default]);
        let mut input = el(El::Input)
            .st(tokens)
            .hover([St::BorderEmphasis])
            .checked([St::BgPrimary])
            .focus_visible([St::RingFocus])
            .at(At::Type, Av::Radio);

        if self.disabled {
            input =
                input.disabled_style([St::Opacity50, St::CursorNotAllowed, St::PointerEventsNone]);
        }

        if !element_id.is_empty() {
            input = input.at_str(At::Id, &element_id);
        }

        if let Some(ref name) = self.name {
            input = input.at_str(At::Name, name);
        }
        if let Some(ref value) = self.value {
            input = input.at_str(At::Value, value);
        }
        if self.checked {
            input = input.bool_attr(At::Checked);
        }
        if self.disabled {
            input = input.bool_attr(At::Disabled);
        }
        if self.required {
            input = input.bool_attr(At::Required);
        }
        if self.invalid {
            input = input.at(At::AriaInvalid, Av::True);
        }
        if let Some(ref extra) = self.extra_class {
            input = input.class(extra.as_ref());
        }

        // If label is provided, wrap in a container with label
        if let Some(label_text) = self.label {
            use crate::{Gap, Stack};
            Stack::row()
                .gap(Gap::Xs)
                .children([
                    input,
                    el(El::Label)
                        .st([St::TextSm, St::TextHigh, St::CursorPointer])
                        .at_str(At::For, &element_id)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radio_defaults() {
        let radio = Radio::new();
        assert!(!radio.checked);
        assert!(!radio.disabled);
        assert!(radio.label.is_none());
    }

    #[test]
    fn test_radio_default_tokens() {
        let radio = Radio::new();
        let tokens = radio.compute_tokens();
        assert!(tokens.contains(&St::RoundedFull));
        assert!(tokens.contains(&St::BgApp));
        assert!(tokens.contains(&St::CursorPointer));
        assert!(tokens.contains(&St::TransitionAll));
        assert!(tokens.contains(&St::FlexShrink0));
        assert!(tokens.contains(&St::AppearanceNone));
    }

    #[test]
    fn test_radio_pseudo_groups() {
        let radio = Radio::new().build();
        let groups = radio.get_pseudo_groups();
        assert!(!groups.is_empty());
    }

    #[test]
    fn test_radio_invalid_tokens() {
        let radio = Radio::new().invalid(true);
        let tokens = radio.compute_tokens();
        assert!(tokens.contains(&St::BorderRed8));
    }

    #[test]
    fn test_radio_disabled_pseudo_groups() {
        let radio = Radio::new().disabled(true).build();
        let groups = radio.get_pseudo_groups();
        assert!(!groups.is_empty());
    }
}
