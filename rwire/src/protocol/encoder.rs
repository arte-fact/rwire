//! Binary encoder for DOM opcodes.

use bytes::{BufMut, BytesMut};

use super::opcodes::{
    APPEND, BATCH_END, BIND_LOCAL, BIND_REMOTE, BIND_REMOTE_PARAM, CLEAR_CHILDREN,
    COMPOSITE_TABLE, CREATE, CREATE_SYNCED, DEF_LOCAL_HANDLER, FORM_CLEAR_ERROR, FORM_SET_REQUIRED,
    FORM_SET_VALIDATION, FORM_SHOW_ERROR, GET_BY_ID, GET_SYNCED, INIT_LOCAL_STATE, ROUTE_PUSH,
    ROUTE_REPLACE, SET_ATTR, SET_ATTR_BOOL, SET_ATTR_ENUM, SET_ATTR_KEY_SYM, SET_CLASS, SET_DATA,
    SET_TEXT, SET_TEXT_INT, SET_TEXT_WORDS, STYLE_COMPOSITE, STYLE_MULTI, STYLE_PROP,
    STYLE_PSEUDO, STYLE_SET, STYLE_UTIL, SYMBOLS, SYMBOLS_EXTEND, SYMBOL_SESSION_START,
    WORD_TABLE,
};
use super::varint::write_varint;

/// Buffer for building opcode sequences.
///
/// Supports both legacy u8 refs (for backward compatibility) and
/// extended u32 refs using varint encoding (for >255 elements).
///
/// Symbol indices also use varint encoding, allowing unlimited symbols
/// (practical limit ~16K with 2-byte varints).
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

    /// Create a new buffer with extended ref support (varint encoding).
    ///
    /// Use this when you expect more than 127 elements.
    pub fn new_extended() -> Self {
        Self {
            buf: BytesMut::with_capacity(256),
            next_ref: 0,
            next_symbol: SYMBOL_SESSION_START as u32,
        }
    }

    /// Check if we've exceeded the u8 ref limit.
    pub fn needs_extended_refs(&self) -> bool {
        self.next_ref > 127
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
        assert!(
            s.len() <= 255,
            "add_symbol: symbol too long ({} bytes, max 255)",
            s.len()
        );
        let idx = self.next_symbol;
        self.buf.put_u8(s.len() as u8);
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
    ///
    /// Note: Returns u8 for backward compatibility. Use `create_ext` for >255 elements.
    pub fn create(&mut self, element_type: u8) -> u8 {
        self.create_ext(element_type) as u8
    }

    /// Create an element with extended ref support. Returns the ref index as u32.
    pub fn create_ext(&mut self, element_type: u8) -> u32 {
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
    ///
    /// Note: Returns u8 for backward compatibility. Use `create_synced_ext` for >255 elements.
    pub fn create_synced(&mut self, synced_id: u32) -> u8 {
        self.create_synced_ext(synced_id) as u8
    }

    /// Create a synced wrapper span with extended ref support.
    pub fn create_synced_ext(&mut self, synced_id: u32) -> u32 {
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
    ///
    /// Note: Returns u8 for backward compatibility. Use `get_synced_ext` for >255 elements.
    pub fn get_synced(&mut self, synced_id: u32) -> u8 {
        self.get_synced_ext(synced_id) as u8
    }

    /// Get a synced element by numeric ID with extended ref support.
    pub fn get_synced_ext(&mut self, synced_id: u32) -> u32 {
        let ref_idx = self.next_ref;
        self.buf.put_u8(GET_SYNCED);
        write_varint(&mut self.buf, synced_id);
        self.next_ref += 1;
        ref_idx
    }

    /// Set class on an element.
    ///
    /// Symbol index uses varint encoding.
    pub fn set_class(&mut self, ref_idx: u8, symbol_idx: u32) -> &mut Self {
        self.buf.put_u8(SET_CLASS);
        self.buf.put_u8(ref_idx);
        write_varint(&mut self.buf, symbol_idx);
        self
    }

    /// Set text content on an element.
    ///
    /// Symbol index uses varint encoding.
    pub fn set_text(&mut self, ref_idx: u8, symbol_idx: u32) -> &mut Self {
        self.buf.put_u8(SET_TEXT);
        self.buf.put_u8(ref_idx);
        write_varint(&mut self.buf, symbol_idx);
        self
    }

    /// Set text content from word indices (words joined with spaces).
    ///
    /// More compact than symbol table when words are reused across strings.
    /// Format: [SET_TEXT_WORDS, ref, count, idx0, idx1, ...]
    pub fn set_text_words(&mut self, ref_idx: u8, word_indices: &[u8]) -> &mut Self {
        assert!(
            word_indices.len() <= 255,
            "set_text_words: too many words ({}, max 255)",
            word_indices.len()
        );
        self.buf.put_u8(SET_TEXT_WORDS);
        self.buf.put_u8(ref_idx);
        self.buf.put_u8(word_indices.len() as u8);
        for &idx in word_indices {
            self.buf.put_u8(idx);
        }
        self
    }

    /// Set text content to a number (varint encoded).
    ///
    /// More compact than symbol table for dynamic numeric values.
    /// Format: [SET_TEXT_INT, ref, varint]
    pub fn set_text_int(&mut self, ref_idx: u8, value: i32) -> &mut Self {
        self.buf.put_u8(SET_TEXT_INT);
        self.buf.put_u8(ref_idx);
        // Encode as zigzag varint for signed integers
        let zigzag = ((value << 1) ^ (value >> 31)) as u32;
        write_varint(&mut self.buf, zigzag);
        self
    }

    /// Set an attribute on an element.
    ///
    /// Symbol indices use varint encoding.
    pub fn set_attr(&mut self, ref_idx: u8, attr_symbol: u32, value_symbol: u32) -> &mut Self {
        self.buf.put_u8(SET_ATTR);
        self.buf.put_u8(ref_idx);
        write_varint(&mut self.buf, attr_symbol);
        write_varint(&mut self.buf, value_symbol);
        self
    }

    /// Set a data attribute on an element.
    ///
    /// Symbol indices use varint encoding.
    pub fn set_data(&mut self, ref_idx: u8, key_symbol: u32, value_symbol: u32) -> &mut Self {
        self.buf.put_u8(SET_DATA);
        self.buf.put_u8(ref_idx);
        write_varint(&mut self.buf, key_symbol);
        write_varint(&mut self.buf, value_symbol);
        self
    }

    /// Append a child to a parent element.
    pub fn append(&mut self, parent_ref: u8, child_ref: u8) -> &mut Self {
        self.buf.put_u8(APPEND);
        self.buf.put_u8(parent_ref);
        self.buf.put_u8(child_ref);
        self
    }

    /// Append a child to the document body (parent = 0xFF).
    pub fn append_to_body(&mut self, child_ref: u8) -> &mut Self {
        self.buf.put_u8(APPEND);
        self.buf.put_u8(0xFF); // Special value for document.body
        self.buf.put_u8(child_ref);
        self
    }

    /// Clear all children from an element.
    pub fn clear_children(&mut self, ref_idx: u8) -> &mut Self {
        self.buf.put_u8(CLEAR_CHILDREN);
        self.buf.put_u8(ref_idx);
        self
    }

    /// Bind a local (WASM-only) event handler.
    pub fn bind_local(&mut self, ref_idx: u8, event_type: u8, handler_idx: u8) -> &mut Self {
        self.buf.put_u8(BIND_LOCAL);
        self.buf.put_u8(ref_idx);
        self.buf.put_u8(event_type);
        self.buf.put_u8(handler_idx);
        self
    }

    /// Bind a remote event handler (requires server round-trip).
    pub fn bind_remote(&mut self, ref_idx: u8, event_type: u8, handler_idx: u8) -> &mut Self {
        self.buf.put_u8(BIND_REMOTE);
        self.buf.put_u8(ref_idx);
        self.buf.put_u8(event_type);
        self.buf.put_u8(handler_idx);
        self
    }

    /// Bind a remote event handler with parameter bytes.
    ///
    /// The param_bytes are stored on the element and sent back with the event,
    /// enabling item-specific handlers for dynamically generated content.
    ///
    /// Format: [BIND_REMOTE_PARAM, ref, event_type, handler_idx, param_len, ...param_bytes]
    pub fn bind_remote_param(
        &mut self,
        ref_idx: u8,
        event_type: u8,
        handler_idx: u8,
        param_bytes: &[u8],
    ) -> &mut Self {
        assert!(
            param_bytes.len() <= 255,
            "bind_remote_param: param_bytes too large ({} bytes, max 255)",
            param_bytes.len()
        );
        self.buf.put_u8(BIND_REMOTE_PARAM);
        self.buf.put_u8(ref_idx);
        self.buf.put_u8(event_type);
        self.buf.put_u8(handler_idx);
        self.buf.put_u8(param_bytes.len() as u8);
        self.buf.put_slice(param_bytes);
        self
    }

    /// Get element by ID (for updates). Returns next ref index.
    ///
    /// Symbol index uses varint encoding.
    /// Note: Returns u8 for backward compatibility. Use `get_by_id_ext` for >255 elements.
    pub fn get_by_id(&mut self, symbol_idx: u32) -> u8 {
        self.get_by_id_ext(symbol_idx) as u8
    }

    /// Get element by ID with extended ref support. Returns the ref index as u32.
    ///
    /// Symbol index uses varint encoding.
    pub fn get_by_id_ext(&mut self, symbol_idx: u32) -> u32 {
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
    /// Symbol index uses varint encoding.
    pub fn form_set_validation(&mut self, ref_idx: u8, rules_symbol: u32) -> &mut Self {
        self.buf.put_u8(FORM_SET_VALIDATION);
        self.buf.put_u8(ref_idx);
        write_varint(&mut self.buf, rules_symbol);
        self
    }

    /// Show validation error on a field.
    ///
    /// Symbol index uses varint encoding.
    pub fn form_show_error(&mut self, ref_idx: u8, message_symbol: u32) -> &mut Self {
        self.buf.put_u8(FORM_SHOW_ERROR);
        self.buf.put_u8(ref_idx);
        write_varint(&mut self.buf, message_symbol);
        self
    }

    /// Clear validation error on a field.
    pub fn form_clear_error(&mut self, ref_idx: u8) -> &mut Self {
        self.buf.put_u8(FORM_CLEAR_ERROR);
        self.buf.put_u8(ref_idx);
        self
    }

    /// Set field as required.
    pub fn form_set_required(&mut self, ref_idx: u8, required: bool) -> &mut Self {
        self.buf.put_u8(FORM_SET_REQUIRED);
        self.buf.put_u8(ref_idx);
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
    /// Symbol index uses varint encoding.
    pub fn style_set(&mut self, ref_idx: u8, style_symbol: u32) -> &mut Self {
        self.buf.put_u8(STYLE_SET);
        self.buf.put_u8(ref_idx);
        write_varint(&mut self.buf, style_symbol);
        self
    }

    /// Set a style utility token on an element.
    ///
    /// Format: [STYLE_UTIL, ref, util_varint]
    /// 3-4 bytes total depending on token value (varint for extended tokens).
    pub fn style_util(&mut self, ref_idx: u8, util: u16) -> &mut Self {
        self.buf.put_u8(STYLE_UTIL);
        self.buf.put_u8(ref_idx);
        write_varint(&mut self.buf, util as u32);
        self
    }

    /// Set a style property+value on an element.
    ///
    /// Format: [STYLE_PROP, ref, prop_byte, value_byte]
    /// 4 bytes total - flexible for any property+value combination.
    pub fn style_prop(&mut self, ref_idx: u8, prop: u8, value: u8) -> &mut Self {
        self.buf.put_u8(STYLE_PROP);
        self.buf.put_u8(ref_idx);
        self.buf.put_u8(prop);
        self.buf.put_u8(value);
        self
    }

    /// Set multiple style utility tokens on an element.
    ///
    /// Format: [STYLE_MULTI, ref, count, util1_varint, util2_varint, ...]
    /// More efficient than multiple STYLE_UTIL calls for >2 utilities.
    pub fn style_multi(&mut self, ref_idx: u8, utils: &[u16]) -> &mut Self {
        assert!(
            utils.len() <= 255,
            "style_multi: too many tokens ({}, max 255)",
            utils.len()
        );
        self.buf.put_u8(STYLE_MULTI);
        self.buf.put_u8(ref_idx);
        self.buf.put_u8(utils.len() as u8);
        for &util in utils {
            write_varint(&mut self.buf, util as u32);
        }
        self
    }

    /// Apply a style composite by ID.
    ///
    /// Format: [STYLE_COMPOSITE, ref, composite_id_varint]
    pub fn style_composite(&mut self, ref_idx: u8, composite_id: u32) -> &mut Self {
        self.buf.put_u8(STYLE_COMPOSITE);
        self.buf.put_u8(ref_idx);
        write_varint(&mut self.buf, composite_id);
        self
    }

    /// Apply composable pseudo-class styles to an element.
    ///
    /// Format: [STYLE_PSEUDO, ref, pc_code, count, st1_varint, st2_varint, ...]
    pub fn style_pseudo(&mut self, ref_idx: u8, pc: u8, st_tokens: &[u16]) -> &mut Self {
        assert!(
            st_tokens.len() <= 255,
            "style_pseudo: too many tokens ({}, max 255)",
            st_tokens.len()
        );
        self.buf.put_u8(STYLE_PSEUDO);
        self.buf.put_u8(ref_idx);
        self.buf.put_u8(pc);
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
    /// Format: [SET_ATTR_ENUM, ref, at, av] = 4 bytes total.
    /// Both key and value are single-byte enum codes from At/Av enums.
    pub fn set_attr_enum(&mut self, ref_idx: u8, at: u8, av: u8) -> &mut Self {
        self.buf.put_u8(SET_ATTR_ENUM);
        self.buf.put_u8(ref_idx);
        self.buf.put_u8(at);
        self.buf.put_u8(av);
        self
    }

    /// Set boolean attribute (presence-only, e.g. disabled, required).
    ///
    /// Format: [SET_ATTR_BOOL, ref, at] = 3 bytes total.
    pub fn set_attr_bool(&mut self, ref_idx: u8, at: u8) -> &mut Self {
        self.buf.put_u8(SET_ATTR_BOOL);
        self.buf.put_u8(ref_idx);
        self.buf.put_u8(at);
        self
    }

    /// Set attribute with enum key + symbol table value.
    ///
    /// Format: [SET_ATTR_KEY_SYM, ref, at, val_sym_varint] = 4-5 bytes total.
    /// Key is an enum code from At, value is a symbol table index.
    pub fn set_attr_key_sym(&mut self, ref_idx: u8, at: u8, value_symbol: u32) -> &mut Self {
        self.buf.put_u8(SET_ATTR_KEY_SYM);
        self.buf.put_u8(ref_idx);
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
