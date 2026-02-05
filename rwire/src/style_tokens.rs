//! Binary-encoded style tokens for compact wire representation.
//!
//! This module provides single-byte codes for CSS properties and values,
//! enabling efficient binary encoding of inline styles without string overhead.
//!
//! # Architecture
//!
//! Three encoding strategies, from most to least compact:
//!
//! 1. **Utility tokens** (3 bytes): Pre-combined property+value
//!    ```text
//!    [STYLE_UTIL, ref, util_byte]
//!    ```
//!
//! 2. **Property+Value** (4 bytes): Flexible combinations
//!    ```text
//!    [STYLE_PROP, ref, prop_byte, value_byte]
//!    ```
//!
//! 3. **Symbol table** (variable): Escape hatch for custom values
//!    ```text
//!    [STYLE_SET, ref, symbol_idx]
//!    ```
//!
//! # Example
//!
//! ```ignore
//! use rwire::{el, El, St};
//!
//! // Most compact: utility tokens (3 bytes each)
//! el(El::Div).st([St::BgApp, St::MinHScreen, St::FlexCenter])
//!
//! // Semantic colors adapt to light/dark theme
//! el(El::Span).st([St::TextDefault, St::TextLg])
//!
//! // Escape hatch: symbol table (variable)
//! el(El::Div).style("width: calc(100% - 20px)")
//! ```

// ============================================================================
// Utility Tokens (Pre-combined property+value)
// ============================================================================

/// Pre-combined style utilities for maximum compactness.
///
/// Each utility maps to a complete CSS declaration like "display:flex".
/// Short name `St` for concise component code.
///
/// # Semantic Colors
///
/// Semantic color utilities (BgApp, TextDefault, etc.) use CSS variables
/// that adapt to light/dark theme automatically.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum St {
    // Display (0x00-0x0F)
    DisplayNone = 0x00,
    DisplayBlock = 0x01,
    DisplayFlex = 0x02,
    DisplayGrid = 0x03,
    DisplayInline = 0x04,
    DisplayInlineFlex = 0x05,
    DisplayInlineBlock = 0x06,
    DisplayContents = 0x07,

    // Flex Direction (0x10-0x17)
    FlexRow = 0x10,
    FlexCol = 0x11,
    FlexRowReverse = 0x12,
    FlexColReverse = 0x13,
    FlexWrap = 0x14,
    FlexNoWrap = 0x15,

    // Justify Content (0x18-0x1F)
    JustifyStart = 0x18,
    JustifyEnd = 0x19,
    JustifyCenter = 0x1A,
    JustifyBetween = 0x1B,
    JustifyAround = 0x1C,
    JustifyEvenly = 0x1D,

    // Align Items (0x20-0x27)
    ItemsStart = 0x20,
    ItemsEnd = 0x21,
    ItemsCenter = 0x22,
    ItemsStretch = 0x23,
    ItemsBaseline = 0x24,

    // Gap - Design Tokens (0x28-0x2F)
    Gap0 = 0x28,
    GapXs = 0x29,  // 0.25rem
    GapSm = 0x2A,  // 0.5rem
    GapMd = 0x2B,  // 1rem
    GapLg = 0x2C,  // 1.5rem
    GapXl = 0x2D,  // 2rem
    Gap2xl = 0x2E, // 3rem

    // Position (0x30-0x37)
    PositionRelative = 0x30,
    PositionAbsolute = 0x31,
    PositionFixed = 0x32,
    PositionSticky = 0x33,
    PositionStatic = 0x34,

    // Inset (0x38-0x3F)
    Inset0 = 0x38,
    InsetAuto = 0x39,
    Top0 = 0x3A,
    Right0 = 0x3B,
    Bottom0 = 0x3C,
    Left0 = 0x3D,

    // Width/Height (0x40-0x4F)
    WFull = 0x40,   // width: 100%
    WAuto = 0x41,   // width: auto
    WScreen = 0x42, // width: 100vw
    HFull = 0x43,   // height: 100%
    HAuto = 0x44,   // height: auto
    HScreen = 0x45, // height: 100vh
    MinW0 = 0x46,
    MinH0 = 0x47,
    MaxWFull = 0x48,
    MaxHFull = 0x49,

    // Padding - Design Tokens (0x50-0x5F)
    P0 = 0x50,
    PXs = 0x51,
    PSm = 0x52,
    PMd = 0x53,
    PLg = 0x54,
    PXl = 0x55,
    PxXs = 0x56, // padding-inline
    PxSm = 0x57,
    PxMd = 0x58,
    PyXs = 0x59, // padding-block
    PySm = 0x5A,
    PyMd = 0x5B,

    // Margin - Design Tokens (0x60-0x6F)
    M0 = 0x60,
    MXs = 0x61,
    MSm = 0x62,
    MMd = 0x63,
    MLg = 0x64,
    MXl = 0x65,
    MAuto = 0x66,
    MxAuto = 0x67, // margin-inline: auto (centering)
    MyAuto = 0x68,

    // Text (0x70-0x7F)
    TextLeft = 0x70,
    TextCenter = 0x71,
    TextRight = 0x72,
    TextJustify = 0x73,
    TextXs = 0x74,  // font-size
    TextSm = 0x75,
    TextBase = 0x76,
    TextLg = 0x77,
    TextXl = 0x78,
    Text2xl = 0x79,
    FontNormal = 0x7A,
    FontMedium = 0x7B,
    FontSemibold = 0x7C,
    FontBold = 0x7D,
    Truncate = 0x7E, // text-overflow: ellipsis + overflow: hidden + white-space: nowrap

    // Overflow (0x80-0x87)
    OverflowHidden = 0x80,
    OverflowAuto = 0x81,
    OverflowScroll = 0x82,
    OverflowVisible = 0x83,
    OverflowXAuto = 0x84,
    OverflowYAuto = 0x85,

    // Border Radius - Design Tokens (0x88-0x8F)
    Rounded0 = 0x88,
    RoundedSm = 0x89,
    RoundedMd = 0x8A,
    RoundedLg = 0x8B,
    RoundedXl = 0x8C,
    RoundedFull = 0x8D,

    // Opacity (0x90-0x97)
    Opacity0 = 0x90,
    Opacity25 = 0x91,
    Opacity50 = 0x92,
    Opacity75 = 0x93,
    Opacity100 = 0x94,

    // Pointer/Cursor (0x98-0x9F)
    CursorPointer = 0x98,
    CursorDefault = 0x99,
    CursorNotAllowed = 0x9A,
    CursorWait = 0x9B,
    CursorText = 0x9C,
    PointerEventsNone = 0x9D,
    PointerEventsAuto = 0x9E,

    // Z-Index (0xA0-0xA7)
    Z0 = 0xA0,
    Z10 = 0xA1,
    Z20 = 0xA2,
    Z30 = 0xA3,
    Z40 = 0xA4,
    Z50 = 0xA5,
    ZAuto = 0xA6,

    // Visibility (0xA8-0xAF)
    Visible = 0xA8,
    Invisible = 0xA9, // visibility: hidden
    SrOnly = 0xAA,    // screen-reader only

    // Common Composites (0xB0-0xBF)
    FlexCenter = 0xB0, // display:flex;justify-content:center;align-items:center
    AbsoluteFill = 0xB1, // position:absolute;inset:0
    FixedFill = 0xB2,    // position:fixed;inset:0
    FlexGrow = 0xB3,     // flex-grow:1
    FlexShrink0 = 0xB4,  // flex-shrink:0
    Flex1 = 0xB5,        // flex:1

    // ========================================================================
    // Semantic Colors (0xC0-0xDF) - Theme-aware via CSS variables
    // ========================================================================

    // Background - Semantic (0xC0-0xC7)
    BgApp = 0xC0,       // background:var(--rw-bg-app)
    BgSubtle = 0xC1,    // background:var(--rw-bg-subtle)
    BgMuted = 0xC2,     // background:var(--rw-bg-muted)
    BgEmphasis = 0xC3,  // background:var(--rw-bg-emphasis)
    BgHover = 0xC4,     // background:var(--rw-bg-hover)
    BgActive = 0xC5,    // background:var(--rw-bg-active)
    BgAccent = 0xC6,    // background:var(--rw-accent-9)
    BgAccentHover = 0xC7, // background:var(--rw-accent-10)

    // Text - Semantic (0xC8-0xCF)
    TextDefault = 0xC8, // color:var(--rw-text-default)
    TextHigh = 0xC9,    // color:var(--rw-text-high)
    TextMuted = 0xCA,   // color:var(--rw-text-muted)
    TextOnAccent = 0xCB, // color:var(--rw-text-on-accent)
    TextSuccess = 0xCC, // color:var(--rw-success)
    TextWarning = 0xCD, // color:var(--rw-warning)
    TextError = 0xCE,   // color:var(--rw-error)
    TextAccent = 0xCF,  // color:var(--rw-accent-11)

    // Border - Semantic (0xD0-0xD7)
    BorderDefault = 0xD0, // border:1px solid var(--rw-border-default)
    BorderSubtle = 0xD1,  // border:1px solid var(--rw-border-subtle)
    BorderEmphasis = 0xD2, // border:1px solid var(--rw-border-emphasis)
    BorderAccent = 0xD3,  // border:1px solid var(--rw-accent-7)
    BorderNone = 0xD4,    // border:none

    // Additional Layout (0xD8-0xDF)
    MinHScreen = 0xD8,  // min-height:100vh
    MinWScreen = 0xD9,  // min-width:100vw
    MaxWMd = 0xDA,      // max-width:28rem (md breakpoint)
    MaxWLg = 0xDB,      // max-width:32rem (lg breakpoint)
    MaxWXl = 0xDC,      // max-width:36rem (xl breakpoint)
    MaxW2xl = 0xDD,     // max-width:42rem (2xl breakpoint)

    // Transition (0xE0-0xE3)
    TransitionAll = 0xE0,    // transition:all 0.2s
    TransitionColors = 0xE1, // transition:color,background-color 0.2s
    TransitionOpacity = 0xE2, // transition:opacity 0.2s
    TransitionTransform = 0xE3, // transition:transform 0.2s
}

impl St {
    /// Convert to wire protocol byte.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Get the CSS declaration for this utility.
    pub fn css(self) -> &'static str {
        match self {
            // Display
            Self::DisplayNone => "display:none",
            Self::DisplayBlock => "display:block",
            Self::DisplayFlex => "display:flex",
            Self::DisplayGrid => "display:grid",
            Self::DisplayInline => "display:inline",
            Self::DisplayInlineFlex => "display:inline-flex",
            Self::DisplayInlineBlock => "display:inline-block",
            Self::DisplayContents => "display:contents",

            // Flex Direction
            Self::FlexRow => "flex-direction:row",
            Self::FlexCol => "flex-direction:column",
            Self::FlexRowReverse => "flex-direction:row-reverse",
            Self::FlexColReverse => "flex-direction:column-reverse",
            Self::FlexWrap => "flex-wrap:wrap",
            Self::FlexNoWrap => "flex-wrap:nowrap",

            // Justify Content
            Self::JustifyStart => "justify-content:flex-start",
            Self::JustifyEnd => "justify-content:flex-end",
            Self::JustifyCenter => "justify-content:center",
            Self::JustifyBetween => "justify-content:space-between",
            Self::JustifyAround => "justify-content:space-around",
            Self::JustifyEvenly => "justify-content:space-evenly",

            // Align Items
            Self::ItemsStart => "align-items:flex-start",
            Self::ItemsEnd => "align-items:flex-end",
            Self::ItemsCenter => "align-items:center",
            Self::ItemsStretch => "align-items:stretch",
            Self::ItemsBaseline => "align-items:baseline",

            // Gap
            Self::Gap0 => "gap:0",
            Self::GapXs => "gap:var(--rw-space-1)",
            Self::GapSm => "gap:var(--rw-space-2)",
            Self::GapMd => "gap:var(--rw-space-4)",
            Self::GapLg => "gap:var(--rw-space-6)",
            Self::GapXl => "gap:var(--rw-space-8)",
            Self::Gap2xl => "gap:var(--rw-space-12)",

            // Position
            Self::PositionRelative => "position:relative",
            Self::PositionAbsolute => "position:absolute",
            Self::PositionFixed => "position:fixed",
            Self::PositionSticky => "position:sticky",
            Self::PositionStatic => "position:static",

            // Inset
            Self::Inset0 => "inset:0",
            Self::InsetAuto => "inset:auto",
            Self::Top0 => "top:0",
            Self::Right0 => "right:0",
            Self::Bottom0 => "bottom:0",
            Self::Left0 => "left:0",

            // Width/Height
            Self::WFull => "width:100%",
            Self::WAuto => "width:auto",
            Self::WScreen => "width:100vw",
            Self::HFull => "height:100%",
            Self::HAuto => "height:auto",
            Self::HScreen => "height:100vh",
            Self::MinW0 => "min-width:0",
            Self::MinH0 => "min-height:0",
            Self::MaxWFull => "max-width:100%",
            Self::MaxHFull => "max-height:100%",

            // Padding
            Self::P0 => "padding:0",
            Self::PXs => "padding:var(--rw-space-1)",
            Self::PSm => "padding:var(--rw-space-2)",
            Self::PMd => "padding:var(--rw-space-4)",
            Self::PLg => "padding:var(--rw-space-6)",
            Self::PXl => "padding:var(--rw-space-8)",
            Self::PxXs => "padding-inline:var(--rw-space-1)",
            Self::PxSm => "padding-inline:var(--rw-space-2)",
            Self::PxMd => "padding-inline:var(--rw-space-4)",
            Self::PyXs => "padding-block:var(--rw-space-1)",
            Self::PySm => "padding-block:var(--rw-space-2)",
            Self::PyMd => "padding-block:var(--rw-space-4)",

            // Margin
            Self::M0 => "margin:0",
            Self::MXs => "margin:var(--rw-space-1)",
            Self::MSm => "margin:var(--rw-space-2)",
            Self::MMd => "margin:var(--rw-space-4)",
            Self::MLg => "margin:var(--rw-space-6)",
            Self::MXl => "margin:var(--rw-space-8)",
            Self::MAuto => "margin:auto",
            Self::MxAuto => "margin-inline:auto",
            Self::MyAuto => "margin-block:auto",

            // Text
            Self::TextLeft => "text-align:left",
            Self::TextCenter => "text-align:center",
            Self::TextRight => "text-align:right",
            Self::TextJustify => "text-align:justify",
            Self::TextXs => "font-size:var(--rw-text-xs)",
            Self::TextSm => "font-size:var(--rw-text-sm)",
            Self::TextBase => "font-size:var(--rw-text-base)",
            Self::TextLg => "font-size:var(--rw-text-lg)",
            Self::TextXl => "font-size:var(--rw-text-xl)",
            Self::Text2xl => "font-size:var(--rw-text-2xl)",
            Self::FontNormal => "font-weight:400",
            Self::FontMedium => "font-weight:500",
            Self::FontSemibold => "font-weight:600",
            Self::FontBold => "font-weight:700",
            Self::Truncate => "overflow:hidden;text-overflow:ellipsis;white-space:nowrap",

            // Overflow
            Self::OverflowHidden => "overflow:hidden",
            Self::OverflowAuto => "overflow:auto",
            Self::OverflowScroll => "overflow:scroll",
            Self::OverflowVisible => "overflow:visible",
            Self::OverflowXAuto => "overflow-x:auto",
            Self::OverflowYAuto => "overflow-y:auto",

            // Border Radius
            Self::Rounded0 => "border-radius:0",
            Self::RoundedSm => "border-radius:var(--rw-radius-sm)",
            Self::RoundedMd => "border-radius:var(--rw-radius-md)",
            Self::RoundedLg => "border-radius:var(--rw-radius-lg)",
            Self::RoundedXl => "border-radius:var(--rw-radius-xl)",
            Self::RoundedFull => "border-radius:9999px",

            // Opacity
            Self::Opacity0 => "opacity:0",
            Self::Opacity25 => "opacity:0.25",
            Self::Opacity50 => "opacity:0.5",
            Self::Opacity75 => "opacity:0.75",
            Self::Opacity100 => "opacity:1",

            // Cursor/Pointer
            Self::CursorPointer => "cursor:pointer",
            Self::CursorDefault => "cursor:default",
            Self::CursorNotAllowed => "cursor:not-allowed",
            Self::CursorWait => "cursor:wait",
            Self::CursorText => "cursor:text",
            Self::PointerEventsNone => "pointer-events:none",
            Self::PointerEventsAuto => "pointer-events:auto",

            // Z-Index
            Self::Z0 => "z-index:0",
            Self::Z10 => "z-index:10",
            Self::Z20 => "z-index:20",
            Self::Z30 => "z-index:30",
            Self::Z40 => "z-index:40",
            Self::Z50 => "z-index:50",
            Self::ZAuto => "z-index:auto",

            // Visibility
            Self::Visible => "visibility:visible",
            Self::Invisible => "visibility:hidden",
            Self::SrOnly => "position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);border:0",

            // Composites
            Self::FlexCenter => "display:flex;justify-content:center;align-items:center",
            Self::AbsoluteFill => "position:absolute;inset:0",
            Self::FixedFill => "position:fixed;inset:0",
            Self::FlexGrow => "flex-grow:1",
            Self::FlexShrink0 => "flex-shrink:0",
            Self::Flex1 => "flex:1",

            // Background - Semantic
            Self::BgApp => "background:var(--rw-bg-app)",
            Self::BgSubtle => "background:var(--rw-bg-subtle)",
            Self::BgMuted => "background:var(--rw-bg-muted)",
            Self::BgEmphasis => "background:var(--rw-bg-emphasis)",
            Self::BgHover => "background:var(--rw-bg-hover)",
            Self::BgActive => "background:var(--rw-bg-active)",
            Self::BgAccent => "background:var(--rw-accent-9)",
            Self::BgAccentHover => "background:var(--rw-accent-10)",

            // Text - Semantic
            Self::TextDefault => "color:var(--rw-text-default)",
            Self::TextHigh => "color:var(--rw-text-high)",
            Self::TextMuted => "color:var(--rw-text-muted)",
            Self::TextOnAccent => "color:var(--rw-text-on-accent)",
            Self::TextSuccess => "color:var(--rw-success)",
            Self::TextWarning => "color:var(--rw-warning)",
            Self::TextError => "color:var(--rw-error)",
            Self::TextAccent => "color:var(--rw-accent-11)",

            // Border - Semantic
            Self::BorderDefault => "border:1px solid var(--rw-border-default)",
            Self::BorderSubtle => "border:1px solid var(--rw-border-subtle)",
            Self::BorderEmphasis => "border:1px solid var(--rw-border-emphasis)",
            Self::BorderAccent => "border:1px solid var(--rw-accent-7)",
            Self::BorderNone => "border:none",

            // Additional Layout
            Self::MinHScreen => "min-height:100vh",
            Self::MinWScreen => "min-width:100vw",
            Self::MaxWMd => "max-width:28rem",
            Self::MaxWLg => "max-width:32rem",
            Self::MaxWXl => "max-width:36rem",
            Self::MaxW2xl => "max-width:42rem",

            // Transition
            Self::TransitionAll => "transition:all 0.2s",
            Self::TransitionColors => "transition:color,background-color 0.2s",
            Self::TransitionOpacity => "transition:opacity 0.2s",
            Self::TransitionTransform => "transition:transform 0.2s",
        }
    }
}


// ============================================================================
// Style Properties (for property+value encoding)
// ============================================================================

/// CSS property codes for binary encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum StyleProp {
    Display = 0x00,
    Position = 0x01,
    FlexDirection = 0x02,
    FlexWrap = 0x03,
    JustifyContent = 0x04,
    AlignItems = 0x05,
    AlignSelf = 0x06,
    Gap = 0x07,
    Width = 0x08,
    Height = 0x09,
    MinWidth = 0x0A,
    MaxWidth = 0x0B,
    MinHeight = 0x0C,
    MaxHeight = 0x0D,
    Padding = 0x0E,
    PaddingInline = 0x0F,
    PaddingBlock = 0x10,
    Margin = 0x11,
    MarginInline = 0x12,
    MarginBlock = 0x13,
    Top = 0x14,
    Right = 0x15,
    Bottom = 0x16,
    Left = 0x17,
    Inset = 0x18,
    Overflow = 0x19,
    OverflowX = 0x1A,
    OverflowY = 0x1B,
    TextAlign = 0x1C,
    FontSize = 0x1D,
    FontWeight = 0x1E,
    BorderRadius = 0x1F,
    Opacity = 0x20,
    Cursor = 0x21,
    ZIndex = 0x22,
    Visibility = 0x23,
    Flex = 0x24,
    FlexGrow = 0x25,
    FlexShrink = 0x26,
    PointerEvents = 0x27,
    WhiteSpace = 0x28,
    TextOverflow = 0x29,
}

impl StyleProp {
    /// Convert to wire protocol byte.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Get the CSS property name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Display => "display",
            Self::Position => "position",
            Self::FlexDirection => "flex-direction",
            Self::FlexWrap => "flex-wrap",
            Self::JustifyContent => "justify-content",
            Self::AlignItems => "align-items",
            Self::AlignSelf => "align-self",
            Self::Gap => "gap",
            Self::Width => "width",
            Self::Height => "height",
            Self::MinWidth => "min-width",
            Self::MaxWidth => "max-width",
            Self::MinHeight => "min-height",
            Self::MaxHeight => "max-height",
            Self::Padding => "padding",
            Self::PaddingInline => "padding-inline",
            Self::PaddingBlock => "padding-block",
            Self::Margin => "margin",
            Self::MarginInline => "margin-inline",
            Self::MarginBlock => "margin-block",
            Self::Top => "top",
            Self::Right => "right",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Inset => "inset",
            Self::Overflow => "overflow",
            Self::OverflowX => "overflow-x",
            Self::OverflowY => "overflow-y",
            Self::TextAlign => "text-align",
            Self::FontSize => "font-size",
            Self::FontWeight => "font-weight",
            Self::BorderRadius => "border-radius",
            Self::Opacity => "opacity",
            Self::Cursor => "cursor",
            Self::ZIndex => "z-index",
            Self::Visibility => "visibility",
            Self::Flex => "flex",
            Self::FlexGrow => "flex-grow",
            Self::FlexShrink => "flex-shrink",
            Self::PointerEvents => "pointer-events",
            Self::WhiteSpace => "white-space",
            Self::TextOverflow => "text-overflow",
        }
    }
}

// ============================================================================
// Style Values
// ============================================================================

/// CSS value codes for binary encoding.
///
/// Values are organized by type to allow property+value combinations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum StyleValue {
    // Keywords (0x00-0x1F)
    None = 0x00,
    Auto = 0x01,
    Hidden = 0x02,
    Visible = 0x03,
    Scroll = 0x04,
    Inherit = 0x05,
    Initial = 0x06,

    // Display values (0x10-0x1F)
    Block = 0x10,
    Flex = 0x11,
    Grid = 0x12,
    Inline = 0x13,
    InlineFlex = 0x14,
    InlineBlock = 0x15,
    Contents = 0x16,

    // Position values (0x20-0x27)
    Relative = 0x20,
    Absolute = 0x21,
    Fixed = 0x22,
    Sticky = 0x23,
    Static = 0x24,

    // Flex values (0x28-0x3F)
    Row = 0x28,
    Column = 0x29,
    RowReverse = 0x2A,
    ColumnReverse = 0x2B,
    Wrap = 0x2C,
    Nowrap = 0x2D,
    FlexStart = 0x2E,
    FlexEnd = 0x2F,
    Center = 0x30,
    SpaceBetween = 0x31,
    SpaceAround = 0x32,
    SpaceEvenly = 0x33,
    Stretch = 0x34,
    Baseline = 0x35,

    // Size values - percentages (0x40-0x4F)
    Full = 0x40,   // 100%
    Half = 0x41,   // 50%
    Third = 0x42,  // 33.333%
    Quarter = 0x43, // 25%
    Screen = 0x44,  // 100vw or 100vh

    // Space tokens (0x50-0x5F) - maps to --rw-space-N
    Space0 = 0x50, // 0
    Space1 = 0x51, // 0.25rem
    Space2 = 0x52, // 0.5rem
    Space3 = 0x53, // 0.75rem
    Space4 = 0x54, // 1rem
    Space5 = 0x55, // 1.25rem
    Space6 = 0x56, // 1.5rem
    Space8 = 0x57, // 2rem
    Space10 = 0x58, // 2.5rem
    Space12 = 0x59, // 3rem
    Space16 = 0x5A, // 4rem

    // Text sizes (0x60-0x6F) - maps to --rw-text-N
    TextXs = 0x60,
    TextSm = 0x61,
    TextBase = 0x62,
    TextLg = 0x63,
    TextXl = 0x64,
    Text2xl = 0x65,
    Text3xl = 0x66,

    // Font weights (0x70-0x77)
    Weight400 = 0x70,
    Weight500 = 0x71,
    Weight600 = 0x72,
    Weight700 = 0x73,

    // Border radius (0x78-0x7F) - maps to --rw-radius-N
    RadiusNone = 0x78,
    RadiusSm = 0x79,
    RadiusMd = 0x7A,
    RadiusLg = 0x7B,
    RadiusXl = 0x7C,
    RadiusFull = 0x7D, // 9999px

    // Opacity (0x80-0x87)
    Opacity0 = 0x80,
    Opacity25 = 0x81,
    Opacity50 = 0x82,
    Opacity75 = 0x83,
    Opacity100 = 0x84,

    // Cursor (0x88-0x8F)
    Pointer = 0x88,
    Default = 0x89,
    NotAllowed = 0x8A,
    Wait = 0x8B,
    Text = 0x8C,

    // Z-index (0x90-0x97)
    Z0 = 0x90,
    Z10 = 0x91,
    Z20 = 0x92,
    Z30 = 0x93,
    Z40 = 0x94,
    Z50 = 0x95,

    // Text align (0x98-0x9F)
    Left = 0x98,
    Right = 0x99,
    // Center already defined at 0x30

    // Numeric (0xA0-0xAF)
    N0 = 0xA0,
    N1 = 0xA1,
}

impl StyleValue {
    /// Convert to wire protocol byte.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Get the CSS value string.
    pub fn css(self) -> &'static str {
        match self {
            // Keywords
            Self::None => "none",
            Self::Auto => "auto",
            Self::Hidden => "hidden",
            Self::Visible => "visible",
            Self::Scroll => "scroll",
            Self::Inherit => "inherit",
            Self::Initial => "initial",

            // Display
            Self::Block => "block",
            Self::Flex => "flex",
            Self::Grid => "grid",
            Self::Inline => "inline",
            Self::InlineFlex => "inline-flex",
            Self::InlineBlock => "inline-block",
            Self::Contents => "contents",

            // Position
            Self::Relative => "relative",
            Self::Absolute => "absolute",
            Self::Fixed => "fixed",
            Self::Sticky => "sticky",
            Self::Static => "static",

            // Flex
            Self::Row => "row",
            Self::Column => "column",
            Self::RowReverse => "row-reverse",
            Self::ColumnReverse => "column-reverse",
            Self::Wrap => "wrap",
            Self::Nowrap => "nowrap",
            Self::FlexStart => "flex-start",
            Self::FlexEnd => "flex-end",
            Self::Center => "center",
            Self::SpaceBetween => "space-between",
            Self::SpaceAround => "space-around",
            Self::SpaceEvenly => "space-evenly",
            Self::Stretch => "stretch",
            Self::Baseline => "baseline",

            // Percentages
            Self::Full => "100%",
            Self::Half => "50%",
            Self::Third => "33.333%",
            Self::Quarter => "25%",
            Self::Screen => "100vw", // context-dependent

            // Space tokens
            Self::Space0 => "0",
            Self::Space1 => "var(--rw-space-1)",
            Self::Space2 => "var(--rw-space-2)",
            Self::Space3 => "var(--rw-space-3)",
            Self::Space4 => "var(--rw-space-4)",
            Self::Space5 => "var(--rw-space-5)",
            Self::Space6 => "var(--rw-space-6)",
            Self::Space8 => "var(--rw-space-8)",
            Self::Space10 => "var(--rw-space-10)",
            Self::Space12 => "var(--rw-space-12)",
            Self::Space16 => "var(--rw-space-16)",

            // Text sizes
            Self::TextXs => "var(--rw-text-xs)",
            Self::TextSm => "var(--rw-text-sm)",
            Self::TextBase => "var(--rw-text-base)",
            Self::TextLg => "var(--rw-text-lg)",
            Self::TextXl => "var(--rw-text-xl)",
            Self::Text2xl => "var(--rw-text-2xl)",
            Self::Text3xl => "var(--rw-text-3xl)",

            // Font weights
            Self::Weight400 => "400",
            Self::Weight500 => "500",
            Self::Weight600 => "600",
            Self::Weight700 => "700",

            // Border radius
            Self::RadiusNone => "0",
            Self::RadiusSm => "var(--rw-radius-sm)",
            Self::RadiusMd => "var(--rw-radius-md)",
            Self::RadiusLg => "var(--rw-radius-lg)",
            Self::RadiusXl => "var(--rw-radius-xl)",
            Self::RadiusFull => "9999px",

            // Opacity
            Self::Opacity0 => "0",
            Self::Opacity25 => "0.25",
            Self::Opacity50 => "0.5",
            Self::Opacity75 => "0.75",
            Self::Opacity100 => "1",

            // Cursor
            Self::Pointer => "pointer",
            Self::Default => "default",
            Self::NotAllowed => "not-allowed",
            Self::Wait => "wait",
            Self::Text => "text",

            // Z-index
            Self::Z0 => "0",
            Self::Z10 => "10",
            Self::Z20 => "20",
            Self::Z30 => "30",
            Self::Z40 => "40",
            Self::Z50 => "50",

            // Text align
            Self::Left => "left",
            Self::Right => "right",

            // Numeric
            Self::N0 => "0",
            Self::N1 => "1",
        }
    }
}

// ============================================================================
// Mapping Generation (for JS runtime)
// ============================================================================

/// All utility token mappings for JS runtime generation.
pub const UTIL_MAPPINGS: &[(u8, &str)] = &[
    // Display
    (0x00, "display:none"),
    (0x01, "display:block"),
    (0x02, "display:flex"),
    (0x03, "display:grid"),
    (0x04, "display:inline"),
    (0x05, "display:inline-flex"),
    (0x06, "display:inline-block"),
    (0x07, "display:contents"),
    // Flex Direction
    (0x10, "flex-direction:row"),
    (0x11, "flex-direction:column"),
    (0x12, "flex-direction:row-reverse"),
    (0x13, "flex-direction:column-reverse"),
    (0x14, "flex-wrap:wrap"),
    (0x15, "flex-wrap:nowrap"),
    // Justify
    (0x18, "justify-content:flex-start"),
    (0x19, "justify-content:flex-end"),
    (0x1A, "justify-content:center"),
    (0x1B, "justify-content:space-between"),
    (0x1C, "justify-content:space-around"),
    (0x1D, "justify-content:space-evenly"),
    // Align
    (0x20, "align-items:flex-start"),
    (0x21, "align-items:flex-end"),
    (0x22, "align-items:center"),
    (0x23, "align-items:stretch"),
    (0x24, "align-items:baseline"),
    // Gap
    (0x28, "gap:0"),
    (0x29, "gap:var(--rw-space-1)"),
    (0x2A, "gap:var(--rw-space-2)"),
    (0x2B, "gap:var(--rw-space-4)"),
    (0x2C, "gap:var(--rw-space-6)"),
    (0x2D, "gap:var(--rw-space-8)"),
    (0x2E, "gap:var(--rw-space-12)"),
    // Position
    (0x30, "position:relative"),
    (0x31, "position:absolute"),
    (0x32, "position:fixed"),
    (0x33, "position:sticky"),
    (0x34, "position:static"),
    // Inset
    (0x38, "inset:0"),
    (0x39, "inset:auto"),
    (0x3A, "top:0"),
    (0x3B, "right:0"),
    (0x3C, "bottom:0"),
    (0x3D, "left:0"),
    // Width/Height
    (0x40, "width:100%"),
    (0x41, "width:auto"),
    (0x42, "width:100vw"),
    (0x43, "height:100%"),
    (0x44, "height:auto"),
    (0x45, "height:100vh"),
    (0x46, "min-width:0"),
    (0x47, "min-height:0"),
    (0x48, "max-width:100%"),
    (0x49, "max-height:100%"),
    // Padding
    (0x50, "padding:0"),
    (0x51, "padding:var(--rw-space-1)"),
    (0x52, "padding:var(--rw-space-2)"),
    (0x53, "padding:var(--rw-space-4)"),
    (0x54, "padding:var(--rw-space-6)"),
    (0x55, "padding:var(--rw-space-8)"),
    (0x56, "padding-inline:var(--rw-space-1)"),
    (0x57, "padding-inline:var(--rw-space-2)"),
    (0x58, "padding-inline:var(--rw-space-4)"),
    (0x59, "padding-block:var(--rw-space-1)"),
    (0x5A, "padding-block:var(--rw-space-2)"),
    (0x5B, "padding-block:var(--rw-space-4)"),
    // Margin
    (0x60, "margin:0"),
    (0x61, "margin:var(--rw-space-1)"),
    (0x62, "margin:var(--rw-space-2)"),
    (0x63, "margin:var(--rw-space-4)"),
    (0x64, "margin:var(--rw-space-6)"),
    (0x65, "margin:var(--rw-space-8)"),
    (0x66, "margin:auto"),
    (0x67, "margin-inline:auto"),
    (0x68, "margin-block:auto"),
    // Text
    (0x70, "text-align:left"),
    (0x71, "text-align:center"),
    (0x72, "text-align:right"),
    (0x73, "text-align:justify"),
    (0x74, "font-size:var(--rw-text-xs)"),
    (0x75, "font-size:var(--rw-text-sm)"),
    (0x76, "font-size:var(--rw-text-base)"),
    (0x77, "font-size:var(--rw-text-lg)"),
    (0x78, "font-size:var(--rw-text-xl)"),
    (0x79, "font-size:var(--rw-text-2xl)"),
    (0x7A, "font-weight:400"),
    (0x7B, "font-weight:500"),
    (0x7C, "font-weight:600"),
    (0x7D, "font-weight:700"),
    (0x7E, "overflow:hidden;text-overflow:ellipsis;white-space:nowrap"),
    // Overflow
    (0x80, "overflow:hidden"),
    (0x81, "overflow:auto"),
    (0x82, "overflow:scroll"),
    (0x83, "overflow:visible"),
    (0x84, "overflow-x:auto"),
    (0x85, "overflow-y:auto"),
    // Border Radius
    (0x88, "border-radius:0"),
    (0x89, "border-radius:var(--rw-radius-sm)"),
    (0x8A, "border-radius:var(--rw-radius-md)"),
    (0x8B, "border-radius:var(--rw-radius-lg)"),
    (0x8C, "border-radius:var(--rw-radius-xl)"),
    (0x8D, "border-radius:9999px"),
    // Opacity
    (0x90, "opacity:0"),
    (0x91, "opacity:0.25"),
    (0x92, "opacity:0.5"),
    (0x93, "opacity:0.75"),
    (0x94, "opacity:1"),
    // Cursor
    (0x98, "cursor:pointer"),
    (0x99, "cursor:default"),
    (0x9A, "cursor:not-allowed"),
    (0x9B, "cursor:wait"),
    (0x9C, "cursor:text"),
    (0x9D, "pointer-events:none"),
    (0x9E, "pointer-events:auto"),
    // Z-index
    (0xA0, "z-index:0"),
    (0xA1, "z-index:10"),
    (0xA2, "z-index:20"),
    (0xA3, "z-index:30"),
    (0xA4, "z-index:40"),
    (0xA5, "z-index:50"),
    (0xA6, "z-index:auto"),
    // Visibility
    (0xA8, "visibility:visible"),
    (0xA9, "visibility:hidden"),
    (0xAA, "position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);border:0"),
    // Composites
    (0xB0, "display:flex;justify-content:center;align-items:center"),
    (0xB1, "position:absolute;inset:0"),
    (0xB2, "position:fixed;inset:0"),
    (0xB3, "flex-grow:1"),
    (0xB4, "flex-shrink:0"),
    (0xB5, "flex:1"),
    // Background - Semantic
    (0xC0, "background:var(--rw-bg-app)"),
    (0xC1, "background:var(--rw-bg-subtle)"),
    (0xC2, "background:var(--rw-bg-muted)"),
    (0xC3, "background:var(--rw-bg-emphasis)"),
    (0xC4, "background:var(--rw-bg-hover)"),
    (0xC5, "background:var(--rw-bg-active)"),
    (0xC6, "background:var(--rw-accent-9)"),
    (0xC7, "background:var(--rw-accent-10)"),
    // Text - Semantic
    (0xC8, "color:var(--rw-text-default)"),
    (0xC9, "color:var(--rw-text-high)"),
    (0xCA, "color:var(--rw-text-muted)"),
    (0xCB, "color:var(--rw-text-on-accent)"),
    (0xCC, "color:var(--rw-success)"),
    (0xCD, "color:var(--rw-warning)"),
    (0xCE, "color:var(--rw-error)"),
    (0xCF, "color:var(--rw-accent-11)"),
    // Border - Semantic
    (0xD0, "border:1px solid var(--rw-border-default)"),
    (0xD1, "border:1px solid var(--rw-border-subtle)"),
    (0xD2, "border:1px solid var(--rw-border-emphasis)"),
    (0xD3, "border:1px solid var(--rw-accent-7)"),
    (0xD4, "border:none"),
    // Additional Layout
    (0xD8, "min-height:100vh"),
    (0xD9, "min-width:100vw"),
    (0xDA, "max-width:28rem"),
    (0xDB, "max-width:32rem"),
    (0xDC, "max-width:36rem"),
    (0xDD, "max-width:42rem"),
    // Transition
    (0xE0, "transition:all 0.2s"),
    (0xE1, "transition:color,background-color 0.2s"),
    (0xE2, "transition:opacity 0.2s"),
    (0xE3, "transition:transform 0.2s"),
];

/// All property mappings for JS runtime generation.
pub const PROP_MAPPINGS: &[(u8, &str)] = &[
    (0x00, "display"),
    (0x01, "position"),
    (0x02, "flex-direction"),
    (0x03, "flex-wrap"),
    (0x04, "justify-content"),
    (0x05, "align-items"),
    (0x06, "align-self"),
    (0x07, "gap"),
    (0x08, "width"),
    (0x09, "height"),
    (0x0A, "min-width"),
    (0x0B, "max-width"),
    (0x0C, "min-height"),
    (0x0D, "max-height"),
    (0x0E, "padding"),
    (0x0F, "padding-inline"),
    (0x10, "padding-block"),
    (0x11, "margin"),
    (0x12, "margin-inline"),
    (0x13, "margin-block"),
    (0x14, "top"),
    (0x15, "right"),
    (0x16, "bottom"),
    (0x17, "left"),
    (0x18, "inset"),
    (0x19, "overflow"),
    (0x1A, "overflow-x"),
    (0x1B, "overflow-y"),
    (0x1C, "text-align"),
    (0x1D, "font-size"),
    (0x1E, "font-weight"),
    (0x1F, "border-radius"),
    (0x20, "opacity"),
    (0x21, "cursor"),
    (0x22, "z-index"),
    (0x23, "visibility"),
    (0x24, "flex"),
    (0x25, "flex-grow"),
    (0x26, "flex-shrink"),
    (0x27, "pointer-events"),
    (0x28, "white-space"),
    (0x29, "text-overflow"),
];

/// All value mappings for JS runtime generation.
pub const VALUE_MAPPINGS: &[(u8, &str)] = &[
    // Keywords
    (0x00, "none"),
    (0x01, "auto"),
    (0x02, "hidden"),
    (0x03, "visible"),
    (0x04, "scroll"),
    (0x05, "inherit"),
    (0x06, "initial"),
    // Display
    (0x10, "block"),
    (0x11, "flex"),
    (0x12, "grid"),
    (0x13, "inline"),
    (0x14, "inline-flex"),
    (0x15, "inline-block"),
    (0x16, "contents"),
    // Position
    (0x20, "relative"),
    (0x21, "absolute"),
    (0x22, "fixed"),
    (0x23, "sticky"),
    (0x24, "static"),
    // Flex
    (0x28, "row"),
    (0x29, "column"),
    (0x2A, "row-reverse"),
    (0x2B, "column-reverse"),
    (0x2C, "wrap"),
    (0x2D, "nowrap"),
    (0x2E, "flex-start"),
    (0x2F, "flex-end"),
    (0x30, "center"),
    (0x31, "space-between"),
    (0x32, "space-around"),
    (0x33, "space-evenly"),
    (0x34, "stretch"),
    (0x35, "baseline"),
    // Sizes
    (0x40, "100%"),
    (0x41, "50%"),
    (0x42, "33.333%"),
    (0x43, "25%"),
    (0x44, "100vw"),
    // Space tokens
    (0x50, "0"),
    (0x51, "var(--rw-space-1)"),
    (0x52, "var(--rw-space-2)"),
    (0x53, "var(--rw-space-3)"),
    (0x54, "var(--rw-space-4)"),
    (0x55, "var(--rw-space-5)"),
    (0x56, "var(--rw-space-6)"),
    (0x57, "var(--rw-space-8)"),
    (0x58, "var(--rw-space-10)"),
    (0x59, "var(--rw-space-12)"),
    (0x5A, "var(--rw-space-16)"),
    // Text sizes
    (0x60, "var(--rw-text-xs)"),
    (0x61, "var(--rw-text-sm)"),
    (0x62, "var(--rw-text-base)"),
    (0x63, "var(--rw-text-lg)"),
    (0x64, "var(--rw-text-xl)"),
    (0x65, "var(--rw-text-2xl)"),
    (0x66, "var(--rw-text-3xl)"),
    // Font weights
    (0x70, "400"),
    (0x71, "500"),
    (0x72, "600"),
    (0x73, "700"),
    // Border radius
    (0x78, "0"),
    (0x79, "var(--rw-radius-sm)"),
    (0x7A, "var(--rw-radius-md)"),
    (0x7B, "var(--rw-radius-lg)"),
    (0x7C, "var(--rw-radius-xl)"),
    (0x7D, "9999px"),
    // Opacity
    (0x80, "0"),
    (0x81, "0.25"),
    (0x82, "0.5"),
    (0x83, "0.75"),
    (0x84, "1"),
    // Cursor
    (0x88, "pointer"),
    (0x89, "default"),
    (0x8A, "not-allowed"),
    (0x8B, "wait"),
    (0x8C, "text"),
    // Z-index
    (0x90, "0"),
    (0x91, "10"),
    (0x92, "20"),
    (0x93, "30"),
    (0x94, "40"),
    (0x95, "50"),
    // Text align
    (0x98, "left"),
    (0x99, "right"),
    // Numeric
    (0xA0, "0"),
    (0xA1, "1"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_util_mappings_consistent() {
        // Verify UTIL_MAPPINGS matches St enum
        for util in [
            St::DisplayFlex,
            St::FlexCol,
            St::GapMd,
            St::FlexCenter,
            St::BgApp,
            St::TextDefault,
            St::MinHScreen,
        ] {
            let code = util.as_u8();
            let mapping = UTIL_MAPPINGS.iter().find(|(c, _)| *c == code);
            assert!(
                mapping.is_some(),
                "St::{:?} (0x{:02X}) not in UTIL_MAPPINGS",
                util,
                code
            );
            let (_, css) = mapping.unwrap();
            assert_eq!(
                util.css(),
                *css,
                "Mismatch for St::{:?}",
                util
            );
        }
    }

    #[test]
    fn test_prop_mappings_consistent() {
        for prop in [
            StyleProp::Display,
            StyleProp::Gap,
            StyleProp::Padding,
        ] {
            let code = prop.as_u8();
            let mapping = PROP_MAPPINGS.iter().find(|(c, _)| *c == code);
            assert!(
                mapping.is_some(),
                "StyleProp::{:?} (0x{:02X}) not in PROP_MAPPINGS",
                prop,
                code
            );
        }
    }

    #[test]
    fn test_no_duplicate_codes() {
        use std::collections::HashSet;

        let mut seen = HashSet::new();
        for (code, _) in UTIL_MAPPINGS {
            assert!(
                seen.insert(code),
                "Duplicate utility code: 0x{:02X}",
                code
            );
        }

        seen.clear();
        for (code, _) in PROP_MAPPINGS {
            assert!(
                seen.insert(code),
                "Duplicate property code: 0x{:02X}",
                code
            );
        }

        seen.clear();
        for (code, _) in VALUE_MAPPINGS {
            assert!(
                seen.insert(code),
                "Duplicate value code: 0x{:02X}",
                code
            );
        }
    }
}
