//! Binary encoder for DOM opcodes.
//!
//! All element refs and handler indices use varint encoding, supporting
//! unlimited elements per message (practical limit ~16K with 2-byte varints).

use bytes::{BufMut, BytesMut};

use super::opcodes::{
    APPEND, BATCH_END, BIND_LOCAL, BIND_REMOTE, BIND_REMOTE_PARAM, CLEAR_CHILDREN, COMPOSITE_TABLE,
    CREATE, CREATE_SYNCED, DEF_LOCAL_HANDLER, FORM_CLEAR_ERROR, FORM_SET_REQUIRED,
    FORM_SET_VALIDATION, FORM_SHOW_ERROR, GET_BY_ID, GET_SYNCED, INIT_LOCAL_STATE, ROUTE_PUSH,
    ROUTE_REPLACE, SET_ATTR, SET_ATTR_BOOL, SET_ATTR_ENUM, SET_ATTR_KEY_SYM, SET_CLASS, SET_DATA,
    SET_TEXT, SET_TEXT_INT, SET_TEXT_WORDS, STYLE_BREAKPOINT, STYLE_COMPOSITE, STYLE_MULTI,
    STYLE_PROP, STYLE_PSEUDO, STYLE_SET, STYLE_UTIL, SYMBOLS, SYMBOLS_EXTEND,
    SYMBOL_SESSION_START, WORD_TABLE,
};
use super::varint::write_varint;

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
}

impl OpcodeBuffer {
    pub fn new() -> Self {
        Self {
            buf: BytesMut::with_capacity(256),
            next_ref: 0,
            next_symbol: SYMBOL_SESSION_START as u32,
        }
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
    pub fn add_word(&mut self, word: &str) -> &mut Self {
        self.buf.put_u8(word.len() as u8);
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

    /// Initialize local state on the client.
    ///
    /// Format: [INIT_LOCAL_STATE, state_idx, len_hi, len_lo, json_bytes...]
    pub fn init_local_state(&mut self, state_idx: u8, json: &str) -> &mut Self {
        let bytes = json.as_bytes();
        self.buf.put_u8(INIT_LOCAL_STATE);
        self.buf.put_u8(state_idx);
        // Length as 2 bytes (up to 65535 bytes of JSON)
        self.buf.put_u8((bytes.len() >> 8) as u8);
        self.buf.put_u8(bytes.len() as u8);
        self.buf.put_slice(bytes);
        self
    }

    /// Define a local handler with mutations.
    ///
    /// Format: [DEF_LOCAL_HANDLER, handler_idx, state_idx, mut_count, ...mutation_bytes]
    pub fn def_local_handler(
        &mut self,
        handler_idx: u8,
        state_idx: u8,
        mutations: &[u8],
        mutation_count: u8,
    ) -> &mut Self {
        self.buf.put_u8(DEF_LOCAL_HANDLER);
        self.buf.put_u8(handler_idx);
        self.buf.put_u8(state_idx);
        self.buf.put_u8(mutation_count);
        self.buf.put_slice(mutations);
        self
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
