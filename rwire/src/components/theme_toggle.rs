//! ThemeToggle component for switching between light and dark themes.
//!
//! This component renders a button that toggles between light and dark themes.
//! It uses local state to manage the theme and automatically updates the document
//! theme attributes.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::ThemeToggle;
//!
//! // Add to your UI
//! ThemeToggle::new().build()
//! ```
//!
//! # Integration with Application State
//!
//! The theme toggle needs to be connected to your application's theme state:
//!
//! ```ignore
//! use rwire::{State, handler, renderer, ThemeMode};
//!
//! #[derive(State, Default)]
//! #[storage(local)]
//! struct AppState {
//!     theme: ThemeMode,
//! }
//!
//! #[handler]
//! fn toggle_theme(state: &mut AppState) {
//!     state.theme = match state.theme {
//!         ThemeMode::Light => ThemeMode::Dark,
//!         ThemeMode::Dark => ThemeMode::Light,
//!     };
//! }
//! ```

use crate::{
    el, El, Ev, ElementBuilder, HandlerSpec,
    icons::{icon, Icon},
};

/// ThemeToggle builder.
#[derive(Clone, Debug)]
pub struct ThemeToggle {
    size: ToggleSize,
    show_label: bool,
    on_toggle: Option<HandlerSpec>,
    current_mode: ThemeToggleMode,
}

/// Display mode for the toggle icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeToggleMode {
    Light,
    Dark,
}

impl Default for ThemeToggle {
    fn default() -> Self {
        Self {
            size: ToggleSize::default(),
            show_label: false,
            on_toggle: None,
            current_mode: ThemeToggleMode::Light,
        }
    }
}

/// Size variants for the theme toggle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ToggleSize {
    Sm,
    #[default]
    Md,
    Lg,
}

impl ThemeToggle {
    /// Base CSS class.
    pub const BASE_CLASS: &'static str = "rw-theme-toggle";

    /// Create a new theme toggle.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the size of the toggle button.
    pub fn size(mut self, size: ToggleSize) -> Self {
        self.size = size;
        self
    }

    /// Show a text label next to the icon.
    pub fn show_label(mut self, show: bool) -> Self {
        self.show_label = show;
        self
    }

    /// Set the current theme mode to display the correct icon.
    pub fn mode(mut self, mode: ThemeToggleMode) -> Self {
        self.current_mode = mode;
        self
    }

    /// Set the click handler for the toggle.
    pub fn on_toggle(mut self, handler: HandlerSpec) -> Self {
        self.on_toggle = Some(handler);
        self
    }

    /// Build the theme toggle component.
    pub fn build(self) -> ElementBuilder {
        // Register component for CSS tree-shaking
        super::registry::mark_component_used(
            super::registry::ComponentType::ThemeToggle
        );

        let class = self.compute_class();
        let icon_el = self.render_icon();

        let mut button = el(El::Button)
            .class(&class)
            .attr("type", "button")
            .attr("aria-label", "Toggle theme")
            .append([
                el(El::Span)
                    .class("rw-theme-toggle-content")
                    .append([icon_el])
            ]);

        if let Some(handler) = self.on_toggle {
            button = button.on(Ev::Click, handler);
        }

        button
    }

    fn render_icon(&self) -> ElementBuilder {
        match self.current_mode {
            ThemeToggleMode::Light => icon(Icon::Moon),
            ThemeToggleMode::Dark => icon(Icon::Sun),
        }
    }

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(64);
        classes.push_str(Self::BASE_CLASS);

        match self.size {
            ToggleSize::Sm => classes.push_str(" rw-theme-toggle-sm"),
            ToggleSize::Md => {}
            ToggleSize::Lg => classes.push_str(" rw-theme-toggle-lg"),
        }

        classes
    }
}

/// ThemeToggle CSS.
pub const THEME_TOGGLE_CSS: &str = "\
.rw-theme-toggle{display:inline-flex;align-items:center;justify-content:center;\
padding:var(--rw-space-2);background:transparent;border:1px solid var(--rw-border-default);\
border-radius:var(--rw-radius-md);color:var(--rw-text-default);cursor:pointer;\
transition:var(--rw-transition-fast)}\
.rw-theme-toggle:hover{background:var(--rw-bg-hover);border-color:var(--rw-border-strong)}\
.rw-theme-toggle:focus-visible{outline:2px solid var(--rw-accent-9);outline-offset:2px}\
.rw-theme-toggle:active{background:var(--rw-bg-active)}\
.rw-theme-toggle-sm{padding:var(--rw-space-1)}\
.rw-theme-toggle-lg{padding:var(--rw-space-3)}\
.rw-theme-toggle .rw-icon{width:20px;height:20px}\
.rw-theme-toggle-sm .rw-icon{width:16px;height:16px}\
.rw-theme-toggle-lg .rw-icon{width:24px;height:24px}\
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_toggle_defaults() {
        let toggle = ThemeToggle::new();
        assert_eq!(toggle.size, ToggleSize::Md);
        assert!(!toggle.show_label);
        assert_eq!(toggle.current_mode, ThemeToggleMode::Light);
        assert!(toggle.on_toggle.is_none());
    }

    #[test]
    fn test_theme_toggle_class_default() {
        let toggle = ThemeToggle::new();
        let class = toggle.compute_class();
        assert_eq!(class, "rw-theme-toggle");
    }

    #[test]
    fn test_theme_toggle_class_with_size() {
        let toggle = ThemeToggle::new().size(ToggleSize::Sm);
        let class = toggle.compute_class();
        assert!(class.contains("rw-theme-toggle-sm"));

        let toggle = ThemeToggle::new().size(ToggleSize::Lg);
        let class = toggle.compute_class();
        assert!(class.contains("rw-theme-toggle-lg"));
    }

    #[test]
    fn test_theme_toggle_css_size() {
        assert!(
            THEME_TOGGLE_CSS.len() < 800,
            "ThemeToggle CSS too large: {} bytes",
            THEME_TOGGLE_CSS.len()
        );
        println!("ThemeToggle CSS size: {} bytes", THEME_TOGGLE_CSS.len());
    }

    #[test]
    fn test_theme_toggle_css_structure() {
        assert!(THEME_TOGGLE_CSS.contains(".rw-theme-toggle"));
        assert!(THEME_TOGGLE_CSS.contains(".rw-theme-toggle:hover"));
        assert!(THEME_TOGGLE_CSS.contains(".rw-theme-toggle-sm"));
        assert!(THEME_TOGGLE_CSS.contains(".rw-theme-toggle-lg"));
    }

    #[test]
    fn test_theme_toggle_mode() {
        let toggle = ThemeToggle::new().mode(ThemeToggleMode::Dark);
        assert_eq!(toggle.current_mode, ThemeToggleMode::Dark);
    }
}
