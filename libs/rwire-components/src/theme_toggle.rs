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

use std::borrow::Cow;

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
    labels: ThemeToggleLabels,
    ghost: bool,
}

/// The toggle's texts: its accessible name (`aria-label`) and the tooltip
/// (`title`) naming each state. English by default.
///
/// ```ignore
/// ThemeToggle::new().labels(ThemeToggleLabels {
///     toggle: "Changer de thème".into(),
///     light: "Thème clair".into(),
///     dark: "Thème sombre".into(),
///     system: "Thème du système".into(),
/// })
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeToggleLabels {
    /// The button's accessible name (`aria-label`).
    pub toggle: Cow<'static, str>,
    /// The tooltip in light mode.
    pub light: Cow<'static, str>,
    /// The tooltip in dark mode.
    pub dark: Cow<'static, str>,
    /// The tooltip when following the system.
    pub system: Cow<'static, str>,
}

impl Default for ThemeToggleLabels {
    fn default() -> Self {
        Self {
            toggle: "Toggle theme".into(),
            light: "Theme: light".into(),
            dark: "Theme: dark".into(),
            system: "Theme: follows the system".into(),
        }
    }
}

impl ThemeToggleLabels {
    /// The tooltip naming `mode`.
    fn title(&self, mode: ThemeToggleMode) -> &str {
        match mode {
            ThemeToggleMode::Light => &self.light,
            ThemeToggleMode::Dark => &self.dark,
            ThemeToggleMode::System => &self.system,
        }
    }
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

impl Default for ThemeToggle {
    fn default() -> Self {
        Self {
            size: ToggleSize::default(),
            show_label: false,
            on_toggle: None,
            current_mode: ThemeToggleMode::Light,
            labels: ThemeToggleLabels::default(),
            ghost: false,
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

    /// The ghost look of a toolbar icon: no ground and no stroke at rest, the
    /// soft ink (`TextMuted`); on hover the raised ground, the hairline and the
    /// ink (`BgSurfaceRaised`, `BorderDefault`, `TextDefault`). Like
    /// `Button::icon_only(…).intent(ButtonIntent::Ghost)`.
    pub fn ghost(mut self) -> Self {
        self.ghost = true;
        self
    }

    /// Set the accessible name and the tooltip of each state (translations).
    pub fn labels(mut self, labels: ThemeToggleLabels) -> Self {
        self.labels = labels;
        self
    }

    /// Set the click handler for the toggle.
    pub fn on_toggle(mut self, handler: HandlerSpec) -> Self {
        self.on_toggle = Some(handler);
        self
    }

    /// Compute style tokens for the theme toggle button.
    pub fn compute_tokens(&self) -> Vec<St> {
        if self.ghost {
            return vec![
                St::DisplayInlineFlex,
                St::ItemsCenter,
                St::JustifyCenter,
                St::BgTransparent,
                St::BorderGhost,
                St::RoundedMd,
                St::TextMuted,
                St::CursorPointer,
                St::TransitionColors,
            ];
        }
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
            // The theme's density sizes the default toggle (`--Qzq`, `--Qzi`).
            ToggleSize::Md => St::PadIconToggle,
            ToggleSize::Lg => St::PSp3,
        }
    }

    fn render_icon(&self) -> ElementBuilder {
        let glyph = match self.current_mode {
            ThemeToggleMode::Light => icon(Icon::Moon),
            ThemeToggleMode::Dark => icon(Icon::Sun),
            ThemeToggleMode::System => icon(Icon::Monitor),
        };
        if self.size == ToggleSize::Md {
            glyph.st([St::GlyphIconToggle])
        } else {
            glyph
        }
    }

    /// Build the theme toggle component.
    pub fn build(self) -> ElementBuilder {
        let icon_el = self.render_icon();
        let mut tokens = self.compute_tokens();
        tokens.push(self.size_token());
        if self.size == ToggleSize::Md {
            tokens.push(St::SqIconToggle);
        }

        let mut button = el(El::Button)
            .st(tokens)
            .hover(if self.ghost {
                vec![St::BgSurfaceRaised, St::BorderDefault, St::TextDefault]
            } else {
                vec![St::BgHover]
            })
            .focus_visible([St::RingFocus])
            .active([St::Scale98])
            .at(At::Type, Av::Button)
            .attr("aria-label", &self.labels.toggle)
            .attr("title", self.labels.title(self.current_mode))
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
    fn labels_reach_the_aria_label_and_each_state_title() {
        let fr = || ThemeToggleLabels {
            toggle: "Changer de thème".into(),
            light: "Thème clair".into(),
            dark: "Thème sombre".into(),
            system: "Thème du système".into(),
        };
        for (mode, title) in [
            (ThemeToggleMode::Light, "Thème clair"),
            (ThemeToggleMode::Dark, "Thème sombre"),
            (ThemeToggleMode::System, "Thème du système"),
        ] {
            let html = ThemeToggle::new()
                .labels(fr())
                .mode(mode)
                .build()
                .to_static_html();
            assert!(html.contains("aria-label=\"Changer de thème\""), "{html}");
            assert!(html.contains(&format!("title=\"{title}\"")), "{html}");
        }
        // The default stays English.
        let html = ThemeToggle::new()
            .mode(ThemeToggleMode::Dark)
            .build()
            .to_static_html();
        assert!(html.contains("aria-label=\"Toggle theme\""), "{html}");
        assert!(html.contains("title=\"Theme: dark\""), "{html}");
    }

    #[test]
    fn the_default_toggle_follows_the_density_hooks() {
        let btn = ThemeToggle::new().build();
        let utils = btn.get_style_utils();
        assert!(utils.contains(&St::SqIconToggle.as_u16()));
        assert!(utils.contains(&St::PadIconToggle.as_u16()));
        assert!(btn.children()[0]
            .get_style_utils()
            .contains(&St::GlyphIconToggle.as_u16()));
        // Comfortable fallbacks: 8 px padding around a 24 px glyph, as before.
        assert_eq!(St::PadIconToggle.css(), "padding:var(--Qzq,var(--S2))");
        assert_eq!(
            St::GlyphIconToggle.css(),
            "width:var(--Qzg,24px);height:var(--Qzg,24px)"
        );
        // Other sizes keep theirs.
        let sm = ThemeToggle::new().size(ToggleSize::Sm).build();
        assert!(!sm.get_style_utils().contains(&St::SqIconToggle.as_u16()));
    }

    #[test]
    fn a_ghost_toggle_rests_on_nothing_and_raises_on_hover() {
        let btn = ThemeToggle::new().ghost().build();
        let rest = btn.get_style_utils();
        for st in [St::BgTransparent, St::TextMuted, St::BorderGhost] {
            assert!(rest.contains(&st.as_u16()), "{st:?} at rest");
        }
        assert!(!rest.contains(&St::BorderDefault.as_u16()));
        let hover: Vec<u16> = btn
            .get_pseudo_groups()
            .iter()
            .filter(|(pc, _)| *pc == 0x00)
            .flat_map(|(_, c)| c.iter().copied())
            .collect();
        for st in [St::BgSurfaceRaised, St::BorderDefault, St::TextDefault] {
            assert!(hover.contains(&st.as_u16()), "{st:?} on hover");
        }
        // Still sized by the density hooks.
        assert!(rest.contains(&St::SqIconToggle.as_u16()));
        // The default toggle keeps its border.
        assert!(ThemeToggle::new()
            .build()
            .get_style_utils()
            .contains(&St::BorderDefault.as_u16()));
    }

    #[test]
    fn test_theme_toggle_mode() {
        let toggle = ThemeToggle::new().mode(ThemeToggleMode::Dark);
        assert_eq!(toggle.current_mode, ThemeToggleMode::Dark);
    }
}
