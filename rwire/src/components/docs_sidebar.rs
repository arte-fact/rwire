//! DocsSidebar component.
//!
//! Navigation sidebar with collapsible sections and active link highlighting.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{DocsSidebar, SidebarSection};
//!
//! DocsSidebar::new()
//!     .section(SidebarSection::new("Getting Started")
//!         .link("/docs/install", "Installation")
//!         .link("/docs/quickstart", "Quick Start"))
//!     .section(SidebarSection::new("Components")
//!         .link("/docs/components/button", "Button")
//!         .link("/docs/components/input", "Input"))
//!     .active_path("/docs/quickstart")
//!     .build()
//! ```

use crate::attr_tokens::At;
use crate::style_tokens::St;
use crate::{el, El, ElementBuilder};
use std::borrow::Cow;

/// A link within a sidebar section.
#[derive(Clone, Debug)]
pub struct SidebarLink {
    path: Cow<'static, str>,
    label: Cow<'static, str>,
}

/// A collapsible section in the sidebar.
#[derive(Clone, Debug)]
pub struct SidebarSection {
    title: Cow<'static, str>,
    links: Vec<SidebarLink>,
    open: bool,
}

impl SidebarSection {
    /// Create a new section with a title.
    pub fn new(title: impl Into<Cow<'static, str>>) -> Self {
        Self {
            title: title.into(),
            links: Vec::new(),
            open: true,
        }
    }

    /// Add a link to this section.
    pub fn link(
        mut self,
        path: impl Into<Cow<'static, str>>,
        label: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.links.push(SidebarLink {
            path: path.into(),
            label: label.into(),
        });
        self
    }

    /// Set whether this section starts open.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }
}

/// DocsSidebar builder.
#[derive(Clone, Default)]
pub struct DocsSidebar {
    sections: Vec<SidebarSection>,
    active_path: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
}

impl DocsSidebar {
    /// Create a new docs sidebar.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a section.
    pub fn section(mut self, section: SidebarSection) -> Self {
        self.sections.push(section);
        self
    }

    /// Set the active path for link highlighting.
    pub fn active_path(mut self, path: impl Into<Cow<'static, str>>) -> Self {
        self.active_path = Some(path.into());
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the sidebar container.
    pub fn compute_tokens() -> Vec<St> {
        vec![St::DisplayFlex, St::FlexCol, St::GapMd, St::PxSm, St::TextSm]
    }

    /// Compute style tokens for a section title.
    pub fn compute_section_title_tokens() -> Vec<St> {
        vec![
            St::TextXsMuted,
            St::FontSemibold,
            St::TextUppercase,
            St::TrackingWider,
            St::PxSm,
            St::PySm,
        ]
    }

    /// Compute style tokens for a regular link.
    pub fn compute_link_tokens() -> Vec<St> {
        vec![
            St::DisplayBlock,
            St::PxSm,
            St::PySm,
            St::RoundedSm,
            St::TextDefault,
            St::NoDecoration,
            St::TransitionColors,
        ]
    }

    /// Compute style tokens for the active link.
    pub fn compute_active_link_tokens() -> Vec<St> {
        vec![
            St::DisplayBlock,
            St::PxSm,
            St::PySm,
            St::RoundedSm,
            St::NoDecoration,
            St::BgAccentSubtle,
            St::TextAccent12,
            St::FontMedium,
        ]
    }

    /// Build the sidebar into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut nav = el(El::Nav).st(Self::compute_tokens());

        if let Some(ref extra) = self.extra_class {
            nav = nav.class(extra.as_ref());
        }

        for section in &self.sections {
            let title = el(El::Div)
                .st(Self::compute_section_title_tokens())
                .text(&section.title);

            let links: Vec<ElementBuilder> = section
                .links
                .iter()
                .map(|link| {
                    let is_active = self
                        .active_path
                        .as_deref()
                        .is_some_and(|p| p == link.path.as_ref());

                    let tokens = if is_active {
                        Self::compute_active_link_tokens()
                    } else {
                        Self::compute_link_tokens()
                    };

                    let mut a = el(El::A)
                        .st(tokens)
                        .at_str(At::Href, &link.path)
                        .text(&link.label);

                    if !is_active {
                        a = a.hover([St::BgHover]);
                    }

                    a
                })
                .collect();

            let link_list = el(El::Div)
                .st([St::DisplayFlex, St::FlexCol])
                .append(links);

            let section_el = if section.open {
                el(El::Div).append([title, link_list])
            } else {
                el(El::Div).append([title])
            };

            nav = nav.append([section_el]);
        }

        nav
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidebar_section_new() {
        let section = SidebarSection::new("Getting Started");
        assert_eq!(section.title.as_ref(), "Getting Started");
        assert!(section.links.is_empty());
        assert!(section.open);
    }

    #[test]
    fn test_sidebar_section_with_links() {
        let section = SidebarSection::new("Docs")
            .link("/install", "Installation")
            .link("/quickstart", "Quick Start");
        assert_eq!(section.links.len(), 2);
        assert_eq!(section.links[0].path.as_ref(), "/install");
        assert_eq!(section.links[0].label.as_ref(), "Installation");
    }

    #[test]
    fn test_docs_sidebar_defaults() {
        let sidebar = DocsSidebar::new();
        assert!(sidebar.sections.is_empty());
        assert!(sidebar.active_path.is_none());
    }

    #[test]
    fn test_docs_sidebar_container_tokens() {
        let tokens = DocsSidebar::compute_tokens();
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::FlexCol));
        assert!(tokens.contains(&St::TextSm));
    }

    #[test]
    fn test_docs_sidebar_section_title_tokens() {
        let tokens = DocsSidebar::compute_section_title_tokens();
        assert!(tokens.contains(&St::TextXsMuted));
        assert!(tokens.contains(&St::FontSemibold));
        assert!(tokens.contains(&St::TextUppercase));
    }

    #[test]
    fn test_docs_sidebar_link_tokens() {
        let tokens = DocsSidebar::compute_link_tokens();
        assert!(tokens.contains(&St::DisplayBlock));
        assert!(tokens.contains(&St::RoundedSm));
        assert!(tokens.contains(&St::NoDecoration));
    }

    #[test]
    fn test_docs_sidebar_active_link_tokens() {
        let tokens = DocsSidebar::compute_active_link_tokens();
        assert!(tokens.contains(&St::BgAccentSubtle));
        assert!(tokens.contains(&St::TextAccent12));
        assert!(tokens.contains(&St::FontMedium));
    }

    #[test]
    fn test_docs_sidebar_with_active_path() {
        let sidebar = DocsSidebar::new()
            .section(SidebarSection::new("Docs")
                .link("/install", "Installation")
                .link("/quickstart", "Quick Start"))
            .active_path("/quickstart");
        assert_eq!(sidebar.active_path.as_deref(), Some("/quickstart"));
    }
}
