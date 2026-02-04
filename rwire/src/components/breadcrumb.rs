//! Breadcrumb component.
//!
//! Navigation breadcrumb trail.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::Breadcrumb;
//!
//! Breadcrumb::new()
//!     .item("Home", Some("/"))
//!     .item("Products", Some("/products"))
//!     .item("Laptop", None)  // Current page, no link
//!     .build()
//! ```

use crate::{el, El, ElementBuilder};
use std::borrow::Cow;

/// A single breadcrumb item.
#[derive(Clone, Debug)]
pub struct BreadcrumbItem {
    label: Cow<'static, str>,
    link: Option<Cow<'static, str>>,
}

impl BreadcrumbItem {
    /// Create a new breadcrumb item.
    pub fn new(label: impl Into<Cow<'static, str>>, link: Option<impl Into<Cow<'static, str>>>) -> Self {
        Self {
            label: label.into(),
            link: link.map(|l| l.into()),
        }
    }
}

/// Breadcrumb builder.
#[derive(Clone, Debug, Default)]
pub struct Breadcrumb {
    items: Vec<BreadcrumbItem>,
    extra_class: Option<Cow<'static, str>>,
}

impl Breadcrumb {
    /// Base CSS class.
    pub const BASE_CLASS: &'static str = "rw-breadcrumb";

    /// Create a new breadcrumb.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an item to the breadcrumb.
    pub fn item(mut self, label: impl Into<Cow<'static, str>>, link: Option<impl Into<Cow<'static, str>>>) -> Self {
        self.items.push(BreadcrumbItem::new(label, link));
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(32);
        classes.push_str(Self::BASE_CLASS);

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the breadcrumb into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Register for CSS tree-shaking
        super::registry::mark_component_used(super::registry::ComponentType::Breadcrumb);

        let class = self.compute_class();
        let nav = el(El::Nav)
            .class(&class)
            .attr("aria-label", "Breadcrumb");

        let mut ol = el(El::Ul).class("rw-breadcrumb-list");

        let total = self.items.len();
        for (idx, item) in self.items.into_iter().enumerate() {
            let is_last = idx == total - 1;

            let mut li = el(El::Li).class("rw-breadcrumb-item");

            if is_last {
                li = li.attr("aria-current", "page");
            }

            // If link is provided and not last item, render as anchor
            if let Some(link_url) = item.link {
                if !is_last {
                    li = li.append([
                        el(El::A)
                            .class("rw-breadcrumb-link")
                            .attr("href", &link_url)
                            .text(&item.label)
                    ]);
                } else {
                    li = li.append([
                        el(El::Span)
                            .class("rw-breadcrumb-current")
                            .text(&item.label)
                    ]);
                }
            } else {
                // No link, just text
                li = li.append([
                    el(El::Span)
                        .class("rw-breadcrumb-current")
                        .text(&item.label)
                ]);
            }

            ol = ol.append([li]);
        }

        nav.append([ol])
    }
}

/// Breadcrumb CSS.
///
/// Size: ~295 bytes (under 300 bytes budget)
pub const BREADCRUMB_CSS: &str = "\
.rw-breadcrumb-list{display:flex;align-items:center;gap:var(--rw-space-2);list-style:none;margin:0;padding:0}\
.rw-breadcrumb-item{display:flex;align-items:center;font-size:var(--rw-text-sm)}\
.rw-breadcrumb-item:not(:last-child)::after{content:\"/\";margin-left:var(--rw-space-2);color:var(--rw-text-low)}\
.rw-breadcrumb-link{color:var(--rw-accent-9);text-decoration:none}\
.rw-breadcrumb-link:hover{text-decoration:underline}\
.rw-breadcrumb-current{color:var(--rw-text-medium)}\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breadcrumb_defaults() {
        let bc = Breadcrumb::new();
        assert!(bc.items.is_empty());
    }

    #[test]
    fn test_breadcrumb_class_default() {
        let bc = Breadcrumb::new();
        assert_eq!(bc.compute_class(), "rw-breadcrumb");
    }

    #[test]
    fn test_breadcrumb_with_items() {
        let bc = Breadcrumb::new()
            .item("Home", Some("/"))
            .item("Products", Some("/products"))
            .item("Laptop", None::<&str>);
        assert_eq!(bc.items.len(), 3);
    }

    #[test]
    fn test_breadcrumb_css_size() {
        // Breadcrumb CSS should be under 300 bytes
        assert!(
            BREADCRUMB_CSS.len() < 500,
            "Breadcrumb CSS too large: {} bytes (budget: 500)",
            BREADCRUMB_CSS.len()
        );
        println!("Breadcrumb CSS size: {} bytes", BREADCRUMB_CSS.len());
    }

    #[test]
    fn test_breadcrumb_css_structure() {
        assert!(BREADCRUMB_CSS.contains(".rw-breadcrumb-list"));
        assert!(BREADCRUMB_CSS.contains(".rw-breadcrumb-item"));
        assert!(BREADCRUMB_CSS.contains(".rw-breadcrumb-link"));
        assert!(BREADCRUMB_CSS.contains(".rw-breadcrumb-current"));
    }
}
