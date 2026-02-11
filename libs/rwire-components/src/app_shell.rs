//! AppShell component.
//!
//! Full-page layout with header, optional sidebar, and main content area.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::AppShell;
//!
//! AppShell::new()
//!     .header(header_content)
//!     .sidebar(sidebar_content)
//!     .main(main_content)
//!     .build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// AppShell builder.
#[derive(Clone, Default)]
pub struct AppShell {
    header: Option<ElementBuilder>,
    sidebar: Option<ElementBuilder>,
    main_content: Option<ElementBuilder>,
    sidebar_width: u16,
    header_height: u16,
    extra_class: Option<Cow<'static, str>>,
}

#[rwire::component]
impl AppShell {
    /// Create a new AppShell with default dimensions.
    pub fn new() -> Self {
        Self {
            sidebar_width: 260,
            header_height: 56,
            ..Self::default()
        }
    }

    /// Set the header content.
    pub fn header(mut self, content: ElementBuilder) -> Self {
        self.header = Some(content);
        self
    }

    /// Set the sidebar content.
    pub fn sidebar(mut self, content: ElementBuilder) -> Self {
        self.sidebar = Some(content);
        self
    }

    /// Set the main content.
    pub fn main(mut self, content: ElementBuilder) -> Self {
        self.main_content = Some(content);
        self
    }

    /// Set the sidebar width in pixels.
    pub fn sidebar_width(mut self, px: u16) -> Self {
        self.sidebar_width = px;
        self
    }

    /// Set the header height in pixels.
    pub fn header_height(mut self, px: u16) -> Self {
        self.header_height = px;
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the shell container.
    pub fn compute_tokens() -> Vec<St> {
        vec![St::GridTemplateShell]
    }

    /// Compute style tokens for the header.
    pub fn compute_header_tokens() -> Vec<St> {
        vec![
            St::GridColFull,
            St::PositionSticky,
            St::Top0,
            St::Z50,
            St::BgApp,
            St::BorderBDefault,
            St::DisplayFlex,
            St::ItemsCenter,
            St::PxMd,
        ]
    }

    /// Compute style tokens for the sidebar.
    pub fn compute_sidebar_tokens() -> Vec<St> {
        vec![
            St::BgSidebar,
            St::BorderRDefault,
            St::OverflowYScroll,
            St::PositionSticky,
            St::PyMd,
        ]
    }

    /// Compute style tokens for the main area.
    pub fn compute_main_tokens() -> Vec<St> {
        vec![St::OverflowYScroll, St::PMd, St::Flex1]
    }

    /// Build the app shell into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let has_sidebar = self.sidebar.is_some();
        let grid_cols = if has_sidebar {
            format!("grid-template-columns:{}px 1fr", self.sidebar_width)
        } else {
            "grid-template-columns:1fr".to_string()
        };

        let mut shell = el(El::Div)
            .st(Self::compute_tokens())
            .attr("style", &grid_cols);

        if let Some(ref extra) = self.extra_class {
            shell = shell.class(extra.as_ref());
        }

        // Header
        if let Some(header) = self.header {
            let header_style = format!("height:{}px", self.header_height);
            let header_el = el(El::Header)
                .st(Self::compute_header_tokens())
                .attr("style", &header_style)
                .append([header]);
            shell = shell.append([header_el]);
        }

        // Sidebar
        if let Some(sidebar) = self.sidebar {
            let sidebar_style = format!(
                "top:{}px;height:calc(100vh - {}px)",
                self.header_height, self.header_height
            );
            let sidebar_el = el(El::Aside)
                .st(Self::compute_sidebar_tokens())
                .attr("style", &sidebar_style)
                .append([sidebar]);
            shell = shell.append([sidebar_el]);
        }

        // Main content
        if let Some(content) = self.main_content {
            let main_el = el(El::Main)
                .st(Self::compute_main_tokens())
                .append([content]);
            shell = shell.append([main_el]);
        }

        shell
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_shell_defaults() {
        let shell = AppShell::new();
        assert_eq!(shell.sidebar_width, 260);
        assert_eq!(shell.header_height, 56);
        assert!(shell.header.is_none());
        assert!(shell.sidebar.is_none());
        assert!(shell.main_content.is_none());
    }

    #[test]
    fn test_app_shell_custom_dimensions() {
        let shell = AppShell::new()
            .sidebar_width(280)
            .header_height(64);
        assert_eq!(shell.sidebar_width, 280);
        assert_eq!(shell.header_height, 64);
    }

    #[test]
    fn test_shell_container_tokens() {
        let tokens = AppShell::compute_tokens();
        assert!(tokens.contains(&St::GridTemplateShell));
    }

    #[test]
    fn test_shell_header_tokens() {
        let tokens = AppShell::compute_header_tokens();
        assert!(tokens.contains(&St::GridColFull));
        assert!(tokens.contains(&St::PositionSticky));
        assert!(tokens.contains(&St::Top0));
        assert!(tokens.contains(&St::Z50));
        assert!(tokens.contains(&St::BgApp));
        assert!(tokens.contains(&St::BorderBDefault));
    }

    #[test]
    fn test_shell_sidebar_tokens() {
        let tokens = AppShell::compute_sidebar_tokens();
        assert!(tokens.contains(&St::BgSidebar));
        assert!(tokens.contains(&St::BorderRDefault));
        assert!(tokens.contains(&St::OverflowYScroll));
        assert!(tokens.contains(&St::PositionSticky));
    }

    #[test]
    fn test_shell_main_tokens() {
        let tokens = AppShell::compute_main_tokens();
        assert!(tokens.contains(&St::OverflowYScroll));
        assert!(tokens.contains(&St::PMd));
        assert!(tokens.contains(&St::Flex1));
    }
}
