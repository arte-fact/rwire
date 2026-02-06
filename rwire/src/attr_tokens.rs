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

define_token_enum! {
    /// Binary-encoded attribute keys.
    ///
    /// Common HTML attribute names encoded as single bytes instead of strings.
    pub enum At(u8) {
        str_method = name;
        mappings = AT_MAPPINGS;

        // HTML core (0x00-0x09)
        Type = 0x00 => "type",
        Name = 0x01 => "name",
        Value = 0x02 => "value",
        Id = 0x03 => "id",
        For = 0x04 => "for",
        Href = 0x05 => "href",
        Target = 0x06 => "target",
        Rel = 0x07 => "rel",
        Placeholder = 0x08 => "placeholder",
        Rows = 0x09 => "rows",

        // Booleans (0x10-0x14)
        Disabled = 0x10 => "disabled",
        Required = 0x11 => "required",
        Readonly = 0x12 => "readonly",
        Checked = 0x13 => "checked",
        Selected = 0x14 => "selected",

        // ARIA (0x20-0x2F)
        Role = 0x20 => "role",
        AriaLabel = 0x21 => "aria-label",
        AriaSelected = 0x22 => "aria-selected",
        AriaInvalid = 0x23 => "aria-invalid",
        AriaModal = 0x24 => "aria-modal",
        AriaBusy = 0x25 => "aria-busy",
        AriaValuenow = 0x26 => "aria-valuenow",
        AriaValuemin = 0x27 => "aria-valuemin",
        AriaValuemax = 0x28 => "aria-valuemax",
        AriaLive = 0x29 => "aria-live",
        Tabindex = 0x2A => "tabindex",
        AriaHidden = 0x2B => "aria-hidden",
        AriaExpanded = 0x2C => "aria-expanded",
        AriaControls = 0x2D => "aria-controls",

        // SVG (0x40-0x49)
        Xmlns = 0x40 => "xmlns",
        ViewBox = 0x41 => "viewBox",
        Fill = 0x42 => "fill",
        Stroke = 0x43 => "stroke",
        StrokeWidth = 0x44 => "stroke-width",
        StrokeLinecap = 0x45 => "stroke-linecap",
        StrokeLinejoin = 0x46 => "stroke-linejoin",
        D = 0x47 => "d",
        Width = 0x48 => "width",
        Height = 0x49 => "height",
    }
}

// ============================================================================
// Attribute Value Enum
// ============================================================================

define_token_enum! {
    /// Binary-encoded attribute values.
    ///
    /// Common HTML attribute values encoded as single bytes.
    pub enum Av(u8) {
        str_method = value;
        mappings = AV_MAPPINGS;

        // Common (0x00-0x03)
        Empty = 0x00 => "",
        True = 0x01 => "true",
        False = 0x02 => "false",
        None = 0x03 => "none",

        // Input types (0x10-0x1B)
        Text = 0x10 => "text",
        Password = 0x11 => "password",
        Email = 0x12 => "email",
        Number = 0x13 => "number",
        Checkbox = 0x14 => "checkbox",
        Radio = 0x15 => "radio",
        Button = 0x16 => "button",
        Submit = 0x17 => "submit",
        Hidden = 0x18 => "hidden",
        Search = 0x19 => "search",
        Tel = 0x1A => "tel",
        Url = 0x1B => "url",

        // ARIA roles (0x20-0x2F)
        RoleButton = 0x20 => "button",
        RoleCheckbox = 0x21 => "checkbox",
        RoleDialog = 0x22 => "dialog",
        RoleSwitch = 0x23 => "switch",
        RoleAlert = 0x24 => "alert",
        RoleStatus = 0x25 => "status",
        RoleTab = 0x26 => "tab",
        RoleTablist = 0x27 => "tablist",
        RoleTabpanel = 0x28 => "tabpanel",
        RoleRadiogroup = 0x29 => "radiogroup",
        RoleRadio = 0x2A => "radio",
        RoleProgressbar = 0x2B => "progressbar",
        RoleList = 0x2C => "list",
        RoleListitem = 0x2D => "listitem",
        RoleNavigation = 0x2E => "navigation",
        RoleSeparator = 0x2F => "separator",

        // Tabindex (0x30-0x31)
        Zero = 0x30 => "0",
        MinusOne = 0x31 => "-1",

        // SVG common (0x40-0x45)
        SvgNs = 0x40 => "http://www.w3.org/2000/svg",
        ViewBox24 = 0x41 => "0 0 24 24",
        ViewBox12 = 0x42 => "0 0 12 12",
        CurrentColor = 0x43 => "currentColor",
        Round = 0x44 => "round",
        Stroke2 = 0x45 => "2",

        // Link targets (0x50-0x52)
        Blank = 0x50 => "_blank",
        Noopener = 0x51 => "noopener",
        NoopenerNoreferrer = 0x52 => "noopener noreferrer",

        // ARIA live (0x58-0x59)
        Polite = 0x58 => "polite",
        Assertive = 0x59 => "assertive",

        // Common values (0x60-0x64)
        V24 = 0x60 => "24",
        V12 = 0x61 => "12",
        V16 = 0x62 => "16",
        V32 = 0x63 => "32",
        V48 = 0x64 => "48",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
