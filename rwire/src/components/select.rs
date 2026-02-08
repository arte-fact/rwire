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

use crate::attr_tokens::{At, Av};
use crate::style_tokens::St;
use crate::{el, El, ElementBuilder, Ev, HandlerSpec};
use std::borrow::Cow;

/// Creates the dropdown arrow SVG for the select component.
fn select_arrow() -> ElementBuilder {
    el(El::Svg)
        .st([St::PositionAbsolute, St::PointerEventsNone, St::TextMuted])
        .at(At::Xmlns, Av::SvgNs)
        .at(At::Width, Av::V12)
        .at(At::Height, Av::V12)
        .at(At::ViewBox, Av::ViewBox12)
        .at(At::Fill, Av::CurrentColor)
        .attr("style", "right:var(--rw-space-3);top:50%;transform:translateY(-50%)")
        .append([
            el(El::Path).at_str(At::D, "M6 9L1 4h10z")
        ])
}

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

    /// Compute style tokens for this select configuration.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![
            St::DisplayBlock, St::WFull, St::TextSm, St::TextHigh,
            St::BgApp, St::BorderDefault, St::RoundedMd, St::CursorPointer,
            St::TransitionColors, St::AppearanceNone,
        ];
        if self.invalid {
            tokens.push(St::BorderRed8);
        }
        tokens
    }

    /// Build the select into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut tokens = self.compute_tokens();
        tokens.extend([St::H2_25rem, St::Py0, St::PxSp3]);
        let mut select = el(El::Select)
            .st(tokens)
            .hover([St::BorderEmphasis])
            .focus_visible([St::RingFocus]);
        if self.disabled {
            select = select.disabled_style([St::Opacity50, St::CursorNotAllowed, St::PointerEventsNone]);
        }

        if let Some(ref id) = self.id {
            select = select.at_str(At::Id, id);
        }
        if let Some(ref name) = self.name {
            select = select.at_str(At::Name, name);
        }
        if self.disabled {
            select = select.bool_attr(At::Disabled);
        }
        if self.required {
            select = select.bool_attr(At::Required);
        }
        if self.invalid {
            select = select.at(At::AriaInvalid, Av::True);
        }
        if let Some(ref extra) = self.extra_class {
            select = select.class(extra.as_ref());
        }

        // Add options
        for opt in self.options {
            let mut option = el(El::Option)
                .at_str(At::Value, &opt.value)
                .text(&opt.label);

            // Check if this option is selected
            if let Some(ref selected_value) = self.value {
                if opt.value == *selected_value {
                    option = option.bool_attr(At::Selected);
                }
            }

            select = select.append([option]);
        }

        // Wrap select with positioned arrow SVG
        el(El::Div)
            .st([St::PositionRelative, St::ItemsCenter, St::WFull])
            .append([select, select_arrow()])
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
    fn test_select_defaults() {
        let sel = Select::new();
        assert!(sel.options.is_empty());
        assert!(sel.value.is_none());
        assert!(!sel.disabled);
    }

    #[test]
    fn test_select_default_tokens() {
        let sel = Select::new();
        let tokens = sel.compute_tokens();
        assert!(tokens.contains(&St::DisplayBlock));
        assert!(tokens.contains(&St::WFull));
        assert!(tokens.contains(&St::TextSm));
        assert!(tokens.contains(&St::BgApp));
        assert!(tokens.contains(&St::BorderDefault));
        assert!(tokens.contains(&St::RoundedMd));
        assert!(tokens.contains(&St::CursorPointer));
        assert!(tokens.contains(&St::AppearanceNone));
    }

    #[test]
    fn test_select_builds_with_arrow() {
        // build() returns a wrapper div containing the select + SVG arrow
        let sel = Select::new()
            .option("a", "Option A")
            .build();
        drop(sel); // smoke test: builds without panic
    }

    #[test]
    fn test_select_invalid_tokens() {
        let sel = Select::new().invalid(true);
        let tokens = sel.compute_tokens();
        assert!(tokens.contains(&St::BorderRed8));
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

}
