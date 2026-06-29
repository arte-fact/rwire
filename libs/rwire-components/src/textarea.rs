//! Textarea component.
//!
//! Multi-line text input with size variants.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Textarea, InputSize};
//!
//! Textarea::new()
//!     .placeholder("Enter description")
//!     .rows(6)
//!     .build()
//! ```

use rwire::attr_tokens::{At, Av};
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder, Ev, HandlerSpec};
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
    autocomplete: Option<Cow<'static, str>>,
    spellcheck: Option<bool>,
    maxlength: Option<u32>,
    wrap: Option<Cow<'static, str>>,
    autofocus: bool,
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
            autocomplete: None,
            spellcheck: None,
            maxlength: None,
            wrap: None,
            autofocus: false,
            extra_class: None,
        }
    }
}

#[rwire::component]
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

    /// Set the `autocomplete` attribute (e.g. `"off"`).
    pub fn autocomplete(mut self, value: impl Into<Cow<'static, str>>) -> Self {
        self.autocomplete = Some(value.into());
        self
    }

    /// Set the `spellcheck` attribute.
    pub fn spellcheck(mut self, on: bool) -> Self {
        self.spellcheck = Some(on);
        self
    }

    /// Set the `maxlength` attribute.
    pub fn maxlength(mut self, max: u32) -> Self {
        self.maxlength = Some(max);
        self
    }

    /// Set the `wrap` attribute (`"soft"`, `"hard"`, or `"off"`).
    pub fn wrap(mut self, wrap: impl Into<Cow<'static, str>>) -> Self {
        self.wrap = Some(wrap.into());
        self
    }

    /// Autofocus the textarea on mount.
    pub fn autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = autofocus;
        self
    }

    // ========================================================================
    // Token computation
    // ========================================================================

    /// Compute style tokens for this textarea configuration.
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
            TextareaSize::Sm => {
                tokens.retain(|t| !matches!(t, St::TextSm));
                tokens.push(St::TextXs);
            }
            TextareaSize::Md => {}
            TextareaSize::Lg => {
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
            TextareaSize::Sm => vec![St::PSm, St::MinH4rem, St::ResizeY, St::FontInheritAll],
            TextareaSize::Md => vec![St::PSp3, St::MinH5rem, St::ResizeY, St::FontInheritAll],
            TextareaSize::Lg => vec![St::PMd, St::MinH6rem, St::ResizeY, St::FontInheritAll],
        }
    }

    // ========================================================================
    // Build
    // ========================================================================

    /// Build the textarea into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut tokens = self.compute_tokens();
        tokens.extend(self.size_tokens());
        let mut builder = el(El::Textarea)
            .st(tokens)
            .placeholder_style([St::TextMuted])
            .hover([St::BorderEmphasis])
            .focus([St::BorderPrimary, St::OutlineNone]);
        if self.disabled {
            builder = builder.disabled_style([
                St::Opacity50,
                St::CursorNotAllowed,
                St::PointerEventsNone,
            ]);
        }

        if let Some(ref placeholder) = self.placeholder {
            builder = builder.at_str(At::Placeholder, placeholder);
        }
        if let Some(ref value) = self.value {
            builder = builder.text(value);
        }
        if let Some(ref name) = self.name {
            builder = builder.at_str(At::Name, name);
        }
        if let Some(ref id) = self.id {
            builder = builder.at_str(At::Id, id);
        }
        if let Some(rows) = self.rows {
            builder = builder.at_str(At::Rows, &rows.to_string());
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
        if let Some(ref autocomplete) = self.autocomplete {
            builder = builder.at_str(At::Autocomplete, autocomplete);
        }
        if let Some(spellcheck) = self.spellcheck {
            builder = builder.at(
                At::Spellcheck,
                if spellcheck { Av::True } else { Av::False },
            );
        }
        if let Some(maxlength) = self.maxlength {
            builder = builder.at_str(At::Maxlength, &maxlength.to_string());
        }
        if let Some(ref wrap) = self.wrap {
            builder = builder.at_str(At::Wrap, wrap);
        }
        if self.autofocus {
            builder = builder.bool_attr(At::Autofocus);
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
    fn test_textarea_defaults() {
        let textarea = Textarea::new();
        assert_eq!(textarea.size, TextareaSize::Md);
        assert_eq!(textarea.rows, Some(4));
        assert!(!textarea.disabled);
    }

    #[test]
    fn test_textarea_default_tokens() {
        let textarea = Textarea::new();
        let tokens = textarea.compute_tokens();
        assert!(tokens.contains(&St::DisplayBlock));
        assert!(tokens.contains(&St::WFull));
        assert!(tokens.contains(&St::BgApp));
        assert!(tokens.contains(&St::BorderDefault));
    }

    #[test]
    fn test_textarea_small_tokens() {
        let textarea = Textarea::new().size(TextareaSize::Sm);
        let tokens = textarea.compute_tokens();
        assert!(tokens.contains(&St::TextXs));
        assert!(!tokens.contains(&St::TextSm));
    }

    #[test]
    fn test_textarea_large_tokens() {
        let textarea = Textarea::new().size(TextareaSize::Lg);
        let tokens = textarea.compute_tokens();
        assert!(tokens.contains(&St::TextBase));
        assert!(!tokens.contains(&St::TextSm));
    }

    #[test]
    fn test_textarea_invalid_tokens() {
        let textarea = Textarea::new().invalid(true);
        let tokens = textarea.compute_tokens();
        assert!(tokens.contains(&St::BorderRed8));
    }

    #[test]
    fn test_textarea_pseudo_groups() {
        let textarea = Textarea::new().build();
        let groups = textarea.get_pseudo_groups();
        assert!(!groups.is_empty());
    }
}
