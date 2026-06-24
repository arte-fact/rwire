//! Binary encoder for DOM opcodes.
//!
//! All element refs and handler indices use varint encoding, supporting
//! unlimited elements per message (practical limit ~16K with 2-byte varints).

use bytes::{BufMut, BytesMut};

use super::opcodes::{
    APPEND, AUTO_TOGGLE, BATCH_END, BIND_DEBOUNCED, BIND_LOCAL, BIND_REMOTE, BIND_REMOTE_PARAM,
    BIND_SELECT, BIND_SELECTOR, BIND_TARGET, BIND_TIMED_TOGGLE, BIND_TOGGLE, CLEAR_CHILDREN,
    COMPOSITE_TABLE, CREATE, CREATE_SYNCED, FORM_CLEAR_ERROR, FORM_SET_REQUIRED,
    FORM_SET_VALIDATION, FORM_SHOW_ERROR, GET_BY_ID, GET_SYNCED, INIT_SELECTOR, INIT_TARGET,
    ROUTE_PUSH, ROUTE_PUSH_INLINE, ROUTE_REPLACE, ROUTE_REPLACE_INLINE, SET_ATTR, SET_ATTR_BOOL,
    SET_ATTR_ENUM, SET_ATTR_KEY_SYM, SET_CLASS, SET_DATA, SET_TEXT, SET_TEXT_INT, SET_TEXT_WORDS,
    STYLE_BREAKPOINT, STYLE_COMPOSITE, STYLE_MULTI, STYLE_PROP, STYLE_PSEUDO, STYLE_SET,
    STYLE_UTIL, SYMBOLS, SYMBOLS_EXTEND, SYMBOL_SESSION_START, WORD_TABLE,
};
use super::varint::write_varint;
use crate::style_tokens::StyleKey;
use std::collections::BTreeSet;

/// Sentinel ref value for document.body in APPEND opcodes.
pub const BODY_REF: u32 = 0xFFFF;

/// Buffer for building opcode sequences.
///
/// Element refs, symbol indices, and handler indices all use varint encoding,
/// supporting pages with thousands of elements.
pub struct OpcodeBuffer {
    buf: BytesMut,
    next_ref: u32,
    next_symbol: u32,
    /// Class-referenced style rules emitted into this buffer, for lazy CSS
    /// delivery (`STYLE_DEF`). Composites are excluded (their CSS is static).
    referenced_styles: BTreeSet<StyleKey>,
}

impl OpcodeBuffer {
    pub fn new() -> Self {
        Self {
            buf: BytesMut::with_capacity(256),
            next_ref: 0,
            next_symbol: SYMBOL_SESSION_START as u32,
            referenced_styles: BTreeSet::new(),
        }
    }

    /// The set of class-referenced style rules emitted so far (for `STYLE_DEF`).
    pub fn referenced_styles(&self) -> &BTreeSet<StyleKey> {
        &self.referenced_styles
    }

    /// Get the current ref count.
    pub fn ref_count(&self) -> u32 {
        self.next_ref
    }

    /// Start a symbol table. Call `add_symbol` for each symbol, then continue with DOM ops.
    ///
    /// Count is encoded as varint to support >255 symbols.
    pub fn begin_symbols(&mut self, count: u32) -> &mut Self {
        self.buf.put_u8(SYMBOLS);
        write_varint(&mut self.buf, count);
        self
    }

    /// Add a symbol to the symbol table. Returns the symbol index.
    ///
    /// Symbol indices use varint encoding, allowing unlimited symbols.
    pub fn add_symbol(&mut self, s: &str) -> u32 {
        let idx = self.next_symbol;
        write_varint(&mut self.buf, s.len() as u32);
        self.buf.put_slice(s.as_bytes());
        self.next_symbol += 1;
        idx
    }

    /// Start extending an existing symbol table.
    ///
    /// Use this for updates when some symbols were already sent in the initial render.
    /// The `start_index` should be the next available symbol index.
    ///
    /// Both count and start_index use varint encoding.
    pub fn begin_symbols_extend(&mut self, count: u32, start_index: u32) -> &mut Self {
        self.buf.put_u8(SYMBOLS_EXTEND);
        write_varint(&mut self.buf, count);
        write_varint(&mut self.buf, start_index);
        self.next_symbol = start_index;
        self
    }

    /// Start a word table for text compression.
    ///
    /// Words are indexed 0..count-1. Use `add_word` to add each word.
    /// Most frequent words should be added first (lowest indices).
    pub fn begin_word_table(&mut self, count: u8) -> &mut Self {
        self.buf.put_u8(WORD_TABLE);
        self.buf.put_u8(count);
        self
    }

    /// Add a word to the word table.
    ///
    /// The byte length is varint-encoded to match the client decoder (`rv`), which
    /// reads it as a varint. A plain `u8` would desync the whole stream for any word
    /// of 128–255 bytes (the high bit makes the decoder read a multi-byte varint) —
    /// e.g. a long unbroken URL token. Words ≤127 bytes encode identically, so this is
    /// wire-compatible with existing clients.
    pub fn add_word(&mut self, word: &str) -> &mut Self {
        write_varint(&mut self.buf, word.len() as u32);
        self.buf.put_slice(word.as_bytes());
        self
    }

    /// Create an element. Returns the ref index.
    pub fn create(&mut self, element_type: u8) -> u32 {
        let ref_idx = self.next_ref;
        self.buf.put_u8(CREATE);
        self.buf.put_u8(element_type);
        self.next_ref += 1;
        ref_idx
    }

    /// Create a synced wrapper span with id="__synced_N".
    ///
    /// This is more compact than CREATE span + SET_ATTR id, saving ~15 bytes per synced element.
    /// Format: [CREATE_SYNCED, synced_id_varint] → creates span, sets id, returns ref
    pub fn create_synced(&mut self, synced_id: u32) -> u32 {
        let ref_idx = self.next_ref;
        self.buf.put_u8(CREATE_SYNCED);
        write_varint(&mut self.buf, synced_id);
        self.next_ref += 1;
        ref_idx
    }

    /// Get a synced element by numeric ID.
    ///
    /// This is more compact than GET_BY_ID with a symbol for "__synced_N".
    /// Format: [GET_SYNCED, synced_id_varint] → returns ref
    pub fn get_synced(&mut self, synced_id: u32) -> u32 {
        let ref_idx = self.next_ref;
        self.buf.put_u8(GET_SYNCED);
        write_varint(&mut self.buf, synced_id);
        self.next_ref += 1;
        ref_idx
    }

    /// Set class on an element.
    ///
    /// Both ref and symbol index use varint encoding.
    pub fn set_class(&mut self, ref_idx: u32, symbol_idx: u32) -> &mut Self {
        self.buf.put_u8(SET_CLASS);
        write_varint(&mut self.buf, ref_idx);
        write_varint(&mut self.buf, symbol_idx);
        self
    }

    /// Set text content on an element.
    ///
    /// Both ref and symbol index use varint encoding.
    pub fn set_text(&mut self, ref_idx: u32, symbol_idx: u32) -> &mut Self {
        self.buf.put_u8(SET_TEXT);
        write_varint(&mut self.buf, ref_idx);
        write_varint(&mut self.buf, symbol_idx);
        self
    }

    /// Set text content from word indices (words joined with spaces).
    ///
    /// More compact than symbol table when words are reused across strings.
    /// Format: [SET_TEXT_WORDS, ref_varint, count, idx0, idx1, ...]
    pub fn set_text_words(&mut self, ref_idx: u32, word_indices: &[u8]) -> &mut Self {
        assert!(
            word_indices.len() <= 255,
            "set_text_words: too many words ({}, max 255)",
            word_indices.len()
        );
        self.buf.put_u8(SET_TEXT_WORDS);
        write_varint(&mut self.buf, ref_idx);
        self.buf.put_u8(word_indices.len() as u8);
        for &idx in word_indices {
            self.buf.put_u8(idx);
        }
        self
    }

    /// Set text content to a number (varint encoded).
    ///
    /// More compact than symbol table for dynamic numeric values.
    /// Format: [SET_TEXT_INT, ref_varint, zigzag_varint]
    pub fn set_text_int(&mut self, ref_idx: u32, value: i32) -> &mut Self {
        self.buf.put_u8(SET_TEXT_INT);
        write_varint(&mut self.buf, ref_idx);
        // Encode as zigzag varint for signed integers
        let zigzag = ((value << 1) ^ (value >> 31)) as u32;
        write_varint(&mut self.buf, zigzag);
        self
    }

    /// Set an attribute on an element.
    ///
    /// Ref and symbol indices use varint encoding.
    pub fn set_attr(&mut self, ref_idx: u32, attr_symbol: u32, value_symbol: u32) -> &mut Self {
        self.buf.put_u8(SET_ATTR);
        write_varint(&mut self.buf, ref_idx);
        write_varint(&mut self.buf, attr_symbol);
        write_varint(&mut self.buf, value_symbol);
        self
    }

    /// Set a data attribute on an element.
    ///
    /// Ref and symbol indices use varint encoding.
    pub fn set_data(&mut self, ref_idx: u32, key_symbol: u32, value_symbol: u32) -> &mut Self {
        self.buf.put_u8(SET_DATA);
        write_varint(&mut self.buf, ref_idx);
        write_varint(&mut self.buf, key_symbol);
        write_varint(&mut self.buf, value_symbol);
        self
    }

    /// Append a child to a parent element.
    pub fn append(&mut self, parent_ref: u32, child_ref: u32) -> &mut Self {
        self.buf.put_u8(APPEND);
        write_varint(&mut self.buf, parent_ref);
        write_varint(&mut self.buf, child_ref);
        self
    }

    /// Append a child to the document body (parent = BODY_REF sentinel).
    pub fn append_to_body(&mut self, child_ref: u32) -> &mut Self {
        self.buf.put_u8(APPEND);
        write_varint(&mut self.buf, BODY_REF);
        write_varint(&mut self.buf, child_ref);
        self
    }

    /// Clear all children from an element.
    pub fn clear_children(&mut self, ref_idx: u32) -> &mut Self {
        self.buf.put_u8(CLEAR_CHILDREN);
        write_varint(&mut self.buf, ref_idx);
        self
    }

    /// Bind a local (WASM-only) event handler.
    pub fn bind_local(&mut self, ref_idx: u32, event_type: u8, handler_idx: u32) -> &mut Self {
        self.buf.put_u8(BIND_LOCAL);
        write_varint(&mut self.buf, ref_idx);
        self.buf.put_u8(event_type);
        write_varint(&mut self.buf, handler_idx);
        self
    }

    /// Bind a remote event handler (requires server round-trip).
    pub fn bind_remote(&mut self, ref_idx: u32, event_type: u8, handler_idx: u32) -> &mut Self {
        self.buf.put_u8(BIND_REMOTE);
        write_varint(&mut self.buf, ref_idx);
        self.buf.put_u8(event_type);
        write_varint(&mut self.buf, handler_idx);
        self
    }

    /// Bind a debounced remote event handler.
    ///
    /// Format: [BIND_DEBOUNCED, ref_varint, event_type, handler_varint, ms_hi, ms_lo]
    pub fn bind_debounced(
        &mut self,
        ref_idx: u32,
        event_type: u8,
        handler_idx: u32,
        delay_ms: u16,
    ) -> &mut Self {
        self.buf.put_u8(BIND_DEBOUNCED);
        write_varint(&mut self.buf, ref_idx);
        self.buf.put_u8(event_type);
        write_varint(&mut self.buf, handler_idx);
        self.buf.put_u8((delay_ms >> 8) as u8);
        self.buf.put_u8((delay_ms & 0xFF) as u8);
        self
    }

    /// Bind a remote event handler with parameter bytes.
    ///
    /// The param_bytes are stored on the element and sent back with the event,
    /// enabling item-specific handlers for dynamically generated content.
    ///
    /// Format: [BIND_REMOTE_PARAM, ref_varint, event_type, handler_varint, param_len, ...param_bytes]
    pub fn bind_remote_param(
        &mut self,
        ref_idx: u32,
        event_type: u8,
        handler_idx: u32,
        param_bytes: &[u8],
    ) -> &mut Self {
        assert!(
            param_bytes.len() <= 255,
            "bind_remote_param: param_bytes too large ({} bytes, max 255)",
            param_bytes.len()
        );
        self.buf.put_u8(BIND_REMOTE_PARAM);
        write_varint(&mut self.buf, ref_idx);
        self.buf.put_u8(event_type);
        write_varint(&mut self.buf, handler_idx);
        self.buf.put_u8(param_bytes.len() as u8);
        self.buf.put_slice(param_bytes);
        self
    }

    /// Get element by ID (for updates). Returns next ref index.
    ///
    /// Symbol index uses varint encoding.
    pub fn get_by_id(&mut self, symbol_idx: u32) -> u32 {
        let ref_idx = self.next_ref;
        self.buf.put_u8(GET_BY_ID);
        write_varint(&mut self.buf, symbol_idx);
        self.next_ref += 1;
        ref_idx
    }

    // ========================================================================
    // Form Operations
    // ========================================================================

    /// Set validation rules on a form field.
    ///
    /// Ref and symbol index use varint encoding.
    pub fn form_set_validation(&mut self, ref_idx: u32, rules_symbol: u32) -> &mut Self {
        self.buf.put_u8(FORM_SET_VALIDATION);
        write_varint(&mut self.buf, ref_idx);
        write_varint(&mut self.buf, rules_symbol);
        self
    }

    /// Show validation error on a field.
    ///
    /// Ref and symbol index use varint encoding.
    pub fn form_show_error(&mut self, ref_idx: u32, message_symbol: u32) -> &mut Self {
        self.buf.put_u8(FORM_SHOW_ERROR);
        write_varint(&mut self.buf, ref_idx);
        write_varint(&mut self.buf, message_symbol);
        self
    }

    /// Clear validation error on a field.
    pub fn form_clear_error(&mut self, ref_idx: u32) -> &mut Self {
        self.buf.put_u8(FORM_CLEAR_ERROR);
        write_varint(&mut self.buf, ref_idx);
        self
    }

    /// Set field as required.
    pub fn form_set_required(&mut self, ref_idx: u32, required: bool) -> &mut Self {
        self.buf.put_u8(FORM_SET_REQUIRED);
        write_varint(&mut self.buf, ref_idx);
        self.buf.put_u8(if required { 1 } else { 0 });
        self
    }

    // ========================================================================
    // Routing Operations
    // ========================================================================

    /// Push new URL to history.
    ///
    /// Symbol index uses varint encoding.
    pub fn route_push(&mut self, url_symbol: u32) -> &mut Self {
        self.buf.put_u8(ROUTE_PUSH);
        write_varint(&mut self.buf, url_symbol);
        self
    }

    /// Replace current URL in history.
    ///
    /// Symbol index uses varint encoding.
    pub fn route_replace(&mut self, url_symbol: u32) -> &mut Self {
        self.buf.put_u8(ROUTE_REPLACE);
        write_varint(&mut self.buf, url_symbol);
        self
    }

    /// Push a new URL to history, carrying the URL inline (no symbol table).
    pub fn route_push_inline(&mut self, url: &str) -> &mut Self {
        self.buf.put_u8(ROUTE_PUSH_INLINE);
        write_varint(&mut self.buf, url.len() as u32);
        self.buf.put_slice(url.as_bytes());
        self
    }

    /// Replace the current URL, carrying the URL inline (no symbol table).
    pub fn route_replace_inline(&mut self, url: &str) -> &mut Self {
        self.buf.put_u8(ROUTE_REPLACE_INLINE);
        write_varint(&mut self.buf, url.len() as u32);
        self.buf.put_slice(url.as_bytes());
        self
    }

    // ========================================================================
    // Styling Operations
    // ========================================================================

    /// Set inline style on an element.
    ///
    /// Ref and symbol index use varint encoding.
    pub fn style_set(&mut self, ref_idx: u32, style_symbol: u32) -> &mut Self {
        self.buf.put_u8(STYLE_SET);
        write_varint(&mut self.buf, ref_idx);
        write_varint(&mut self.buf, style_symbol);
        self
    }

    /// Set a style utility token on an element.
    ///
    /// Format: [STYLE_UTIL, ref_varint, util_varint]
    pub fn style_util(&mut self, ref_idx: u32, util: u16) -> &mut Self {
        self.buf.put_u8(STYLE_UTIL);
        write_varint(&mut self.buf, ref_idx);
        write_varint(&mut self.buf, util as u32);
        self.referenced_styles.insert(StyleKey::Util(util));
        self
    }

    /// Set a style property+value on an element.
    ///
    /// Format: [STYLE_PROP, ref_varint, prop_byte, value_byte]
    pub fn style_prop(&mut self, ref_idx: u32, prop: u8, value: u8) -> &mut Self {
        self.buf.put_u8(STYLE_PROP);
        write_varint(&mut self.buf, ref_idx);
        self.buf.put_u8(prop);
        self.buf.put_u8(value);
        self
    }

    /// Set multiple style utility tokens on an element.
    ///
    /// Format: [STYLE_MULTI, ref_varint, count, util1_varint, util2_varint, ...]
    pub fn style_multi(&mut self, ref_idx: u32, utils: &[u16]) -> &mut Self {
        assert!(
            utils.len() <= 255,
            "style_multi: too many tokens ({}, max 255)",
            utils.len()
        );
        self.buf.put_u8(STYLE_MULTI);
        write_varint(&mut self.buf, ref_idx);
        self.buf.put_u8(utils.len() as u8);
        for &util in utils {
            write_varint(&mut self.buf, util as u32);
            self.referenced_styles.insert(StyleKey::Util(util));
        }
        self
    }

    /// Apply a style composite by ID.
    ///
    /// Format: [STYLE_COMPOSITE, ref_varint, composite_id_varint]
    pub fn style_composite(&mut self, ref_idx: u32, composite_id: u32) -> &mut Self {
        self.buf.put_u8(STYLE_COMPOSITE);
        write_varint(&mut self.buf, ref_idx);
        write_varint(&mut self.buf, composite_id);
        self
    }

    /// Apply composable pseudo-class styles to an element.
    ///
    /// Format: [STYLE_PSEUDO, ref_varint, pc_code, count, st1_varint, st2_varint, ...]
    pub fn style_pseudo(&mut self, ref_idx: u32, pc: u8, st_tokens: &[u16]) -> &mut Self {
        assert!(
            st_tokens.len() <= 255,
            "style_pseudo: too many tokens ({}, max 255)",
            st_tokens.len()
        );
        self.buf.put_u8(STYLE_PSEUDO);
        write_varint(&mut self.buf, ref_idx);
        self.buf.put_u8(pc);
        self.buf.put_u8(st_tokens.len() as u8);
        for &token in st_tokens {
            write_varint(&mut self.buf, token as u32);
            self.referenced_styles.insert(StyleKey::Pseudo(pc, token));
        }
        self
    }

    /// Apply responsive breakpoint styles to an element.
    ///
    /// Format: [STYLE_BREAKPOINT, ref_varint, bp_code, count, st1_varint, st2_varint, ...]
    pub fn style_breakpoint(&mut self, ref_idx: u32, bp: u8, st_tokens: &[u16]) -> &mut Self {
        assert!(
            st_tokens.len() <= 255,
            "style_breakpoint: too many tokens ({}, max 255)",
            st_tokens.len()
        );
        self.buf.put_u8(STYLE_BREAKPOINT);
        write_varint(&mut self.buf, ref_idx);
        self.buf.put_u8(bp);
        self.buf.put_u8(st_tokens.len() as u8);
        for &token in st_tokens {
            write_varint(&mut self.buf, token as u32);
            self.referenced_styles
                .insert(StyleKey::Breakpoint(bp, token));
        }
        self
    }

    /// Define the composite table.
    ///
    /// Format: [COMPOSITE_TABLE, count_varint, ...entries]
    /// Each entry: [id_varint, util_count, util1, util2, ...]
    pub fn composite_table(&mut self, table: &crate::style_groups::CompositeTable) -> &mut Self {
        if table.is_empty() {
            return self;
        }

        self.buf.put_u8(COMPOSITE_TABLE);
        let table_bytes = table.to_bytes();
        self.buf.extend_from_slice(&table_bytes);
        self
    }

    // ========================================================================
    // Binary Attribute Operations
    // ========================================================================

    /// Set attribute with enum key + enum value.
    ///
    /// Format: [SET_ATTR_ENUM, ref_varint, at, av]
    pub fn set_attr_enum(&mut self, ref_idx: u32, at: u8, av: u8) -> &mut Self {
        self.buf.put_u8(SET_ATTR_ENUM);
        write_varint(&mut self.buf, ref_idx);
        self.buf.put_u8(at);
        self.buf.put_u8(av);
        self
    }

    /// Set boolean attribute (presence-only, e.g. disabled, required).
    ///
    /// Format: [SET_ATTR_BOOL, ref_varint, at]
    pub fn set_attr_bool(&mut self, ref_idx: u32, at: u8) -> &mut Self {
        self.buf.put_u8(SET_ATTR_BOOL);
        write_varint(&mut self.buf, ref_idx);
        self.buf.put_u8(at);
        self
    }

    /// Set attribute with enum key + symbol table value.
    ///
    /// Format: [SET_ATTR_KEY_SYM, ref_varint, at, val_sym_varint]
    pub fn set_attr_key_sym(&mut self, ref_idx: u32, at: u8, value_symbol: u32) -> &mut Self {
        self.buf.put_u8(SET_ATTR_KEY_SYM);
        write_varint(&mut self.buf, ref_idx);
        self.buf.put_u8(at);
        write_varint(&mut self.buf, value_symbol);
        self
    }

    // ========================================================================
    // Client Actions (Targets & Selectors)
    // ========================================================================

    /// Initialize a bool target with default value.
    ///
    /// Format: [INIT_TARGET, target_idx, default_u8]
    pub fn init_target(&mut self, idx: u8, default: bool) -> &mut Self {
        self.buf.put_u8(INIT_TARGET);
        self.buf.put_u8(idx);
        self.buf.put_u8(if default { 1 } else { 0 });
        self
    }

    /// Bind an element's class to a target's boolean state.
    ///
    /// When target is true (or false if inverted), adds CSS class `u{st}` to the element.
    ///
    /// Format: [BIND_TARGET, ref_varint, target_idx, st_varint, invert_u8]
    pub fn bind_target(&mut self, ref_idx: u32, idx: u8, st: u16, invert: bool) -> &mut Self {
        self.buf.put_u8(BIND_TARGET);
        write_varint(&mut self.buf, ref_idx);
        self.buf.put_u8(idx);
        write_varint(&mut self.buf, st as u32);
        self.buf.put_u8(if invert { 1 } else { 0 });
        self.referenced_styles.insert(StyleKey::Util(st));
        self
    }

    /// Bind an event to toggle a target.
    ///
    /// Format: [BIND_TOGGLE, ref_varint, ev_type, target_idx]
    pub fn bind_toggle(&mut self, ref_idx: u32, ev_type: u8, idx: u8) -> &mut Self {
        self.buf.put_u8(BIND_TOGGLE);
        write_varint(&mut self.buf, ref_idx);
        self.buf.put_u8(ev_type);
        self.buf.put_u8(idx);
        self
    }

    /// Initialize an enum selector with default value.
    ///
    /// Format: [INIT_SELECTOR, sel_idx, default_val]
    pub fn init_selector(&mut self, idx: u8, default: u8) -> &mut Self {
        self.buf.put_u8(INIT_SELECTOR);
        self.buf.put_u8(idx);
        self.buf.put_u8(default);
        self
    }

    /// Bind an element's class to a selector value match.
    ///
    /// When selector equals `match_val`, adds CSS class `u{st}` to the element.
    ///
    /// Format: [BIND_SELECTOR, ref_varint, sel_idx, match_val, st_varint]
    pub fn bind_selector(&mut self, ref_idx: u32, idx: u8, match_val: u8, st: u16) -> &mut Self {
        self.buf.put_u8(BIND_SELECTOR);
        write_varint(&mut self.buf, ref_idx);
        self.buf.put_u8(idx);
        self.buf.put_u8(match_val);
        write_varint(&mut self.buf, st as u32);
        self.referenced_styles.insert(StyleKey::Util(st));
        self
    }

    /// Bind an event to set a selector value.
    ///
    /// Format: [BIND_SELECT, ref_varint, ev_type, sel_idx, val]
    pub fn bind_select(&mut self, ref_idx: u32, ev_type: u8, idx: u8, val: u8) -> &mut Self {
        self.buf.put_u8(BIND_SELECT);
        write_varint(&mut self.buf, ref_idx);
        self.buf.put_u8(ev_type);
        self.buf.put_u8(idx);
        self.buf.put_u8(val);
        self
    }

    /// Bind an event to set a target true, then revert to false after a delay.
    /// Repeated events restart the timer.
    ///
    /// Format: [BIND_TIMED_TOGGLE, ref_varint, ev_type, target_idx, ms_hi, ms_lo]
    pub fn bind_timed_toggle(
        &mut self,
        ref_idx: u32,
        ev_type: u8,
        idx: u8,
        delay_ms: u16,
    ) -> &mut Self {
        self.buf.put_u8(BIND_TIMED_TOGGLE);
        write_varint(&mut self.buf, ref_idx);
        self.buf.put_u8(ev_type);
        self.buf.put_u8(idx);
        self.buf.put_u8((delay_ms >> 8) as u8);
        self.buf.put_u8((delay_ms & 0xFF) as u8);
        self
    }

    /// After delay from mount, flip target boolean once.
    ///
    /// Format: [AUTO_TOGGLE, target_idx, ms_hi, ms_lo]
    pub fn auto_toggle(&mut self, idx: u8, delay_ms: u16) -> &mut Self {
        self.buf.put_u8(AUTO_TOGGLE);
        self.buf.put_u8(idx);
        self.buf.put_u8((delay_ms >> 8) as u8);
        self.buf.put_u8((delay_ms & 0xFF) as u8);
        self
    }

    /// End the batch.
    pub fn end(&mut self) -> &mut Self {
        self.buf.put_u8(BATCH_END);
        self
    }

    /// Get the encoded bytes.
    pub fn finish(self) -> bytes::Bytes {
        self.buf.freeze()
    }

    /// Get current buffer length.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Check if buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Get the buffer contents as a slice (for debugging).
    pub fn as_slice(&self) -> &[u8] {
        &self.buf
    }
}

impl Default for OpcodeBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_target() {
        let mut buf = OpcodeBuffer::new();
        buf.init_target(0, true);
        let bytes = buf.as_slice();
        assert_eq!(bytes[0], INIT_TARGET);
        assert_eq!(bytes[1], 0); // target_idx
        assert_eq!(bytes[2], 1); // default = true
    }

    #[test]
    fn route_push_inline_carries_the_url_in_place() {
        let mut buf = OpcodeBuffer::new();
        buf.route_push_inline("/chat/abc");
        let bytes = buf.as_slice();
        assert_eq!(bytes[0], ROUTE_PUSH_INLINE);
        assert_eq!(bytes[1], "/chat/abc".len() as u8); // varint length (short)
        assert_eq!(&bytes[2..], b"/chat/abc");
    }

    #[test]
    fn route_replace_inline_uses_its_opcode() {
        let mut buf = OpcodeBuffer::new();
        buf.route_replace_inline("/x");
        assert_eq!(buf.as_slice()[0], ROUTE_REPLACE_INLINE);
    }

    #[test]
    fn add_word_varint_encodes_lengths_at_and_over_128_bytes() {
        // Regression: a word of 128-255 bytes (e.g. a long unbroken URL token) must
        // encode its length as a varint to match the client decoder, which reads it
        // with `rv`. A plain `u8` length sets the high bit, so the decoder mis-reads it
        // as a multi-byte varint and desyncs the entire DOM stream — the whole page
        // renders blank. Words <=127 bytes are unaffected (varint == u8 there).
        let long = "x".repeat(200);
        let mut buf = OpcodeBuffer::new();
        buf.begin_word_table(1).add_word(&long);
        let bytes = buf.as_slice();
        assert_eq!(bytes[0], WORD_TABLE);
        assert_eq!(bytes[1], 1, "word count");
        let (len, consumed) = crate::protocol::read_varint(&bytes[2..]).unwrap();
        assert_eq!(len, 200, "length decodes as a varint, not a wrapped u8");
        assert_eq!(consumed, 2, "200 takes two varint bytes");
        assert_eq!(&bytes[2 + consumed..], long.as_bytes());
    }

    #[test]
    fn test_bind_target() {
        let mut buf = OpcodeBuffer::new();
        buf.bind_target(5, 0, 0xC0, false);
        let bytes = buf.as_slice();
        assert_eq!(bytes[0], BIND_TARGET);
        assert_eq!(bytes[1], 5); // ref (varint, single byte)
        assert_eq!(bytes[2], 0); // target_idx
                                 // St code 0xC0 as varint: 0x80 | ((0xC0-0x80)>>8) = 0x80, then low byte
                                 // Actually 0xC0 = 192. 192 >= 128 so varint is 2 bytes: 0x80 + ((192-128)>>8)=0x80, (192-128)&255=64
                                 // Wait: varint encoding: if v < 128 -> 1 byte. v=192 >= 128.
                                 // v-128 = 64. 0x80 | (64>>8) = 0x80. Second byte: 64&255 = 64.
                                 // So varint(192) = [0x80, 64]
        assert_eq!(bytes[3], 0x80); // st varint high byte
        assert_eq!(bytes[4], 64); // st varint low byte
        assert_eq!(bytes[5], 0); // invert = false
    }

    #[test]
    fn test_bind_toggle() {
        let mut buf = OpcodeBuffer::new();
        buf.bind_toggle(3, 1, 0); // ref=3, ev=click(1), target_idx=0
        let bytes = buf.as_slice();
        assert_eq!(bytes[0], BIND_TOGGLE);
        assert_eq!(bytes[1], 3); // ref
        assert_eq!(bytes[2], 1); // ev_type
        assert_eq!(bytes[3], 0); // target_idx
    }

    #[test]
    fn test_init_selector() {
        let mut buf = OpcodeBuffer::new();
        buf.init_selector(2, 1); // idx=2, default=1
        let bytes = buf.as_slice();
        assert_eq!(bytes[0], INIT_SELECTOR);
        assert_eq!(bytes[1], 2);
        assert_eq!(bytes[2], 1);
    }

    #[test]
    fn test_bind_selector() {
        let mut buf = OpcodeBuffer::new();
        buf.bind_selector(7, 1, 2, 50); // ref=7, idx=1, match=2, st=50
        let bytes = buf.as_slice();
        assert_eq!(bytes[0], BIND_SELECTOR);
        assert_eq!(bytes[1], 7); // ref
        assert_eq!(bytes[2], 1); // sel_idx
        assert_eq!(bytes[3], 2); // match_val
        assert_eq!(bytes[4], 50); // st varint (< 128 = 1 byte)
    }

    #[test]
    fn test_bind_select() {
        let mut buf = OpcodeBuffer::new();
        buf.bind_select(4, 1, 0, 2); // ref=4, ev=click(1), sel_idx=0, val=2
        let bytes = buf.as_slice();
        assert_eq!(bytes[0], BIND_SELECT);
        assert_eq!(bytes[1], 4); // ref
        assert_eq!(bytes[2], 1); // ev_type
        assert_eq!(bytes[3], 0); // sel_idx
        assert_eq!(bytes[4], 2); // val
    }

    #[test]
    fn test_bind_timed_toggle() {
        let mut buf = OpcodeBuffer::new();
        buf.bind_timed_toggle(3, 1, 0, 2000); // ref=3, ev=click(1), target_idx=0, delay=2000ms
        let bytes = buf.as_slice();
        assert_eq!(bytes[0], BIND_TIMED_TOGGLE);
        assert_eq!(bytes[1], 3); // ref
        assert_eq!(bytes[2], 1); // ev_type
        assert_eq!(bytes[3], 0); // target_idx
        assert_eq!(bytes[4], (2000 >> 8) as u8); // ms_hi = 7
        assert_eq!(bytes[5], (2000 & 0xFF) as u8); // ms_lo = 208
    }

    #[test]
    fn test_auto_toggle() {
        let mut buf = OpcodeBuffer::new();
        buf.auto_toggle(1, 5000); // target_idx=1, delay=5000ms
        let bytes = buf.as_slice();
        assert_eq!(bytes[0], AUTO_TOGGLE);
        assert_eq!(bytes[1], 1); // target_idx
        assert_eq!(bytes[2], (5000 >> 8) as u8); // ms_hi = 19
        assert_eq!(bytes[3], (5000 & 0xFF) as u8); // ms_lo = 136
    }
}
