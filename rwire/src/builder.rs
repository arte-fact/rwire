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
use std::any::Any;
use std::collections::{HashMap, HashSet};

use crate::protocol::{El, Ev, OpcodeBuffer};
use crate::state::{ClientState, HandlerFn, Renderer};

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
pub(crate) trait SyncedRenderer: Send + Sync {
    /// Render with the given state, returning a new ElementBuilder.
    fn render_with_state(&self, state: &dyn Any) -> Option<ElementBuilder>;
    /// Clone this renderer into a boxed trait object.
    fn clone_box(&self) -> Box<dyn SyncedRenderer>;
}

/// Implementation of SyncedRenderer for a specific state type.
struct SyncedRendererImpl<S: ClientState> {
    render: Renderer<S>,
}

impl<S: ClientState> SyncedRenderer for SyncedRendererImpl<S> {
    fn render_with_state(&self, state: &dyn Any) -> Option<ElementBuilder> {
        state.downcast_ref::<S>().map(|s| (self.render)(s))
    }

    fn clone_box(&self) -> Box<dyn SyncedRenderer> {
        Box::new(SyncedRendererImpl { render: self.render })
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
    events: Vec<(Ev, HandlerFn)>,
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
    /// This is called by the `#[renderer]` macro.
    pub fn synced<S: ClientState>(render: Renderer<S>) -> Self {
        Self {
            el_type: El::Div, // Placeholder, will be replaced by rendered content
            text: None,
            class: None,
            attrs: Vec::new(),
            events: Vec::new(),
            children: Vec::new(),
            synced: Some(Box::new(SyncedRendererImpl { render })),
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

    /// Bind an event handler to this element.
    ///
    /// The handler function will be called when the event occurs.
    pub fn on(mut self, ev: Ev, handler: HandlerFn) -> Self {
        self.events.push((ev, handler));
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
    pub fn events(&self) -> &[(Ev, HandlerFn)] {
        &self.events
    }
}

/// Context for building the DOM tree with state support.
pub struct BuildContext {
    buf: OpcodeBuffer,
    symbols: Vec<String>,
    symbol_map: HashMap<String, u8>,
    handlers: Vec<HandlerFn>,
    synced_elements: Vec<SyncedElement>,
    next_synced_id: u32,
    used_elements: HashSet<u8>,
    used_events: HashSet<u8>,
}

/// Information about a synced element for later updates.
pub struct SyncedElement {
    pub(crate) id: u32,
    pub(crate) renderer: Box<dyn SyncedRenderer>,
}

impl Clone for SyncedElement {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            renderer: self.renderer.clone_box(),
        }
    }
}

impl BuildContext {
    pub fn new() -> Self {
        Self {
            buf: OpcodeBuffer::new(),
            symbols: Vec::new(),
            symbol_map: HashMap::new(),
            handlers: Vec::new(),
            synced_elements: Vec::new(),
            next_synced_id: 0,
            used_elements: HashSet::new(),
            used_events: HashSet::new(),
        }
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

    fn register_handler(&mut self, handler: HandlerFn) -> u8 {
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

    /// Collect all symbols from an element tree (first pass).
    /// Also tracks used element and event types for tree shaking.
    pub fn collect_symbols(&mut self, el: &ElementBuilder, state: &dyn Any) {
        // If this is a synced element, render it first with state
        if let Some(renderer) = &el.synced {
            // Track the span wrapper element type
            self.used_elements.insert(El::Span.as_u8());
            if let Some(rendered) = renderer.render_with_state(state) {
                self.collect_symbols(&rendered, state);
            }
            return;
        }

        // Track element type usage
        self.used_elements.insert(el.el_type.as_u8());

        if let Some(ref text) = el.text {
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
        // Intern synced element IDs
        for child in &el.children {
            if child.synced.is_some() {
                // Pre-intern the synced element's wrapper ID
                let synced_id = format!("__synced_{}", self.next_synced_id);
                self.intern(&synced_id);
                self.next_synced_id += 1;
            }
            self.collect_symbols(child, state);
        }
        // Reset synced_id counter after collecting - we'll increment again during emit
        self.next_synced_id = 0;
    }

    /// Emit opcodes for an element tree (second pass).
    pub fn emit(&mut self, el: &ElementBuilder, state: &dyn Any) -> u8 {
        // Emit symbol table first (only on first call)
        if !self.symbols.is_empty() {
            self.buf.begin_symbols(self.symbols.len() as u8);
            for sym in self.symbols.drain(..) {
                self.buf.add_symbol(&sym);
            }
        }

        self.emit_element(el, None, state)
    }

    fn emit_element(&mut self, el: &ElementBuilder, parent_ref: Option<u8>, state: &dyn Any) -> u8 {
        // If this is a synced element, render it and wrap with an ID
        if let Some(renderer) = &el.synced {
            let synced_id = self.next_synced_id;
            self.next_synced_id += 1;

            // Store the synced element info for later updates
            self.synced_elements.push(SyncedElement {
                id: synced_id,
                renderer: renderer.clone_box(),
            });

            if let Some(rendered) = renderer.render_with_state(state) {
                // Create a wrapper span with an ID for later targeting
                let wrapper_id = format!("__synced_{}", synced_id);
                let wrapper_id_sym = *self.symbol_map.get(&wrapper_id).unwrap();

                // Track span element usage for synced wrapper
                self.used_elements.insert(El::Span.as_u8());

                let ref_idx = self.buf.create(El::Span.as_u8());
                // Built-in symbol 0x04 is "id"
                self.buf.set_attr(ref_idx, 0x04, wrapper_id_sym);

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
            let sym = *self.symbol_map.get(text).unwrap();
            self.buf.set_text(ref_idx, sym);
        }

        for (key, value) in &el.attrs {
            let key_sym = *self.symbol_map.get(key).unwrap();
            let val_sym = *self.symbol_map.get(value).unwrap();
            self.buf.set_attr(ref_idx, key_sym, val_sym);
        }

        // Bind events and track event type usage
        for (ev, handler) in &el.events {
            self.used_events.insert(ev.as_u8());
            let handler_idx = self.register_handler(handler.clone());
            self.buf.bind_local(ref_idx, ev.as_u8(), handler_idx);
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

    /// Finish building and return the bytes.
    pub fn finish(mut self) -> Bytes {
        self.buf.end();
        self.buf.finish()
    }

    /// Get the registered handlers.
    pub fn handlers(&self) -> &[HandlerFn] {
        &self.handlers
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
}

impl Default for BuildContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Build an update for synced elements that need to re-render.
pub fn build_synced_update(synced: &[SyncedElement], state: &dyn Any) -> Bytes {
    let mut buf = OpcodeBuffer::new();

    // Collect all symbols first
    let mut symbols: Vec<String> = Vec::new();
    let mut symbol_map: HashMap<String, u8> = HashMap::new();

    let intern = |s: &str, symbols: &mut Vec<String>, symbol_map: &mut HashMap<String, u8>| -> u8 {
        if let Some(&idx) = symbol_map.get(s) {
            return idx;
        }
        let idx = 0x80 + symbols.len() as u8;
        symbols.push(s.to_string());
        symbol_map.insert(s.to_string(), idx);
        idx
    };

    // First pass: collect symbols
    for se in synced {
        let wrapper_id = format!("__synced_{}", se.id);
        intern(&wrapper_id, &mut symbols, &mut symbol_map);

        if let Some(rendered) = se.renderer.render_with_state(state) {
            collect_symbols_recursive(&rendered, &mut symbols, &mut symbol_map);
        }
    }

    if symbols.is_empty() {
        return Bytes::new();
    }

    // Emit symbol table
    buf.begin_symbols(symbols.len() as u8);
    for sym in &symbols {
        buf.add_symbol(sym);
    }

    // Second pass: emit updates
    for se in synced {
        let wrapper_id = format!("__synced_{}", se.id);
        let wrapper_id_sym = *symbol_map.get(&wrapper_id).unwrap();

        if let Some(rendered) = se.renderer.render_with_state(state) {
            // Get the wrapper by ID
            buf.get_by_id(wrapper_id_sym);
            let wrapper_ref = 0u8; // GET_BY_ID assigns ref 0

            // Clear and rebuild the wrapper's content
            // For now, we'll just update the text if it's a simple element
            if let Some(text) = rendered.text_content() {
                let text_sym = *symbol_map.get(text).unwrap();
                buf.set_text(wrapper_ref, text_sym);
            }
        }
    }

    buf.end();
    buf.finish()
}

fn collect_symbols_recursive(el: &ElementBuilder, symbols: &mut Vec<String>, symbol_map: &mut HashMap<String, u8>) {
    let mut intern = |s: &str| {
        if symbol_map.contains_key(s) {
            return;
        }
        let idx = 0x80 + symbols.len() as u8;
        symbols.push(s.to_string());
        symbol_map.insert(s.to_string(), idx);
    };

    if let Some(ref text) = el.text {
        intern(text);
    }
    if let Some(ref class) = el.class {
        intern(class);
    }
    for (key, value) in &el.attrs {
        intern(key);
        intern(value);
    }
    for child in &el.children {
        collect_symbols_recursive(child, symbols, symbol_map);
    }
}
