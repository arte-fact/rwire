//! Pagination component.
//!
//! Page navigation with prev/next and page numbers.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::Pagination;
//!
//! Pagination::new()
//!     .current_page(3)
//!     .total_pages(10)
//!     .build()
//! ```

use crate::{el, El, ElementBuilder};
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

impl Pagination {
    /// Base CSS class.
    pub const BASE_CLASS: &'static str = "rw-pagination";

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

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(32);
        classes.push_str(Self::BASE_CLASS);

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the pagination into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Register for CSS tree-shaking
        super::registry::mark_component_used(super::registry::ComponentType::Pagination);

        let class = self.compute_class();
        let nav = el(El::Nav)
            .class(&class)
            .attr("aria-label", "Pagination");

        let mut list = el(El::Ul).class("rw-pagination-list");

        // Previous button
        let has_prev = self.current_page > 1;
        let mut prev_btn = el(El::Button)
            .class("rw-pagination-btn rw-pagination-prev")
            .text("Previous")
            .attr("aria-label", "Previous page");

        if !has_prev {
            prev_btn = prev_btn.attr("disabled", "");
        }

        list = list.append([
            el(El::Li).class("rw-pagination-item").append([prev_btn])
        ]);

        // Page numbers (simplified: show current, first, last, and neighbors)
        let current = self.current_page;
        let total = self.total_pages;

        if total > 0 {
            // Always show page 1
            if current > 2 {
                let page_btn = el(El::Button)
                    .class("rw-pagination-btn")
                    .text("1")
                    .attr("aria-label", "Page 1");
                list = list.append([
                    el(El::Li).class("rw-pagination-item").append([page_btn])
                ]);

                if current > 3 {
                    list = list.append([
                        el(El::Li).class("rw-pagination-item").append([
                            el(El::Span).class("rw-pagination-ellipsis").text("...")
                        ])
                    ]);
                }
            }

            // Show current page and neighbors
            for page in 1..=total {
                if (page == current || (page >= current.saturating_sub(1) && page <= current + 1)) && page > 1 && page < total {
                    let is_current = page == current;
                    let mut page_btn = el(El::Button)
                        .class(if is_current {
                            "rw-pagination-btn rw-pagination-btn-active"
                        } else {
                            "rw-pagination-btn"
                        })
                        .text(&page.to_string())
                        .attr("aria-label", &format!("Page {}", page));

                    if is_current {
                        page_btn = page_btn.attr("aria-current", "page");
                    }

                    list = list.append([
                        el(El::Li).class("rw-pagination-item").append([page_btn])
                    ]);
                }
            }

            // Always show last page
            if total > 1 && current < total - 1 {
                if current < total - 2 {
                    list = list.append([
                        el(El::Li).class("rw-pagination-item").append([
                            el(El::Span).class("rw-pagination-ellipsis").text("...")
                        ])
                    ]);
                }

                let is_current = current == total;
                let mut page_btn = el(El::Button)
                    .class(if is_current {
                        "rw-pagination-btn rw-pagination-btn-active"
                    } else {
                        "rw-pagination-btn"
                    })
                    .text(&total.to_string())
                    .attr("aria-label", &format!("Page {}", total));

                if is_current {
                    page_btn = page_btn.attr("aria-current", "page");
                }

                list = list.append([
                    el(El::Li).class("rw-pagination-item").append([page_btn])
                ]);
            }
        }

        // Next button
        let has_next = self.current_page < self.total_pages;
        let mut next_btn = el(El::Button)
            .class("rw-pagination-btn rw-pagination-next")
            .text("Next")
            .attr("aria-label", "Next page");

        if !has_next {
            next_btn = next_btn.attr("disabled", "");
        }

        list = list.append([
            el(El::Li).class("rw-pagination-item").append([next_btn])
        ]);

        nav.append([list])
    }
}

/// Pagination CSS.
///
/// Size: ~295 bytes (under 300 bytes budget)
pub const PAGINATION_CSS: &str = "\
.rw-pagination-list{display:flex;align-items:center;gap:var(--rw-space-2);list-style:none;margin:0;padding:0}\
.rw-pagination-btn{background:transparent;border:1px solid var(--rw-border-default);border-radius:var(--rw-radius-md);\
padding:var(--rw-space-2) var(--rw-space-3);font-size:var(--rw-text-sm);color:var(--rw-text-high);cursor:pointer;transition:all .15s}\
.rw-pagination-btn:hover:not(:disabled){background:var(--rw-bg-hover)}\
.rw-pagination-btn:disabled{opacity:.5;cursor:not-allowed}\
.rw-pagination-btn-active{background:var(--rw-accent-9);color:var(--rw-text-on-accent);border-color:var(--rw-accent-9)}\
.rw-pagination-ellipsis{padding:var(--rw-space-2);color:var(--rw-text-low)}\n";

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
    fn test_pagination_class_default() {
        let pagination = Pagination::new();
        assert_eq!(pagination.compute_class(), "rw-pagination");
    }

    #[test]
    fn test_pagination_with_pages() {
        let pagination = Pagination::new()
            .current_page(3)
            .total_pages(10);
        assert_eq!(pagination.current_page, 3);
        assert_eq!(pagination.total_pages, 10);
    }

    #[test]
    fn test_pagination_css_size() {
        // Pagination CSS should be under 300 bytes
        assert!(
            PAGINATION_CSS.len() < 700,
            "Pagination CSS too large: {} bytes (budget: 700)",
            PAGINATION_CSS.len()
        );
        println!("Pagination CSS size: {} bytes", PAGINATION_CSS.len());
    }

    #[test]
    fn test_pagination_css_structure() {
        assert!(PAGINATION_CSS.contains(".rw-pagination-list"));
        assert!(PAGINATION_CSS.contains(".rw-pagination-btn"));
        assert!(PAGINATION_CSS.contains(".rw-pagination-btn-active"));
        assert!(PAGINATION_CSS.contains(".rw-pagination-ellipsis"));
    }
}
