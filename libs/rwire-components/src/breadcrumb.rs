//! Breadcrumb component.
//!
//! Navigation breadcrumb trail.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::Breadcrumb;
//!
//! Breadcrumb::new()
//!     .item("Home", Some("/"))
//!     .item("Products", Some("/products"))
//!     .item("Laptop", None)  // Current page, no link
//!     .build()
//! ```

use rwire::attr_tokens::At;
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
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

#[rwire::component]
impl Breadcrumb {
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

    /// Compute style tokens for the breadcrumb list.
    pub fn compute_tokens(&self) -> Vec<St> {
        vec![St::DisplayFlex, St::ItemsCenter, St::GapSm, St::ListStyleNone, St::M0, St::P0]
    }

    /// Build the breadcrumb into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut nav = el(El::Nav).at_str(At::AriaLabel, "Breadcrumb");

        if let Some(ref extra) = self.extra_class {
            nav = nav.class(extra.as_ref());
        }

        let mut ol = el(El::Ul).st(self.compute_tokens());

        let total = self.items.len();
        for (idx, item) in self.items.into_iter().enumerate() {
            let is_last = idx == total - 1;

            let mut li = el(El::Li)
                .st([St::DisplayFlex, St::ItemsCenter, St::TextSm]);

            if !is_last {
                li = li.after([St::ContentSlash, St::MxSp2, St::TextMuted]);
            }

            if is_last {
                li = li.attr("aria-current", "page");
            }

            // If link is provided and not last item, render as anchor
            if let Some(link_url) = item.link {
                if !is_last {
                    li = li.append([
                        el(El::A)
                            .st([St::TextAccent, St::NoDecoration])
                            .hover([St::Underline])
                            .at_str(At::Href, &link_url)
                            .text(&item.label)
                    ]);
                } else {
                    li = li.append([
                        el(El::Span)
                            .st([St::TextMedium])
                            .text(&item.label)
                    ]);
                }
            } else {
                // No link, just text
                li = li.append([
                    el(El::Span)
                        .st([St::TextMedium])
                        .text(&item.label)
                ]);
            }

            ol = ol.append([li]);
        }

        nav.append([ol])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breadcrumb_defaults() {
        let bc = Breadcrumb::new();
        assert!(bc.items.is_empty());
    }

    #[test]
    fn test_breadcrumb_tokens() {
        let bc = Breadcrumb::new();
        let tokens = bc.compute_tokens();
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::ItemsCenter));
        assert!(tokens.contains(&St::GapSm));
        assert!(tokens.contains(&St::ListStyleNone));
        assert!(tokens.contains(&St::M0));
        assert!(tokens.contains(&St::P0));
    }

    #[test]
    fn test_breadcrumb_with_items() {
        let bc = Breadcrumb::new()
            .item("Home", Some("/"))
            .item("Products", Some("/products"))
            .item("Laptop", None::<&str>);
        assert_eq!(bc.items.len(), 3);
    }

}
