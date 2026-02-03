//! Fluent builder API for constructing DOM elements with reactive synced regions.
//!
//! This module provides a high-level, ergonomic API for building DOM structures
//! that get compiled down to the binary opcode protocol.
//!
//! # Example
//!
//! ```ignore
//! use rwire::{el, El, Ev, ClientState, handler, renderer};
//!
//! #[derive(ClientState, Default)]
//! struct Counter {
//!     count: i32,
//! }
//!
//! fn build_counter() -> ElementBuilder {
//!     el(El::Div).class("counter").append([
//!         el(El::Button).text("-").on(Ev::Click, decrement),
//!         render_count(),
//!         el(El::Button).text("+").on(Ev::Click, increment),
//!     ])
//! }
//!
//! #[renderer]
//! fn render_count(state: &Counter) -> ElementBuilder {
//!     el(El::Span).text(&state.count.to_string())
//! }
//!
//! #[handler]
//! fn increment(state: &mut Counter) {
//!     state.count += 1;
//! }
//!
//! #[handler]
//! fn decrement(state: &mut Counter) {
//!     state.count -= 1;
//! }
//! ```

use bytes::Bytes;
use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};

use crate::item_ref::ItemRef;
use crate::protocol::{El, Ev, OpcodeBuffer};
use crate::state::{
    ChangeSet, HandlerFn, HandlerSpec, LocalMutations, Renderer, RendererDeps, StorageType,
};

/// Create a new element builder.
///
/// # Example
///
/// ```ignore
/// use rwire::{el, El};
///
/// let button = el(El::Button).text("Click me").class("primary");
/// ```
pub fn el(el_type: El) -> ElementBuilder {
    ElementBuilder::new(el_type)
}

/// Trait for type-erased synced renderers.
///
/// This trait allows renderers to be stored and invoked without knowing
/// the concrete state type at compile time.
pub trait SyncedRenderer: Send + Sync {
    /// Render with the given state, returning a new ElementBuilder.
    fn render_with_state(&self, state: &dyn Any) -> Option<ElementBuilder>;
    /// Clone this renderer into a boxed trait object.
    fn clone_box(&self) -> Box<dyn SyncedRenderer>;
    /// Get the TypeId of the state type this renderer expects.
    fn state_type_id(&self) -> TypeId;
    /// Create a default state instance for this renderer's state type.
    fn create_default_state(&self) -> Box<dyn Any + Send + Sync>;
    /// Get the dependency information for this renderer.
    fn deps(&self) -> RendererDeps;
}

/// Implementation of SyncedRenderer for a specific state type.
struct SyncedRendererImpl<S: Default + Send + Sync + 'static> {
    render: Renderer<S>,
    deps: RendererDeps,
}

impl<S: Default + Send + Sync + 'static> SyncedRenderer for SyncedRendererImpl<S> {
    fn render_with_state(&self, state: &dyn Any) -> Option<ElementBuilder> {
        state.downcast_ref::<S>().map(|s| (self.render)(s))
    }

    fn clone_box(&self) -> Box<dyn SyncedRenderer> {
        Box::new(SyncedRendererImpl {
            render: self.render,
            deps: self.deps,
        })
    }

    fn state_type_id(&self) -> TypeId {
        TypeId::of::<S>()
    }

    fn create_default_state(&self) -> Box<dyn Any + Send + Sync> {
        Box::new(S::default())
    }

    fn deps(&self) -> RendererDeps {
        self.deps
    }
}

impl Clone for Box<dyn SyncedRenderer> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Builder for constructing DOM elements with a fluent API.
#[derive(Clone)]
pub struct ElementBuilder {
    el_type: El,
    text: Option<String>,
    class: Option<String>,
    attrs: Vec<(String, String)>,
    events: Vec<(Ev, HandlerSpec)>,
    children: Vec<ElementBuilder>,
    synced: Option<Box<dyn SyncedRenderer>>,
}

impl ElementBuilder {
    /// Create a new element builder with the given element type.
    pub fn new(el_type: El) -> Self {
        Self {
            el_type,
            text: None,
            class: None,
            attrs: Vec::new(),
            events: Vec::new(),
            children: Vec::new(),
            synced: None,
        }
    }

    /// Create a synced element that will re-render when state changes.
    ///
    /// This is the legacy method that always re-renders on any state change.
    /// Prefer using `synced_with_deps` for fine-grained reactivity.
    pub fn synced<S: Default + Send + Sync + 'static>(render: Renderer<S>) -> Self {
        Self::synced_with_deps::<S>(render, RendererDeps::always())
    }

    /// Create a synced element with explicit dependency tracking.
    ///
    /// This is called by the `#[renderer]` macro with auto-detected or
    /// explicitly specified dependencies.
    ///
    /// # Arguments
    ///
    /// * `render` - The render function that takes state and returns an ElementBuilder
    /// * `deps` - Dependency information specifying which fields trigger re-renders
    pub fn synced_with_deps<S: Default + Send + Sync + 'static>(
        render: Renderer<S>,
        deps: RendererDeps,
    ) -> Self {
        Self {
            el_type: El::Div, // Placeholder, will be replaced by rendered content
            text: None,
            class: None,
            attrs: Vec::new(),
            events: Vec::new(),
            children: Vec::new(),
            synced: Some(Box::new(SyncedRendererImpl { render, deps })),
        }
    }

    /// Set the text content of this element.
    pub fn text(mut self, s: &str) -> Self {
        self.text = Some(s.to_string());
        self
    }

    /// Set the CSS class of this element.
    pub fn class(mut self, s: &str) -> Self {
        self.class = Some(s.to_string());
        self
    }

    /// Set an attribute on this element.
    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.attrs.push((key.to_string(), value.to_string()));
        self
    }

    /// Set a data attribute on this element (e.g., `data-id="5"`).
    ///
    /// This is a convenience method that prefixes the key with "data-".
    /// The value can be retrieved in handlers via `ctx.data("key")`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// el(El::Button)
    ///     .text("Delete")
    ///     .data("id", &item.id.to_string())
    ///     .on(Ev::Click, delete_item())
    /// ```
    pub fn data(self, key: &str, value: &str) -> Self {
        self.attr(&format!("data-{}", key), value)
    }

    /// Set inline style on this element.
    pub fn style(self, style: crate::style::Style) -> Self {
        self.attr("style", &style.to_css())
    }

    /// Bind an event handler to this element.
    ///
    /// The handler function will be called when the event occurs.
    /// For local state handlers, the event is handled entirely on the client.
    /// For memory/persisted state handlers, the event triggers a server round-trip.
    pub fn on(mut self, ev: Ev, handler: HandlerSpec) -> Self {
        self.events.push((ev, handler));
        self
    }

    /// Bind an event handler with an item reference parameter.
    ///
    /// This is used for dynamically-generated content where each item needs
    /// its own event handler. The `ItemRef<T>` is encoded and sent back with
    /// the event, enabling type-safe item lookup in the handler.
    ///
    /// # Example
    ///
    /// ```ignore
    /// state.items.iter_with_ref().map(|(item_ref, item)| {
    ///     el(El::Li)
    ///         .text(&item.text)
    ///         .on_ref(Ev::Click, toggle_item(), item_ref)
    /// })
    /// ```
    pub fn on_ref<T: 'static>(
        mut self,
        ev: Ev,
        handler: HandlerSpec,
        item_ref: ItemRef<T>,
    ) -> Self {
        // Encode the item reference to bytes
        let mut param_bytes = Vec::new();
        item_ref.encode(&mut param_bytes);

        // Clone the handler and attach the param bytes
        let handler_with_params = handler.with_param_bytes(param_bytes);
        self.events.push((ev, handler_with_params));
        self
    }

    /// Append child elements to this element.
    pub fn append<I>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = ElementBuilder>,
    {
        self.children.extend(children);
        self
    }

    /// Check if this element is a synced element.
    pub fn is_synced(&self) -> bool {
        self.synced.is_some()
    }

    /// Get the element type.
    pub fn el_type(&self) -> El {
        self.el_type
    }

    /// Get the text content.
    pub fn text_content(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// Get the class.
    pub fn class_name(&self) -> Option<&str> {
        self.class.as_deref()
    }

    /// Get the attributes.
    pub fn attributes(&self) -> &[(String, String)] {
        &self.attrs
    }

    /// Get the children.
    pub fn children(&self) -> &[ElementBuilder] {
        &self.children
    }

    /// Get the events.
    pub fn events(&self) -> &[(Ev, HandlerSpec)] {
        &self.events
    }
}

/// Represents how a text string should be encoded.
#[derive(Clone, Debug)]
pub enum TextEncoding {
    /// Use symbol table (traditional approach)
    Symbol(u8),
    /// Use word indices (space-separated words)
    Words(Vec<u8>),
    /// Use integer encoding (for pure numeric strings)
    Int(i32),
}

/// Context for building the DOM tree with state support.
pub struct BuildContext {
    buf: OpcodeBuffer,
    symbols: Vec<String>,
    symbol_map: HashMap<String, u8>,
    /// Remote handlers (Memory/Persisted state)
    handlers: Vec<HandlerFn>,
    /// Local handlers with their mutations
    local_handlers: Vec<LocalMutations>,
    synced_elements: Vec<SyncedElement>,
    next_synced_id: u32,
    used_elements: HashSet<u8>,
    used_events: HashSet<u8>,
    /// Whether any local handlers are used (for tree shaking capsule)
    has_local_handlers: bool,
    /// Mapping from local state TypeId to state index
    local_state_indices: HashMap<TypeId, u8>,
    /// Next available local state index
    next_local_state_idx: u8,
    /// Word frequency counts (built during collect_symbols)
    word_counts: HashMap<String, usize>,
    /// Word table: word -> index (built after collect_symbols, before emit)
    word_indices: HashMap<String, u8>,
    /// Ordered word list (most frequent first)
    words: Vec<String>,
    /// Text encoding decisions (text -> encoding)
    text_encodings: HashMap<String, TextEncoding>,
}

/// Information about a synced element for later updates.
pub struct SyncedElement {
    /// Unique ID for this synced element (used in __synced_N wrapper IDs).
    pub id: u32,
    pub(crate) renderer: Box<dyn SyncedRenderer>,
    pub(crate) state_type_id: TypeId,
    /// Dependency information for fine-grained updates.
    pub deps: RendererDeps,
}

impl Clone for SyncedElement {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            renderer: self.renderer.clone_box(),
            state_type_id: self.state_type_id,
            deps: self.deps,
        }
    }
}

impl SyncedElement {
    /// Create a new SyncedElement with explicit dependencies.
    ///
    /// This is primarily used for testing the fine-grained reactivity system.
    pub fn new_with_deps(
        id: u32,
        renderer: Box<dyn SyncedRenderer>,
        state_type_id: TypeId,
        deps: RendererDeps,
    ) -> Self {
        Self {
            id,
            renderer,
            state_type_id,
            deps,
        }
    }

    /// Get the TypeId of the state type this element renders from.
    pub fn state_type_id(&self) -> TypeId {
        self.state_type_id
    }

    /// Create a default state instance for this element's state type.
    pub fn create_default_state(&self) -> Box<dyn Any + Send + Sync> {
        self.renderer.create_default_state()
    }
}

impl BuildContext {
    pub fn new() -> Self {
        Self {
            buf: OpcodeBuffer::new(),
            symbols: Vec::new(),
            symbol_map: HashMap::new(),
            handlers: Vec::new(),
            local_handlers: Vec::new(),
            synced_elements: Vec::new(),
            next_synced_id: 0,
            used_elements: HashSet::new(),
            used_events: HashSet::new(),
            has_local_handlers: false,
            local_state_indices: HashMap::new(),
            next_local_state_idx: 0,
            word_counts: HashMap::new(),
            word_indices: HashMap::new(),
            words: Vec::new(),
            text_encodings: HashMap::new(),
        }
    }

    /// Analyze a text string and count word frequencies.
    fn analyze_text(&mut self, text: &str) {
        // Check if it's a pure integer
        if text.parse::<i32>().is_ok() {
            return; // Will use SET_TEXT_INT
        }

        // Tokenize into words and count
        for word in text.split_whitespace() {
            *self.word_counts.entry(word.to_string()).or_insert(0) += 1;
        }
    }

    /// Build the word table after collecting all text.
    /// Call this after collect_symbols and before emit.
    pub fn build_word_table(&mut self) {
        // Sort words by frequency (most common first)
        let mut word_freq: Vec<_> = self.word_counts.drain().collect();
        word_freq.sort_by(|a, b| b.1.cmp(&a.1));

        // Only include words that appear more than once or are short common words
        // Limit to 255 words (u8 index)
        for (word, count) in word_freq.into_iter().take(255) {
            // Include if: appears multiple times, OR is a common short word
            if count > 1 || (word.len() <= 4 && count > 0) {
                let idx = self.words.len() as u8;
                self.word_indices.insert(word.clone(), idx);
                self.words.push(word);
            }
        }

        // Now decide encoding for each text string
        self.decide_text_encodings();
    }

    /// Decide optimal encoding for each text string.
    fn decide_text_encodings(&mut self) {
        // Collect all text strings from symbols that look like text content
        let texts: Vec<String> = self.symbols.clone();

        for text in texts {
            let encoding = self.choose_encoding(&text);
            self.text_encodings.insert(text, encoding);
        }
    }

    /// Choose the best encoding for a text string.
    fn choose_encoding(&self, text: &str) -> TextEncoding {
        // Try integer encoding first
        if let Ok(n) = text.parse::<i32>() {
            // Integer encoding: 3 bytes base + varint (1-5 bytes)
            // Symbol encoding: 3 bytes + symbol table entry
            // Integer is better for most numbers
            return TextEncoding::Int(n);
        }

        // Try word encoding
        let words: Vec<&str> = text.split_whitespace().collect();
        if !words.is_empty() {
            let word_indices: Vec<u8> = words
                .iter()
                .filter_map(|w| self.word_indices.get(*w).copied())
                .collect();

            // If all words are in the table, consider word encoding
            if word_indices.len() == words.len() {
                // Word encoding cost: 3 + word_count bytes
                let word_cost = 3 + word_indices.len();
                // Symbol encoding cost: 3 bytes (but symbol adds to table)
                // For reused symbols, symbol is better
                // For unique text with common words, words may be better

                // Use word encoding if text is longer than encoding
                if text.len() > word_cost + 2 {
                    return TextEncoding::Words(word_indices);
                }
            }
        }

        // Fall back to symbol encoding
        if let Some(&idx) = self.symbol_map.get(text) {
            TextEncoding::Symbol(idx)
        } else {
            // Will be interned during emit
            TextEncoding::Symbol(0) // Placeholder
        }
    }

    /// Get the encoding for a text string.
    pub fn get_text_encoding(&self, text: &str) -> Option<&TextEncoding> {
        self.text_encodings.get(text)
    }

    /// Get the word table for emission.
    pub fn word_table(&self) -> &[String] {
        &self.words
    }

    /// Get or allocate a state index for a local state type.
    fn get_or_create_local_state_idx(&mut self, state_type_id: TypeId) -> u8 {
        if let Some(&idx) = self.local_state_indices.get(&state_type_id) {
            return idx;
        }
        let idx = self.next_local_state_idx;
        self.next_local_state_idx += 1;
        self.local_state_indices.insert(state_type_id, idx);
        idx
    }

    fn intern(&mut self, s: &str) -> u8 {
        if let Some(&idx) = self.symbol_map.get(s) {
            return idx;
        }
        let idx = 0x80 + self.symbols.len() as u8;
        self.symbols.push(s.to_string());
        self.symbol_map.insert(s.to_string(), idx);
        idx
    }

    fn register_remote_handler(&mut self, handler: HandlerFn) -> u8 {
        // Check if handler is already registered (by comparing function pointers)
        for (i, h) in self.handlers.iter().enumerate() {
            // Compare the underlying function pointers
            if std::ptr::eq(
                h as *const HandlerFn as *const (),
                &handler as *const HandlerFn as *const (),
            ) {
                return i as u8;
            }
        }
        let idx = self.handlers.len() as u8;
        self.handlers.push(handler);
        idx
    }

    fn register_local_handler(
        &mut self,
        mut mutations: LocalMutations,
        state_type_id: Option<TypeId>,
    ) -> u8 {
        self.has_local_handlers = true;
        // Assign state index if state type is known
        if let Some(type_id) = state_type_id {
            mutations.state_idx = self.get_or_create_local_state_idx(type_id);
        }
        let idx = self.local_handlers.len() as u8;
        self.local_handlers.push(mutations);
        idx
    }

    /// Collect all symbols from an element tree (first pass).
    /// Also tracks used element and event types for tree shaking.
    ///
    /// Note: Synced element IDs are no longer interned as symbols - they use
    /// dedicated CREATE_SYNCED/GET_SYNCED opcodes with varint encoding.
    pub fn collect_symbols(&mut self, el: &ElementBuilder, state: &dyn Any) {
        // If this is a synced element, render it first with state
        if let Some(renderer) = &el.synced {
            // Track the span wrapper element type (still used by CREATE_SYNCED)
            self.used_elements.insert(El::Span.as_u8());
            if let Some(rendered) = renderer.render_with_state(state) {
                self.collect_symbols(&rendered, state);
            }
            return;
        }

        // Track element type usage
        self.used_elements.insert(el.el_type.as_u8());

        if let Some(ref text) = el.text {
            self.analyze_text(text);
            self.intern(text);
        }
        if let Some(ref class) = el.class {
            self.intern(class);
        }
        for (key, value) in &el.attrs {
            self.intern(key);
            self.intern(value);
        }
        // Track event type usage
        for (ev, _) in &el.events {
            self.used_events.insert(ev.as_u8());
        }
        // Process synced children - just track the ID counter, no symbol interning needed
        for child in &el.children {
            if child.synced.is_some() {
                // Track synced ID but don't intern - using CREATE_SYNCED opcode instead
                self.next_synced_id += 1;
            }
            self.collect_symbols(child, state);
        }
    }

    /// Collect all symbols from an element tree with multi-state support.
    /// Each synced element is rendered with its corresponding state type.
    ///
    /// Note: Synced element IDs are no longer interned as symbols - they use
    /// dedicated CREATE_SYNCED/GET_SYNCED opcodes with varint encoding.
    pub fn collect_symbols_multi(
        &mut self,
        el: &ElementBuilder,
        states: &HashMap<TypeId, &(dyn Any + Send + Sync)>,
    ) {
        // If this is a synced element, render it first with the appropriate state
        if let Some(renderer) = &el.synced {
            // Track the span wrapper element type (still used by CREATE_SYNCED)
            self.used_elements.insert(El::Span.as_u8());
            // Track synced ID but don't intern - using CREATE_SYNCED opcode instead
            self.next_synced_id += 1;

            // Find the state for this renderer's state type
            let state_type_id = renderer.state_type_id();
            if let Some(state) = states.get(&state_type_id) {
                if let Some(rendered) = renderer.render_with_state(*state) {
                    self.collect_symbols_multi(&rendered, states);
                }
            }
            return;
        }

        // Track element type usage
        self.used_elements.insert(el.el_type.as_u8());

        if let Some(ref text) = el.text {
            self.analyze_text(text);
            self.intern(text);
        }
        if let Some(ref class) = el.class {
            self.intern(class);
        }
        for (key, value) in &el.attrs {
            self.intern(key);
            self.intern(value);
        }
        // Track event type usage
        for (ev, _) in &el.events {
            self.used_events.insert(ev.as_u8());
        }
        // Process children
        for child in &el.children {
            self.collect_symbols_multi(child, states);
        }
    }

    /// Emit opcodes for an element tree (second pass).
    pub fn emit(&mut self, el: &ElementBuilder, state: &dyn Any) -> u8 {
        // Reset synced_id counter - we increment again during emit
        self.next_synced_id = 0;

        // Build word table and decide text encodings
        self.build_word_table();

        // Emit symbol table first (only on first call)
        if !self.symbols.is_empty() {
            self.buf.begin_symbols(self.symbols.len() as u8);
            for sym in self.symbols.drain(..) {
                self.buf.add_symbol(&sym);
            }
        }

        // Emit word table if we have words
        if !self.words.is_empty() {
            self.buf.begin_word_table(self.words.len() as u8);
            for word in &self.words {
                self.buf.add_word(word);
            }
        }

        self.emit_element(el, None, state)
    }

    /// Emit opcodes for an element tree with multi-state support.
    pub fn emit_multi(
        &mut self,
        el: &ElementBuilder,
        states: &HashMap<TypeId, &(dyn Any + Send + Sync)>,
    ) -> u8 {
        // Reset synced_id counter - we increment again during emit
        self.next_synced_id = 0;

        // Build word table and decide text encodings
        self.build_word_table();

        // Emit symbol table first (only on first call)
        if !self.symbols.is_empty() {
            self.buf.begin_symbols(self.symbols.len() as u8);
            for sym in self.symbols.drain(..) {
                self.buf.add_symbol(&sym);
            }
        }

        // Emit word table if we have words
        if !self.words.is_empty() {
            self.buf.begin_word_table(self.words.len() as u8);
            for word in &self.words {
                self.buf.add_word(word);
            }
        }

        self.emit_element_multi(el, None, states)
    }

    /// Emit text content using the best encoding.
    fn emit_text(&mut self, ref_idx: u8, text: &str) {
        match self.text_encodings.get(text) {
            Some(TextEncoding::Int(n)) => {
                self.buf.set_text_int(ref_idx, *n);
            }
            Some(TextEncoding::Words(indices)) => {
                self.buf.set_text_words(ref_idx, indices);
            }
            _ => {
                // Fall back to symbol encoding
                let sym = *self.symbol_map.get(text).unwrap();
                self.buf.set_text(ref_idx, sym);
            }
        }
    }

    fn emit_element(&mut self, el: &ElementBuilder, parent_ref: Option<u8>, state: &dyn Any) -> u8 {
        // If this is a synced element, render it and wrap with an ID
        if let Some(renderer) = &el.synced {
            let synced_id = self.next_synced_id;
            self.next_synced_id += 1;

            // Store the synced element info for later updates
            self.synced_elements.push(SyncedElement {
                id: synced_id,
                state_type_id: renderer.state_type_id(),
                renderer: renderer.clone_box(),
                deps: renderer.deps(),
            });

            if let Some(rendered) = renderer.render_with_state(state) {
                // Track span element usage for synced wrapper
                self.used_elements.insert(El::Span.as_u8());

                // Use CREATE_SYNCED opcode - more compact than CREATE span + SET_ATTR id
                let ref_idx = self.buf.create_synced(synced_id);

                // Emit the rendered content as a child
                self.emit_element(&rendered, Some(ref_idx), state);

                // Append wrapper to parent
                if let Some(parent) = parent_ref {
                    self.buf.append(parent, ref_idx);
                } else {
                    self.buf.append_to_body(ref_idx);
                }

                return ref_idx;
            }
        }

        // Track element type usage
        self.used_elements.insert(el.el_type.as_u8());

        let ref_idx = self.buf.create(el.el_type.as_u8());

        if let Some(ref class) = el.class {
            let sym = *self.symbol_map.get(class).unwrap();
            self.buf.set_class(ref_idx, sym);
        }

        if let Some(ref text) = el.text {
            self.emit_text(ref_idx, text);
        }

        for (key, value) in &el.attrs {
            let key_sym = *self.symbol_map.get(key).unwrap();
            let val_sym = *self.symbol_map.get(value).unwrap();
            self.buf.set_attr(ref_idx, key_sym, val_sym);
        }

        // Bind events and track event type usage
        for (ev, handler_spec) in &el.events {
            self.used_events.insert(ev.as_u8());

            match handler_spec.storage_type {
                StorageType::Local => {
                    // Local handler - register mutations and emit BIND_LOCAL
                    if let Some(mutations) = &handler_spec.local_mutations {
                        let handler_idx = self
                            .register_local_handler(mutations.clone(), handler_spec.state_type_id);
                        self.buf.bind_local(ref_idx, ev.as_u8(), handler_idx);
                    }
                }
                StorageType::Memory | StorageType::Persisted => {
                    // Remote handler - register handler and emit BIND_REMOTE or BIND_REMOTE_PARAM
                    if let Some(handler) = &handler_spec.remote_handler {
                        let handler_idx = self.register_remote_handler(handler.clone());

                        // Use BIND_REMOTE_PARAM if we have param bytes, otherwise BIND_REMOTE
                        if let Some(param_bytes) = &handler_spec.param_bytes {
                            self.buf.bind_remote_param(
                                ref_idx,
                                ev.as_u8(),
                                handler_idx,
                                param_bytes,
                            );
                        } else {
                            self.buf.bind_remote(ref_idx, ev.as_u8(), handler_idx);
                        }
                    }
                }
            }
        }

        // Emit children
        for child in &el.children {
            self.emit_element(child, Some(ref_idx), state);
        }

        // Append to parent
        if let Some(parent) = parent_ref {
            self.buf.append(parent, ref_idx);
        } else {
            self.buf.append_to_body(ref_idx);
        }

        ref_idx
    }

    /// Emit an element with multi-state support.
    fn emit_element_multi(
        &mut self,
        el: &ElementBuilder,
        parent_ref: Option<u8>,
        states: &HashMap<TypeId, &(dyn Any + Send + Sync)>,
    ) -> u8 {
        // If this is a synced element, render it and wrap with an ID
        if let Some(renderer) = &el.synced {
            let synced_id = self.next_synced_id;
            self.next_synced_id += 1;

            // Store the synced element info for later updates
            self.synced_elements.push(SyncedElement {
                id: synced_id,
                state_type_id: renderer.state_type_id(),
                renderer: renderer.clone_box(),
                deps: renderer.deps(),
            });

            // Find the state for this renderer's state type
            let state_type_id = renderer.state_type_id();
            if let Some(state) = states.get(&state_type_id) {
                if let Some(rendered) = renderer.render_with_state(*state) {
                    // Track span element usage for synced wrapper
                    self.used_elements.insert(El::Span.as_u8());

                    // Use CREATE_SYNCED opcode - more compact than CREATE span + SET_ATTR id
                    let ref_idx = self.buf.create_synced(synced_id);

                    // Emit the rendered content as a child
                    self.emit_element_multi(&rendered, Some(ref_idx), states);

                    // Append wrapper to parent
                    if let Some(parent) = parent_ref {
                        self.buf.append(parent, ref_idx);
                    } else {
                        self.buf.append_to_body(ref_idx);
                    }

                    return ref_idx;
                }
            }
            // If state not found or render failed, skip this synced element
            return 0;
        }

        // Track element type usage
        self.used_elements.insert(el.el_type.as_u8());

        let ref_idx = self.buf.create(el.el_type.as_u8());

        if let Some(ref class) = el.class {
            let sym = *self.symbol_map.get(class).unwrap();
            self.buf.set_class(ref_idx, sym);
        }

        if let Some(ref text) = el.text {
            self.emit_text(ref_idx, text);
        }

        for (key, value) in &el.attrs {
            let key_sym = *self.symbol_map.get(key).unwrap();
            let val_sym = *self.symbol_map.get(value).unwrap();
            self.buf.set_attr(ref_idx, key_sym, val_sym);
        }

        // Bind events and track event type usage
        for (ev, handler_spec) in &el.events {
            self.used_events.insert(ev.as_u8());

            match handler_spec.storage_type {
                StorageType::Local => {
                    // Local handler - register mutations and emit BIND_LOCAL
                    if let Some(mutations) = &handler_spec.local_mutations {
                        let handler_idx = self
                            .register_local_handler(mutations.clone(), handler_spec.state_type_id);
                        self.buf.bind_local(ref_idx, ev.as_u8(), handler_idx);
                    }
                }
                StorageType::Memory | StorageType::Persisted => {
                    // Remote handler - register handler and emit BIND_REMOTE or BIND_REMOTE_PARAM
                    if let Some(handler) = &handler_spec.remote_handler {
                        let handler_idx = self.register_remote_handler(handler.clone());

                        // Use BIND_REMOTE_PARAM if we have param bytes, otherwise BIND_REMOTE
                        if let Some(param_bytes) = &handler_spec.param_bytes {
                            self.buf.bind_remote_param(
                                ref_idx,
                                ev.as_u8(),
                                handler_idx,
                                param_bytes,
                            );
                        } else {
                            self.buf.bind_remote(ref_idx, ev.as_u8(), handler_idx);
                        }
                    }
                }
            }
        }

        // Emit children
        for child in &el.children {
            self.emit_element_multi(child, Some(ref_idx), states);
        }

        // Append to parent
        if let Some(parent) = parent_ref {
            self.buf.append(parent, ref_idx);
        } else {
            self.buf.append_to_body(ref_idx);
        }

        ref_idx
    }

    /// Emit local handler definitions to the buffer.
    ///
    /// This should be called after emit() but before finish().
    pub fn emit_local_handlers(&mut self) {
        for (idx, mutations) in self.local_handlers.iter().enumerate() {
            let encoded = mutations.encode();
            self.buf.def_local_handler(
                idx as u8,
                mutations.state_idx,
                &encoded,
                mutations.mutations.len() as u8,
            );
        }
    }

    /// Finish building and return the bytes.
    pub fn finish(mut self) -> Bytes {
        self.buf.end();
        self.buf.finish()
    }

    /// Get the registered remote handlers.
    pub fn handlers(&self) -> &[HandlerFn] {
        &self.handlers
    }

    /// Get the registered local handlers.
    pub fn local_handlers(&self) -> &[LocalMutations] {
        &self.local_handlers
    }

    /// Check if any local handlers are registered.
    pub fn has_local_handlers(&self) -> bool {
        self.has_local_handlers
    }

    /// Take the synced elements.
    pub fn take_synced_elements(&mut self) -> Vec<SyncedElement> {
        std::mem::take(&mut self.synced_elements)
    }

    /// Get the set of used element type byte codes.
    pub fn used_elements(&self) -> &HashSet<u8> {
        &self.used_elements
    }

    /// Get the set of used event type byte codes.
    pub fn used_events(&self) -> &HashSet<u8> {
        &self.used_events
    }

    /// Get the local state type indices mapping.
    pub fn local_state_indices(&self) -> &HashMap<TypeId, u8> {
        &self.local_state_indices
    }

    /// Get the symbol map for tracking sent symbols.
    ///
    /// This returns a clone of the symbol map after rendering, which can be
    /// used to track which symbols were sent to the client for incremental
    /// symbol updates.
    pub fn take_symbol_map(&self) -> HashMap<String, u8> {
        self.symbol_map.clone()
    }

    /// Emit local state initialization opcodes.
    ///
    /// This should be called after emit() and emit_local_handlers() to initialize
    /// client-side state. The `serializer` function takes a TypeId and returns
    /// the JSON serialization of the default state for that type.
    pub fn emit_local_state<F>(&mut self, serializer: F)
    where
        F: Fn(TypeId) -> Option<String>,
    {
        for (&type_id, &state_idx) in &self.local_state_indices {
            if let Some(json) = serializer(type_id) {
                self.buf.init_local_state(state_idx, &json);
            }
        }
    }
}

impl Default for BuildContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Build an update for synced elements that need to re-render.
///
/// This version uses a single state for all synced elements (backwards compatible).
/// Re-renders all synced elements (uses `ChangeSet::all()`).
/// Note: This version doesn't support registering new handlers during re-render.
pub fn build_synced_update(synced: &[SyncedElement], state: &(dyn Any + Send + Sync)) -> Bytes {
    let mut states = HashMap::new();
    states.insert(state.type_id(), state);
    let mut handlers = Vec::new();
    build_synced_update_multi(synced, &states, &mut handlers, ChangeSet::all())
}

/// Build an update for synced elements with multi-state support.
///
/// Each synced element will be rendered with its corresponding state type.
/// Handlers are passed to enable event rebinding on dynamically created elements.
/// New handlers discovered during re-render will be added to the handlers vector.
///
/// The `changes` parameter specifies which state fields were modified by the handler.
/// Only synced elements whose dependencies overlap with the changed fields will be
/// re-rendered. This provides zero-runtime overhead filtering using u64 bitmask operations.
///
/// Nested synced elements are properly handled: when a parent synced element
/// re-renders, any nested synced elements within its content are wrapped with
/// their correct IDs so subsequent updates can find them.
///
/// Note: This function uses GET_SYNCED opcodes with varint-encoded IDs instead of
/// GET_BY_ID with symbol table entries for "__synced_N" strings. This significantly
/// reduces update message sizes (~15 bytes per synced element).
pub fn build_synced_update_multi(
    synced: &[SyncedElement],
    states: &HashMap<TypeId, &(dyn Any + Send + Sync)>,
    handlers: &mut Vec<HandlerFn>,
    changes: ChangeSet,
) -> Bytes {
    build_synced_update_with_known_symbols(synced, states, handlers, changes, None)
}

/// Build an update for synced elements with incremental symbol support.
///
/// This is the same as `build_synced_update_multi` but supports tracking which
/// symbols have already been sent to the client. If `known_symbols` is provided:
/// - Only new symbols are sent using SYMBOLS_EXTEND
/// - Known symbols use their existing indices
/// - `known_symbols` is updated with any new symbols after this call
///
/// This can reduce update message sizes by 50-90% for repeated updates.
pub fn build_synced_update_with_known_symbols(
    synced: &[SyncedElement],
    states: &HashMap<TypeId, &(dyn Any + Send + Sync)>,
    handlers: &mut Vec<HandlerFn>,
    changes: ChangeSet,
    known_symbols: Option<&mut HashMap<String, u8>>,
) -> Bytes {
    let mut buf = OpcodeBuffer::new();

    // Collect all symbols first (but NOT synced element IDs - those use GET_SYNCED opcode)
    let mut new_symbols: Vec<String> = Vec::new();
    let mut symbol_map: HashMap<String, u8> = HashMap::new();

    // If we have known symbols, seed the symbol_map with them
    let next_symbol_idx = if let Some(known) = &known_symbols {
        // Copy known symbols into symbol_map
        for (sym, idx) in known.iter() {
            symbol_map.insert(sym.clone(), *idx);
        }
        // Next index is 0x80 + count of known symbols
        0x80u8.saturating_add(known.len() as u8)
    } else {
        0x80u8
    };

    let mut current_next_idx = next_symbol_idx;

    // First pass: collect symbols (NOT including synced element wrapper IDs anymore)
    // We need to track synced counter to assign correct IDs to nested synced elements
    // Only process synced elements that need updating based on the ChangeSet
    let mut synced_counter: u32 = 0;
    let mut has_updates = false;
    for se in synced {
        // Track the highest synced ID to know where nested ones start
        if se.id >= synced_counter {
            synced_counter = se.id + 1;
        }

        // Skip elements that don't need updating (zero-cost bitmask check)
        if !se.deps.needs_update(changes) {
            continue;
        }
        has_updates = true;

        // Find the state for this synced element's state type
        if let Some(state) = states.get(&se.state_type_id) {
            if let Some(rendered) = se.renderer.render_with_state(*state) {
                collect_symbols_recursive_with_known(
                    &rendered,
                    &mut new_symbols,
                    &mut symbol_map,
                    &mut current_next_idx,
                    &mut synced_counter,
                    states,
                );
            }
        }
    }

    // Early return if no updates needed
    if !has_updates {
        return Bytes::new();
    }

    // Emit symbol table if we have any NEW symbols
    if !new_symbols.is_empty() {
        if known_symbols.is_some() {
            // Use SYMBOLS_EXTEND for incremental update
            buf.begin_symbols_extend(new_symbols.len() as u8, next_symbol_idx);
        } else {
            // Use regular SYMBOLS for full table
            buf.begin_symbols(new_symbols.len() as u8);
        }
        for sym in &new_symbols {
            buf.add_symbol(sym);
        }
    }

    // Update known_symbols with new symbols
    if let Some(known) = known_symbols {
        for (sym, idx) in symbol_map.iter() {
            known.insert(sym.clone(), *idx);
        }
    }

    // Second pass: emit updates with full re-render
    // Track which synced elements have been rendered as nested elements of a parent.
    // These should be skipped in the main loop since they're already updated.
    let mut rendered_ids: HashSet<u32> = HashSet::new();

    // Build a map of state_type_id -> list of synced element IDs (in order)
    // This allows us to match nested synced elements by type and order
    let mut ids_by_type: HashMap<TypeId, Vec<u32>> = HashMap::new();
    for se in synced {
        ids_by_type.entry(se.state_type_id).or_default().push(se.id);
    }

    // Track next index to use for each state type during emit
    let mut next_idx_by_type: HashMap<TypeId, usize> = HashMap::new();

    // Counter for any truly new nested synced elements (not in original list)
    let mut emit_synced_counter: u32 = synced
        .iter()
        .map(|se| se.id)
        .max()
        .map(|m| m + 1)
        .unwrap_or(0);

    for se in synced {
        // Skip elements that were already rendered as nested elements
        if rendered_ids.contains(&se.id) {
            continue;
        }

        // Skip elements that don't need updating (zero-cost bitmask check)
        if !se.deps.needs_update(changes) {
            continue;
        }

        // Mark this element as being processed (it's a top-level render, not nested)
        rendered_ids.insert(se.id);
        // Advance the index for this type since we're rendering it now
        let idx = next_idx_by_type.entry(se.state_type_id).or_insert(0);
        *idx += 1;

        // Find the state for this synced element's state type
        if let Some(state) = states.get(&se.state_type_id) {
            if let Some(rendered) = se.renderer.render_with_state(*state) {
                // Use GET_SYNCED opcode - more compact than GET_BY_ID with symbol
                let wrapper_ref = buf.get_synced(se.id);

                // Clear all existing children
                buf.clear_children(wrapper_ref);

                // Emit the full rendered tree as children of wrapper
                // Pass type->ID mapping so nested elements can find their original IDs
                emit_update_element(
                    &rendered,
                    wrapper_ref,
                    &mut buf,
                    &symbol_map,
                    handlers,
                    &ids_by_type,
                    &mut next_idx_by_type,
                    &mut rendered_ids,
                    &mut emit_synced_counter,
                    states,
                );
            }
        }
    }

    buf.end();
    buf.finish()
}

/// Emit an element and its children during a synced update.
///
/// This creates new DOM elements and appends them to the parent.
/// Event handlers are rebound using existing handler indices, or new handlers
/// are registered if they weren't present during initial render.
///
/// For nested synced elements, this finds their original ID by matching state type
/// and order, then marks them as already rendered to avoid duplicate updates.
#[allow(clippy::too_many_arguments)]
fn emit_update_element(
    el: &ElementBuilder,
    parent_ref: u8,
    buf: &mut OpcodeBuffer,
    symbol_map: &HashMap<String, u8>,
    handlers: &mut Vec<HandlerFn>,
    ids_by_type: &HashMap<TypeId, Vec<u32>>,
    next_idx_by_type: &mut HashMap<TypeId, usize>,
    rendered_ids: &mut HashSet<u32>,
    synced_counter: &mut u32,
    states: &HashMap<TypeId, &(dyn Any + Send + Sync)>,
) -> u8 {
    // Handle nested synced elements - create wrapper with ORIGINAL ID
    if let Some(renderer) = &el.synced {
        let state_type_id = renderer.state_type_id();

        // Find the original synced element ID by matching state_type_id and order
        let synced_id = if let Some(ids) = ids_by_type.get(&state_type_id) {
            let idx = next_idx_by_type.entry(state_type_id).or_insert(0);
            if *idx < ids.len() {
                let id = ids[*idx];
                *idx += 1;
                // Mark as rendered so it won't be processed in main loop
                rendered_ids.insert(id);
                id
            } else {
                // More nested elements than original - assign new ID
                let id = *synced_counter;
                *synced_counter += 1;
                id
            }
        } else {
            // Truly new synced element type, assign new ID
            let id = *synced_counter;
            *synced_counter += 1;
            id
        };

        // Find the state for this renderer's state type
        if let Some(state) = states.get(&state_type_id) {
            if let Some(rendered) = renderer.render_with_state(*state) {
                // Use CREATE_SYNCED opcode with the ORIGINAL ID
                let wrapper_ref = buf.create_synced(synced_id);

                // Emit the rendered content as a child of the wrapper
                emit_update_element(
                    &rendered,
                    wrapper_ref,
                    buf,
                    symbol_map,
                    handlers,
                    ids_by_type,
                    next_idx_by_type,
                    rendered_ids,
                    synced_counter,
                    states,
                );

                // Append wrapper to parent
                buf.append(parent_ref, wrapper_ref);

                return wrapper_ref;
            }
        }
        // If state not found or render failed, skip this synced element
        return 0;
    }

    // Create the element
    let ref_idx = buf.create(el.el_type.as_u8());

    // Set class
    if let Some(ref class) = el.class {
        if let Some(&sym) = symbol_map.get(class) {
            buf.set_class(ref_idx, sym);
        }
    }

    // Set text content
    if let Some(ref text) = el.text {
        if let Some(&sym) = symbol_map.get(text) {
            buf.set_text(ref_idx, sym);
        }
    }

    // Set attributes
    for (key, value) in &el.attrs {
        if let (Some(&key_sym), Some(&val_sym)) = (symbol_map.get(key), symbol_map.get(value)) {
            buf.set_attr(ref_idx, key_sym, val_sym);
        }
    }

    // Bind events - look up handler index from existing handlers by function pointer
    // If handler not found, register it as a new handler
    for (ev, spec) in &el.events {
        if let Some(handler) = &spec.remote_handler {
            let handler_fn_id = handler.fn_id();
            // Find matching handler by function pointer ID
            let handler_idx = handlers
                .iter()
                .position(|h| h.fn_id() == handler_fn_id)
                .unwrap_or_else(|| {
                    // Handler not found - register it as new
                    let idx = handlers.len();
                    handlers.push(handler.clone());
                    idx
                });

            // Use BIND_REMOTE_PARAM if we have param bytes
            if let Some(param_bytes) = &spec.param_bytes {
                buf.bind_remote_param(ref_idx, ev.as_u8(), handler_idx as u8, param_bytes);
            } else {
                buf.bind_remote(ref_idx, ev.as_u8(), handler_idx as u8);
            }
        }
    }

    // Recursively emit children
    for child in &el.children {
        emit_update_element(
            child,
            ref_idx,
            buf,
            symbol_map,
            handlers,
            ids_by_type,
            next_idx_by_type,
            rendered_ids,
            synced_counter,
            states,
        );
    }

    // Append to parent
    buf.append(parent_ref, ref_idx);

    ref_idx
}

// Note: This function is kept for backwards compatibility with tests that use
// build_synced_update_multi without known symbols tracking.
#[allow(dead_code)]
fn collect_symbols_recursive(
    el: &ElementBuilder,
    symbols: &mut Vec<String>,
    symbol_map: &mut HashMap<String, u8>,
    synced_counter: &mut u32,
    states: &HashMap<TypeId, &(dyn Any + Send + Sync)>,
) {
    fn intern(s: &str, symbols: &mut Vec<String>, symbol_map: &mut HashMap<String, u8>) {
        if symbol_map.contains_key(s) {
            return;
        }
        let idx = 0x80 + symbols.len() as u8;
        symbols.push(s.to_string());
        symbol_map.insert(s.to_string(), idx);
    }

    // Handle synced elements - recursive symbol collection but NO wrapper ID interning
    // (using CREATE_SYNCED opcode instead)
    if let Some(renderer) = &el.synced {
        // Track synced ID but don't intern - using CREATE_SYNCED opcode instead
        *synced_counter += 1;

        // Render and collect symbols from the rendered content
        let state_type_id = renderer.state_type_id();
        if let Some(state) = states.get(&state_type_id) {
            if let Some(rendered) = renderer.render_with_state(*state) {
                collect_symbols_recursive(&rendered, symbols, symbol_map, synced_counter, states);
            }
        }
        return;
    }

    if let Some(ref text) = el.text {
        intern(text, symbols, symbol_map);
    }
    if let Some(ref class) = el.class {
        intern(class, symbols, symbol_map);
    }
    for (key, value) in &el.attrs {
        intern(key, symbols, symbol_map);
        intern(value, symbols, symbol_map);
    }
    for child in &el.children {
        collect_symbols_recursive(child, symbols, symbol_map, synced_counter, states);
    }
}

/// Variant of collect_symbols_recursive that supports incremental symbol updates.
/// Only new symbols (not in the initial symbol_map) are added to new_symbols.
fn collect_symbols_recursive_with_known(
    el: &ElementBuilder,
    new_symbols: &mut Vec<String>,
    symbol_map: &mut HashMap<String, u8>,
    next_idx: &mut u8,
    synced_counter: &mut u32,
    states: &HashMap<TypeId, &(dyn Any + Send + Sync)>,
) {
    fn intern_with_known(
        s: &str,
        new_symbols: &mut Vec<String>,
        symbol_map: &mut HashMap<String, u8>,
        next_idx: &mut u8,
    ) {
        if symbol_map.contains_key(s) {
            return;
        }
        let idx = *next_idx;
        *next_idx = next_idx.saturating_add(1);
        new_symbols.push(s.to_string());
        symbol_map.insert(s.to_string(), idx);
    }

    // Handle synced elements - recursive symbol collection but NO wrapper ID interning
    // (using CREATE_SYNCED opcode instead)
    if let Some(renderer) = &el.synced {
        // Track synced ID but don't intern - using CREATE_SYNCED opcode instead
        *synced_counter += 1;

        // Render and collect symbols from the rendered content
        let state_type_id = renderer.state_type_id();
        if let Some(state) = states.get(&state_type_id) {
            if let Some(rendered) = renderer.render_with_state(*state) {
                collect_symbols_recursive_with_known(
                    &rendered,
                    new_symbols,
                    symbol_map,
                    next_idx,
                    synced_counter,
                    states,
                );
            }
        }
        return;
    }

    if let Some(ref text) = el.text {
        intern_with_known(text, new_symbols, symbol_map, next_idx);
    }
    if let Some(ref class) = el.class {
        intern_with_known(class, new_symbols, symbol_map, next_idx);
    }
    for (key, value) in &el.attrs {
        intern_with_known(key, new_symbols, symbol_map, next_idx);
        intern_with_known(value, new_symbols, symbol_map, next_idx);
    }
    for child in &el.children {
        collect_symbols_recursive_with_known(
            child,
            new_symbols,
            symbol_map,
            next_idx,
            synced_counter,
            states,
        );
    }
}
