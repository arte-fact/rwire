//! DOM opcode constants for the RustWire binary protocol.

// ============================================================================
// DOM Operations
// ============================================================================

/// Create an element. Format: [CREATE, element_type] → assigns next ref
pub const CREATE: u8 = 0x02;

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

// ============================================================================
// Control
// ============================================================================

/// Symbol table header. Format: [SYMBOLS, count, ...symbols]
pub const SYMBOLS: u8 = 0xF0;

/// End of batch marker
pub const BATCH_END: u8 = 0xFF;

// ============================================================================
// Element Types
// ============================================================================

pub const EL_DIV: u8 = 0x00;
pub const EL_SPAN: u8 = 0x01;
pub const EL_BUTTON: u8 = 0x02;
pub const EL_INPUT: u8 = 0x03;
pub const EL_FORM: u8 = 0x10;
pub const EL_P: u8 = 0x04;
pub const EL_H1: u8 = 0x05;
pub const EL_H2: u8 = 0x06;
pub const EL_A: u8 = 0x07;

/// Element type enum for fluent builder API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum El {
    Div,
    Span,
    Button,
    Input,
    Form,
    P,
    H1,
    H2,
    A,
}

impl El {
    /// Convert to the wire protocol byte value.
    pub fn as_u8(self) -> u8 {
        match self {
            El::Div => EL_DIV,
            El::Span => EL_SPAN,
            El::Button => EL_BUTTON,
            El::Input => EL_INPUT,
            El::Form => EL_FORM,
            El::P => EL_P,
            El::H1 => EL_H1,
            El::H2 => EL_H2,
            El::A => EL_A,
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
