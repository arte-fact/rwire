//! DOM opcode constants for the RustWire binary protocol.

// ============================================================================
// DOM Operations
// ============================================================================

/// Create an element. Format: [CREATE, element_type] → assigns next ref
pub const CREATE: u8 = 0x02;

/// Create a synced wrapper span with id="__synced_N".
/// Format: [CREATE_SYNCED, synced_id_varint] → creates span, sets id, returns ref
/// This is more compact than CREATE span + SET_ATTR id.
pub const CREATE_SYNCED: u8 = 0x03;

/// Get a synced element by numeric ID.
/// Format: [GET_SYNCED, synced_id_varint] → returns ref to existing element
/// This is more compact than GET_BY_ID with a symbol table entry for "__synced_N".
pub const GET_SYNCED: u8 = 0x05;

/// Set class on element. Format: [SET_CLASS, ref, symbol_idx]
pub const SET_CLASS: u8 = 0x10;

/// Set text content. Format: [SET_TEXT, ref, symbol_idx]
pub const SET_TEXT: u8 = 0x11;

/// Set attribute. Format: [SET_ATTR, ref, attr_symbol, value_symbol]
pub const SET_ATTR: u8 = 0x12;

/// Set data attribute. Format: [SET_DATA, ref, key_symbol, value_symbol]
pub const SET_DATA: u8 = 0x14;

/// Append child to parent. Format: [APPEND, parent_ref, child_ref]
pub const APPEND: u8 = 0x20;

/// Clear all children from element. Format: [CLEAR_CHILDREN, ref]
pub const CLEAR_CHILDREN: u8 = 0x25;

/// Set attribute with enum key + enum value. Format: [SET_ATTR_ENUM, ref, at, av]
/// Both key and value are single-byte enum codes. 4 bytes total.
pub const SET_ATTR_ENUM: u8 = 0x26;

/// Set boolean attribute (presence-only). Format: [SET_ATTR_BOOL, ref, at]
/// The attribute is set with empty string value. 3 bytes total.
pub const SET_ATTR_BOOL: u8 = 0x27;

/// Set attribute with enum key + symbol value. Format: [SET_ATTR_KEY_SYM, ref, at, val_sym_varint]
/// Key is an enum code, value is a symbol table index. 4-5 bytes total.
pub const SET_ATTR_KEY_SYM: u8 = 0x28;

/// Get element by ID (for updates). Format: [GET_BY_ID, symbol_idx] → ref
pub const GET_BY_ID: u8 = 0x01;

// ============================================================================
// Event Binding
// ============================================================================

/// Bind local handler (WASM-only). Format: [BIND_LOCAL, ref, event_type, handler_idx]
pub const BIND_LOCAL: u8 = 0x30;

/// Bind remote handler (server round-trip). Format: [BIND_REMOTE, ref, event_type, handler_idx]
pub const BIND_REMOTE: u8 = 0x31;

/// Bind optimistic handler. Format: [BIND_OPTIMISTIC, ref, event_type, handler_idx]
pub const BIND_OPTIMISTIC: u8 = 0x32;

/// Bind debounced handler. Format: [BIND_DEBOUNCED, ref, event_type, handler_idx, ms_hi, ms_lo]
pub const BIND_DEBOUNCED: u8 = 0x33;

/// Bind remote handler with parameter. Format: [BIND_REMOTE_PARAM, ref, event_type, handler_idx, param_len, ...param_bytes]
/// The param_bytes are sent back with the event, enabling item-specific handlers.
pub const BIND_REMOTE_PARAM: u8 = 0x34;

/// Reserved for WASM builds: inline-local handler. Never emitted by this
/// server; the JS runtime skips it (stubbed `xi`). Registered here so the
/// generated runtime opcode table (`runtime/src/opcodes.ts`) has a single
/// source of truth.
pub const INLINE_LOCAL: u8 = 0x40;

/// Reserved for WASM builds: handler definition. Never emitted; see
/// [`INLINE_LOCAL`].
pub const DEF_HANDLER: u8 = 0x42;

// ============================================================================
// Client Actions (Targets & Selectors)
// ============================================================================

/// Initialize a bool target. Format: [INIT_TARGET, target_idx, default_u8]
pub const INIT_TARGET: u8 = 0x47;

/// Bind element class to target state. Format: [BIND_TARGET, ref_varint, target_idx, st_varint, invert_u8]
pub const BIND_TARGET: u8 = 0x48;

/// On event, toggle target and update bindings. Format: [BIND_TOGGLE, ref_varint, ev_type, target_idx]
pub const BIND_TOGGLE: u8 = 0x49;

/// Initialize an enum selector. Format: [INIT_SELECTOR, sel_idx, default_val]
pub const INIT_SELECTOR: u8 = 0x4A;

/// Bind element class to selector value. Format: [BIND_SELECTOR, ref_varint, sel_idx, match_val, st_varint]
pub const BIND_SELECTOR: u8 = 0x4B;

/// On event, set selector value and update bindings. Format: [BIND_SELECT, ref_varint, ev_type, sel_idx, val]
pub const BIND_SELECT: u8 = 0x4C;

/// On event: set target to true, revert to false after delay. Repeated events restart timer.
/// Format: [BIND_TIMED_TOGGLE, ref_varint, ev_type, target_idx, ms_hi, ms_lo]
pub const BIND_TIMED_TOGGLE: u8 = 0x4D;

/// After delay from mount, flip target boolean once.
/// Format: [AUTO_TOGGLE, target_idx, ms_hi, ms_lo]
pub const AUTO_TOGGLE: u8 = 0x4E;

/// Bind a one-shot visibility sentinel: an IntersectionObserver fires a remote
/// `Ev::Visible` event (with the param bytes, like BIND_REMOTE_PARAM) when the
/// element nears the viewport, then disconnects. The server's response renders
/// a fresh sentinel with new params — its binding key changes, so the morph
/// swaps in the new node with a live observer. One-request-in-flight falls out
/// structurally. Format: [BIND_SENTINEL, ref_varint, handler_varint, param_len, ...params]
pub const BIND_SENTINEL: u8 = 0x4F;

/// Bind a horizontal resize handle: pointer-dragging the element resizes its
/// **previous element sibling** (width in px, min 8rem) — entirely
/// client-side, the SplitPane primitive. Pairing by adjacency avoids
/// cross-element ref plumbing at emit time.
/// Format: [BIND_RESIZE, ref_varint]
pub const BIND_RESIZE: u8 = 0x50;

// ============================================================================
// Form Operations
// ============================================================================

/// Set validation rules on a form field. Format: [FORM_SET_VALIDATION, ref, rules_symbol]
pub const FORM_SET_VALIDATION: u8 = 0x60;

/// Show validation error on a field. Format: [FORM_SHOW_ERROR, ref, message_symbol]
pub const FORM_SHOW_ERROR: u8 = 0x61;

/// Clear validation error on a field. Format: [FORM_CLEAR_ERROR, ref]
pub const FORM_CLEAR_ERROR: u8 = 0x62;

/// Set field as required. Format: [FORM_SET_REQUIRED, ref, 0|1]
pub const FORM_SET_REQUIRED: u8 = 0x63;

// ============================================================================
// Routing Operations
// ============================================================================

/// Push new URL to history. Format: [ROUTE_PUSH, url_symbol]
pub const ROUTE_PUSH: u8 = 0x70;

/// Replace current URL in history. Format: [ROUTE_REPLACE, url_symbol]
pub const ROUTE_REPLACE: u8 = 0x71;

/// Push a new URL to history, URL carried inline (no symbol table). For
/// server-initiated navigation where the URL is computed at request time (e.g. a
/// freshly created resource). Format: [ROUTE_PUSH_INLINE, varint len, utf8 bytes]
pub const ROUTE_PUSH_INLINE: u8 = 0x72;

/// Replace the current URL, carried inline. Format: [ROUTE_REPLACE_INLINE, varint len, utf8 bytes]
pub const ROUTE_REPLACE_INLINE: u8 = 0x73;

// ============================================================================
// Styling Operations
// ============================================================================

/// Set inline style. Format: [STYLE_SET, ref, style_symbol]
pub const STYLE_SET: u8 = 0x81;

/// Set style utility token. Format: [STYLE_UTIL, ref, util_byte]
/// Single-byte utility tokens for common style declarations.
/// More compact than symbol table for frequently used styles.
pub const STYLE_UTIL: u8 = 0x82;

/// Set style property+value. Format: [STYLE_PROP, ref, prop_byte, value_byte]
/// Separate property and value codes for flexible combinations.
pub const STYLE_PROP: u8 = 0x83;

/// Set multiple style utilities. Format: [STYLE_MULTI, ref, count, util1, util2, ...]
/// Batch multiple utility tokens in a single opcode.
pub const STYLE_MULTI: u8 = 0x84;

/// Apply style composite. Format: [STYLE_COMPOSITE, ref, composite_id_varint]
/// Use pre-analyzed style patterns for maximum compression.
pub const STYLE_COMPOSITE: u8 = 0x85;

/// Define composite table. Format: [COMPOSITE_TABLE, count_varint, ...entries]
/// Each entry: [id_varint, util_count, util1, util2, ...]
pub const COMPOSITE_TABLE: u8 = 0x86;

/// Define CSS rules lazily (one batch). Format: [STYLE_DEF, count_varint,
/// (rule_len_varint, rule_utf8){count}]. Each rule is a complete CSS rule
/// including selector, e.g. `.u192{background:var(--a)}`. The client appends
/// each to a dedicated stylesheet. Sent the first time a connection references a
/// utility/pseudo/breakpoint class — see docs/tree-shaking-redesign.md (Phase 2).
pub const STYLE_DEF: u8 = 0x87;

/// Define element/event/attribute/style name-map entries lazily (one batch).
/// Format: `[MAP_DEF, count_varint, (kind_u8, code_u8, name_len_varint, name_utf8){count}]`.
/// `kind`: 0=element (`E[code]=name`), 1=event (`V`), 2=attr-key (`AT`), 3=attr-value (`AV`),
/// 4=style-prop (`P`), 5=style-value (`Y`), 6=svg-element (sets both `E[code]=name` and
/// `SE[code]=1`). The capsule ships empty maps; the server sends each `(kind, code)→name` the
/// first time a connection references it — the lazy-delivery analogue of `STYLE_DEF` for CSS.
pub const MAP_DEF: u8 = 0x88;

/// Apply composable pseudo-class styles.
/// Format: [STYLE_PSEUDO, ref, pc_code, count, st1_varint, st2_varint, ...]
/// pc_code is the Pc selector (u8), st tokens are varint-encoded St codes.
pub const STYLE_PSEUDO: u8 = 0x89;

/// Apply responsive breakpoint styles.
/// Format: [STYLE_BREAKPOINT, ref, bp_code, count, st1_varint, st2_varint, ...]
/// bp_code is the Bp breakpoint (u8), st tokens are varint-encoded St codes.
/// CSS: `@media(min-width:{px}px){.b{bp}u{st}{declaration}}`
pub const STYLE_BREAKPOINT: u8 = 0x8A;

/// Lazy runtime-extension hint: names of JS modules the DOM in this message
/// needs. The runtime dynamic-imports each from `/_rw/ext/{name}.js` once per
/// page; the server dedupes per connection (`sent_mods`), mirroring MAP_DEF.
/// Format: `[MOD_DEF, count_varint, (name_len_varint, name_utf8){count}]`.
pub const MOD_DEF: u8 = 0x8B;

// ============================================================================
// Control
// ============================================================================

/// Symbol table header. Format: [SYMBOLS, count, ...symbols]
pub const SYMBOLS: u8 = 0xF0;

/// Extend existing symbol table. Format: [SYMBOLS_EXTEND, count_varint, start_index_varint, ...symbols]
/// New symbols are added starting at start_index (typically 0x80 + existing count).
/// Use this for updates when some symbols were already sent in initial render.
pub const SYMBOLS_EXTEND: u8 = 0xF1;

/// Word table for text compression. Format: [WORD_TABLE, count, len, word, len, word, ...]
/// Words are indexed 0..count-1, most frequent words should have lowest indices.
/// Used with SET_TEXT_WORDS for efficient text encoding.
pub const WORD_TABLE: u8 = 0xF2;

/// Set text content from word indices. Format: [SET_TEXT_WORDS, ref, count, idx0, idx1, ...]
/// Words are joined with spaces. More compact than symbol table for repeated words.
pub const SET_TEXT_WORDS: u8 = 0x13;

/// Set text content to a number. Format: [SET_TEXT_INT, ref, varint]
/// More compact than symbol table for dynamic numeric values.
pub const SET_TEXT_INT: u8 = 0x15;

/// Set a morph key on an element. Format: [SET_KEY, ref_varint, key_varint]
/// The client stores it as a `__k` expando (never a DOM attribute): sibling-
/// local identity for keyed morphing, so id-less list items reorder by
/// identity instead of positionally. Keys come from `ElementBuilder::key`
/// (strings FNV-1a hashed, integers used directly).
pub const SET_KEY: u8 = 0x16;

/// End of batch marker
pub const BATCH_END: u8 = 0xFF;

// ============================================================================
// Element Types
// ============================================================================

define_token_enum! {
    /// Element type enum for fluent builder API.
    pub enum El(u8) {
        str_method = name;
        mappings = ELEMENT_MAPPINGS;

        Div = 0x00 => "div",
        Span = 0x01 => "span",
        Button = 0x02 => "button",
        Input = 0x03 => "input",
        P = 0x04 => "p",
        H1 = 0x05 => "h1",
        H2 = 0x06 => "h2",
        A = 0x07 => "a",
        Textarea = 0x08 => "textarea",
        Select = 0x09 => "select",
        Option = 0x0A => "option",
        Label = 0x0B => "label",
        Fieldset = 0x0C => "fieldset",
        Legend = 0x0D => "legend",
        Form = 0x10 => "form",
        Ul = 0x11 => "ul",
        Li = 0x12 => "li",
        Nav = 0x13 => "nav",
        Header = 0x14 => "header",
        Footer = 0x15 => "footer",
        Section = 0x16 => "section",
        Article = 0x17 => "article",
        Svg = 0x18 => "svg",
        Path = 0x19 => "path",
        H3 = 0x1A => "h3",
        Hr = 0x1B => "hr",
        Ol = 0x1C => "ol",
        Pre = 0x1D => "pre",
        Code = 0x1E => "code",
        Blockquote = 0x1F => "blockquote",
        Strong = 0x20 => "strong",
        Em = 0x21 => "em",
        Img = 0x22 => "img",
        Table = 0x23 => "table",
        Thead = 0x24 => "thead",
        Tbody = 0x25 => "tbody",
        Tr = 0x26 => "tr",
        Th = 0x27 => "th",
        Td = 0x28 => "td",
        Aside = 0x29 => "aside",
        Main = 0x2A => "main",
        Kbd = 0x2B => "kbd",
        Circle = 0x2C => "circle",
        Line = 0x2D => "line",
        Polyline = 0x2E => "polyline",
        Rect = 0x2F => "rect",
        G = 0x30 => "g",
        Style = 0x31 => "style",
        Details = 0x32 => "details",
        Summary = 0x33 => "summary",
    }
}

// ============================================================================
// Event Types
// ============================================================================

define_token_enum! {
    /// Event type enum for fluent builder API.
    pub enum Ev(u8) {
        str_method = name;
        mappings = EVENT_MAPPINGS;

        Click = 0x01 => "click",
        DblClick = 0x02 => "dblclick",
        MouseDown = 0x03 => "mousedown",
        MouseUp = 0x04 => "mouseup",
        MouseMove = 0x05 => "mousemove",
        Submit = 0x06 => "submit",
        Input = 0x07 => "input",
        Change = 0x08 => "change",
        KeyDown = 0x09 => "keydown",
        KeyUp = 0x0A => "keyup",
        Focus = 0x0B => "focus",
        Blur = 0x0C => "blur",
        Scroll = 0x0D => "scroll",
        /// Synthetic: fired by the scroll-sentinel primitive (BIND_SENTINEL)
        /// when the element nears the viewport; never bound as a DOM listener.
        Visible = 0x0E => "visible",
    }
}

// ============================================================================
// Symbol Index Range
// ============================================================================

/// SVG element type codes — these require `createElementNS` in the browser.
/// Used by capsule_gen to produce a tree-shaken SVG type set for the JS runtime.
pub const SVG_ELEMENT_CODES: &[u8] = &[
    0x18, // Svg
    0x19, // Path
    0x2C, // Circle
    0x2D, // Line
    0x2E, // Polyline
    0x2F, // Rect
    0x30, // G
];

/// Symbols 0x00-0x7F are reserved/built-in
/// Symbols 0x80-0xFF are session-specific (defined via SYMBOLS opcode)
pub const SYMBOL_SESSION_START: u8 = 0x80;
