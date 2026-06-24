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
use crate::style_tokens::St;
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
    Archive,
    ArchiveRestore,
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
    LogOut,
    Calendar,
    Clock,

    // Brand (fill-based)
    GitHub,
    Discord,
    Twitter,

    // Dev
    Crate,
    Terminal,
    Clipboard,
    ClipboardCheck,

    // Feature (landing page)
    Zap,
    Feather,
    Shield,
    Cpu,
    Palette,
    Leaf,

    // Content & comms
    MessageSquare,
    Activity,
    FileText,
    Folder,
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
            Icon::Archive => "M1 3h22v5H1z M21 8v13H3V8 M10 12h4",
            Icon::ArchiveRestore => "M2 3h20v5H2z M4 8v11a2 2 0 0 0 2 2h2 M20 8v11a2 2 0 0 1-2 2h-2 M9 15l3-3 3 3 M12 12v9",
            Icon::Copy =>"M8 4v12a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V7.242a2 2 0 0 0-.602-1.43L16.083 2.57A2 2 0 0 0 14.685 2H10a2 2 0 0 0-2 2zM16 18v2a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2h2",
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
            Icon::LogOut => "M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4 M16 17l5-5-5-5 M21 12H9",
            Icon::Calendar =>"M19 4h-1V2h-2v2H8V2H6v2H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2zM21 20H3V10h18v10zM3 8V6h18v2H3z",
            Icon::Clock => "M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10zM12 6v6l4 2",

            // Brand (fill-based)
            Icon::GitHub => "M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0 1 12 6.844a9.59 9.59 0 0 1 2.504.337c1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.02 10.02 0 0 0 22 12.017C22 6.484 17.522 2 12 2z",
            Icon::Discord => "M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994a.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128c.126-.094.252-.192.372-.291a.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z",
            Icon::Twitter => "M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z",

            // Dev
            Icon::Crate => "M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z M3.27 6.96L12 12.01l8.73-5.05M12 22.08V12",
            Icon::Terminal => "M4 17l6-5-6-5M12 19h8",
            Icon::Clipboard => "M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2M9 2h6a1 1 0 0 1 1 1v2a1 1 0 0 1-1 1H9a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1z",
            Icon::ClipboardCheck => "M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2M9 2h6a1 1 0 0 1 1 1v2a1 1 0 0 1-1 1H9a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1zM9 14l2 2 4-4",

            // Feature (landing page)
            Icon::Zap => "M13 2L3 14h9l-1 8 10-12h-9l1-8z",
            Icon::Feather => "M20.24 12.24a6 6 0 0 0-8.49-8.49L5 10.5V19h8.5zM16 8L2 22M17.5 15H9",
            Icon::Shield => "M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z",
            Icon::Cpu => "M18 4H6a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2zM9 9h6v6H9V9zM9 1v3M15 1v3M9 20v3M15 20v3M20 9h3M20 14h3M1 9h3M1 14h3",
            Icon::Palette => "M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.555C21.965 6.012 17.461 2 12 2zM6.5 12a1.5 1.5 0 1 1 0-3 1.5 1.5 0 0 1 0 3zM9.5 8a1.5 1.5 0 1 1 0-3 1.5 1.5 0 0 1 0 3zM14.5 8a1.5 1.5 0 1 1 0-3 1.5 1.5 0 0 1 0 3zM17.5 12a1.5 1.5 0 1 1 0-3 1.5 1.5 0 0 1 0 3z",
            Icon::Leaf => "M17 8C8 10 5.9 16.17 3.82 21.34l1.89.66.95-2.3c.48.17.98.3 1.34.3C19 20 22 3 22 3c-1 2-8 2.25-13 3.25S2 11.5 2 13.5s1.75 3.75 1.75 3.75",

            // Content & comms
            Icon::MessageSquare => "M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z",
            Icon::Activity => "M22 12h-4l-3 9L9 3l-3 9H2",
            Icon::FileText => "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z M14 2v6h6 M16 13H8 M16 17H8 M10 9H8",
            Icon::Folder => "M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z",
        }
    }

    /// Returns true for brand icons that use fill instead of stroke.
    fn is_fill_based(&self) -> bool {
        matches!(self, Icon::GitHub | Icon::Discord | Icon::Twitter)
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
pub fn icon(i: Icon) -> ElementBuilder {
    let mut svg = el(El::Svg)
        .st([
            St::DisplayInlineBlock,
            St::VerticalAlignMiddle,
            St::FlexShrink0,
        ])
        .at(At::Xmlns, Av::SvgNs)
        .at(At::Width, Av::V24)
        .at(At::Height, Av::V24)
        .at(At::ViewBox, Av::ViewBox24);

    if i.is_fill_based() {
        svg = svg.at(At::Fill, Av::CurrentColor);
    } else {
        svg = svg
            .at(At::Fill, Av::None)
            .at(At::Stroke, Av::CurrentColor)
            .at(At::StrokeWidth, Av::Stroke2)
            .at(At::StrokeLinecap, Av::Round)
            .at(At::StrokeLinejoin, Av::Round);
    }

    svg.append([el(El::Path).at_str(At::D, i.svg_path())])
}

/// Creates an icon element with a custom CSS class.
///
/// # Arguments
///
/// * `icon` - The icon to render
/// * `class` - CSS class name to apply (escape hatch for custom styling)
///
/// # Example
///
/// ```rust
/// use rwire::{el, El};
/// use rwire::icons::{icon_with_class, Icon};
///
/// let large_icon = icon_with_class(Icon::Check, "custom-icon");
/// ```
pub fn icon_with_class(i: Icon, class: &str) -> ElementBuilder {
    let mut svg = el(El::Svg)
        .class(class)
        .at(At::Xmlns, Av::SvgNs)
        .at(At::Width, Av::V24)
        .at(At::Height, Av::V24)
        .at(At::ViewBox, Av::ViewBox24);

    if i.is_fill_based() {
        svg = svg.at(At::Fill, Av::CurrentColor);
    } else {
        svg = svg
            .at(At::Fill, Av::None)
            .at(At::Stroke, Av::CurrentColor)
            .at(At::StrokeWidth, Av::Stroke2)
            .at(At::StrokeLinecap, Av::Round)
            .at(At::StrokeLinejoin, Av::Round);
    }

    svg.append([el(El::Path).at_str(At::D, i.svg_path())])
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
pub fn icon_sized(i: Icon, size: u32) -> ElementBuilder {
    let size_str = size.to_string();
    let mut svg = el(El::Svg)
        .st([
            St::DisplayInlineBlock,
            St::VerticalAlignMiddle,
            St::FlexShrink0,
        ])
        .at(At::Xmlns, Av::SvgNs)
        .attr("width", &size_str)
        .attr("height", &size_str)
        .at(At::ViewBox, Av::ViewBox24);

    if i.is_fill_based() {
        svg = svg.at(At::Fill, Av::CurrentColor);
    } else {
        svg = svg
            .at(At::Fill, Av::None)
            .at(At::Stroke, Av::CurrentColor)
            .at(At::StrokeWidth, Av::Stroke2)
            .at(At::StrokeLinecap, Av::Round)
            .at(At::StrokeLinejoin, Av::Round);
    }

    svg.append([el(El::Path).at_str(At::D, i.svg_path())])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_icons_have_paths() {
        // Ensure all icon variants return non-empty paths
        let icons = [
            Icon::ChevronDown,
            Icon::ChevronUp,
            Icon::ChevronLeft,
            Icon::ChevronRight,
            Icon::ArrowLeft,
            Icon::ArrowRight,
            Icon::ArrowUp,
            Icon::ArrowDown,
            Icon::Menu,
            Icon::Close,
            Icon::Check,
            Icon::Plus,
            Icon::Minus,
            Icon::Edit,
            Icon::Trash,
            Icon::Archive,
            Icon::ArchiveRestore,
            Icon::Copy,
            Icon::Download,
            Icon::Upload,
            Icon::Search,
            Icon::Filter,
            Icon::Info,
            Icon::Warning,
            Icon::Error,
            Icon::Success,
            Icon::AlertCircle,
            Icon::CheckCircle,
            Icon::Sun,
            Icon::Moon,
            Icon::Play,
            Icon::Pause,
            Icon::Settings,
            Icon::User,
            Icon::Home,
            Icon::External,
            Icon::LogOut,
            Icon::Calendar,
            Icon::Clock,
            Icon::GitHub,
            Icon::Discord,
            Icon::Twitter,
            Icon::Crate,
            Icon::Terminal,
            Icon::Clipboard,
            Icon::ClipboardCheck,
            Icon::Zap,
            Icon::Feather,
            Icon::Shield,
            Icon::Cpu,
            Icon::Palette,
            Icon::Leaf,
            Icon::MessageSquare,
            Icon::Activity,
            Icon::FileText,
            Icon::Folder,
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
