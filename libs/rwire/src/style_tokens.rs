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

define_token_enum! {
    /// Pre-combined style utilities for maximum compactness.
    ///
    /// Each utility maps to a complete CSS declaration like "display:flex".
    /// Short name `St` for concise component code.
    ///
    /// # Semantic Colors
    ///
    /// Semantic color utilities (BgApp, TextDefault, etc.) use CSS variables
    /// that adapt to light/dark theme automatically.
    pub enum St(u16) {
        str_method = css;
        mappings = UTIL_MAPPINGS;

        // Display (0x00-0x0F)
        DisplayNone = 0x00 => "display:none",
        DisplayBlock = 0x01 => "display:block",
        DisplayFlex = 0x02 => "display:flex",
        DisplayGrid = 0x03 => "display:grid",
        DisplayInline = 0x04 => "display:inline",
        DisplayInlineFlex = 0x05 => "display:inline-flex",
        DisplayInlineBlock = 0x06 => "display:inline-block",
        DisplayContents = 0x07 => "display:contents",

        // Flex Direction (0x10-0x17)
        FlexRow = 0x10 => "flex-direction:row",
        FlexCol = 0x11 => "flex-direction:column",
        FlexRowReverse = 0x12 => "flex-direction:row-reverse",
        FlexColReverse = 0x13 => "flex-direction:column-reverse",
        FlexWrap = 0x14 => "flex-wrap:wrap",
        FlexNoWrap = 0x15 => "flex-wrap:nowrap",

        // Justify Content (0x18-0x1F)
        JustifyStart = 0x18 => "justify-content:flex-start",
        JustifyEnd = 0x19 => "justify-content:flex-end",
        JustifyCenter = 0x1A => "justify-content:center",
        JustifyBetween = 0x1B => "justify-content:space-between",
        JustifyAround = 0x1C => "justify-content:space-around",
        JustifyEvenly = 0x1D => "justify-content:space-evenly",

        // Align Items (0x20-0x27)
        ItemsStart = 0x20 => "align-items:flex-start",
        ItemsEnd = 0x21 => "align-items:flex-end",
        ItemsCenter = 0x22 => "align-items:center",
        ItemsStretch = 0x23 => "align-items:stretch",
        ItemsBaseline = 0x24 => "align-items:baseline",

        // Gap - Design Tokens (0x28-0x2F)
        Gap0 = 0x28 => "gap:0",
        GapXs = 0x29 => "gap:var(--S1)",
        GapSm = 0x2A => "gap:var(--S2)",
        GapMd = 0x2B => "gap:var(--S4)",
        GapLg = 0x2C => "gap:var(--S6)",
        GapXl = 0x2D => "gap:var(--S8)",
        Gap2xl = 0x2E => "gap:var(--S12)",

        // Position (0x30-0x37)
        PositionRelative = 0x30 => "position:relative",
        PositionAbsolute = 0x31 => "position:absolute",
        PositionFixed = 0x32 => "position:fixed",
        PositionSticky = 0x33 => "position:sticky",
        PositionStatic = 0x34 => "position:static",

        // Inset (0x38-0x3F)
        Inset0 = 0x38 => "inset:0",
        InsetAuto = 0x39 => "inset:auto",
        Top0 = 0x3A => "top:0",
        Right0 = 0x3B => "right:0",
        Bottom0 = 0x3C => "bottom:0",
        Left0 = 0x3D => "left:0",

        // Width/Height (0x40-0x4F)
        WFull = 0x40 => "width:100%",
        WAuto = 0x41 => "width:auto",
        WScreen = 0x42 => "width:100vw",
        HFull = 0x43 => "height:100%",
        HAuto = 0x44 => "height:auto",
        HScreen = 0x45 => "height:100vh",
        MinW0 = 0x46 => "min-width:0",
        MinH0 = 0x47 => "min-height:0",
        MaxWFull = 0x48 => "max-width:100%",
        MaxHFull = 0x49 => "max-height:100%",

        // Padding - Design Tokens (0x50-0x5F)
        P0 = 0x50 => "padding:0",
        PXs = 0x51 => "padding:var(--S1)",
        PSm = 0x52 => "padding:var(--S2)",
        PMd = 0x53 => "padding:var(--S4)",
        PLg = 0x54 => "padding:var(--S6)",
        PXl = 0x55 => "padding:var(--S8)",
        PxXs = 0x56 => "padding-inline:var(--S1)",
        PxSm = 0x57 => "padding-inline:var(--S2)",
        PxMd = 0x58 => "padding-inline:var(--S4)",
        PyXs = 0x59 => "padding-block:var(--S1)",
        PySm = 0x5A => "padding-block:var(--S2)",
        PyMd = 0x5B => "padding-block:var(--S4)",

        // Margin - Design Tokens (0x60-0x6F)
        M0 = 0x60 => "margin:0",
        MXs = 0x61 => "margin:var(--S1)",
        MSm = 0x62 => "margin:var(--S2)",
        MMd = 0x63 => "margin:var(--S4)",
        MLg = 0x64 => "margin:var(--S6)",
        MXl = 0x65 => "margin:var(--S8)",
        MAuto = 0x66 => "margin:auto",
        MxAuto = 0x67 => "margin-inline:auto",
        MyAuto = 0x68 => "margin-block:auto",

        // Text (0x70-0x7F)
        TextLeft = 0x70 => "text-align:left",
        TextCenter = 0x71 => "text-align:center",
        TextRight = 0x72 => "text-align:right",
        TextJustify = 0x73 => "text-align:justify",
        TextXs = 0x74 => "font-size:var(--T1)",
        TextSm = 0x75 => "font-size:var(--T2)",
        TextBase = 0x76 => "font-size:var(--T3)",
        TextLg = 0x77 => "font-size:var(--T4)",
        TextXl = 0x78 => "font-size:var(--T5)",
        Text2xl = 0x79 => "font-size:var(--T6)",
        FontNormal = 0x7A => "font-weight:400",
        FontMedium = 0x7B => "font-weight:500",
        FontSemibold = 0x7C => "font-weight:600",
        FontBold = 0x7D => "font-weight:700",
        Truncate = 0x7E => "overflow:hidden;text-overflow:ellipsis;white-space:nowrap",

        // Overflow (0x80-0x87)
        OverflowHidden = 0x80 => "overflow:hidden",
        OverflowAuto = 0x81 => "overflow:auto",
        OverflowScroll = 0x82 => "overflow:scroll",
        OverflowVisible = 0x83 => "overflow:visible",
        OverflowXAuto = 0x84 => "overflow-x:auto",
        OverflowYAuto = 0x85 => "overflow-y:auto",

        // Border Radius - Design Tokens (0x88-0x8F)
        Rounded0 = 0x88 => "border-radius:0",
        RoundedSm = 0x89 => "border-radius:var(--R1)",
        RoundedMd = 0x8A => "border-radius:var(--R2)",
        RoundedLg = 0x8B => "border-radius:var(--R3)",
        RoundedXl = 0x8C => "border-radius:var(--R4)",
        RoundedFull = 0x8D => "border-radius:9999px",

        // Opacity (0x90-0x97)
        Opacity0 = 0x90 => "opacity:0",
        Opacity25 = 0x91 => "opacity:0.25",
        Opacity50 = 0x92 => "opacity:0.5",
        Opacity75 = 0x93 => "opacity:0.75",
        Opacity100 = 0x94 => "opacity:1",

        // Pointer/Cursor (0x98-0x9F)
        CursorPointer = 0x98 => "cursor:pointer",
        CursorDefault = 0x99 => "cursor:default",
        CursorNotAllowed = 0x9A => "cursor:not-allowed",
        CursorWait = 0x9B => "cursor:wait",
        CursorText = 0x9C => "cursor:text",
        PointerEventsNone = 0x9D => "pointer-events:none",
        PointerEventsAuto = 0x9E => "pointer-events:auto",

        // Z-Index (0xA0-0xA7)
        Z0 = 0xA0 => "z-index:0",
        Z10 = 0xA1 => "z-index:10",
        Z20 = 0xA2 => "z-index:20",
        Z30 = 0xA3 => "z-index:30",
        Z40 = 0xA4 => "z-index:40",
        Z50 = 0xA5 => "z-index:50",
        ZAuto = 0xA6 => "z-index:auto",

        // Visibility (0xA8-0xAF)
        Visible = 0xA8 => "visibility:visible",
        Invisible = 0xA9 => "visibility:hidden",
        SrOnly = 0xAA => "position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);border:0",

        // Common Composites (0xB0-0xBF)
        FlexCenter = 0xB0 => "display:flex;justify-content:center;align-items:center",
        AbsoluteFill = 0xB1 => "position:absolute;inset:0",
        FixedFill = 0xB2 => "position:fixed;inset:0",
        FlexGrow = 0xB3 => "flex-grow:1",
        FlexShrink0 = 0xB4 => "flex-shrink:0",
        Flex1 = 0xB5 => "flex:1",

        // ====================================================================
        // Semantic Colors (0xC0-0xDF) - Theme-aware via CSS variables
        // ====================================================================

        // Background - Semantic (0xC0-0xC7)
        BgApp = 0xC0 => "background:var(--a)",
        BgSubtle = 0xC1 => "background:var(--b)",
        BgMuted = 0xC2 => "background:var(--c)",
        BgEmphasis = 0xC3 => "background:var(--d)",
        BgHover = 0xC4 => "background:var(--e)",
        BgActive = 0xC5 => "background:var(--f)",
        BgAccent = 0xC6 => "background:var(--n9)",
        BgAccentHover = 0xC7 => "background:var(--n10)",

        // Text - Semantic (0xC8-0xCF)
        TextDefault = 0xC8 => "color:var(--k)",
        TextHigh = 0xC9 => "color:var(--l)",
        TextMuted = 0xCA => "color:var(--j)",
        TextOnAccent = 0xCB => "color:var(--m)",
        TextSuccess = 0xCC => "color:var(--o)",
        TextWarning = 0xCD => "color:var(--p)",
        TextError = 0xCE => "color:var(--q)",
        TextAccent = 0xCF => "color:var(--n11)",

        // Border - Semantic (0xD0-0xD7)
        BorderDefault = 0xD0 => "border:1px solid var(--h)",
        BorderSubtle = 0xD1 => "border:1px solid var(--g)",
        BorderEmphasis = 0xD2 => "border:1px solid var(--i)",
        BorderAccent = 0xD3 => "border:1px solid var(--n7)",
        BorderNone = 0xD4 => "border:none",

        // Additional Layout (0xD8-0xDF)
        MinHScreen = 0xD8 => "min-height:100vh",
        MinWScreen = 0xD9 => "min-width:100vw",
        MaxWMd = 0xDA => "max-width:28rem",
        MaxWLg = 0xDB => "max-width:32rem",
        MaxWXl = 0xDC => "max-width:36rem",
        MaxW2xl = 0xDD => "max-width:42rem",

        // Transition (0xE0-0xE7)
        TransitionAll = 0xE0 => "transition:all 0.2s",
        TransitionColors = 0xE1 => "transition:color,background-color 0.2s",
        TransitionOpacity = 0xE2 => "transition:opacity 0.2s",
        TransitionTransform = 0xE3 => "transition:transform 0.2s",
        TransitionNone = 0xE4 => "transition:none",
        TransitionFast = 0xE5 => "transition:all 0.1s",
        TransitionSlow = 0xE6 => "transition:all 0.3s",

        // Shadow (0xE8-0xEF) - Box shadows
        ShadowNone = 0xE8 => "box-shadow:none",
        ShadowSm = 0xE9 => "box-shadow:var(--Z1)",
        ShadowMd = 0xEA => "box-shadow:var(--Z2)",
        ShadowLg = 0xEB => "box-shadow:var(--Z3)",
        ShadowXl = 0xEC => "box-shadow:var(--Z4)",
        ShadowInner = 0xED => "box-shadow:inset 0 2px 4px rgba(0,0,0,0.1)",

        // Outline (0xF0-0xF7) - Focus styles
        OutlineNone = 0xF0 => "outline:none",
        OutlineAccent = 0xF1 => "outline:2px solid var(--n8)",
        OutlineOffset2 = 0xF2 => "outline-offset:2px",
        RingAccent = 0xF3 => "box-shadow:0 0 0 2px var(--n8)",
        RingInset = 0xF4 => "box-shadow:inset 0 0 0 1px var(--h)",

        // ====================================================================
        // Extended CSS3 (0x100+) - Using two-byte encoding
        // ====================================================================

        // Text Decoration (0x100-0x107)
        Underline = 0x100 => "text-decoration:underline",
        LineThrough = 0x101 => "text-decoration:line-through",
        NoUnderline = 0x102 => "text-decoration:none",

        // Whitespace & Text (0x108-0x10F)
        WhitespaceNormal = 0x108 => "white-space:normal",
        WhitespaceNowrap = 0x109 => "white-space:nowrap",
        WhitespacePre = 0x10A => "white-space:pre",
        WhitespacePreWrap = 0x10B => "white-space:pre-wrap",
        WordBreakNormal = 0x10C => "word-break:normal",
        WordBreakAll = 0x10D => "word-break:break-all",
        WordBreakKeep = 0x10E => "word-break:keep-all",
        BreakWords = 0x10F => "overflow-wrap:break-word",

        // User Interaction (0x110-0x117)
        SelectNone = 0x110 => "user-select:none",
        SelectText = 0x111 => "user-select:text",
        SelectAll = 0x112 => "user-select:all",
        TouchNone = 0x113 => "touch-action:none",
        TouchPan = 0x114 => "touch-action:pan-x pan-y",
        ScrollSmooth = 0x115 => "scroll-behavior:smooth",
        ScrollAuto = 0x116 => "scroll-behavior:auto",

        // Background Extended (0x118-0x11F)
        BgTransparent = 0x118 => "background:transparent",
        BgCurrentColor = 0x119 => "background:currentColor",
        BgWhite = 0x11A => "background:var(--Yw)",
        BgBlack = 0x11B => "background:var(--Yb)",
        BgSuccess = 0x11C => "background:var(--o)",
        BgWarning = 0x11D => "background:var(--p)",
        BgError = 0x11E => "background:var(--q)",

        // Text Extended (0x120-0x127)
        TextWhite = 0x120 => "color:var(--Yw)",
        TextBlack = 0x121 => "color:var(--Yb)",
        TextInherit = 0x122 => "color:inherit",
        TextCurrentColor = 0x123 => "color:currentColor",
        Text3xl = 0x124 => "font-size:var(--T7)",
        Text4xl = 0x125 => "font-size:var(--T8)",

        // Line Height (0x128-0x12F)
        LeadingTight = 0x128 => "line-height:var(--X1)",
        LeadingSnug = 0x129 => "line-height:var(--X2)",
        LeadingNormal = 0x12A => "line-height:var(--X3)",
        LeadingRelaxed = 0x12B => "line-height:var(--X4)",
        LeadingLoose = 0x12C => "line-height:var(--X5)",
        LeadingNone = 0x12D => "line-height:1",

        // Letter Spacing (0x130-0x137)
        TrackingTighter = 0x130 => "letter-spacing:-0.05em",
        TrackingTight = 0x131 => "letter-spacing:-0.025em",
        TrackingNormal = 0x132 => "letter-spacing:0",
        TrackingWide = 0x133 => "letter-spacing:0.025em",
        TrackingWider = 0x134 => "letter-spacing:0.05em",
        TrackingWidest = 0x135 => "letter-spacing:0.1em",

        // Aspect Ratio (0x138-0x13F)
        AspectAuto = 0x138 => "aspect-ratio:auto",
        AspectSquare = 0x139 => "aspect-ratio:1",
        AspectVideo = 0x13A => "aspect-ratio:16/9",
        AspectPortrait = 0x13B => "aspect-ratio:3/4",

        // Object Fit (0x140-0x147)
        ObjectContain = 0x140 => "object-fit:contain",
        ObjectCover = 0x141 => "object-fit:cover",
        ObjectFill = 0x142 => "object-fit:fill",
        ObjectNone = 0x143 => "object-fit:none",
        ObjectScaleDown = 0x144 => "object-fit:scale-down",

        // Grid (0x148-0x15F) - Note: DisplayGrid already exists at 0x03
        GridCols1 = 0x149 => "grid-template-columns:repeat(1,minmax(0,1fr))",
        GridCols2 = 0x14A => "grid-template-columns:repeat(2,minmax(0,1fr))",
        GridCols3 = 0x14B => "grid-template-columns:repeat(3,minmax(0,1fr))",
        GridCols4 = 0x14C => "grid-template-columns:repeat(4,minmax(0,1fr))",
        GridColsNone = 0x14D => "grid-template-columns:none",
        ColSpan2 = 0x14E => "grid-column:span 2",
        ColSpan3 = 0x14F => "grid-column:span 3",
        ColSpanFull = 0x150 => "grid-column:1/-1",

        // Align Self (0x158-0x15F)
        SelfAuto = 0x158 => "align-self:auto",
        SelfStart = 0x159 => "align-self:flex-start",
        SelfCenter = 0x15A => "align-self:center",
        SelfEnd = 0x15B => "align-self:flex-end",
        SelfStretch = 0x15C => "align-self:stretch",

        // Justify Self (0x160-0x167)
        JustifySelfAuto = 0x160 => "justify-self:auto",
        JustifySelfStart = 0x161 => "justify-self:start",
        JustifySelfCenter = 0x162 => "justify-self:center",
        JustifySelfEnd = 0x163 => "justify-self:end",
        JustifySelfStretch = 0x164 => "justify-self:stretch",

        // Place Content (0x168-0x16F)
        PlaceCenter = 0x168 => "place-content:center",
        PlaceStart = 0x169 => "place-content:start",
        PlaceEnd = 0x16A => "place-content:end",
        PlaceBetween = 0x16B => "place-content:space-between",

        // More Borders (0x170-0x17F)
        BorderTransparent = 0x170 => "border-color:transparent",
        BorderT = 0x171 => "border-top:1px solid var(--h)",
        BorderR = 0x172 => "border-right:1px solid var(--h)",
        BorderB = 0x173 => "border-bottom:1px solid var(--h)",
        BorderL = 0x174 => "border-left:1px solid var(--h)",
        Border2 = 0x175 => "border-width:2px",
        DivideY = 0x176 => "& > * + *{border-top:1px solid var(--g)}",

        // Sizing Extended (0x180-0x18F)
        WMax = 0x180 => "width:max-content",
        WMin = 0x181 => "width:min-content",
        WFit = 0x182 => "width:fit-content",
        HMax = 0x183 => "height:max-content",
        HMin = 0x184 => "height:min-content",
        HFit = 0x185 => "height:fit-content",
        MinHFull = 0x186 => "min-height:100%",
        MaxHScreen = 0x187 => "max-height:100vh",

        // Spacing Extended (0x190-0x19F)
        P2xl = 0x190 => "padding:var(--S10)",
        PxLg = 0x191 => "padding-inline:var(--S6)",
        PxXl = 0x192 => "padding-inline:var(--S8)",
        PyLg = 0x193 => "padding-block:var(--S6)",
        PyXl = 0x194 => "padding-block:var(--S8)",
        Mx0 = 0x195 => "margin-inline:0",
        My0 = 0x196 => "margin-block:0",
        MlAuto = 0x197 => "margin-left:auto",
        MrAuto = 0x198 => "margin-right:auto",

        // Gap Extended (0x1A0-0x1A7)
        GapX0 = 0x1A0 => "column-gap:0",
        GapXSm = 0x1A1 => "column-gap:var(--S2)",
        GapXMd = 0x1A2 => "column-gap:var(--S4)",
        GapXLg = 0x1A3 => "column-gap:var(--S6)",
        GapY0 = 0x1A4 => "row-gap:0",
        GapYSm = 0x1A5 => "row-gap:var(--S2)",
        GapYMd = 0x1A6 => "row-gap:var(--S4)",
        GapYLg = 0x1A7 => "row-gap:var(--S6)",

        // Appearance (0x1B0-0x1B7)
        AppearanceNone = 0x1B0 => "appearance:none",
        ResizeNone = 0x1B1 => "resize:none",
        ResizeY = 0x1B2 => "resize:vertical",
        ResizeX = 0x1B3 => "resize:horizontal",
        ResizeBoth = 0x1B4 => "resize:both",

        // Transform (0x1C0-0x1CF)
        TransformNone = 0x1C0 => "transform:none",
        Rotate45 = 0x1C1 => "transform:rotate(45deg)",
        Rotate90 = 0x1C2 => "transform:rotate(90deg)",
        Rotate180 = 0x1C3 => "transform:rotate(180deg)",
        ScaleX = 0x1C4 => "transform:scaleX(-1)",
        ScaleY = 0x1C5 => "transform:scaleY(-1)",
        TranslateYFull = 0x1C6 => "transform:translateY(-100%)",
        Scale95 = 0x1C7 => "transform:scale(0.95)",
        Scale100 = 0x1C8 => "transform:scale(1)",
        Scale105 = 0x1C9 => "transform:scale(1.05)",

        // Animation (0x1D0-0x1D7)
        AnimateSpin = 0x1D0 => "animation:rw-spin 1s linear infinite",
        AnimatePing = 0x1D1 => "animation:rw-ping 1s cubic-bezier(0,0,0.2,1) infinite",
        AnimatePulse = 0x1D2 => "animation:rw-pulse 2s cubic-bezier(0.4,0,0.6,1) infinite",
        AnimateBounce = 0x1D3 => "animation:rw-bounce 1s infinite",
        AnimateNone = 0x1D4 => "animation:none",

        // ====================================================================
        // Directional Spacing (0x1D5-0x1F4) - Individual margin/padding sides
        // ====================================================================

        // Margin-top (0x1D5-0x1D8)
        MtXs = 0x1D5 => "margin-top:var(--S1)",
        MtSm = 0x1D6 => "margin-top:var(--S2)",
        MtMd = 0x1D7 => "margin-top:var(--S4)",
        MtLg = 0x1D8 => "margin-top:var(--S6)",

        // Margin-bottom (0x1D9-0x1DC)
        MbXs = 0x1D9 => "margin-bottom:var(--S1)",
        MbSm = 0x1DA => "margin-bottom:var(--S2)",
        MbMd = 0x1DB => "margin-bottom:var(--S4)",
        MbLg = 0x1DC => "margin-bottom:var(--S6)",

        // Margin-left (0x1DD-0x1E0)
        MlXs = 0x1DD => "margin-left:var(--S1)",
        MlSm = 0x1DE => "margin-left:var(--S2)",
        MlMd = 0x1DF => "margin-left:var(--S4)",
        MlLg = 0x1E0 => "margin-left:var(--S6)",

        // Margin-right (0x1E1-0x1E4)
        MrXs = 0x1E1 => "margin-right:var(--S1)",
        MrSm = 0x1E2 => "margin-right:var(--S2)",
        MrMd = 0x1E3 => "margin-right:var(--S4)",
        MrLg = 0x1E4 => "margin-right:var(--S6)",

        // Padding-top (0x1E5-0x1E8)
        PtXs = 0x1E5 => "padding-top:var(--S1)",
        PtSm = 0x1E6 => "padding-top:var(--S2)",
        PtMd = 0x1E7 => "padding-top:var(--S4)",
        PtLg = 0x1E8 => "padding-top:var(--S6)",

        // Padding-bottom (0x1E9-0x1EC)
        PbXs = 0x1E9 => "padding-bottom:var(--S1)",
        PbSm = 0x1EA => "padding-bottom:var(--S2)",
        PbMd = 0x1EB => "padding-bottom:var(--S4)",
        PbLg = 0x1EC => "padding-bottom:var(--S6)",

        // Padding-left (0x1ED-0x1F0)
        PlXs = 0x1ED => "padding-left:var(--S1)",
        PlSm = 0x1EE => "padding-left:var(--S2)",
        PlMd = 0x1EF => "padding-left:var(--S4)",
        PlLg = 0x1F0 => "padding-left:var(--S6)",

        // Padding-right (0x1F1-0x1F4)
        PrXs = 0x1F1 => "padding-right:var(--S1)",
        PrSm = 0x1F2 => "padding-right:var(--S2)",
        PrMd = 0x1F3 => "padding-right:var(--S4)",
        PrLg = 0x1F4 => "padding-right:var(--S6)",

        // ====================================================================
        // Text Transforms & Extended (0x1F5-0x202)
        // ====================================================================

        // Text transforms (0x1F5-0x1F8)
        TextUppercase = 0x1F5 => "text-transform:uppercase",
        TextLowercase = 0x1F6 => "text-transform:lowercase",
        TextCapitalize = 0x1F7 => "text-transform:capitalize",
        TextNormalCase = 0x1F8 => "text-transform:none",

        // Font style (0x1F9-0x1FA)
        Italic = 0x1F9 => "font-style:italic",
        NotItalic = 0x1FA => "font-style:normal",

        // Extended sizing (0x1FB-0x202)
        WFit2 = 0x1FB => "width:fit-content",
        MinWFit = 0x1FC => "min-width:fit-content",
        MaxWFit = 0x1FD => "max-width:fit-content",
        MinWMax = 0x1FE => "min-width:max-content",
        HFit2 = 0x1FF => "height:fit-content",
        MinHFit = 0x200 => "min-height:fit-content",
        MaxHFit = 0x201 => "max-height:fit-content",
        MinHMax = 0x202 => "min-height:max-content",

        // ====================================================================
        // Migration Tokens (0x210+) - Used by component migration
        // ====================================================================

        // Padding (0x212)
        Py0 = 0x212 => "padding-block:0",

        // Border (0x213)
        BorderL4 = 0x213 => "border-left:4px solid",

        // Palette Backgrounds (0x220-0x226)
        BgGreen4 = 0x220 => "background:var(--P4)",
        BgAmber4 = 0x221 => "background:var(--M4)",
        BgRed4 = 0x222 => "background:var(--O4)",
        BgBlue2 = 0x223 => "background:var(--U2)",
        BgAmber2 = 0x224 => "background:var(--M2)",

        // Palette Text Colors (0x225-0x227)
        TextGreen11 = 0x225 => "color:var(--P11)",
        TextAmber11 = 0x226 => "color:var(--M11)",
        TextRed11 = 0x227 => "color:var(--O11)",

        // Palette Border Colors (0x228-0x22B)
        BorderGreen8 = 0x228 => "border-color:var(--P8)",
        BorderBlue8 = 0x229 => "border-color:var(--U8)",
        BorderAmber8 = 0x22A => "border-color:var(--M8)",
        BorderRed8 = 0x22B => "border-color:var(--O8)",

        // Margin-block (0x230-0x234) - used by Divider component
        MyXs = 0x230 => "margin-block:var(--S1)",
        MySm = 0x231 => "margin-block:var(--S2)",
        MyMd = 0x232 => "margin-block:var(--S4)",
        MyLg = 0x233 => "margin-block:var(--S6)",
        MyXl = 0x234 => "margin-block:var(--S8)",

        // Border variants (0x235-0x236)
        BorderTSubtle = 0x235 => "border-top:1px solid var(--g)",
        BorderLSubtle = 0x236 => "border-left:1px solid var(--g)",

        // Palette accents (0x237)
        BgAccent4 = 0x237 => "background:var(--n4)",
        TextAccent11 = 0x238 => "color:var(--n11)",

        // Flex shrink (0x239)
        FlexShrink = 0x239 => "flex-shrink:0",

        // Phase 3 migration tokens (0x23A+) - Interactive components
        BgRed9 = 0x23A => "background:var(--O9)",
        TextTransparent = 0x23B => "color:transparent",
        ListStyleNone = 0x23C => "list-style:none",
        ListDecimal = 0x23D => "list-style-type:decimal",
        TextMedium = 0x23E => "color:var(--N9)",
        TextLow = 0x23F => "color:var(--N8)",
        NoDecoration = 0x240 => "text-decoration:none",
        BorderBDefault = 0x241 => "border-bottom:1px solid var(--h)",
        BorderBAccent = 0x242 => "border-bottom-color:var(--n9)",

        // ====================================================================
        // Component Sizing Tokens (0x250+) - Replaces inline style strings
        // ====================================================================

        // Fixed-rem heights (0x250-0x258)
        H1rem = 0x250 => "height:1rem",
        H1_25rem = 0x251 => "height:1.25rem",
        H1_5rem = 0x252 => "height:1.5rem",
        H1_75rem = 0x253 => "height:1.75rem",
        H2rem = 0x254 => "height:2rem",
        H2_25rem = 0x255 => "height:2.25rem",
        H2_5rem = 0x256 => "height:2.5rem",
        H2_75rem = 0x257 => "height:2.75rem",
        H3rem = 0x258 => "height:3rem",

        // Spacing-scale heights (0x259-0x25F)
        HSp0 = 0x259 => "height:0",
        HSp1 = 0x25A => "height:var(--S1)",
        HSp2 = 0x25B => "height:var(--S2)",
        HSp4 = 0x25C => "height:var(--S4)",
        HSp6 = 0x25D => "height:var(--S6)",
        HSp8 = 0x25E => "height:var(--S8)",
        H05rem = 0x25F => "height:0.5rem",

        // Fixed-rem widths (0x260-0x264)
        W1rem = 0x260 => "width:1rem",
        W1_5rem = 0x261 => "width:1.5rem",
        W2rem = 0x262 => "width:2rem",
        W2_5rem = 0x263 => "width:2.5rem",
        W3rem = 0x264 => "width:3rem",

        // Spacing-scale widths (0x265-0x26A)
        WSp0 = 0x265 => "width:0",
        WSp1 = 0x266 => "width:var(--S1)",
        WSp2 = 0x267 => "width:var(--S2)",
        WSp4 = 0x268 => "width:var(--S4)",
        WSp6 = 0x269 => "width:var(--S6)",
        WSp8 = 0x26A => "width:var(--S8)",

        // Fixed-px widths for modals (0x26B-0x26E)
        W400px = 0x26B => "width:400px",
        W600px = 0x26C => "width:600px",
        W800px = 0x26D => "width:800px",
        W1000px = 0x26E => "width:1000px",

        // Min/Max sizing (0x270-0x278)
        MinH4rem = 0x270 => "min-height:4rem",
        MinH5rem = 0x271 => "min-height:5rem",
        MinH6rem = 0x272 => "min-height:6rem",
        MinW05rem = 0x273 => "min-width:0.5rem",
        MaxW40rem = 0x274 => "max-width:40rem",
        MaxW48rem = 0x275 => "max-width:48rem",
        MaxW64rem = 0x276 => "max-width:64rem",
        MaxW80rem = 0x277 => "max-width:80rem",
        MaxH90vh = 0x278 => "max-height:90vh",
        MaxW60rem = 0x279 => "max-width:60rem",
        /// Bottom auto margin: anchors a lone flex child to the far edge without breaking the
        /// container's scrollability (unlike `justify-content`, an auto margin collapses to 0
        /// once content overflows).
        MbAuto = 0x27A => "margin-bottom:auto",

        // Padding/Margin extended (0x280-0x285)
        PxSp3 = 0x280 => "padding-inline:var(--S3)",
        Px0 = 0x281 => "padding-inline:0",
        PySp3 = 0x282 => "padding-block:var(--S3)",
        PSp3 = 0x283 => "padding:var(--S3)",
        MxMd = 0x284 => "margin-inline:var(--S4)",
        MbNeg2px = 0x285 => "margin-bottom:-2px",

        // Border extended (0x290-0x295)
        Bw3 = 0x290 => "border-width:3px",
        BorderRTransparent = 0x291 => "border-right-color:transparent",
        Border2Default = 0x292 => "border:2px solid var(--h)",
        BorderB2Default = 0x293 => "border-bottom:2px solid var(--h)",
        BorderB2Accent = 0x294 => "border-bottom:2px solid var(--n9)",
        BorderB2Transparent = 0x295 => "border-bottom:2px solid transparent",

        // Component misc (0x2A0-0x2A7)
        FontInherit = 0x2A0 => "font-family:inherit",
        AnimateSpinFast = 0x2A1 => "animation:rw-spin .6s linear infinite",
        BgSizeCover = 0x2A2 => "background-size:cover",
        BgPosCenter = 0x2A3 => "background-position:center",
        Z1300 = 0x2A4 => "z-index:1300",
        Z1400 = 0x2A5 => "z-index:1400",
        Z9999 = 0x2A6 => "z-index:9999",
        BgOverlay50 = 0x2A7 => "background:rgba(0,0,0,0.5)",

        // Pseudo-class decomposition tokens (0x2B0-0x2BF)
        BgRedHover = 0x2B0 => "background:var(--O10)",
        BorderColorAccent = 0x2B1 => "border-color:var(--n8)",
        Scale98 = 0x2B2 => "transform:scale(0.98)",
        Mb0 = 0x2B3 => "margin-bottom:0",
        BorderBSubtle = 0x2B4 => "border-bottom:1px solid var(--g)",
        ContentEmpty = 0x2B5 => "content:\"\"",
        ContentAsterisk = 0x2B6 => "content:\" *\"",
        ContentSlash = 0x2B7 => "content:\"/\"",
        TranslateXFull = 0x2B8 => "transform:translateX(100%)",
        TransitionTransformFast = 0x2B9 => "transition:transform .2s",
        MxSp2 = 0x2BA => "margin-inline:var(--S2)",
        BorderStyleSolid = 0x2BB => "border-style:solid",

        // Vertical alignment (0x2BC)
        VerticalAlignMiddle = 0x2BC => "vertical-align:middle",

        // ====================================================================
        // Foundation Component Tokens (0x2C0+)
        // ====================================================================

        // Font family (0x2C0)
        FontMono = 0x2C0 => "font-family:var(--Qm,ui-monospace,SFMono-Regular,monospace)",

        // Code background (0x2C1)
        BgCode = 0x2C1 => "background:var(--Qc,var(--b))",

        // Max height for accordion (0x2C2)
        MaxH96 = 0x2C2 => "max-height:24rem",

        // Border-left for blockquote (0x2C3)
        BorderL3Accent = 0x2C3 => "border-left:3px solid var(--n7)",

        // List style (0x2C4)
        ListDisc = 0x2C4 => "list-style-type:disc",

        // Max-height 0 for collapsed accordion (0x2C5)
        MaxH0 = 0x2C5 => "max-height:0",

        // Prose container tokens (0x2C6-0x2CA)
        LeadingRelaxedProse = 0x2C6 => "line-height:1.75",
        MaxWProse = 0x2C7 => "max-width:65ch",
        SpaceYMd = 0x2C8 => "& > * + *{margin-top:var(--S4)}",
        SpaceYSm = 0x2C9 => "& > * + *{margin-top:var(--S2)}",

        // Skeleton shimmer (0x2CA)
        BgShimmer = 0x2CA => "background:linear-gradient(90deg,var(--c) 0%,var(--b) 50%,var(--c) 100%);background-size:200% 100%;animation:rw-shimmer 1.5s ease-in-out infinite",

        // ====================================================================
        // Phase 2: Layout & Navigation Tokens (0x2CB+)
        // ====================================================================

        // AppShell layout (0x2CB-0x2CF)
        GridTemplateShell = 0x2CB => "display:grid;grid-template-rows:auto 1fr;grid-template-columns:auto 1fr;min-height:100vh",
        GridColFull = 0x2CC => "grid-column:1/-1",
        OverflowYScroll = 0x2CD => "overflow-y:auto;-webkit-overflow-scrolling:touch",
        BgSidebar = 0x2CE => "background:var(--Qs,var(--b))",
        BorderRDefault = 0x2CF => "border-right:1px solid var(--h)",

        // Sidebar/TOC navigation (0x2D0-0x2D3)
        TextXsMuted = 0x2D0 => "font-size:var(--T1);color:var(--j)",
        BgAccentSubtle = 0x2D1 => "background:var(--n3)",
        TextAccent12 = 0x2D2 => "color:var(--n12)",
        PlMdIndent = 0x2D3 => "padding-left:var(--S6)",

        // ====================================================================
        // Phase 4: Tier 2 Component Tokens (0x2D4+)
        // ====================================================================

        // Tooltip (0x2D4-0x2D8)
        TooltipBg = 0x2D4 => "background:var(--d)",
        TextOnEmphasis = 0x2D5 => "color:var(--Qe,#fff)",
        WhitespaceNowrapInline = 0x2D6 => "white-space:nowrap;max-width:20rem",
        TransformCenterX = 0x2D7 => "transform:translateX(-50%)",
        TransformCenterY = 0x2D8 => "transform:translateY(-50%)",

        // Drawer (0x2D9-0x2DB)
        TranslateXNegFull = 0x2D9 => "transform:translateX(-100%)",
        TransitionTransformMd = 0x2DA => "transition:transform .3s ease-in-out",
        W320px = 0x2DB => "width:320px",

        // Tooltip hover-show pattern (0x2DC... shifted toast)
        HoverShowChild = 0x2DF => "&:hover>[data-tip],&:focus-within>[data-tip]{opacity:1}",

        // Toast (0x2E0-0x2E2)
        AnimateSlideIn = 0x2E0 => "animation:rw-slide-in .3s ease-out",
        FixedBottomRight = 0x2E1 => "position:fixed;bottom:var(--S4);right:var(--S4)",
        MaxW360px = 0x2E2 => "max-width:360px",

        // Tier 3 components (0x2E3-0x2F8)
        // Kbd
        KbdBg = 0x2E3 => "background:var(--b);border:1px solid var(--h);border-bottom:2px solid var(--h)",
        KbdShadow = 0x2E4 => "box-shadow:0 1px 0 var(--h)",
        MinWKbd = 0x2E5 => "min-width:1.5em;text-align:center",
        // (GapXs already at 0x29)
        // AvatarGroup (negative overlap)
        NegMlOverlap = 0x2E7 => "margin-left:-8px",
        AvatarRing = 0x2E8 => "outline:2px solid var(--a);outline-offset:-1px",
        // EmptyState
        Py2xl = 0x2E9 => "padding-top:var(--S10);padding-bottom:var(--S10)",
        // Timeline
        TimelineLine = 0x2EA => "border-left:2px solid var(--h)",
        TimelineDot = 0x2EB => "width:12px;height:12px;border-radius:50%;border:2px solid var(--h);background:var(--a)",
        TimelineDotActive = 0x2EC => "width:12px;height:12px;border-radius:50%;border:2px solid var(--n8);background:var(--n3)",
        NegMl7px = 0x2ED => "margin-left:-7px",
        // Stat (Text4xl already at 0x125)
        Text5xl = 0x2EF => "font-size:var(--T9)",
        TextGreen12 = 0x2F0 => "color:var(--P12)",
        TextRed12 = 0x2F1 => "color:var(--O12)",
        // Slider
        SliderTrack = 0x2F2 => "width:100%;height:6px;border-radius:3px;background:var(--d)",
        SliderFill = 0x2F3 => "height:100%;border-radius:3px;background:var(--n8)",
        SliderInput = 0x2F4 => "-webkit-appearance:none;appearance:none;background:transparent;width:100%;height:20px;margin:0;cursor:pointer",
        SliderThumb = 0x2F5 => "&::-webkit-slider-thumb{-webkit-appearance:none;width:18px;height:18px;border-radius:50%;background:var(--n9);border:2px solid var(--r);box-shadow:0 1px 2px rgba(0,0,0,.25);cursor:pointer;transition:background .15s,transform .15s,box-shadow .15s}&:hover::-webkit-slider-thumb{background:var(--n10);transform:scale(1.12);box-shadow:0 2px 6px rgba(0,0,0,.3)}&:active::-webkit-slider-thumb{transform:scale(1.04)}&::-moz-range-thumb{width:18px;height:18px;border-radius:50%;background:var(--n9);border:2px solid var(--r);cursor:pointer}&:hover::-moz-range-thumb{background:var(--n10)}",
        // Stepper
        StepLine = 0x2F6 => "flex:1;height:2px;background:var(--h)",
        StepLineActive = 0x2F7 => "flex:1;height:2px;background:var(--n8)",
        StepCircle = 0x2F8 => "width:32px;height:32px;border-radius:50%;display:flex;align-items:center;justify-content:center;border:2px solid var(--h);background:var(--a);font-size:var(--T2);font-weight:500",
        StepCircleActive = 0x2F9 => "width:32px;height:32px;border-radius:50%;display:flex;align-items:center;justify-content:center;border:2px solid var(--n8);background:var(--n3);color:var(--n11);font-size:var(--T2);font-weight:500",
        StepCircleDone = 0x2FA => "width:32px;height:32px;border-radius:50%;display:flex;align-items:center;justify-content:center;border:2px solid var(--n8);background:var(--n9);color:white;font-size:var(--T2);font-weight:500",

        // Prose heading spacing (0x2FB-0x2FE)
        MtXl = 0x2FB => "margin-top:var(--S8)",
        Mt2xl = 0x2FC => "margin-top:var(--S12)",
        TopHeader = 0x2FE => "top:var(--Qh,3.5rem)",

        // ====================================================================
        // Semantic Paired Tokens (0x300+) — ThemeStyle-aware via CSS variables
        // ====================================================================

        // Surface pairs (0x300-0x303)
        BgSurface = 0x300 => "background:var(--r)",
        TextOnSurface = 0x301 => "color:var(--s)",
        BgSurfaceRaised = 0x302 => "background:var(--t)",
        TextOnSurfaceRaised = 0x303 => "color:var(--u)",

        // Primary pairs (0x304-0x309)
        BgPrimary = 0x304 => "background:var(--v)",
        TextOnPrimary = 0x305 => "color:var(--w)",
        BgPrimaryHover = 0x306 => "background:var(--x)",
        BgPrimarySubtle = 0x307 => "background:var(--y)",
        TextOnPrimarySubtle = 0x308 => "color:var(--z)",
        BorderPrimary = 0x309 => "border-color:var(--L)",

        // Secondary pairs (0x30A-0x30C)
        BgSecondary = 0x30A => "background:var(--A)",
        TextOnSecondary = 0x30B => "color:var(--B)",
        BgSecondaryHover = 0x30C => "background:var(--C)",

        // Muted pair (0x30D)
        TextOnMuted = 0x30D => "color:var(--E)",

        // Destructive pairs (0x30E-0x312)
        BgDestructive = 0x30E => "background:var(--F)",
        TextOnDestructive = 0x30F => "color:var(--G)",
        BgDestructiveHover = 0x310 => "background:var(--H)",
        BgDestructiveSubtle = 0x311 => "background:var(--I)",
        TextOnDestructiveSubtle = 0x312 => "color:var(--J)",

        // Focus ring (0x313)
        RingFocus = 0x313 => "outline:2px solid var(--K);outline-offset:2px",

        // Full accent scale — bg variants (0x314-0x31C)
        BgAccent1 = 0x314 => "background:var(--n1)",
        BgAccent2 = 0x315 => "background:var(--n2)",
        BgAccent3 = 0x316 => "background:var(--n3)",
        BgAccent5 = 0x317 => "background:var(--n5)",
        BgAccent6 = 0x318 => "background:var(--n6)",
        BgAccent7 = 0x319 => "background:var(--n7)",
        BgAccent8 = 0x31A => "background:var(--n8)",
        BgAccent11 = 0x31B => "background:var(--n11)",
        BgAccent12 = 0x31C => "background:var(--n12)",

        // Full accent scale — text variants (0x31D)
        TextAccent1 = 0x31D => "color:var(--n1)",

        // Full accent scale — border variants (0x31E-0x31F)
        BorderAccent6 = 0x31E => "border:1px solid var(--n6)",
        BorderAccent8 = 0x31F => "border:1px solid var(--n8)",

        // ====================================================================
        // Website Foundation Tokens (0x320+)
        // ====================================================================

        Text6xl = 0x320 => "font-size:var(--T10)",
        GridColsAuto = 0x321 => "grid-template-columns:repeat(auto-fill,minmax(280px,1fr))",
        Grayscale = 0x322 => "filter:grayscale(1)",
        MaxW56rem = 0x323 => "max-width:56rem",

        // Extended spacing: padding-top/bottom xl+ (0x324-0x32B)
        PtXl = 0x324 => "padding-top:var(--S8)",
        PbXl = 0x325 => "padding-bottom:var(--S8)",
        Pt3xl = 0x326 => "padding-top:var(--S12)",
        Pb3xl = 0x327 => "padding-bottom:var(--S12)",
        Py3xl = 0x328 => "padding-block:var(--S12)",
        Py4xl = 0x329 => "padding-block:var(--S16)",

        // Layout-specific tokens (0x32A-0x32C)
        HHeader = 0x32A => "height:var(--Qh,3.5rem)",
        W220px = 0x32B => "width:220px",
        MaxW280px = 0x32C => "max-width:280px",

        // Theme-hook tokens (Q-var references) (0x32D-0x33A)
        ShadowTheme = 0x32D => "box-shadow:var(--Qd)",
        GlowTheme = 0x32E => "box-shadow:var(--Qgw)",
        BorderWTheme = 0x32F => "border-width:var(--Qb)",
        BorderLTheme = 0x330 => "border-left-width:var(--Qbl)",
        BorderTTheme = 0x331 => "border-top-width:var(--Qbt)",
        BorderCTheme = 0x332 => "border-color:var(--Qbc)",
        BorderSTheme = 0x333 => "border-style:var(--Qbs)",
        OutlineTheme = 0x334 => "outline:var(--Qol) solid var(--Qf);outline-offset:var(--Qoo)",
        RingTheme = 0x335 => "outline:var(--Qol) solid var(--Qf);outline-offset:var(--Qoo)",
        BackdropTheme = 0x336 => "backdrop-filter:var(--Qbf)",
        OpacityTheme = 0x337 => "opacity:var(--Qso)",
        GradientTheme = 0x338 => "background:var(--Qgr)",
        TransTheme = 0x339 => "transition:all var(--Qt) ease",
        TextShadowTheme = 0x33A => "text-shadow:var(--Qts)",

        // Status subtle pairs — mode-aware (0x33B-0x344)
        BgInfoSubtle = 0x33B => "background:var(--O)",
        TextOnInfoSubtle = 0x33C => "color:var(--O1)",
        BgSuccessSubtle = 0x33D => "background:var(--M)",
        TextOnSuccessSubtle = 0x33E => "color:var(--M1)",
        BgWarningSubtle = 0x33F => "background:var(--N)",
        TextOnWarningSubtle = 0x340 => "color:var(--N1)",
        BgErrorSubtle = 0x341 => "background:var(--P)",
        TextOnErrorSubtle = 0x342 => "color:var(--P1)",

        // Font reset for form controls — the full `font` shorthand (inherits family,
        // size, weight, style, line-height) so inputs/buttons match the page font, unlike
        // `FontInherit` which only inherits font-family.
        FontInheritAll = 0x343 => "font:inherit",

        // Dynamic viewport heights — mobile-safe alternative to the `vh` Screen tokens,
        // which over/undershoot when the browser chrome shows/hides.
        HDvh = 0x344 => "height:100dvh",
        MinHDvh = 0x345 => "min-height:100dvh",
        MaxHDvh = 0x346 => "max-height:100dvh",
        /// Auto-grow a textarea/input to its content (pair with a max-height cap).
        FieldSizingContent = 0x347 => "field-sizing:content",
        BgGreen9 = 0x348 => "background:var(--P9)",
        BgAmber9 = 0x349 => "background:var(--M9)",
    }
}

impl St {
    /// Check if this token fits in a single byte.
    pub fn is_single_byte(self) -> bool {
        (self as u16) <= 0xFF
    }
}

// ============================================================================
// Style Properties (for property+value encoding)
// ============================================================================

define_token_enum! {
    /// CSS property codes for binary encoding.
    pub enum StyleProp(u8) {
        str_method = name;
        mappings = PROP_MAPPINGS;

        Display = 0x00 => "display",
        Position = 0x01 => "position",
        FlexDirection = 0x02 => "flex-direction",
        FlexWrap = 0x03 => "flex-wrap",
        JustifyContent = 0x04 => "justify-content",
        AlignItems = 0x05 => "align-items",
        AlignSelf = 0x06 => "align-self",
        Gap = 0x07 => "gap",
        Width = 0x08 => "width",
        Height = 0x09 => "height",
        MinWidth = 0x0A => "min-width",
        MaxWidth = 0x0B => "max-width",
        MinHeight = 0x0C => "min-height",
        MaxHeight = 0x0D => "max-height",
        Padding = 0x0E => "padding",
        PaddingInline = 0x0F => "padding-inline",
        PaddingBlock = 0x10 => "padding-block",
        Margin = 0x11 => "margin",
        MarginInline = 0x12 => "margin-inline",
        MarginBlock = 0x13 => "margin-block",
        Top = 0x14 => "top",
        Right = 0x15 => "right",
        Bottom = 0x16 => "bottom",
        Left = 0x17 => "left",
        Inset = 0x18 => "inset",
        Overflow = 0x19 => "overflow",
        OverflowX = 0x1A => "overflow-x",
        OverflowY = 0x1B => "overflow-y",
        TextAlign = 0x1C => "text-align",
        FontSize = 0x1D => "font-size",
        FontWeight = 0x1E => "font-weight",
        BorderRadius = 0x1F => "border-radius",
        Opacity = 0x20 => "opacity",
        Cursor = 0x21 => "cursor",
        ZIndex = 0x22 => "z-index",
        Visibility = 0x23 => "visibility",
        Flex = 0x24 => "flex",
        FlexGrow = 0x25 => "flex-grow",
        FlexShrink = 0x26 => "flex-shrink",
        PointerEvents = 0x27 => "pointer-events",
        WhiteSpace = 0x28 => "white-space",
        TextOverflow = 0x29 => "text-overflow",
    }
}

// ============================================================================
// Style Values
// ============================================================================

define_token_enum! {
    /// CSS value codes for binary encoding.
    ///
    /// Values are organized by type to allow property+value combinations.
    pub enum StyleValue(u8) {
        str_method = css;
        mappings = VALUE_MAPPINGS;

        // Keywords (0x00-0x1F)
        None = 0x00 => "none",
        Auto = 0x01 => "auto",
        Hidden = 0x02 => "hidden",
        Visible = 0x03 => "visible",
        Scroll = 0x04 => "scroll",
        Inherit = 0x05 => "inherit",
        Initial = 0x06 => "initial",

        // Display values (0x10-0x1F)
        Block = 0x10 => "block",
        Flex = 0x11 => "flex",
        Grid = 0x12 => "grid",
        Inline = 0x13 => "inline",
        InlineFlex = 0x14 => "inline-flex",
        InlineBlock = 0x15 => "inline-block",
        Contents = 0x16 => "contents",

        // Position values (0x20-0x27)
        Relative = 0x20 => "relative",
        Absolute = 0x21 => "absolute",
        Fixed = 0x22 => "fixed",
        Sticky = 0x23 => "sticky",
        Static = 0x24 => "static",

        // Flex values (0x28-0x3F)
        Row = 0x28 => "row",
        Column = 0x29 => "column",
        RowReverse = 0x2A => "row-reverse",
        ColumnReverse = 0x2B => "column-reverse",
        Wrap = 0x2C => "wrap",
        Nowrap = 0x2D => "nowrap",
        FlexStart = 0x2E => "flex-start",
        FlexEnd = 0x2F => "flex-end",
        Center = 0x30 => "center",
        SpaceBetween = 0x31 => "space-between",
        SpaceAround = 0x32 => "space-around",
        SpaceEvenly = 0x33 => "space-evenly",
        Stretch = 0x34 => "stretch",
        Baseline = 0x35 => "baseline",

        // Size values - percentages (0x40-0x4F)
        Full = 0x40 => "100%",
        Half = 0x41 => "50%",
        Third = 0x42 => "33.333%",
        Quarter = 0x43 => "25%",
        Screen = 0x44 => "100vw",

        // Space tokens (0x50-0x5F) - maps to --S{N}
        Space0 = 0x50 => "0",
        Space1 = 0x51 => "var(--S1)",
        Space2 = 0x52 => "var(--S2)",
        Space3 = 0x53 => "var(--S3)",
        Space4 = 0x54 => "var(--S4)",
        Space5 = 0x55 => "var(--S5)",
        Space6 = 0x56 => "var(--S6)",
        Space8 = 0x57 => "var(--S8)",
        Space10 = 0x58 => "var(--S10)",
        Space12 = 0x59 => "var(--S12)",
        Space16 = 0x5A => "var(--S16)",

        // Text sizes (0x60-0x6F) - maps to --T{N}
        TextXs = 0x60 => "var(--T1)",
        TextSm = 0x61 => "var(--T2)",
        TextBase = 0x62 => "var(--T3)",
        TextLg = 0x63 => "var(--T4)",
        TextXl = 0x64 => "var(--T5)",
        Text2xl = 0x65 => "var(--T6)",
        Text3xl = 0x66 => "var(--T7)",

        // Font weights (0x70-0x77)
        Weight400 = 0x70 => "400",
        Weight500 = 0x71 => "500",
        Weight600 = 0x72 => "600",
        Weight700 = 0x73 => "700",

        // Border radius (0x78-0x7F) - maps to --R{N}
        RadiusNone = 0x78 => "0",
        RadiusSm = 0x79 => "var(--R1)",
        RadiusMd = 0x7A => "var(--R2)",
        RadiusLg = 0x7B => "var(--R3)",
        RadiusXl = 0x7C => "var(--R4)",
        RadiusFull = 0x7D => "9999px",

        // Opacity (0x80-0x87)
        Opacity0 = 0x80 => "0",
        Opacity25 = 0x81 => "0.25",
        Opacity50 = 0x82 => "0.5",
        Opacity75 = 0x83 => "0.75",
        Opacity100 = 0x84 => "1",

        // Cursor (0x88-0x8F)
        Pointer = 0x88 => "pointer",
        Default = 0x89 => "default",
        NotAllowed = 0x8A => "not-allowed",
        Wait = 0x8B => "wait",
        Text = 0x8C => "text",

        // Z-index (0x90-0x97)
        Z0 = 0x90 => "0",
        Z10 = 0x91 => "10",
        Z20 = 0x92 => "20",
        Z30 = 0x93 => "30",
        Z40 = 0x94 => "40",
        Z50 = 0x95 => "50",

        // Text align (0x98-0x9F)
        Left = 0x98 => "left",
        Right = 0x99 => "right",
        // Center already defined at 0x30

        // Numeric (0xA0-0xAF)
        N0 = 0xA0 => "0",
        N1 = 0xA1 => "1",
    }
}

// ============================================================================
// Pseudo-Class/Pseudo-Element Tokens
// ============================================================================

define_token_enum! {
    /// Pseudo-class/pseudo-element selector tokens.
    ///
    /// Combined with St tokens to create composable pseudo-class CSS rules.
    /// Any St token can be used under any Pc selector.
    ///
    /// # CSS Class Naming
    ///
    /// `h{pc_code}u{st_code}` -> `.h0u199:hover{background:var(--n10)}`
    pub enum Pc(u8) {
        str_method = selector;
        mappings = PC_MAPPINGS;

        Hover = 0x00 => ":hover",
        Focus = 0x01 => ":focus",
        FocusVisible = 0x02 => ":focus-visible",
        Active = 0x03 => ":active",
        Disabled = 0x04 => "",
        Checked = 0x05 => ":checked",
        Placeholder = 0x06 => "::placeholder",
        Before = 0x07 => "::before",
        After = 0x08 => "::after",
        FirstChild = 0x09 => ":first-child",
        LastChild = 0x0A => ":last-child",
        NthEven = 0x0B => ":nth-child(even)",
        NthOdd = 0x0C => ":nth-child(odd)",
        NotLastChild = 0x0D => ":not(:last-child)",
        FocusWithin = 0x0E => ":focus-within",
        CheckedAfter = 0x0F => ":checked::after",
    }
}

// ============================================================================
// Responsive Breakpoint Tokens
// ============================================================================

define_token_enum! {
    /// Responsive breakpoint tokens (mobile-first, min-width).
    ///
    /// Combined with St tokens to create responsive CSS rules.
    /// Any St token can be used under any Bp breakpoint.
    ///
    /// # CSS Class Naming
    ///
    /// `b{bp_code}u{st_code}` -> `@media(min-width:768px){.b1u2{display:flex}}`
    pub enum Bp(u8) {
        str_method = min_width;
        mappings = BP_MAPPINGS;

        Sm = 0x00 => "640",
        Md = 0x01 => "768",
        Lg = 0x02 => "1024",
        Xl = 0x03 => "1280",
    }
}

/// Global CSS rules injected alongside pseudo tokens (e.g., @keyframes).
pub const PSEUDO_GLOBAL_CSS: &str = "@keyframes rw-spin{to{transform:rotate(360deg)}}@keyframes rw-shimmer{0%{background-position:200% 0}to{background-position:-200% 0}}@keyframes rw-slide-in{from{transform:translateY(1rem);opacity:0}to{transform:translateY(0);opacity:1}}@keyframes rw-ping{75%,100%{transform:scale(2);opacity:0}}@keyframes rw-pulse{50%{opacity:.5}}@keyframes rw-bounce{0%,100%{transform:translateY(-25%);animation-timing-function:cubic-bezier(0.8,0,1,1)}50%{transform:none;animation-timing-function:cubic-bezier(0,0,0.2,1)}}";

// ============================================================================
// CSS Generation Functions
// ============================================================================

/// Generate CSS rules for all used utility tokens.
///
/// Each used token becomes a CSS class rule: `.u{code}{declaration}`
/// These rules are embedded in the capsule `<style>` tag and replace the JS lookup table.
pub fn generate_utility_css(used: &std::collections::HashSet<u16>) -> String {
    let mut css = String::with_capacity(used.len() * 30);
    let mut needs_global_keyframes = false;
    for &(code, declaration) in UTIL_MAPPINGS {
        if used.contains(&code) {
            use std::fmt::Write;
            let _ = write!(css, ".u{}{{{}}}", code, declaration);
            if declaration.contains("rw-spin")
                || declaration.contains("rw-shimmer")
                || declaration.contains("rw-slide-in")
                || declaration.contains("rw-ping")
                || declaration.contains("rw-pulse")
                || declaration.contains("rw-bounce")
            {
                needs_global_keyframes = true;
            }
        }
    }
    if needs_global_keyframes {
        css.push_str(PSEUDO_GLOBAL_CSS);
    }
    css
}

/// Generate CSS rules for all used pseudo (Pc, St) pairs.
///
/// Each used pair becomes: `.h{pc}u{st}{selector}{declaration}`
/// where the selector comes from Pc and the declaration from St.
pub fn generate_pseudo_css(used: &std::collections::HashSet<(u8, u16)>) -> String {
    let mut css = String::with_capacity(used.len() * 50);
    let mut needs_spin_keyframes = false;

    // Sort for deterministic output
    let mut pairs: Vec<_> = used.iter().copied().collect();
    pairs.sort();

    for (pc_code, st_code) in pairs {
        let selector = PC_MAPPINGS
            .iter()
            .find(|(c, _)| *c == pc_code)
            .map(|(_, s)| *s)
            .unwrap_or("");
        let declaration = UTIL_MAPPINGS
            .iter()
            .find(|(c, _)| *c == st_code)
            .map(|(_, d)| *d)
            .unwrap_or("");

        if !declaration.is_empty() {
            use std::fmt::Write;
            let _ = write!(
                css,
                ".h{}u{}{}{{{}}}",
                pc_code, st_code, selector, declaration
            );
            if declaration.contains("rw-spin")
                || declaration.contains("rw-shimmer")
                || declaration.contains("rw-slide-in")
                || declaration.contains("rw-ping")
                || declaration.contains("rw-pulse")
                || declaration.contains("rw-bounce")
            {
                needs_spin_keyframes = true;
            }
        }
    }

    if needs_spin_keyframes {
        css.push_str(PSEUDO_GLOBAL_CSS);
    }

    css
}

/// Generate CSS rules for all used breakpoint (Bp, St) pairs.
///
/// Each breakpoint groups its rules into a single `@media` block:
/// `@media(min-width:{px}px){.b{bp}u{st1}{decl1}.b{bp}u{st2}{decl2}...}`
pub fn generate_breakpoint_css(used: &std::collections::HashSet<(u8, u16)>) -> String {
    if used.is_empty() {
        return String::new();
    }

    let mut css = String::with_capacity(used.len() * 60);

    // Sort pairs and group by bp_code
    let mut pairs: Vec<_> = used.iter().copied().collect();
    pairs.sort();

    // Group by breakpoint
    let mut current_bp: Option<u8> = None;

    for (bp_code, st_code) in pairs {
        let min_width = BP_MAPPINGS
            .iter()
            .find(|(c, _)| *c == bp_code)
            .map(|(_, s)| *s)
            .unwrap_or("0");
        let declaration = UTIL_MAPPINGS
            .iter()
            .find(|(c, _)| *c == st_code)
            .map(|(_, d)| *d)
            .unwrap_or("");

        if declaration.is_empty() {
            continue;
        }

        use std::fmt::Write;

        // Close previous group if switching breakpoint
        if current_bp.is_some() && current_bp != Some(bp_code) {
            css.push('}');
        }

        // Open new group if needed
        if current_bp != Some(bp_code) {
            let _ = write!(css, "@media(min-width:{}px){{", min_width);
            current_bp = Some(bp_code);
        }

        let _ = write!(css, ".b{}u{}{{{}}}", bp_code, st_code, declaration);
    }

    // Close final group
    if current_bp.is_some() {
        css.push('}');
    }

    css
}

/// Look up the CSS declaration for a utility token code.
pub fn lookup_util_css(code: u16) -> Option<&'static str> {
    UTIL_MAPPINGS
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, css)| *css)
}

/// Identifies a single class-referenced CSS rule for lazy (per-connection)
/// delivery via the `STYLE_DEF` opcode.
///
/// Composites are intentionally excluded: composite ids are only created by the
/// startup render's pattern analysis, so the set of emittable `.c{id}` classes is
/// fixed at startup and their CSS stays in the static capsule. See
/// `docs/tree-shaking-redesign.md` (Phase 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StyleKey {
    /// `.u{code}` utility class.
    Util(u16),
    /// `.h{pc}u{st}` pseudo-class rule.
    Pseudo(u8, u16),
    /// `.b{bp}u{st}` responsive rule (wrapped in `@media`).
    Breakpoint(u8, u16),
}

impl StyleKey {
    /// Render this key as a complete CSS rule (selector + declaration), matching
    /// the bulk generators (`generate_utility_css`/`generate_pseudo_css`/
    /// `generate_breakpoint_css`). Returns `None` if the token has no declaration.
    pub fn to_css_rule(self) -> Option<String> {
        match self {
            StyleKey::Util(code) => {
                let decl = lookup_util_css(code)?;
                Some(format!(".u{}{{{}}}", code, decl))
            }
            StyleKey::Pseudo(pc, st) => {
                let decl = lookup_util_css(st)?;
                if decl.is_empty() {
                    return None;
                }
                let selector = PC_MAPPINGS
                    .iter()
                    .find(|(c, _)| *c == pc)
                    .map(|(_, s)| *s)
                    .unwrap_or("");
                Some(format!(".h{}u{}{}{{{}}}", pc, st, selector, decl))
            }
            StyleKey::Breakpoint(bp, st) => {
                let decl = lookup_util_css(st)?;
                if decl.is_empty() {
                    return None;
                }
                let min_width = BP_MAPPINGS
                    .iter()
                    .find(|(c, _)| *c == bp)
                    .map(|(_, s)| *s)
                    .unwrap_or("0");
                Some(format!(
                    "@media(min-width:{}px){{.b{}u{}{{{}}}}}",
                    min_width, bp, st, decl
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

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

    #[test]
    fn test_no_duplicate_pc_codes() {
        let mut seen: HashSet<u8> = HashSet::new();
        for (code, _) in PC_MAPPINGS {
            assert!(seen.insert(*code), "Duplicate pseudo code: 0x{:02X}", code);
        }
    }

    #[test]
    fn test_generate_utility_css() {
        let mut used = HashSet::new();
        used.insert(0x02); // DisplayFlex
        used.insert(0x11); // FlexCol

        let css = generate_utility_css(&used);
        assert!(
            css.contains(".u2{display:flex}"),
            "Missing DisplayFlex rule: {css}"
        );
        assert!(
            css.contains(".u17{flex-direction:column}"),
            "Missing FlexCol rule: {css}"
        );
        // Should not contain unused tokens
        assert!(
            !css.contains(".u0{"),
            "Should not contain unused token: {css}"
        );
    }

    #[test]
    fn test_generate_pseudo_css() {
        let mut used = HashSet::new();
        // (Hover, BgAccentHover=0xC7)
        used.insert((Pc::Hover.as_u8(), St::BgAccentHover.as_u16()));
        // (FocusVisible, OutlineAccent=0xF1)
        used.insert((Pc::FocusVisible.as_u8(), St::OutlineAccent.as_u16()));

        let css = generate_pseudo_css(&used);
        assert!(css.contains(":hover{"), "Missing hover rule: {css}");
        assert!(css.contains(":focus-visible{"), "Missing focus rule: {css}");
    }

    #[test]
    fn test_generate_pseudo_css_with_keyframes() {
        let mut used = HashSet::new();
        // (After, AnimateSpinFast=0x2A1) - contains rw-spin
        used.insert((Pc::After.as_u8(), St::AnimateSpinFast.as_u16()));

        let css = generate_pseudo_css(&used);
        assert!(css.contains("rw-spin"), "Missing spinner animation: {css}");
        assert!(
            css.contains("@keyframes rw-spin"),
            "Missing keyframes: {css}"
        );
    }

    #[test]
    fn test_generate_pseudo_css_empty() {
        let used = HashSet::new();
        let css = generate_pseudo_css(&used);
        assert!(css.is_empty());
    }

    #[test]
    fn test_pc_enum_values() {
        assert_eq!(Pc::Hover.as_u8(), 0x00);
        assert_eq!(Pc::FocusVisible.as_u8(), 0x02);
        assert_eq!(Pc::Disabled.as_u8(), 0x04);
        assert_eq!(Pc::After.as_u8(), 0x08);
    }

    #[test]
    fn test_lookup_util_css() {
        assert_eq!(lookup_util_css(0x02), Some("display:flex"));
        assert_eq!(lookup_util_css(0x11), Some("flex-direction:column"));
        assert_eq!(lookup_util_css(0xFFFF), None);
    }

    #[test]
    fn test_no_duplicate_bp_codes() {
        let mut seen: HashSet<u8> = HashSet::new();
        for (code, _) in BP_MAPPINGS {
            assert!(
                seen.insert(*code),
                "Duplicate breakpoint code: 0x{:02X}",
                code
            );
        }
    }

    #[test]
    fn test_bp_enum_values() {
        assert_eq!(Bp::Sm.as_u8(), 0x00);
        assert_eq!(Bp::Md.as_u8(), 0x01);
        assert_eq!(Bp::Lg.as_u8(), 0x02);
        assert_eq!(Bp::Xl.as_u8(), 0x03);
    }

    #[test]
    fn test_generate_breakpoint_css() {
        let mut used = HashSet::new();
        // (Md, DisplayFlex=0x02)
        used.insert((Bp::Md.as_u8(), St::DisplayFlex.as_u16()));
        // (Md, FlexRow=0x10)
        used.insert((Bp::Md.as_u8(), St::FlexRow.as_u16()));
        // (Lg, GridCols3=0x14B)
        used.insert((Bp::Lg.as_u8(), St::GridCols3.as_u16()));

        let css = generate_breakpoint_css(&used);
        assert!(
            css.contains("@media(min-width:768px)"),
            "Missing md breakpoint: {css}"
        );
        assert!(
            css.contains("@media(min-width:1024px)"),
            "Missing lg breakpoint: {css}"
        );
        assert!(
            css.contains(".b1u2{display:flex}"),
            "Missing md flex rule: {css}"
        );
        assert!(css.contains(".b2u331{"), "Missing lg grid rule: {css}");
    }

    #[test]
    fn test_generate_breakpoint_css_empty() {
        let used = HashSet::new();
        let css = generate_breakpoint_css(&used);
        assert!(css.is_empty());
    }

    #[test]
    fn test_animate_ping_includes_keyframes() {
        let mut used = HashSet::new();
        used.insert(St::AnimatePing.as_u16());
        let css = generate_utility_css(&used);
        assert!(css.contains("rw-ping"), "Missing rw-ping animation: {css}");
        assert!(
            css.contains("@keyframes rw-ping"),
            "Missing rw-ping keyframes: {css}"
        );
    }

    #[test]
    fn test_animate_pulse_includes_keyframes() {
        let mut used = HashSet::new();
        used.insert(St::AnimatePulse.as_u16());
        let css = generate_utility_css(&used);
        assert!(
            css.contains("rw-pulse"),
            "Missing rw-pulse animation: {css}"
        );
        assert!(
            css.contains("@keyframes rw-pulse"),
            "Missing rw-pulse keyframes: {css}"
        );
    }

    #[test]
    fn test_animate_bounce_includes_keyframes() {
        let mut used = HashSet::new();
        used.insert(St::AnimateBounce.as_u16());
        let css = generate_utility_css(&used);
        assert!(
            css.contains("rw-bounce"),
            "Missing rw-bounce animation: {css}"
        );
        assert!(
            css.contains("@keyframes rw-bounce"),
            "Missing rw-bounce keyframes: {css}"
        );
    }
}
