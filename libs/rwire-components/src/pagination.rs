//! Pagination component.
//!
//! Page navigation with prev/next and page numbers.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::Pagination;
//!
//! Pagination::new()
//!     .current_page(3)
//!     .total_pages(10)
//!     .build()
//! ```

use rwire::attr_tokens::At;
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Pagination builder.
#[derive(Clone, Debug)]
pub struct Pagination {
    current_page: usize,
    total_pages: usize,
    max_visible: usize,
    extra_class: Option<Cow<'static, str>>,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            current_page: 1,
            total_pages: 1,
            max_visible: 5,
            extra_class: None,
        }
    }
}

#[rwire::component]
impl Pagination {
    /// Create a new pagination component.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the current page (1-indexed).
    pub fn current_page(mut self, page: usize) -> Self {
        self.current_page = page;
        self
    }

    /// Set the total number of pages.
    pub fn total_pages(mut self, total: usize) -> Self {
        self.total_pages = total;
        self
    }

    /// Set the maximum number of visible page buttons.
    pub fn max_visible(mut self, max: usize) -> Self {
        self.max_visible = max;
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the pagination list.
    pub fn compute_tokens(&self) -> Vec<St> {
        vec![St::DisplayFlex, St::ItemsCenter, St::GapSm, St::ListStyleNone, St::M0, St::P0]
    }

    /// Build a page button with common tokens.
    fn build_page_button(text: &str, is_active: bool, is_disabled: bool) -> ElementBuilder {
        let mut tokens = vec![
            St::BgTransparent, St::BorderDefault, St::RoundedMd,
            St::TextSm, St::TextHigh, St::CursorPointer, St::TransitionAll,
        ];

        if is_active {
            tokens.extend([St::BgAccent, St::TextOnAccent, St::BorderAccent]);
        }

        tokens.extend([St::PySm, St::PxSp3]);
        let mut button = el(El::Button)
            .st(tokens)
            .hover([St::BgHover])
            .text(text);

        if is_disabled {
            button = button.bool_attr(At::Disabled);
        }

        button
    }

    /// Build the pagination into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut nav = el(El::Nav).at_str(At::AriaLabel, "Pagination");

        if let Some(ref extra) = self.extra_class {
            nav = nav.class(extra.as_ref());
        }

        let mut list = el(El::Ul).st(self.compute_tokens());

        // Previous button
        let has_prev = self.current_page > 1;
        let prev_btn = Self::build_page_button("Previous", false, !has_prev)
            .at_str(At::AriaLabel, "Previous page");

        list = list.append([
            el(El::Li).append([prev_btn])
        ]);

        // Page numbers (simplified: show current, first, last, and neighbors)
        let current = self.current_page;
        let total = self.total_pages;

        if total > 0 {
            // Always show page 1
            if current > 2 {
                let page_btn = Self::build_page_button("1", false, false)
                    .at_str(At::AriaLabel, "Page 1");
                list = list.append([
                    el(El::Li).append([page_btn])
                ]);

                if current > 3 {
                    list = list.append([
                        el(El::Li).append([
                            el(El::Span)
                                .st([St::TextLow, St::PSm])
                                .text("...")
                        ])
                    ]);
                }
            }

            // Show current page and neighbors
            for page in 1..=total {
                if (page == current || (page >= current.saturating_sub(1) && page <= current + 1)) && page > 1 && page < total {
                    let is_current = page == current;
                    let mut page_btn = Self::build_page_button(&page.to_string(), is_current, false)
                        .at_str(At::AriaLabel, &format!("Page {}", page));

                    if is_current {
                        page_btn = page_btn.attr("aria-current", "page");
                    }

                    list = list.append([
                        el(El::Li).append([page_btn])
                    ]);
                }
            }

            // Always show last page
            if total > 1 && current < total - 1 {
                if current < total - 2 {
                    list = list.append([
                        el(El::Li).append([
                            el(El::Span)
                                .st([St::TextLow, St::PSm])
                                .text("...")
                        ])
                    ]);
                }

                let is_current = current == total;
                let mut page_btn = Self::build_page_button(&total.to_string(), is_current, false)
                    .at_str(At::AriaLabel, &format!("Page {}", total));

                if is_current {
                    page_btn = page_btn.attr("aria-current", "page");
                }

                list = list.append([
                    el(El::Li).append([page_btn])
                ]);
            }
        }

        // Next button
        let has_next = self.current_page < self.total_pages;
        let next_btn = Self::build_page_button("Next", false, !has_next)
            .at_str(At::AriaLabel, "Next page");

        list = list.append([
            el(El::Li).append([next_btn])
        ]);

        nav.append([list])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_defaults() {
        let pagination = Pagination::new();
        assert_eq!(pagination.current_page, 1);
        assert_eq!(pagination.total_pages, 1);
        assert_eq!(pagination.max_visible, 5);
    }

    #[test]
    fn test_pagination_tokens() {
        let pagination = Pagination::new();
        let tokens = pagination.compute_tokens();
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::ItemsCenter));
        assert!(tokens.contains(&St::GapSm));
        assert!(tokens.contains(&St::ListStyleNone));
        assert!(tokens.contains(&St::M0));
        assert!(tokens.contains(&St::P0));
    }

    #[test]
    fn test_pagination_with_pages() {
        let pagination = Pagination::new()
            .current_page(3)
            .total_pages(10);
        assert_eq!(pagination.current_page, 3);
        assert_eq!(pagination.total_pages, 10);
    }

}
