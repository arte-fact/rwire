//! EmptyState component.
//!
//! Placeholder for empty lists, search results, or data views.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::EmptyState;
//!
//! EmptyState::new()
//!     .title("No results found")
//!     .description("Try adjusting your search terms.")
//!     .action(Button::primary("Clear filters").on_click(clear()))
//!     .build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// EmptyState builder.
#[derive(Clone, Default)]
pub struct EmptyState {
    title: Option<Cow<'static, str>>,
    description: Option<Cow<'static, str>>,
    icon: Option<ElementBuilder>,
    action: Option<ElementBuilder>,
    extra_class: Option<Cow<'static, str>>,
}

#[rwire::component]
impl EmptyState {
    /// Create a new empty state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the title.
    pub fn title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set a custom icon element.
    pub fn icon(mut self, icon: ElementBuilder) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set an action element (button, link, etc.).
    pub fn action(mut self, action: ElementBuilder) -> Self {
        self.action = Some(action);
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the container.
    pub fn compute_tokens() -> Vec<St> {
        vec![
            St::DisplayFlex,
            St::FlexCol,
            St::ItemsCenter,
            St::JustifyCenter,
            St::Py2xl,
            St::PxMd,
            St::TextCenter,
            St::GapMd,
        ]
    }

    /// Build the empty state into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut container = el(El::Div).st(Self::compute_tokens());

        if let Some(ref extra) = self.extra_class {
            container = container.class(extra.as_ref());
        }

        if let Some(icon) = self.icon {
            container = container.append([
                el(El::Div)
                    .st([St::TextMuted, St::MbSm])
                    .append([icon]),
            ]);
        }

        if let Some(title) = self.title {
            container = container.append([
                el(El::P)
                    .st([St::TextLg, St::FontSemibold, St::TextDefault, St::M0])
                    .text(&title),
            ]);
        }

        if let Some(description) = self.description {
            container = container.append([
                el(El::P)
                    .st([St::TextSm, St::TextMuted, St::MaxWProse])
                    .text(&description),
            ]);
        }

        if let Some(action) = self.action {
            container = container.append([
                el(El::Div).st([St::MtSm]).append([action]),
            ]);
        }

        container
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_state_defaults() {
        let es = EmptyState::new();
        assert!(es.title.is_none());
        assert!(es.description.is_none());
        assert!(es.icon.is_none());
        assert!(es.action.is_none());
    }

    #[test]
    fn test_empty_state_tokens() {
        let tokens = EmptyState::compute_tokens();
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::FlexCol));
        assert!(tokens.contains(&St::ItemsCenter));
        assert!(tokens.contains(&St::TextCenter));
        assert!(tokens.contains(&St::Py2xl));
    }

    #[test]
    fn test_empty_state_with_content() {
        let es = EmptyState::new()
            .title("No items")
            .description("Add some items to get started.");
        assert_eq!(es.title.as_deref(), Some("No items"));
        assert_eq!(es.description.as_deref(), Some("Add some items to get started."));
    }
}
