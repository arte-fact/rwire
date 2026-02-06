//! Binary-encoded attribute tokens for compact wire representation.
//!
//! Provides single-byte enum codes for common HTML attribute keys and values,
//! enabling efficient binary encoding without string overhead via the symbol table.
//!
//! # Example
//!
//! ```ignore
//! use rwire::{el, El, At, Av};
//!
//! el(El::Button)
//!     .at(At::Type, Av::Button)           // 4 bytes on wire
//!     .bool_attr(At::Disabled)             // 3 bytes on wire
//!     .at_str(At::AriaLabel, "Close")      // 4-5 bytes on wire
//! ```

// ============================================================================
// Attribute Key Enum
// ============================================================================

/// Binary-encoded attribute keys.
///
/// Common HTML attribute names encoded as single bytes instead of strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum At {
    // HTML core (0x00-0x09)
    Type = 0x00,
    Name = 0x01,
    Value = 0x02,
    Id = 0x03,
    For = 0x04,
    Href = 0x05,
    Target = 0x06,
    Rel = 0x07,
    Placeholder = 0x08,
    Rows = 0x09,

    // Booleans (0x10-0x14)
    Disabled = 0x10,
    Required = 0x11,
    Readonly = 0x12,
    Checked = 0x13,
    Selected = 0x14,

    // ARIA (0x20-0x2F)
    Role = 0x20,
    AriaLabel = 0x21,
    AriaSelected = 0x22,
    AriaInvalid = 0x23,
    AriaModal = 0x24,
    AriaBusy = 0x25,
    AriaValuenow = 0x26,
    AriaValuemin = 0x27,
    AriaValuemax = 0x28,
    AriaLive = 0x29,
    Tabindex = 0x2A,
    AriaHidden = 0x2B,
    AriaExpanded = 0x2C,
    AriaControls = 0x2D,

    // SVG (0x40-0x47)
    Xmlns = 0x40,
    ViewBox = 0x41,
    Fill = 0x42,
    Stroke = 0x43,
    StrokeWidth = 0x44,
    StrokeLinecap = 0x45,
    StrokeLinejoin = 0x46,
    D = 0x47,
    Width = 0x48,
    Height = 0x49,
}

impl At {
    /// Convert to wire protocol byte.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Get the HTML attribute name string.
    pub fn name(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Name => "name",
            Self::Value => "value",
            Self::Id => "id",
            Self::For => "for",
            Self::Href => "href",
            Self::Target => "target",
            Self::Rel => "rel",
            Self::Placeholder => "placeholder",
            Self::Rows => "rows",
            Self::Disabled => "disabled",
            Self::Required => "required",
            Self::Readonly => "readonly",
            Self::Checked => "checked",
            Self::Selected => "selected",
            Self::Role => "role",
            Self::AriaLabel => "aria-label",
            Self::AriaSelected => "aria-selected",
            Self::AriaInvalid => "aria-invalid",
            Self::AriaModal => "aria-modal",
            Self::AriaBusy => "aria-busy",
            Self::AriaValuenow => "aria-valuenow",
            Self::AriaValuemin => "aria-valuemin",
            Self::AriaValuemax => "aria-valuemax",
            Self::AriaLive => "aria-live",
            Self::Tabindex => "tabindex",
            Self::AriaHidden => "aria-hidden",
            Self::AriaExpanded => "aria-expanded",
            Self::AriaControls => "aria-controls",
            Self::Xmlns => "xmlns",
            Self::ViewBox => "viewBox",
            Self::Fill => "fill",
            Self::Stroke => "stroke",
            Self::StrokeWidth => "stroke-width",
            Self::StrokeLinecap => "stroke-linecap",
            Self::StrokeLinejoin => "stroke-linejoin",
            Self::D => "d",
            Self::Width => "width",
            Self::Height => "height",
        }
    }
}

// ============================================================================
// Attribute Value Enum
// ============================================================================

/// Binary-encoded attribute values.
///
/// Common HTML attribute values encoded as single bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Av {
    // Common (0x00-0x03)
    Empty = 0x00,
    True = 0x01,
    False = 0x02,
    None = 0x03,

    // Input types (0x10-0x1B)
    Text = 0x10,
    Password = 0x11,
    Email = 0x12,
    Number = 0x13,
    Checkbox = 0x14,
    Radio = 0x15,
    Button = 0x16,
    Submit = 0x17,
    Hidden = 0x18,
    Search = 0x19,
    Tel = 0x1A,
    Url = 0x1B,

    // ARIA roles (0x20-0x2F)
    RoleButton = 0x20,
    RoleCheckbox = 0x21,
    RoleDialog = 0x22,
    RoleSwitch = 0x23,
    RoleAlert = 0x24,
    RoleStatus = 0x25,
    RoleTab = 0x26,
    RoleTablist = 0x27,
    RoleTabpanel = 0x28,
    RoleRadiogroup = 0x29,
    RoleRadio = 0x2A,
    RoleProgressbar = 0x2B,
    RoleList = 0x2C,
    RoleListitem = 0x2D,
    RoleNavigation = 0x2E,
    RoleSeparator = 0x2F,

    // Tabindex (0x30-0x31)
    Zero = 0x30,
    MinusOne = 0x31,

    // SVG common (0x40-0x45)
    SvgNs = 0x40,
    ViewBox24 = 0x41,
    ViewBox12 = 0x42,
    CurrentColor = 0x43,
    Round = 0x44,
    Stroke2 = 0x45,

    // Link targets (0x50-0x53)
    Blank = 0x50,
    Noopener = 0x51,
    NoopenerNoreferrer = 0x52,

    // ARIA live (0x58-0x5A)
    Polite = 0x58,
    Assertive = 0x59,

    // Common values (0x60-0x67)
    V24 = 0x60,
    V12 = 0x61,
    V16 = 0x62,
    V32 = 0x63,
    V48 = 0x64,
}

impl Av {
    /// Convert to wire protocol byte.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Get the HTML attribute value string.
    pub fn value(self) -> &'static str {
        match self {
            Self::Empty => "",
            Self::True => "true",
            Self::False => "false",
            Self::None => "none",
            Self::Text => "text",
            Self::Password => "password",
            Self::Email => "email",
            Self::Number => "number",
            Self::Checkbox => "checkbox",
            Self::Radio => "radio",
            Self::Button => "button",
            Self::Submit => "submit",
            Self::Hidden => "hidden",
            Self::Search => "search",
            Self::Tel => "tel",
            Self::Url => "url",
            Self::RoleButton => "button",
            Self::RoleCheckbox => "checkbox",
            Self::RoleDialog => "dialog",
            Self::RoleSwitch => "switch",
            Self::RoleAlert => "alert",
            Self::RoleStatus => "status",
            Self::RoleTab => "tab",
            Self::RoleTablist => "tablist",
            Self::RoleTabpanel => "tabpanel",
            Self::RoleRadiogroup => "radiogroup",
            Self::RoleRadio => "radio",
            Self::RoleProgressbar => "progressbar",
            Self::RoleList => "list",
            Self::RoleListitem => "listitem",
            Self::RoleNavigation => "navigation",
            Self::RoleSeparator => "separator",
            Self::Zero => "0",
            Self::MinusOne => "-1",
            Self::SvgNs => "http://www.w3.org/2000/svg",
            Self::ViewBox24 => "0 0 24 24",
            Self::ViewBox12 => "0 0 12 12",
            Self::CurrentColor => "currentColor",
            Self::Round => "round",
            Self::Stroke2 => "2",
            Self::Blank => "_blank",
            Self::Noopener => "noopener",
            Self::NoopenerNoreferrer => "noopener noreferrer",
            Self::Polite => "polite",
            Self::Assertive => "assertive",
            Self::V24 => "24",
            Self::V12 => "12",
            Self::V16 => "16",
            Self::V32 => "32",
            Self::V48 => "48",
        }
    }
}

// ============================================================================
// Lookup Tables for JS Runtime
// ============================================================================

/// Attribute key mappings: (code, js_attr_name).
/// Used to generate tree-shaken AT lookup table in the JS capsule.
pub const AT_MAPPINGS: &[(u8, &str)] = &[
    (0x00, "type"),
    (0x01, "name"),
    (0x02, "value"),
    (0x03, "id"),
    (0x04, "for"),
    (0x05, "href"),
    (0x06, "target"),
    (0x07, "rel"),
    (0x08, "placeholder"),
    (0x09, "rows"),
    (0x10, "disabled"),
    (0x11, "required"),
    (0x12, "readonly"),
    (0x13, "checked"),
    (0x14, "selected"),
    (0x20, "role"),
    (0x21, "aria-label"),
    (0x22, "aria-selected"),
    (0x23, "aria-invalid"),
    (0x24, "aria-modal"),
    (0x25, "aria-busy"),
    (0x26, "aria-valuenow"),
    (0x27, "aria-valuemin"),
    (0x28, "aria-valuemax"),
    (0x29, "aria-live"),
    (0x2A, "tabindex"),
    (0x2B, "aria-hidden"),
    (0x2C, "aria-expanded"),
    (0x2D, "aria-controls"),
    (0x40, "xmlns"),
    (0x41, "viewBox"),
    (0x42, "fill"),
    (0x43, "stroke"),
    (0x44, "stroke-width"),
    (0x45, "stroke-linecap"),
    (0x46, "stroke-linejoin"),
    (0x47, "d"),
    (0x48, "width"),
    (0x49, "height"),
];

/// Attribute value mappings: (code, js_attr_value).
/// Used to generate tree-shaken AV lookup table in the JS capsule.
pub const AV_MAPPINGS: &[(u8, &str)] = &[
    (0x00, ""),
    (0x01, "true"),
    (0x02, "false"),
    (0x03, "none"),
    (0x10, "text"),
    (0x11, "password"),
    (0x12, "email"),
    (0x13, "number"),
    (0x14, "checkbox"),
    (0x15, "radio"),
    (0x16, "button"),
    (0x17, "submit"),
    (0x18, "hidden"),
    (0x19, "search"),
    (0x1A, "tel"),
    (0x1B, "url"),
    (0x20, "button"),
    (0x21, "checkbox"),
    (0x22, "dialog"),
    (0x23, "switch"),
    (0x24, "alert"),
    (0x25, "status"),
    (0x26, "tab"),
    (0x27, "tablist"),
    (0x28, "tabpanel"),
    (0x29, "radiogroup"),
    (0x2A, "radio"),
    (0x2B, "progressbar"),
    (0x2C, "list"),
    (0x2D, "listitem"),
    (0x2E, "navigation"),
    (0x2F, "separator"),
    (0x30, "0"),
    (0x31, "-1"),
    (0x40, "http://www.w3.org/2000/svg"),
    (0x41, "0 0 24 24"),
    (0x42, "0 0 12 12"),
    (0x43, "currentColor"),
    (0x44, "round"),
    (0x45, "2"),
    (0x50, "_blank"),
    (0x51, "noopener"),
    (0x52, "noopener noreferrer"),
    (0x58, "polite"),
    (0x59, "assertive"),
    (0x60, "24"),
    (0x61, "12"),
    (0x62, "16"),
    (0x63, "32"),
    (0x64, "48"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_at_name_matches_mappings() {
        for &(code, name) in AT_MAPPINGS {
            // Find the At variant with this code
            let at_variants: Vec<At> = vec![
                At::Type, At::Name, At::Value, At::Id, At::For, At::Href,
                At::Target, At::Rel, At::Placeholder, At::Rows,
                At::Disabled, At::Required, At::Readonly, At::Checked, At::Selected,
                At::Role, At::AriaLabel, At::AriaSelected, At::AriaInvalid,
                At::AriaModal, At::AriaBusy, At::AriaValuenow, At::AriaValuemin,
                At::AriaValuemax, At::AriaLive, At::Tabindex, At::AriaHidden,
                At::AriaExpanded, At::AriaControls,
                At::Xmlns, At::ViewBox, At::Fill, At::Stroke, At::StrokeWidth,
                At::StrokeLinecap, At::StrokeLinejoin, At::D, At::Width, At::Height,
            ];
            if let Some(at) = at_variants.iter().find(|a| a.as_u8() == code) {
                assert_eq!(at.name(), name, "Mismatch for At code 0x{:02X}", code);
            }
        }
    }

    #[test]
    fn test_av_value_matches_mappings() {
        for &(code, value) in AV_MAPPINGS {
            let av_variants: Vec<Av> = vec![
                Av::Empty, Av::True, Av::False, Av::None,
                Av::Text, Av::Password, Av::Email, Av::Number,
                Av::Checkbox, Av::Radio, Av::Button, Av::Submit,
                Av::Hidden, Av::Search, Av::Tel, Av::Url,
                Av::RoleButton, Av::RoleCheckbox, Av::RoleDialog, Av::RoleSwitch,
                Av::RoleAlert, Av::RoleStatus, Av::RoleTab, Av::RoleTablist,
                Av::RoleTabpanel, Av::RoleRadiogroup, Av::RoleRadio, Av::RoleProgressbar,
                Av::RoleList, Av::RoleListitem, Av::RoleNavigation, Av::RoleSeparator,
                Av::Zero, Av::MinusOne,
                Av::SvgNs, Av::ViewBox24, Av::ViewBox12, Av::CurrentColor,
                Av::Round, Av::Stroke2,
                Av::Blank, Av::Noopener, Av::NoopenerNoreferrer,
                Av::Polite, Av::Assertive,
                Av::V24, Av::V12, Av::V16, Av::V32, Av::V48,
            ];
            if let Some(av) = av_variants.iter().find(|a| a.as_u8() == code) {
                assert_eq!(av.value(), value, "Mismatch for Av code 0x{:02X}", code);
            }
        }
    }

    #[test]
    fn test_at_repr_u8() {
        assert_eq!(At::Type.as_u8(), 0x00);
        assert_eq!(At::Disabled.as_u8(), 0x10);
        assert_eq!(At::Role.as_u8(), 0x20);
        assert_eq!(At::Xmlns.as_u8(), 0x40);
    }

    #[test]
    fn test_av_repr_u8() {
        assert_eq!(Av::Empty.as_u8(), 0x00);
        assert_eq!(Av::True.as_u8(), 0x01);
        assert_eq!(Av::Text.as_u8(), 0x10);
        assert_eq!(Av::RoleButton.as_u8(), 0x20);
        assert_eq!(Av::SvgNs.as_u8(), 0x40);
    }
}
