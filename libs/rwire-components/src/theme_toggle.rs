//! ThemeToggle component for switching between light and dark themes, or
//! following the operating system.
//!
//! This component renders a button that toggles between light and dark themes,
//! or cycles through three states (system → light → dark) with
//! [`ThemeToggleMode::System`] and [`ThemeMode::cycle`](rwire::theme::ThemeMode::cycle).
//! It mutates the framework-provided `Theme` state, which reactively updates
//! CSS variables via the synced element system.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::ThemeToggle;
//!
//! // Add to your UI
//! ThemeToggle::new().build()
//! ```
//!
//! # Integration with Application State
//!
//! The theme toggle needs a handler that mutates `&mut Theme`:
//!
//! ```ignore
//! use rwire::{handler, Theme};
//!
//! #[handler]
//! fn toggle_theme(theme: &mut Theme) {
//!     theme.mode = theme.mode.toggle();
//! }
//! ```
//!
//! # Three states (follow the OS, or force light or dark)
//!
//! ```ignore
//! use rwire::{handler, renderer, theme::ThemeMode, Theme};
//!
//! #[renderer]
//! fn theme_button(theme: &Theme) -> ElementBuilder {
//!     ThemeToggle::new()
//!         .mode(theme.mode.into()) // Light, Dark or System
//!         .on_toggle(cycle_theme())
//!         .build()
//! }
//!
//! #[handler]
//! fn cycle_theme(theme: &mut Theme) {
//!     theme.mode = theme.mode.cycle(); // system → light → dark → system
//! }
//! ```
//!
//! The mode picked survives a reload (the runtime keeps it in a cookie).

use rwire::attr_tokens::{At, Av};
use rwire::style_tokens::St;
use rwire::{
    el,
    icons::{icon, Icon},
    El, ElementBuilder, Ev, HandlerSpec,
};

/// ThemeToggle builder.
#[derive(Clone, Debug)]
pub struct ThemeToggle {
    size: ToggleSize,
    show_label: bool,
    on_toggle: Option<HandlerSpec>,
    current_mode: ThemeToggleMode,
}

/// The current mode, shown by the toggle's icon: a moon in light mode (click
/// for dark), a sun in dark mode, a screen when following the system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeToggleMode {
    Light,
    Dark,
    /// Following the OS (`prefers-color-scheme`): the third state.
    System,
}

impl From<rwire::theme::ThemeMode> for ThemeToggleMode {
    fn from(mode: rwire::theme::ThemeMode) -> Self {
        use rwire::theme::ThemeMode;
        match mode {
            ThemeMode::Light => Self::Light,
            ThemeMode::Dark => Self::Dark,
            ThemeMode::System => Self::System,
        }
    }
}

impl ThemeToggleMode {
    /// The tooltip naming the current mode.
    fn title(self) -> &'static str {
        match self {
            Self::Light => "Theme: light",
            Self::Dark => "Theme: dark",
            Self::System => "Theme: follows the system",
        }
    }
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

#[rwire::component]
impl ThemeToggle {
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

    /// Compute style tokens for the theme toggle button.
    pub fn compute_tokens(&self) -> Vec<St> {
        vec![
            St::DisplayInlineFlex,
            St::ItemsCenter,
            St::JustifyCenter,
            St::BgTransparent,
            St::BorderDefault,
            St::RoundedMd,
            St::TextDefault,
            St::CursorPointer,
            St::TransitionColors,
        ]
    }

    fn size_token(&self) -> St {
        match self.size {
            ToggleSize::Sm => St::PXs,
            ToggleSize::Md => St::PSm,
            ToggleSize::Lg => St::PSp3,
        }
    }

    fn render_icon(&self) -> ElementBuilder {
        match self.current_mode {
            ThemeToggleMode::Light => icon(Icon::Moon),
            ThemeToggleMode::Dark => icon(Icon::Sun),
            ThemeToggleMode::System => icon(Icon::Monitor),
        }
    }

    /// Build the theme toggle component.
    pub fn build(self) -> ElementBuilder {
        let icon_el = self.render_icon();
        let mut tokens = self.compute_tokens();
        tokens.push(self.size_token());

        let mut button = el(El::Button)
            .st(tokens)
            .hover([St::BgHover])
            .focus_visible([St::RingFocus])
            .active([St::Scale98])
            .at(At::Type, Av::Button)
            .at_str(At::AriaLabel, "Toggle theme")
            .attr("title", self.current_mode.title())
            .append([icon_el]);

        if let Some(handler) = self.on_toggle {
            button = button.on(Ev::Click, handler);
        }

        button
    }
}

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
    fn test_theme_toggle_tokens() {
        let toggle = ThemeToggle::new();
        let tokens = toggle.compute_tokens();
        assert!(tokens.contains(&St::DisplayInlineFlex));
        assert!(tokens.contains(&St::ItemsCenter));
        assert!(tokens.contains(&St::JustifyCenter));
        assert!(tokens.contains(&St::BgTransparent));
        assert!(tokens.contains(&St::BorderDefault));
        assert!(tokens.contains(&St::RoundedMd));
        assert!(tokens.contains(&St::TextDefault));
        assert!(tokens.contains(&St::CursorPointer));
        assert!(tokens.contains(&St::TransitionColors));
    }

    #[test]
    fn test_theme_toggle_pseudo() {
        let toggle = ThemeToggle::new().build();
        let groups = toggle.get_pseudo_groups();
        assert!(groups.iter().any(|(pc, _)| *pc == 0x00)); // Pc::Hover
        assert!(groups.iter().any(|(pc, _)| *pc == 0x02)); // Pc::FocusVisible
        assert!(groups.iter().any(|(pc, _)| *pc == 0x03)); // Pc::Active
    }

    #[test]
    fn the_third_state_follows_the_system() {
        use rwire::theme::ThemeMode;
        assert_eq!(
            ThemeToggleMode::from(ThemeMode::System),
            ThemeToggleMode::System
        );
        assert_eq!(
            ThemeToggleMode::from(ThemeMode::Dark),
            ThemeToggleMode::Dark
        );
        let html = ThemeToggle::new()
            .mode(ThemeToggleMode::System)
            .build()
            .to_static_html();
        assert!(
            html.contains("title=\"Theme: follows the system\""),
            "{html}"
        );
        assert!(html.contains("aria-label=\"Toggle theme\""), "{html}");
        // Each state has its own icon.
        let svg = |m| ThemeToggle::new().mode(m).build().to_static_html();
        assert_ne!(svg(ThemeToggleMode::System), svg(ThemeToggleMode::Light));
        assert_ne!(svg(ThemeToggleMode::System), svg(ThemeToggleMode::Dark));
    }

    #[test]
    fn test_theme_toggle_mode() {
        let toggle = ThemeToggle::new().mode(ThemeToggleMode::Dark);
        assert_eq!(toggle.current_mode, ThemeToggleMode::Dark);
    }
}
