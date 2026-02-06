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
#[repr(u16)]
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

    // Transition (0xE0-0xE7)
    TransitionAll = 0xE0,      // transition:all 0.2s
    TransitionColors = 0xE1,   // transition:color,background-color 0.2s
    TransitionOpacity = 0xE2,  // transition:opacity 0.2s
    TransitionTransform = 0xE3, // transition:transform 0.2s
    TransitionNone = 0xE4,     // transition:none
    TransitionFast = 0xE5,     // transition:all 0.1s
    TransitionSlow = 0xE6,     // transition:all 0.3s

    // Shadow (0xE8-0xEF) - Box shadows
    ShadowNone = 0xE8,   // box-shadow:none
    ShadowSm = 0xE9,     // box-shadow:var(--rw-shadow-sm)
    ShadowMd = 0xEA,     // box-shadow:var(--rw-shadow-md)
    ShadowLg = 0xEB,     // box-shadow:var(--rw-shadow-lg)
    ShadowXl = 0xEC,     // box-shadow:var(--rw-shadow-xl)
    ShadowInner = 0xED,  // box-shadow:inset 0 2px 4px rgba(0,0,0,0.1)

    // Outline (0xF0-0xF7) - Focus styles
    OutlineNone = 0xF0,      // outline:none
    OutlineAccent = 0xF1,    // outline:2px solid var(--rw-accent-8)
    OutlineOffset2 = 0xF2,   // outline-offset:2px
    RingAccent = 0xF3,       // box-shadow:0 0 0 2px var(--rw-accent-8)
    RingInset = 0xF4,        // box-shadow:inset 0 0 0 1px var(--rw-border-default)

    // ========================================================================
    // Extended CSS3 (0x100+) - Using two-byte encoding
    // ========================================================================

    // Text Decoration (0x100-0x107)
    Underline = 0x100,       // text-decoration:underline
    LineThrough = 0x101,     // text-decoration:line-through
    NoUnderline = 0x102,     // text-decoration:none

    // Whitespace & Text (0x108-0x10F)
    WhitespaceNormal = 0x108,  // white-space:normal
    WhitespaceNowrap = 0x109,  // white-space:nowrap
    WhitespacePre = 0x10A,     // white-space:pre
    WhitespacePreWrap = 0x10B, // white-space:pre-wrap
    WordBreakNormal = 0x10C,   // word-break:normal
    WordBreakAll = 0x10D,      // word-break:break-all
    WordBreakKeep = 0x10E,     // word-break:keep-all
    BreakWords = 0x10F,        // overflow-wrap:break-word

    // User Interaction (0x110-0x117)
    SelectNone = 0x110,    // user-select:none
    SelectText = 0x111,    // user-select:text
    SelectAll = 0x112,     // user-select:all
    TouchNone = 0x113,     // touch-action:none
    TouchPan = 0x114,      // touch-action:pan-x pan-y
    ScrollSmooth = 0x115,  // scroll-behavior:smooth
    ScrollAuto = 0x116,    // scroll-behavior:auto

    // Background Extended (0x118-0x11F)
    BgTransparent = 0x118, // background:transparent
    BgCurrentColor = 0x119, // background:currentColor
    BgWhite = 0x11A,       // background:var(--rw-white)
    BgBlack = 0x11B,       // background:var(--rw-black)
    BgSuccess = 0x11C,     // background:var(--rw-success)
    BgWarning = 0x11D,     // background:var(--rw-warning)
    BgError = 0x11E,       // background:var(--rw-error)

    // Text Extended (0x120-0x127)
    TextWhite = 0x120,     // color:var(--rw-white)
    TextBlack = 0x121,     // color:var(--rw-black)
    TextInherit = 0x122,   // color:inherit
    TextCurrentColor = 0x123, // color:currentColor
    Text3xl = 0x124,       // font-size:var(--rw-text-3xl)
    Text4xl = 0x125,       // font-size:var(--rw-text-4xl)

    // Line Height (0x128-0x12F)
    LeadingTight = 0x128,   // line-height:var(--rw-leading-tight)
    LeadingSnug = 0x129,    // line-height:var(--rw-leading-snug)
    LeadingNormal = 0x12A,  // line-height:var(--rw-leading-normal)
    LeadingRelaxed = 0x12B, // line-height:var(--rw-leading-relaxed)
    LeadingLoose = 0x12C,   // line-height:var(--rw-leading-loose)
    LeadingNone = 0x12D,    // line-height:1

    // Letter Spacing (0x130-0x137)
    TrackingTighter = 0x130, // letter-spacing:-0.05em
    TrackingTight = 0x131,   // letter-spacing:-0.025em
    TrackingNormal = 0x132,  // letter-spacing:0
    TrackingWide = 0x133,    // letter-spacing:0.025em
    TrackingWider = 0x134,   // letter-spacing:0.05em
    TrackingWidest = 0x135,  // letter-spacing:0.1em

    // Aspect Ratio (0x138-0x13F)
    AspectAuto = 0x138,    // aspect-ratio:auto
    AspectSquare = 0x139,  // aspect-ratio:1
    AspectVideo = 0x13A,   // aspect-ratio:16/9
    AspectPortrait = 0x13B, // aspect-ratio:3/4

    // Object Fit (0x140-0x147)
    ObjectContain = 0x140, // object-fit:contain
    ObjectCover = 0x141,   // object-fit:cover
    ObjectFill = 0x142,    // object-fit:fill
    ObjectNone = 0x143,    // object-fit:none
    ObjectScaleDown = 0x144, // object-fit:scale-down

    // Grid (0x148-0x15F) - Note: DisplayGrid already exists at 0x03
    GridCols1 = 0x149,       // grid-template-columns:repeat(1,minmax(0,1fr))
    GridCols2 = 0x14A,       // grid-template-columns:repeat(2,minmax(0,1fr))
    GridCols3 = 0x14B,       // grid-template-columns:repeat(3,minmax(0,1fr))
    GridCols4 = 0x14C,       // grid-template-columns:repeat(4,minmax(0,1fr))
    GridColsNone = 0x14D,    // grid-template-columns:none
    ColSpan2 = 0x14E,        // grid-column:span 2
    ColSpan3 = 0x14F,        // grid-column:span 3
    ColSpanFull = 0x150,     // grid-column:1/-1

    // Align Self (0x158-0x15F)
    SelfAuto = 0x158,      // align-self:auto
    SelfStart = 0x159,     // align-self:flex-start
    SelfCenter = 0x15A,    // align-self:center
    SelfEnd = 0x15B,       // align-self:flex-end
    SelfStretch = 0x15C,   // align-self:stretch

    // Justify Self (0x160-0x167)
    JustifySelfAuto = 0x160,   // justify-self:auto
    JustifySelfStart = 0x161,  // justify-self:start
    JustifySelfCenter = 0x162, // justify-self:center
    JustifySelfEnd = 0x163,    // justify-self:end
    JustifySelfStretch = 0x164, // justify-self:stretch

    // Place Content (0x168-0x16F)
    PlaceCenter = 0x168,   // place-content:center
    PlaceStart = 0x169,    // place-content:start
    PlaceEnd = 0x16A,      // place-content:end
    PlaceBetween = 0x16B,  // place-content:space-between

    // More Borders (0x170-0x17F)
    BorderTransparent = 0x170, // border-color:transparent
    BorderT = 0x171,       // border-top:1px solid var(--rw-border-default)
    BorderR = 0x172,       // border-right:1px solid var(--rw-border-default)
    BorderB = 0x173,       // border-bottom:1px solid var(--rw-border-default)
    BorderL = 0x174,       // border-left:1px solid var(--rw-border-default)
    Border2 = 0x175,       // border-width:2px
    DivideY = 0x176,       // & > * + * { border-top:1px solid var(--rw-border-subtle) }

    // Sizing Extended (0x180-0x18F)
    WMax = 0x180,          // width:max-content
    WMin = 0x181,          // width:min-content
    WFit = 0x182,          // width:fit-content
    HMax = 0x183,          // height:max-content
    HMin = 0x184,          // height:min-content
    HFit = 0x185,          // height:fit-content
    MinHFull = 0x186,      // min-height:100%
    MaxHScreen = 0x187,    // max-height:100vh

    // Spacing Extended (0x190-0x19F)
    P2xl = 0x190,          // padding:var(--rw-space-10)
    PxLg = 0x191,          // padding-inline:var(--rw-space-6)
    PxXl = 0x192,          // padding-inline:var(--rw-space-8)
    PyLg = 0x193,          // padding-block:var(--rw-space-6)
    PyXl = 0x194,          // padding-block:var(--rw-space-8)
    Mx0 = 0x195,           // margin-inline:0
    My0 = 0x196,           // margin-block:0
    MlAuto = 0x197,        // margin-left:auto
    MrAuto = 0x198,        // margin-right:auto

    // Gap Extended (0x1A0-0x1A7)
    GapX0 = 0x1A0,         // column-gap:0
    GapXSm = 0x1A1,        // column-gap:var(--rw-space-2)
    GapXMd = 0x1A2,        // column-gap:var(--rw-space-4)
    GapXLg = 0x1A3,        // column-gap:var(--rw-space-6)
    GapY0 = 0x1A4,         // row-gap:0
    GapYSm = 0x1A5,        // row-gap:var(--rw-space-2)
    GapYMd = 0x1A6,        // row-gap:var(--rw-space-4)
    GapYLg = 0x1A7,        // row-gap:var(--rw-space-6)

    // Appearance (0x1B0-0x1B7)
    AppearanceNone = 0x1B0, // appearance:none
    ResizeNone = 0x1B1,     // resize:none
    ResizeY = 0x1B2,        // resize:vertical
    ResizeX = 0x1B3,        // resize:horizontal
    ResizeBoth = 0x1B4,     // resize:both

    // Transform (0x1C0-0x1CF)
    TransformNone = 0x1C0,    // transform:none
    Rotate45 = 0x1C1,         // transform:rotate(45deg)
    Rotate90 = 0x1C2,         // transform:rotate(90deg)
    Rotate180 = 0x1C3,        // transform:rotate(180deg)
    ScaleX = 0x1C4,           // transform:scaleX(-1)
    ScaleY = 0x1C5,           // transform:scaleY(-1)
    TranslateYFull = 0x1C6,   // transform:translateY(-100%)
    Scale95 = 0x1C7,          // transform:scale(0.95)
    Scale100 = 0x1C8,         // transform:scale(1)
    Scale105 = 0x1C9,         // transform:scale(1.05)

    // Animation (0x1D0-0x1D7)
    AnimateSpin = 0x1D0,   // animation:rw-spin 1s linear infinite
    AnimatePing = 0x1D1,   // animation:rw-ping 1s cubic-bezier(0,0,0.2,1) infinite
    AnimatePulse = 0x1D2,  // animation:rw-pulse 2s cubic-bezier(0.4,0,0.6,1) infinite
    AnimateBounce = 0x1D3, // animation:rw-bounce 1s infinite
    AnimateNone = 0x1D4,   // animation:none

    // ========================================================================
    // Directional Spacing (0x1D5-0x1F4) - Individual margin/padding sides
    // ========================================================================

    // Margin-top (0x1D5-0x1D8)
    MtXs = 0x1D5,  // margin-top:var(--rw-space-1)
    MtSm = 0x1D6,  // margin-top:var(--rw-space-2)
    MtMd = 0x1D7,  // margin-top:var(--rw-space-4)
    MtLg = 0x1D8,  // margin-top:var(--rw-space-6)

    // Margin-bottom (0x1D9-0x1DC)
    MbXs = 0x1D9,  // margin-bottom:var(--rw-space-1)
    MbSm = 0x1DA,  // margin-bottom:var(--rw-space-2)
    MbMd = 0x1DB,  // margin-bottom:var(--rw-space-4)
    MbLg = 0x1DC,  // margin-bottom:var(--rw-space-6)

    // Margin-left (0x1DD-0x1E0)
    MlXs = 0x1DD,  // margin-left:var(--rw-space-1)
    MlSm = 0x1DE,  // margin-left:var(--rw-space-2)
    MlMd = 0x1DF,  // margin-left:var(--rw-space-4)
    MlLg = 0x1E0,  // margin-left:var(--rw-space-6)

    // Margin-right (0x1E1-0x1E4)
    MrXs = 0x1E1,  // margin-right:var(--rw-space-1)
    MrSm = 0x1E2,  // margin-right:var(--rw-space-2)
    MrMd = 0x1E3,  // margin-right:var(--rw-space-4)
    MrLg = 0x1E4,  // margin-right:var(--rw-space-6)

    // Padding-top (0x1E5-0x1E8)
    PtXs = 0x1E5,  // padding-top:var(--rw-space-1)
    PtSm = 0x1E6,  // padding-top:var(--rw-space-2)
    PtMd = 0x1E7,  // padding-top:var(--rw-space-4)
    PtLg = 0x1E8,  // padding-top:var(--rw-space-6)

    // Padding-bottom (0x1E9-0x1EC)
    PbXs = 0x1E9,  // padding-bottom:var(--rw-space-1)
    PbSm = 0x1EA,  // padding-bottom:var(--rw-space-2)
    PbMd = 0x1EB,  // padding-bottom:var(--rw-space-4)
    PbLg = 0x1EC,  // padding-bottom:var(--rw-space-6)

    // Padding-left (0x1ED-0x1F0)
    PlXs = 0x1ED,  // padding-left:var(--rw-space-1)
    PlSm = 0x1EE,  // padding-left:var(--rw-space-2)
    PlMd = 0x1EF,  // padding-left:var(--rw-space-4)
    PlLg = 0x1F0,  // padding-left:var(--rw-space-6)

    // Padding-right (0x1F1-0x1F4)
    PrXs = 0x1F1,  // padding-right:var(--rw-space-1)
    PrSm = 0x1F2,  // padding-right:var(--rw-space-2)
    PrMd = 0x1F3,  // padding-right:var(--rw-space-4)
    PrLg = 0x1F4,  // padding-right:var(--rw-space-6)

    // ========================================================================
    // Text Transforms & Extended (0x1F5-0x202)
    // ========================================================================

    // Text transforms (0x1F5-0x1F8)
    TextUppercase = 0x1F5,   // text-transform:uppercase
    TextLowercase = 0x1F6,   // text-transform:lowercase
    TextCapitalize = 0x1F7,  // text-transform:capitalize
    TextNormalCase = 0x1F8,  // text-transform:none

    // Font style (0x1F9-0x1FA)
    Italic = 0x1F9,          // font-style:italic
    NotItalic = 0x1FA,       // font-style:normal

    // Extended sizing (0x1FB-0x202)
    WFit2 = 0x1FB,           // width:fit-content (alias for clearer API)
    MinWFit = 0x1FC,         // min-width:fit-content
    MaxWFit = 0x1FD,         // max-width:fit-content
    MinWMax = 0x1FE,         // min-width:max-content
    HFit2 = 0x1FF,           // height:fit-content (alias)
    MinHFit = 0x200,         // min-height:fit-content
    MaxHFit = 0x201,         // max-height:fit-content
    MinHMax = 0x202,         // min-height:max-content
}

impl St {
    /// Convert to wire protocol value.
    /// Values 0-255 encode as single byte, 256+ as two bytes.
    pub fn as_u16(self) -> u16 {
        self as u16
    }

    /// Check if this token fits in a single byte.
    pub fn is_single_byte(self) -> bool {
        (self as u16) <= 0xFF
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
            Self::TransitionNone => "transition:none",
            Self::TransitionFast => "transition:all 0.1s",
            Self::TransitionSlow => "transition:all 0.3s",

            // Shadow
            Self::ShadowNone => "box-shadow:none",
            Self::ShadowSm => "box-shadow:var(--rw-shadow-sm)",
            Self::ShadowMd => "box-shadow:var(--rw-shadow-md)",
            Self::ShadowLg => "box-shadow:var(--rw-shadow-lg)",
            Self::ShadowXl => "box-shadow:var(--rw-shadow-xl)",
            Self::ShadowInner => "box-shadow:inset 0 2px 4px rgba(0,0,0,0.1)",

            // Outline
            Self::OutlineNone => "outline:none",
            Self::OutlineAccent => "outline:2px solid var(--rw-accent-8)",
            Self::OutlineOffset2 => "outline-offset:2px",
            Self::RingAccent => "box-shadow:0 0 0 2px var(--rw-accent-8)",
            Self::RingInset => "box-shadow:inset 0 0 0 1px var(--rw-border-default)",

            // Text Decoration
            Self::Underline => "text-decoration:underline",
            Self::LineThrough => "text-decoration:line-through",
            Self::NoUnderline => "text-decoration:none",

            // Whitespace & Text
            Self::WhitespaceNormal => "white-space:normal",
            Self::WhitespaceNowrap => "white-space:nowrap",
            Self::WhitespacePre => "white-space:pre",
            Self::WhitespacePreWrap => "white-space:pre-wrap",
            Self::WordBreakNormal => "word-break:normal",
            Self::WordBreakAll => "word-break:break-all",
            Self::WordBreakKeep => "word-break:keep-all",
            Self::BreakWords => "overflow-wrap:break-word",

            // User Interaction
            Self::SelectNone => "user-select:none",
            Self::SelectText => "user-select:text",
            Self::SelectAll => "user-select:all",
            Self::TouchNone => "touch-action:none",
            Self::TouchPan => "touch-action:pan-x pan-y",
            Self::ScrollSmooth => "scroll-behavior:smooth",
            Self::ScrollAuto => "scroll-behavior:auto",

            // Background Extended
            Self::BgTransparent => "background:transparent",
            Self::BgCurrentColor => "background:currentColor",
            Self::BgWhite => "background:var(--rw-white)",
            Self::BgBlack => "background:var(--rw-black)",
            Self::BgSuccess => "background:var(--rw-success)",
            Self::BgWarning => "background:var(--rw-warning)",
            Self::BgError => "background:var(--rw-error)",

            // Text Extended
            Self::TextWhite => "color:var(--rw-white)",
            Self::TextBlack => "color:var(--rw-black)",
            Self::TextInherit => "color:inherit",
            Self::TextCurrentColor => "color:currentColor",
            Self::Text3xl => "font-size:var(--rw-text-3xl)",
            Self::Text4xl => "font-size:var(--rw-text-4xl)",

            // Line Height
            Self::LeadingTight => "line-height:var(--rw-leading-tight)",
            Self::LeadingSnug => "line-height:var(--rw-leading-snug)",
            Self::LeadingNormal => "line-height:var(--rw-leading-normal)",
            Self::LeadingRelaxed => "line-height:var(--rw-leading-relaxed)",
            Self::LeadingLoose => "line-height:var(--rw-leading-loose)",
            Self::LeadingNone => "line-height:1",

            // Letter Spacing
            Self::TrackingTighter => "letter-spacing:-0.05em",
            Self::TrackingTight => "letter-spacing:-0.025em",
            Self::TrackingNormal => "letter-spacing:0",
            Self::TrackingWide => "letter-spacing:0.025em",
            Self::TrackingWider => "letter-spacing:0.05em",
            Self::TrackingWidest => "letter-spacing:0.1em",

            // Aspect Ratio
            Self::AspectAuto => "aspect-ratio:auto",
            Self::AspectSquare => "aspect-ratio:1",
            Self::AspectVideo => "aspect-ratio:16/9",
            Self::AspectPortrait => "aspect-ratio:3/4",

            // Object Fit
            Self::ObjectContain => "object-fit:contain",
            Self::ObjectCover => "object-fit:cover",
            Self::ObjectFill => "object-fit:fill",
            Self::ObjectNone => "object-fit:none",
            Self::ObjectScaleDown => "object-fit:scale-down",

            // Grid (DisplayGrid already covered above at 0x03)
            Self::GridCols1 => "grid-template-columns:repeat(1,minmax(0,1fr))",
            Self::GridCols2 => "grid-template-columns:repeat(2,minmax(0,1fr))",
            Self::GridCols3 => "grid-template-columns:repeat(3,minmax(0,1fr))",
            Self::GridCols4 => "grid-template-columns:repeat(4,minmax(0,1fr))",
            Self::GridColsNone => "grid-template-columns:none",
            Self::ColSpan2 => "grid-column:span 2",
            Self::ColSpan3 => "grid-column:span 3",
            Self::ColSpanFull => "grid-column:1/-1",

            // Align Self
            Self::SelfAuto => "align-self:auto",
            Self::SelfStart => "align-self:flex-start",
            Self::SelfCenter => "align-self:center",
            Self::SelfEnd => "align-self:flex-end",
            Self::SelfStretch => "align-self:stretch",

            // Justify Self
            Self::JustifySelfAuto => "justify-self:auto",
            Self::JustifySelfStart => "justify-self:start",
            Self::JustifySelfCenter => "justify-self:center",
            Self::JustifySelfEnd => "justify-self:end",
            Self::JustifySelfStretch => "justify-self:stretch",

            // Place Content
            Self::PlaceCenter => "place-content:center",
            Self::PlaceStart => "place-content:start",
            Self::PlaceEnd => "place-content:end",
            Self::PlaceBetween => "place-content:space-between",

            // More Borders
            Self::BorderTransparent => "border-color:transparent",
            Self::BorderT => "border-top:1px solid var(--rw-border-default)",
            Self::BorderR => "border-right:1px solid var(--rw-border-default)",
            Self::BorderB => "border-bottom:1px solid var(--rw-border-default)",
            Self::BorderL => "border-left:1px solid var(--rw-border-default)",
            Self::Border2 => "border-width:2px",
            Self::DivideY => "& > * + *{border-top:1px solid var(--rw-border-subtle)}",

            // Sizing Extended
            Self::WMax => "width:max-content",
            Self::WMin => "width:min-content",
            Self::WFit => "width:fit-content",
            Self::HMax => "height:max-content",
            Self::HMin => "height:min-content",
            Self::HFit => "height:fit-content",
            Self::MinHFull => "min-height:100%",
            Self::MaxHScreen => "max-height:100vh",

            // Spacing Extended
            Self::P2xl => "padding:var(--rw-space-10)",
            Self::PxLg => "padding-inline:var(--rw-space-6)",
            Self::PxXl => "padding-inline:var(--rw-space-8)",
            Self::PyLg => "padding-block:var(--rw-space-6)",
            Self::PyXl => "padding-block:var(--rw-space-8)",
            Self::Mx0 => "margin-inline:0",
            Self::My0 => "margin-block:0",
            Self::MlAuto => "margin-left:auto",
            Self::MrAuto => "margin-right:auto",

            // Gap Extended
            Self::GapX0 => "column-gap:0",
            Self::GapXSm => "column-gap:var(--rw-space-2)",
            Self::GapXMd => "column-gap:var(--rw-space-4)",
            Self::GapXLg => "column-gap:var(--rw-space-6)",
            Self::GapY0 => "row-gap:0",
            Self::GapYSm => "row-gap:var(--rw-space-2)",
            Self::GapYMd => "row-gap:var(--rw-space-4)",
            Self::GapYLg => "row-gap:var(--rw-space-6)",

            // Appearance
            Self::AppearanceNone => "appearance:none",
            Self::ResizeNone => "resize:none",
            Self::ResizeY => "resize:vertical",
            Self::ResizeX => "resize:horizontal",
            Self::ResizeBoth => "resize:both",

            // Transform
            Self::TransformNone => "transform:none",
            Self::Rotate45 => "transform:rotate(45deg)",
            Self::Rotate90 => "transform:rotate(90deg)",
            Self::Rotate180 => "transform:rotate(180deg)",
            Self::ScaleX => "transform:scaleX(-1)",
            Self::ScaleY => "transform:scaleY(-1)",
            Self::TranslateYFull => "transform:translateY(-100%)",
            Self::Scale95 => "transform:scale(0.95)",
            Self::Scale100 => "transform:scale(1)",
            Self::Scale105 => "transform:scale(1.05)",

            // Animation
            Self::AnimateSpin => "animation:rw-spin 1s linear infinite",
            Self::AnimatePing => "animation:rw-ping 1s cubic-bezier(0,0,0.2,1) infinite",
            Self::AnimatePulse => "animation:rw-pulse 2s cubic-bezier(0.4,0,0.6,1) infinite",
            Self::AnimateBounce => "animation:rw-bounce 1s infinite",
            Self::AnimateNone => "animation:none",

            // Directional Spacing - Margins
            Self::MtXs => "margin-top:var(--rw-space-1)",
            Self::MtSm => "margin-top:var(--rw-space-2)",
            Self::MtMd => "margin-top:var(--rw-space-4)",
            Self::MtLg => "margin-top:var(--rw-space-6)",
            Self::MbXs => "margin-bottom:var(--rw-space-1)",
            Self::MbSm => "margin-bottom:var(--rw-space-2)",
            Self::MbMd => "margin-bottom:var(--rw-space-4)",
            Self::MbLg => "margin-bottom:var(--rw-space-6)",
            Self::MlXs => "margin-left:var(--rw-space-1)",
            Self::MlSm => "margin-left:var(--rw-space-2)",
            Self::MlMd => "margin-left:var(--rw-space-4)",
            Self::MlLg => "margin-left:var(--rw-space-6)",
            Self::MrXs => "margin-right:var(--rw-space-1)",
            Self::MrSm => "margin-right:var(--rw-space-2)",
            Self::MrMd => "margin-right:var(--rw-space-4)",
            Self::MrLg => "margin-right:var(--rw-space-6)",

            // Directional Spacing - Padding
            Self::PtXs => "padding-top:var(--rw-space-1)",
            Self::PtSm => "padding-top:var(--rw-space-2)",
            Self::PtMd => "padding-top:var(--rw-space-4)",
            Self::PtLg => "padding-top:var(--rw-space-6)",
            Self::PbXs => "padding-bottom:var(--rw-space-1)",
            Self::PbSm => "padding-bottom:var(--rw-space-2)",
            Self::PbMd => "padding-bottom:var(--rw-space-4)",
            Self::PbLg => "padding-bottom:var(--rw-space-6)",
            Self::PlXs => "padding-left:var(--rw-space-1)",
            Self::PlSm => "padding-left:var(--rw-space-2)",
            Self::PlMd => "padding-left:var(--rw-space-4)",
            Self::PlLg => "padding-left:var(--rw-space-6)",
            Self::PrXs => "padding-right:var(--rw-space-1)",
            Self::PrSm => "padding-right:var(--rw-space-2)",
            Self::PrMd => "padding-right:var(--rw-space-4)",
            Self::PrLg => "padding-right:var(--rw-space-6)",

            // Text transforms
            Self::TextUppercase => "text-transform:uppercase",
            Self::TextLowercase => "text-transform:lowercase",
            Self::TextCapitalize => "text-transform:capitalize",
            Self::TextNormalCase => "text-transform:none",

            // Font style
            Self::Italic => "font-style:italic",
            Self::NotItalic => "font-style:normal",

            // Extended sizing
            Self::WFit2 => "width:fit-content",
            Self::MinWFit => "min-width:fit-content",
            Self::MaxWFit => "max-width:fit-content",
            Self::MinWMax => "min-width:max-content",
            Self::HFit2 => "height:fit-content",
            Self::MinHFit => "min-height:fit-content",
            Self::MaxHFit => "max-height:fit-content",
            Self::MinHMax => "min-height:max-content",
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
pub const UTIL_MAPPINGS: &[(u16, &str)] = &[
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
    (0xE4, "transition:none"),
    (0xE5, "transition:all 0.1s"),
    (0xE6, "transition:all 0.3s"),
    // Shadow
    (0xE8, "box-shadow:none"),
    (0xE9, "box-shadow:var(--rw-shadow-sm)"),
    (0xEA, "box-shadow:var(--rw-shadow-md)"),
    (0xEB, "box-shadow:var(--rw-shadow-lg)"),
    (0xEC, "box-shadow:var(--rw-shadow-xl)"),
    (0xED, "box-shadow:inset 0 2px 4px rgba(0,0,0,0.1)"),
    // Outline
    (0xF0, "outline:none"),
    (0xF1, "outline:2px solid var(--rw-accent-8)"),
    (0xF2, "outline-offset:2px"),
    (0xF3, "box-shadow:0 0 0 2px var(--rw-accent-8)"),
    (0xF4, "box-shadow:inset 0 0 0 1px var(--rw-border-default)"),
    // Extended CSS3 (0x100+)
    // Text Decoration
    (0x100, "text-decoration:underline"),
    (0x101, "text-decoration:line-through"),
    (0x102, "text-decoration:none"),
    // Whitespace & Text
    (0x108, "white-space:normal"),
    (0x109, "white-space:nowrap"),
    (0x10A, "white-space:pre"),
    (0x10B, "white-space:pre-wrap"),
    (0x10C, "word-break:normal"),
    (0x10D, "word-break:break-all"),
    (0x10E, "word-break:keep-all"),
    (0x10F, "overflow-wrap:break-word"),
    // User Interaction
    (0x110, "user-select:none"),
    (0x111, "user-select:text"),
    (0x112, "user-select:all"),
    (0x113, "touch-action:none"),
    (0x114, "touch-action:pan-x pan-y"),
    (0x115, "scroll-behavior:smooth"),
    (0x116, "scroll-behavior:auto"),
    // Background Extended
    (0x118, "background:transparent"),
    (0x119, "background:currentColor"),
    (0x11A, "background:var(--rw-white)"),
    (0x11B, "background:var(--rw-black)"),
    (0x11C, "background:var(--rw-success)"),
    (0x11D, "background:var(--rw-warning)"),
    (0x11E, "background:var(--rw-error)"),
    // Text Extended
    (0x120, "color:var(--rw-white)"),
    (0x121, "color:var(--rw-black)"),
    (0x122, "color:inherit"),
    (0x123, "color:currentColor"),
    (0x124, "font-size:var(--rw-text-3xl)"),
    (0x125, "font-size:var(--rw-text-4xl)"),
    // Line Height
    (0x128, "line-height:var(--rw-leading-tight)"),
    (0x129, "line-height:var(--rw-leading-snug)"),
    (0x12A, "line-height:var(--rw-leading-normal)"),
    (0x12B, "line-height:var(--rw-leading-relaxed)"),
    (0x12C, "line-height:var(--rw-leading-loose)"),
    (0x12D, "line-height:1"),
    // Letter Spacing
    (0x130, "letter-spacing:-0.05em"),
    (0x131, "letter-spacing:-0.025em"),
    (0x132, "letter-spacing:0"),
    (0x133, "letter-spacing:0.025em"),
    (0x134, "letter-spacing:0.05em"),
    (0x135, "letter-spacing:0.1em"),
    // Aspect Ratio
    (0x138, "aspect-ratio:auto"),
    (0x139, "aspect-ratio:1"),
    (0x13A, "aspect-ratio:16/9"),
    (0x13B, "aspect-ratio:3/4"),
    // Object Fit
    (0x140, "object-fit:contain"),
    (0x141, "object-fit:cover"),
    (0x142, "object-fit:fill"),
    (0x143, "object-fit:none"),
    (0x144, "object-fit:scale-down"),
    // Grid
    (0x149, "grid-template-columns:repeat(1,minmax(0,1fr))"),
    (0x14A, "grid-template-columns:repeat(2,minmax(0,1fr))"),
    (0x14B, "grid-template-columns:repeat(3,minmax(0,1fr))"),
    (0x14C, "grid-template-columns:repeat(4,minmax(0,1fr))"),
    (0x14D, "grid-template-columns:none"),
    (0x14E, "grid-column:span 2"),
    (0x14F, "grid-column:span 3"),
    (0x150, "grid-column:1/-1"),
    // Align Self
    (0x158, "align-self:auto"),
    (0x159, "align-self:flex-start"),
    (0x15A, "align-self:center"),
    (0x15B, "align-self:flex-end"),
    (0x15C, "align-self:stretch"),
    // Justify Self
    (0x160, "justify-self:auto"),
    (0x161, "justify-self:start"),
    (0x162, "justify-self:center"),
    (0x163, "justify-self:end"),
    (0x164, "justify-self:stretch"),
    // Place Content
    (0x168, "place-content:center"),
    (0x169, "place-content:start"),
    (0x16A, "place-content:end"),
    (0x16B, "place-content:space-between"),
    // More Borders
    (0x170, "border-color:transparent"),
    (0x171, "border-top:1px solid var(--rw-border-default)"),
    (0x172, "border-right:1px solid var(--rw-border-default)"),
    (0x173, "border-bottom:1px solid var(--rw-border-default)"),
    (0x174, "border-left:1px solid var(--rw-border-default)"),
    (0x175, "border-width:2px"),
    // Sizing Extended
    (0x180, "width:max-content"),
    (0x181, "width:min-content"),
    (0x182, "width:fit-content"),
    (0x183, "height:max-content"),
    (0x184, "height:min-content"),
    (0x185, "height:fit-content"),
    (0x186, "min-height:100%"),
    (0x187, "max-height:100vh"),
    // Spacing Extended
    (0x190, "padding:var(--rw-space-10)"),
    (0x191, "padding-inline:var(--rw-space-6)"),
    (0x192, "padding-inline:var(--rw-space-8)"),
    (0x193, "padding-block:var(--rw-space-6)"),
    (0x194, "padding-block:var(--rw-space-8)"),
    (0x195, "margin-inline:0"),
    (0x196, "margin-block:0"),
    (0x197, "margin-left:auto"),
    (0x198, "margin-right:auto"),
    // Gap Extended
    (0x1A0, "column-gap:0"),
    (0x1A1, "column-gap:var(--rw-space-2)"),
    (0x1A2, "column-gap:var(--rw-space-4)"),
    (0x1A3, "column-gap:var(--rw-space-6)"),
    (0x1A4, "row-gap:0"),
    (0x1A5, "row-gap:var(--rw-space-2)"),
    (0x1A6, "row-gap:var(--rw-space-4)"),
    (0x1A7, "row-gap:var(--rw-space-6)"),
    // Appearance
    (0x1B0, "appearance:none"),
    (0x1B1, "resize:none"),
    (0x1B2, "resize:vertical"),
    (0x1B3, "resize:horizontal"),
    (0x1B4, "resize:both"),
    // Transform
    (0x1C0, "transform:none"),
    (0x1C1, "transform:rotate(45deg)"),
    (0x1C2, "transform:rotate(90deg)"),
    (0x1C3, "transform:rotate(180deg)"),
    (0x1C4, "transform:scaleX(-1)"),
    (0x1C5, "transform:scaleY(-1)"),
    (0x1C6, "transform:translateY(-100%)"),
    (0x1C7, "transform:scale(0.95)"),
    (0x1C8, "transform:scale(1)"),
    (0x1C9, "transform:scale(1.05)"),
    // Animation
    (0x1D0, "animation:rw-spin 1s linear infinite"),
    (0x1D1, "animation:rw-ping 1s cubic-bezier(0,0,0.2,1) infinite"),
    (0x1D2, "animation:rw-pulse 2s cubic-bezier(0.4,0,0.6,1) infinite"),
    (0x1D3, "animation:rw-bounce 1s infinite"),
    (0x1D4, "animation:none"),
    // Directional Spacing - Margins
    (0x1D5, "margin-top:var(--rw-space-1)"),
    (0x1D6, "margin-top:var(--rw-space-2)"),
    (0x1D7, "margin-top:var(--rw-space-4)"),
    (0x1D8, "margin-top:var(--rw-space-6)"),
    (0x1D9, "margin-bottom:var(--rw-space-1)"),
    (0x1DA, "margin-bottom:var(--rw-space-2)"),
    (0x1DB, "margin-bottom:var(--rw-space-4)"),
    (0x1DC, "margin-bottom:var(--rw-space-6)"),
    (0x1DD, "margin-left:var(--rw-space-1)"),
    (0x1DE, "margin-left:var(--rw-space-2)"),
    (0x1DF, "margin-left:var(--rw-space-4)"),
    (0x1E0, "margin-left:var(--rw-space-6)"),
    (0x1E1, "margin-right:var(--rw-space-1)"),
    (0x1E2, "margin-right:var(--rw-space-2)"),
    (0x1E3, "margin-right:var(--rw-space-4)"),
    (0x1E4, "margin-right:var(--rw-space-6)"),
    // Directional Spacing - Padding
    (0x1E5, "padding-top:var(--rw-space-1)"),
    (0x1E6, "padding-top:var(--rw-space-2)"),
    (0x1E7, "padding-top:var(--rw-space-4)"),
    (0x1E8, "padding-top:var(--rw-space-6)"),
    (0x1E9, "padding-bottom:var(--rw-space-1)"),
    (0x1EA, "padding-bottom:var(--rw-space-2)"),
    (0x1EB, "padding-bottom:var(--rw-space-4)"),
    (0x1EC, "padding-bottom:var(--rw-space-6)"),
    (0x1ED, "padding-left:var(--rw-space-1)"),
    (0x1EE, "padding-left:var(--rw-space-2)"),
    (0x1EF, "padding-left:var(--rw-space-4)"),
    (0x1F0, "padding-left:var(--rw-space-6)"),
    (0x1F1, "padding-right:var(--rw-space-1)"),
    (0x1F2, "padding-right:var(--rw-space-2)"),
    (0x1F3, "padding-right:var(--rw-space-4)"),
    (0x1F4, "padding-right:var(--rw-space-6)"),
    // Text transforms
    (0x1F5, "text-transform:uppercase"),
    (0x1F6, "text-transform:lowercase"),
    (0x1F7, "text-transform:capitalize"),
    (0x1F8, "text-transform:none"),
    // Font style
    (0x1F9, "font-style:italic"),
    (0x1FA, "font-style:normal"),
    // Extended sizing
    (0x1FB, "width:fit-content"),
    (0x1FC, "min-width:fit-content"),
    (0x1FD, "max-width:fit-content"),
    (0x1FE, "min-width:max-content"),
    (0x1FF, "height:fit-content"),
    (0x200, "min-height:fit-content"),
    (0x201, "max-height:fit-content"),
    (0x202, "min-height:max-content"),
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
            let code = util.as_u16();
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

        let mut seen_u16: HashSet<u16> = HashSet::new();
        for (code, _) in UTIL_MAPPINGS {
            assert!(
                seen_u16.insert(*code),
                "Duplicate utility code: 0x{:02X}",
                code
            );
        }

        let mut seen_u8: HashSet<u8> = HashSet::new();
        for (code, _) in PROP_MAPPINGS {
            assert!(
                seen_u8.insert(*code),
                "Duplicate property code: 0x{:02X}",
                code
            );
        }

        seen_u8.clear();
        for (code, _) in VALUE_MAPPINGS {
            assert!(
                seen_u8.insert(*code),
                "Duplicate value code: 0x{:02X}",
                code
            );
        }
    }
}
