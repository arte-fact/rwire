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

pub const EL_DIV: u8 = 0x00;
pub const EL_SPAN: u8 = 0x01;
pub const EL_BUTTON: u8 = 0x02;
pub const EL_INPUT: u8 = 0x03;
pub const EL_P: u8 = 0x04;
pub const EL_H1: u8 = 0x05;
pub const EL_H2: u8 = 0x06;
pub const EL_A: u8 = 0x07;
pub const EL_TEXTAREA: u8 = 0x08;
pub const EL_SELECT: u8 = 0x09;
pub const EL_OPTION: u8 = 0x0A;
pub const EL_LABEL: u8 = 0x0B;
pub const EL_FIELDSET: u8 = 0x0C;
pub const EL_LEGEND: u8 = 0x0D;
pub const EL_FORM: u8 = 0x10;
pub const EL_UL: u8 = 0x11;
pub const EL_LI: u8 = 0x12;
pub const EL_NAV: u8 = 0x13;
pub const EL_HEADER: u8 = 0x14;
pub const EL_FOOTER: u8 = 0x15;
pub const EL_SECTION: u8 = 0x16;
pub const EL_ARTICLE: u8 = 0x17;
pub const EL_SVG: u8 = 0x18;
pub const EL_PATH: u8 = 0x19;
pub const EL_H3: u8 = 0x1A;
pub const EL_HR: u8 = 0x1B;
pub const EL_OL: u8 = 0x1C;

/// Element type enum for fluent builder API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum El {
    Div,
    Span,
    Button,
    Input,
    P,
    H1,
    H2,
    A,
    Textarea,
    Select,
    Option,
    Label,
    Fieldset,
    Legend,
    Form,
    Ul,
    Li,
    Nav,
    Header,
    Footer,
    Section,
    Article,
    Svg,
    Path,
    H3,
    Hr,
    Ol,
}

impl El {
    /// Convert to the wire protocol byte value.
    pub fn as_u8(self) -> u8 {
        match self {
            El::Div => EL_DIV,
            El::Span => EL_SPAN,
            El::Button => EL_BUTTON,
            El::Input => EL_INPUT,
            El::P => EL_P,
            El::H1 => EL_H1,
            El::H2 => EL_H2,
            El::A => EL_A,
            El::Textarea => EL_TEXTAREA,
            El::Select => EL_SELECT,
            El::Option => EL_OPTION,
            El::Label => EL_LABEL,
            El::Fieldset => EL_FIELDSET,
            El::Legend => EL_LEGEND,
            El::Form => EL_FORM,
            El::Ul => EL_UL,
            El::Li => EL_LI,
            El::Nav => EL_NAV,
            El::Header => EL_HEADER,
            El::Footer => EL_FOOTER,
            El::Section => EL_SECTION,
            El::Article => EL_ARTICLE,
            El::Svg => EL_SVG,
            El::Path => EL_PATH,
            El::H3 => EL_H3,
            El::Hr => EL_HR,
            El::Ol => EL_OL,
        }
    }
}

// ============================================================================
// Event Types
// ============================================================================

pub const EV_CLICK: u8 = 0x01;
pub const EV_DBLCLICK: u8 = 0x02;
pub const EV_MOUSEDOWN: u8 = 0x03;
pub const EV_MOUSEUP: u8 = 0x04;
pub const EV_MOUSEMOVE: u8 = 0x05;
pub const EV_SUBMIT: u8 = 0x06;
pub const EV_INPUT: u8 = 0x07;
pub const EV_CHANGE: u8 = 0x08;
pub const EV_KEYDOWN: u8 = 0x09;
pub const EV_KEYUP: u8 = 0x0A;
pub const EV_FOCUS: u8 = 0x0B;
pub const EV_BLUR: u8 = 0x0C;

/// Event type enum for fluent builder API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ev {
    Click,
    DblClick,
    MouseDown,
    MouseUp,
    MouseMove,
    Submit,
    Input,
    Change,
    KeyDown,
    KeyUp,
    Focus,
    Blur,
}

impl Ev {
    /// Convert to the wire protocol byte value.
    pub fn as_u8(self) -> u8 {
        match self {
            Ev::Click => EV_CLICK,
            Ev::DblClick => EV_DBLCLICK,
            Ev::MouseDown => EV_MOUSEDOWN,
            Ev::MouseUp => EV_MOUSEUP,
            Ev::MouseMove => EV_MOUSEMOVE,
            Ev::Submit => EV_SUBMIT,
            Ev::Input => EV_INPUT,
            Ev::Change => EV_CHANGE,
            Ev::KeyDown => EV_KEYDOWN,
            Ev::KeyUp => EV_KEYUP,
            Ev::Focus => EV_FOCUS,
            Ev::Blur => EV_BLUR,
        }
    }
}

// ============================================================================
// Symbol Index Range
// ============================================================================

/// Symbols 0x00-0x7F are reserved/built-in
/// Symbols 0x80-0xFF are session-specific (defined via SYMBOLS opcode)
pub const SYMBOL_SESSION_START: u8 = 0x80;
