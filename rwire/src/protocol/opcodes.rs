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

// ============================================================================
// State Operations
// ============================================================================

/// Initialize local state on client. Format: [INIT_LOCAL_STATE, state_idx, len, json_bytes...]
pub const INIT_LOCAL_STATE: u8 = 0x40;

/// Define a local handler. Format: [DEF_LOCAL_HANDLER, handler_idx, state_idx, mut_count, ...mutations]
pub const DEF_LOCAL_HANDLER: u8 = 0x42;

// ============================================================================
// Mutation Opcodes (for local state handlers)
// ============================================================================

/// Toggle boolean field. Format: [MUT_TOGGLE, field_idx]
pub const MUT_TOGGLE: u8 = 0x50;

/// Add i8 to numeric field. Format: [MUT_ADD_I8, field_idx, value_i8]
pub const MUT_ADD_I8: u8 = 0x51;

/// Add i32 to numeric field. Format: [MUT_ADD_I32, field_idx, b3, b2, b1, b0] (big-endian)
pub const MUT_ADD_I32: u8 = 0x52;

/// Set boolean field. Format: [MUT_SET_BOOL, field_idx, 0|1]
pub const MUT_SET_BOOL: u8 = 0x53;

/// Set i32 field. Format: [MUT_SET_I32, field_idx, b3, b2, b1, b0] (big-endian)
pub const MUT_SET_I32: u8 = 0x54;

/// Set string field. Format: [MUT_SET_STR, field_idx, len, bytes...]
pub const MUT_SET_STR: u8 = 0x55;

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

/// Apply composable pseudo-class styles.
/// Format: [STYLE_PSEUDO, ref, pc_code, count, st1_varint, st2_varint, ...]
/// pc_code is the Pc selector (u8), st tokens are varint-encoded St codes.
pub const STYLE_PSEUDO: u8 = 0x89;

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
    }
}

// ============================================================================
// Symbol Index Range
// ============================================================================

/// Symbols 0x00-0x7F are reserved/built-in
/// Symbols 0x80-0xFF are session-specific (defined via SYMBOLS opcode)
pub const SYMBOL_SESSION_START: u8 = 0x80;
