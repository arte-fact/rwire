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

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(32);
        classes.push_str("rw-list");

        if self.ordered {
            classes.push_str(" rw-list-ordered");
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the list into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        super::registry::mark_component_used(super::registry::ComponentType::List);

        let class = self.compute_class();
        let element = if self.ordered { El::Ol } else { El::Ul };
        let mut builder = el(element).class(&class);

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

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(24);
        classes.push_str("rw-list-item");

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the list item into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        super::registry::mark_component_used(super::registry::ComponentType::List);

        let class = self.compute_class();
        let mut builder = el(El::Li).class(&class);

        if let Some(content) = self.content {
            builder = builder.text(&content);
        }

        for child in self.children {
            builder = builder.append([child]);
        }

        builder
    }
}

/// List CSS.
pub const LIST_CSS: &str = "\
.rw-list{margin:0;padding-left:var(--rw-space-6);color:var(--rw-text-default)}\
.rw-list-ordered{list-style-type:decimal}\
.rw-list-item{margin-bottom:var(--rw-space-2)}\
.rw-list-item:last-child{margin-bottom:0}\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_defaults() {
        let list = List::new();
        assert!(!list.ordered);
    }

    #[test]
    fn test_list_class_default() {
        let list = List::new();
        assert_eq!(list.compute_class(), "rw-list");
    }

    #[test]
    fn test_list_class_ordered() {
        let list = List::ordered();
        assert_eq!(list.compute_class(), "rw-list rw-list-ordered");
    }

    #[test]
    fn test_list_item_class() {
        let item = ListItem::new("Test");
        assert_eq!(item.compute_class(), "rw-list-item");
    }

    #[test]
    fn test_list_css_size() {
        assert!(LIST_CSS.len() < 250, "List CSS too large: {} bytes", LIST_CSS.len());
        println!("List CSS size: {} bytes", LIST_CSS.len());
    }
}
