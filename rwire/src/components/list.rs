//! List and ListItem components.
//!
//! Styled lists with bullets or numbers.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{List, ListItem};
//!
//! List::unordered()
//!     .children([
//!         ListItem::new("First item").build(),
//!         ListItem::new("Second item").build(),
//!     ])
//!     .build()
//!
//! List::ordered()
//!     .children([
//!         ListItem::new("Step 1").build(),
//!         ListItem::new("Step 2").build(),
//!     ])
//!     .build()
//! ```

use crate::style_tokens::St;
use crate::{el, El, ElementBuilder};
use std::borrow::Cow;

/// List component builder.
#[derive(Clone, Default)]
pub struct List {
    ordered: bool,
    extra_class: Option<Cow<'static, str>>,
    children: Vec<ElementBuilder>,
}

impl List {
    /// Create a new unordered list (bulleted).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an unordered list (bulleted).
    pub fn unordered() -> Self {
        Self::default()
    }

    /// Create an ordered list (numbered).
    pub fn ordered() -> Self {
        Self {
            ordered: true,
            ..Self::default()
        }
    }

    /// Set whether this is an ordered list.
    pub fn is_ordered(mut self, ordered: bool) -> Self {
        self.ordered = ordered;
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Add children to the list.
    pub fn children(mut self, children: impl IntoIterator<Item = ElementBuilder>) -> Self {
        self.children.extend(children);
        self
    }

    /// Add a single child.
    pub fn child(mut self, child: ElementBuilder) -> Self {
        self.children.push(child);
        self
    }

    /// Compute style tokens for the list container.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![St::M0, St::TextDefault];
        if self.ordered {
            tokens.push(St::ListDecimal);
        }
        tokens
    }

    /// Build the list into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let element = if self.ordered { El::Ol } else { El::Ul };
        let mut tokens = self.compute_tokens();
        tokens.push(St::PlLg);
        let mut builder = el(element)
            .st(tokens);

        if let Some(ref extra) = self.extra_class {
            builder = builder.class(extra.as_ref());
        }

        for child in self.children {
            builder = builder.append([child]);
        }

        builder
    }
}

/// ListItem component builder.
#[derive(Clone, Default)]
pub struct ListItem {
    content: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
    children: Vec<ElementBuilder>,
}

impl ListItem {
    /// Create a new list item.
    pub fn new(content: impl Into<Cow<'static, str>>) -> Self {
        Self {
            content: Some(content.into()),
            ..Self::default()
        }
    }

    /// Create an empty list item (for custom children).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Set the text content.
    pub fn content(mut self, content: impl Into<Cow<'static, str>>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Add children to the list item.
    pub fn children(mut self, children: impl IntoIterator<Item = ElementBuilder>) -> Self {
        self.children.extend(children);
        self
    }

    /// Add a single child.
    pub fn child(mut self, child: ElementBuilder) -> Self {
        self.children.push(child);
        self
    }

    /// Compute style tokens for the list item.
    pub fn compute_tokens(&self) -> Vec<St> {
        vec![St::MbSm]
    }

    /// Build the list item into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut builder = el(El::Li)
            .st(self.compute_tokens())
            .last_child([St::Mb0]);

        if let Some(ref extra) = self.extra_class {
            builder = builder.class(extra.as_ref());
        }

        if let Some(content) = self.content {
            builder = builder.text(&content);
        }

        for child in self.children {
            builder = builder.append([child]);
        }

        builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_defaults() {
        let list = List::new();
        assert!(!list.ordered);
    }

    #[test]
    fn test_list_tokens_unordered() {
        let list = List::new();
        let tokens = list.compute_tokens();
        assert!(tokens.contains(&St::M0));
        assert!(tokens.contains(&St::TextDefault));
        assert!(!tokens.contains(&St::ListDecimal));
    }

    #[test]
    fn test_list_tokens_ordered() {
        let list = List::ordered();
        let tokens = list.compute_tokens();
        assert!(tokens.contains(&St::ListDecimal));
    }

    #[test]
    fn test_list_item_tokens() {
        let item = ListItem::new("Test");
        let tokens = item.compute_tokens();
        assert!(tokens.contains(&St::MbSm));
    }

    #[test]
    fn test_list_item_pseudo() {
        let item = ListItem::new("Test").build();
        let groups = item.get_pseudo_groups();
        assert!(groups.iter().any(|(pc, _)| *pc == 0x0A)); // Pc::LastChild
    }

}
