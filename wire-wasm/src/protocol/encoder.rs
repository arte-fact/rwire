//! Binary encoder for DOM opcodes.

use bytes::{BufMut, BytesMut};

use super::opcodes::*;

/// Buffer for building opcode sequences.
pub struct OpcodeBuffer {
    buf: BytesMut,
    next_ref: u8,
    next_symbol: u8,
}

impl OpcodeBuffer {
    pub fn new() -> Self {
        Self {
            buf: BytesMut::with_capacity(256),
            next_ref: 0,
            next_symbol: SYMBOL_SESSION_START,
        }
    }

    /// Start a symbol table. Call `add_symbol` for each symbol, then continue with DOM ops.
    pub fn begin_symbols(&mut self, count: u8) -> &mut Self {
        self.buf.put_u8(SYMBOLS);
        self.buf.put_u8(count);
        self
    }

    /// Add a symbol to the symbol table. Returns the symbol index.
    pub fn add_symbol(&mut self, s: &str) -> u8 {
        let idx = self.next_symbol;
        self.buf.put_u8(s.len() as u8);
        self.buf.put_slice(s.as_bytes());
        self.next_symbol += 1;
        idx
    }

    /// Create an element. Returns the ref index.
    pub fn create(&mut self, element_type: u8) -> u8 {
        let ref_idx = self.next_ref;
        self.buf.put_u8(CREATE);
        self.buf.put_u8(element_type);
        self.next_ref += 1;
        ref_idx
    }

    /// Set class on an element.
    pub fn set_class(&mut self, ref_idx: u8, symbol_idx: u8) -> &mut Self {
        self.buf.put_u8(SET_CLASS);
        self.buf.put_u8(ref_idx);
        self.buf.put_u8(symbol_idx);
        self
    }

    /// Set text content on an element.
    pub fn set_text(&mut self, ref_idx: u8, symbol_idx: u8) -> &mut Self {
        self.buf.put_u8(SET_TEXT);
        self.buf.put_u8(ref_idx);
        self.buf.put_u8(symbol_idx);
        self
    }

    /// Set an attribute on an element.
    pub fn set_attr(&mut self, ref_idx: u8, attr_symbol: u8, value_symbol: u8) -> &mut Self {
        self.buf.put_u8(SET_ATTR);
        self.buf.put_u8(ref_idx);
        self.buf.put_u8(attr_symbol);
        self.buf.put_u8(value_symbol);
        self
    }

    /// Set a data attribute on an element.
    pub fn set_data(&mut self, ref_idx: u8, key_symbol: u8, value_symbol: u8) -> &mut Self {
        self.buf.put_u8(SET_DATA);
        self.buf.put_u8(ref_idx);
        self.buf.put_u8(key_symbol);
        self.buf.put_u8(value_symbol);
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

    /// Bind an optimistic handler (local + background sync).
    pub fn bind_optimistic(&mut self, ref_idx: u8, event_type: u8, handler_idx: u8) -> &mut Self {
        self.buf.put_u8(BIND_OPTIMISTIC);
        self.buf.put_u8(ref_idx);
        self.buf.put_u8(event_type);
        self.buf.put_u8(handler_idx);
        self
    }

    /// Get element by ID (for updates). Returns next ref index.
    pub fn get_by_id(&mut self, symbol_idx: u8) -> u8 {
        let ref_idx = self.next_ref;
        self.buf.put_u8(GET_BY_ID);
        self.buf.put_u8(symbol_idx);
        self.next_ref += 1;
        ref_idx
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
}

impl Default for OpcodeBuffer {
    fn default() -> Self {
        Self::new()
    }
}
