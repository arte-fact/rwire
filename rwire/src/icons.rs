//! Inline SVG icons for rwire components.
//!
//! This module provides a lightweight icon system using inline SVG strings.
//! Icons are designed to be embedded directly in ElementBuilder chains.
//!
//! # Example
//!
//! ```rust
//! use rwire::{el, El};
//! use rwire::icons::{icon, Icon};
//!
//! let button = el(El::Button)
//!     .class("btn")
//!     .append([
//!         icon(Icon::Check),
//!         el(El::Span).text("Confirm"),
//!     ]);
//! ```

use crate::attr_tokens::{At, Av};
use crate::{el, El, ElementBuilder};

/// Icon identifiers for all available icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    // Navigation & UI
    ChevronDown,
    ChevronUp,
    ChevronLeft,
    ChevronRight,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Menu,
    Close,

    // Actions
    Check,
    Plus,
    Minus,
    Edit,
    Trash,
    Copy,
    Download,
    Upload,
    Search,
    Filter,

    // Status & Feedback
    Info,
    Warning,
    Error,
    Success,
    AlertCircle,
    CheckCircle,

    // Theme
    Sun,
    Moon,

    // Media
    Play,
    Pause,

    // Misc
    Settings,
    User,
    Home,
    External,
    Calendar,
    Clock,
}

impl Icon {
    /// Returns the SVG path data for this icon.
    /// All icons are designed for a 24x24 viewBox.
    fn svg_path(&self) -> &'static str {
        match self {
            // Navigation & UI
            Icon::ChevronDown => "M6 9l6 6 6-6",
            Icon::ChevronUp => "M18 15l-6-6-6 6",
            Icon::ChevronLeft => "M15 18l-6-6 6-6",
            Icon::ChevronRight => "M9 18l6-6-6-6",
            Icon::ArrowLeft => "M19 12H5M12 19l-7-7 7-7",
            Icon::ArrowRight => "M5 12h14M12 5l7 7-7 7",
            Icon::ArrowUp => "M12 19V5M5 12l7-7 7 7",
            Icon::ArrowDown => "M12 5v14M19 12l-7 7-7-7",
            Icon::Menu => "M3 12h18M3 6h18M3 18h18",
            Icon::Close => "M18 6L6 18M6 6l12 12",

            // Actions
            Icon::Check => "M20 6L9 17l-5-5",
            Icon::Plus => "M12 5v14M5 12h14",
            Icon::Minus => "M5 12h14",
            Icon::Edit => "M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z",
            Icon::Trash => "M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2M10 11v6M14 11v6",
            Icon::Copy => "M8 4v12a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V7.242a2 2 0 0 0-.602-1.43L16.083 2.57A2 2 0 0 0 14.685 2H10a2 2 0 0 0-2 2zM16 18v2a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2h2",
            Icon::Download => "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M7 10l5 5 5-5M12 15V3",
            Icon::Upload => "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M17 8l-5-5-5 5M12 3v12",
            Icon::Search => "M21 21l-6-6m2-5a7 7 0 1 1-14 0 7 7 0 0 1 14 0z",
            Icon::Filter => "M22 3H2l8 9.46V19l4 2v-8.54L22 3z",

            // Status & Feedback
            Icon::Info => "M12 16v-4M12 8h.01M22 12c0 5.523-4.477 10-10 10S2 17.523 2 12 6.477 2 12 2s10 4.477 10 10z",
            Icon::Warning => "M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0zM12 9v4M12 17h.01",
            Icon::Error => "M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10zM15 9l-6 6M9 9l6 6",
            Icon::Success => "M22 11.08V12a10 10 0 1 1-5.93-9.14M22 4L12 14.01l-3-3",
            Icon::AlertCircle => "M12 8v4M12 16h.01M22 12c0 5.523-4.477 10-10 10S2 17.523 2 12 6.477 2 12 2s10 4.477 10 10z",
            Icon::CheckCircle => "M22 11.08V12a10 10 0 1 1-5.93-9.14M22 4L12 14.01l-3-3",

            // Theme
            Icon::Sun => "M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0z",
            Icon::Moon => "M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z",

            // Media
            Icon::Play => "M5 3l14 9-14 9V3z",
            Icon::Pause => "M10 4H6v16h4V4zM18 4h-4v16h4V4z",

            // Misc
            Icon::Settings => "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z",
            Icon::User => "M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2M12 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8z",
            Icon::Home => "M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z M9 22V12h6v10",
            Icon::External => "M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6M15 3h6v6M10 14L21 3",
            Icon::Calendar => "M19 4h-1V2h-2v2H8V2H6v2H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2zM21 20H3V10h18v10zM3 8V6h18v2H3z",
            Icon::Clock => "M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10zM12 6v6l4 2",
        }
    }

}

/// Creates an icon element.
///
/// # Arguments
///
/// * `icon` - The icon to render
///
/// # Example
///
/// ```rust
/// use rwire::{el, El};
/// use rwire::icons::{icon, Icon};
///
/// let close_button = el(El::Button)
///     .append([icon(Icon::Close)]);
/// ```
pub fn icon(icon: Icon) -> ElementBuilder {
    icon_with_class(icon, "rw-icon")
}

/// Creates an icon element with a custom CSS class.
///
/// # Arguments
///
/// * `icon` - The icon to render
/// * `class` - CSS class name to apply
///
/// # Example
///
/// ```rust
/// use rwire::{el, El};
/// use rwire::icons::{icon_with_class, Icon};
///
/// let large_icon = icon_with_class(Icon::Check, "rw-icon rw-icon-lg");
/// ```
pub fn icon_with_class(icon: Icon, class: &str) -> ElementBuilder {
    el(El::Svg)
        .class(class)
        .at(At::Xmlns, Av::SvgNs)
        .at(At::Width, Av::V24)
        .at(At::Height, Av::V24)
        .at(At::ViewBox, Av::ViewBox24)
        .at(At::Fill, Av::None)
        .at(At::Stroke, Av::CurrentColor)
        .at(At::StrokeWidth, Av::Stroke2)
        .at(At::StrokeLinecap, Av::Round)
        .at(At::StrokeLinejoin, Av::Round)
        .append([
            el(El::Path).at_str(At::D, icon.svg_path())
        ])
}

/// Creates an icon element with custom size.
///
/// # Arguments
///
/// * `icon` - The icon to render
/// * `size` - Size in pixels (both width and height)
///
/// # Example
///
/// ```rust
/// use rwire::icons::{icon_sized, Icon};
///
/// let small_icon = icon_sized(Icon::Search, 16);
/// let large_icon = icon_sized(Icon::Menu, 32);
/// ```
pub fn icon_sized(icon: Icon, size: u32) -> ElementBuilder {
    let size_str = size.to_string();
    el(El::Svg)
        .class("rw-icon")
        .at(At::Xmlns, Av::SvgNs)
        .attr("width", &size_str)
        .attr("height", &size_str)
        .at(At::ViewBox, Av::ViewBox24)
        .at(At::Fill, Av::None)
        .at(At::Stroke, Av::CurrentColor)
        .at(At::StrokeWidth, Av::Stroke2)
        .at(At::StrokeLinecap, Av::Round)
        .at(At::StrokeLinejoin, Av::Round)
        .append([
            el(El::Path).at_str(At::D, icon.svg_path())
        ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_icons_have_paths() {
        // Ensure all icon variants return non-empty paths
        let icons = [
            Icon::ChevronDown, Icon::ChevronUp, Icon::ChevronLeft, Icon::ChevronRight,
            Icon::ArrowLeft, Icon::ArrowRight, Icon::ArrowUp, Icon::ArrowDown,
            Icon::Menu, Icon::Close,
            Icon::Check, Icon::Plus, Icon::Minus, Icon::Edit, Icon::Trash,
            Icon::Copy, Icon::Download, Icon::Upload, Icon::Search, Icon::Filter,
            Icon::Info, Icon::Warning, Icon::Error, Icon::Success,
            Icon::AlertCircle, Icon::CheckCircle,
            Icon::Sun, Icon::Moon,
            Icon::Play, Icon::Pause,
            Icon::Settings, Icon::User, Icon::Home, Icon::External,
            Icon::Calendar, Icon::Clock,
        ];

        for icon in &icons {
            let path = icon.svg_path();
            assert!(!path.is_empty(), "Icon {:?} has empty path", icon);
        }
    }

    #[test]
    fn test_icon_builder_has_svg_attributes() {
        let builder = icon(Icon::Check);
        // The builder should create an SVG element with proper structure
        // This is a basic smoke test - full rendering is tested in integration
        drop(builder); // Just ensure it compiles and drops cleanly
    }

    #[test]
    fn test_icon_sized_custom_dimensions() {
        let builder = icon_sized(Icon::Search, 16);
        drop(builder);

        let builder = icon_sized(Icon::Menu, 48);
        drop(builder);
    }
}
