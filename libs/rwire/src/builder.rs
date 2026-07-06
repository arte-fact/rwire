//! Fluent builder API for constructing DOM elements with reactive synced regions.
//!
//! This module provides a high-level, ergonomic API for building DOM structures
//! that get compiled down to the binary opcode protocol.
//!
//! # Example
//!
//! ```ignore
//! use rwire::{el, El, Ev, State, handler, renderer};
//!
//! #[derive(State, Default)]
//! #[storage(memory)]
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

use bytes::{BufMut, Bytes, BytesMut};
use std::any::{Any, TypeId};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::action::{Selector, Target};
use crate::attr_tokens::{At, Av};
use crate::item_ref::ItemRef;
use crate::protocol::encoder::{
    NAME_ATTR_KEY, NAME_ATTR_VALUE, NAME_ELEMENT, NAME_EVENT, NAME_STYLE_PROP, NAME_STYLE_VALUE,
};
use crate::protocol::opcodes::{
    ELEMENT_MAPPINGS, EVENT_MAPPINGS, MAP_DEF, STYLE_DEF, SVG_ELEMENT_CODES,
};
use crate::protocol::varint::write_varint;
use crate::protocol::{El, Ev, OpcodeBuffer};
use crate::state::{ChangeSet, HandlerFn, HandlerSpec, Renderer, RendererDeps, State, StorageType};
use crate::style_tokens::StyleKey;

/// Encode a `STYLE_DEF` opcode block carrying complete CSS rule strings.
///
/// Format: `[STYLE_DEF, count_varint, (rule_len_varint, rule_utf8){count}]`.
fn encode_style_def(rules: &[String]) -> BytesMut {
    let mut buf = BytesMut::new();
    buf.put_u8(STYLE_DEF);
    write_varint(&mut buf, rules.len() as u32);
    for rule in rules {
        write_varint(&mut buf, rule.len() as u32);
        buf.put_slice(rule.as_bytes());
    }
    buf
}

/// HTML void elements: emitted with no closing tag and no children.
fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "source"
            | "track"
            | "wbr"
    )
}

fn push_attr_escaped(out: &mut String, value: &str) {
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

fn push_text_escaped(out: &mut String, value: &str) {
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
}

/// Build the `STYLE_DEF` prefix for class-referenced rules in `referenced` that
/// this connection (`sent`) has not yet received, marking them sent.
///
/// Returns empty bytes when there is nothing new. Prepend the result to a batch's
/// body so each rule lands before (or with) the node that uses it. Used for both
/// the initial DOM and every incremental update — see
/// `docs/tree-shaking-redesign.md` (Phase 2).
pub fn style_def_prefix(referenced: &BTreeSet<StyleKey>, sent: &mut HashSet<StyleKey>) -> BytesMut {
    let mut new_rules: Vec<String> = Vec::new();
    for &key in referenced {
        // `insert` returns true the first time this connection sees the key.
        if sent.insert(key) {
            if let Some(rule) = key.to_css_rule() {
                new_rules.push(rule);
            }
        }
    }
    if new_rules.is_empty() {
        return BytesMut::new();
    }
    encode_style_def(&new_rules)
}

/// Prepend a (possibly empty) lazy-delivery prefix to a body buffer.
fn prepend(prefix: BytesMut, body: Bytes) -> Bytes {
    if prefix.is_empty() {
        return body;
    }
    let mut out = prefix;
    out.extend_from_slice(&body);
    out.freeze()
}

/// Encode a `MAP_DEF` opcode block carrying `(kind, code, name)` entries.
///
/// Format: `[MAP_DEF, count_varint, (kind_u8, code_u8, name_len_varint, name_utf8){count}]`.
fn encode_map_def(entries: &[(u8, u8, &'static str)]) -> BytesMut {
    let mut buf = BytesMut::new();
    buf.put_u8(MAP_DEF);
    write_varint(&mut buf, entries.len() as u32);
    for &(kind, code, name) in entries {
        buf.put_u8(kind);
        buf.put_u8(code);
        write_varint(&mut buf, name.len() as u32);
        buf.put_slice(name.as_bytes());
    }
    buf
}

/// Look up the wire name for a `(category, code)` name-map reference.
fn name_for(category: u8, code: u8) -> Option<&'static str> {
    use crate::attr_tokens::{AT_MAPPINGS, AV_MAPPINGS};
    use crate::style_tokens::{PROP_MAPPINGS, VALUE_MAPPINGS};
    let table: &[(u8, &str)] = match category {
        NAME_ELEMENT => ELEMENT_MAPPINGS,
        NAME_EVENT => EVENT_MAPPINGS,
        NAME_ATTR_KEY => AT_MAPPINGS,
        NAME_ATTR_VALUE => AV_MAPPINGS,
        NAME_STYLE_PROP => PROP_MAPPINGS,
        NAME_STYLE_VALUE => VALUE_MAPPINGS,
        _ => return None,
    };
    table.iter().find(|(c, _)| *c == code).map(|(_, n)| *n)
}

/// Build the `MAP_DEF` prefix for `(category, code)` name references in `referenced`
/// that this connection (`sent`) has not yet received, marking them sent.
///
/// Mirrors [`style_def_prefix`] for the lazy delivery of element/event/attribute/
/// style-token names: the capsule ships empty maps and each name arrives the first time
/// its code is referenced. SVG element codes are emitted with wire `kind` 6, so the client
/// sets both `E[code]` and `SE[code]=1`. Returns empty bytes when there is nothing new.
pub fn map_def_prefix(referenced: &BTreeSet<(u8, u8)>, sent: &mut HashSet<(u8, u8)>) -> BytesMut {
    let mut new_entries: Vec<(u8, u8, &'static str)> = Vec::new();
    for &(category, code) in referenced {
        // `insert` returns true the first time this connection sees the (category, code).
        if sent.insert((category, code)) {
            if let Some(name) = name_for(category, code) {
                let kind = if category == NAME_ELEMENT && SVG_ELEMENT_CODES.contains(&code) {
                    6
                } else {
                    category
                };
                new_entries.push((kind, code, name));
            }
        }
    }
    if new_entries.is_empty() {
        return BytesMut::new();
    }
    encode_map_def(&new_entries)
}

/// A per-connection set of already-delivered `(category, code)` name entries (`MAP_DEF`).
pub type SentMaps<'a> = Option<&'a mut HashSet<(u8, u8)>>;

/// A typed attribute: enum key + enum value, enum key + symbol value, or bool attr.
#[derive(Clone, Debug)]
pub enum TypedAttr {
    /// Enum key + enum value (SET_ATTR_ENUM: 4 bytes)
    Enum(At, Av),
    /// Boolean attribute, presence-only (SET_ATTR_BOOL: 3 bytes)
    Bool(At),
    /// Enum key + string value (SET_ATTR_KEY_SYM: 4-5 bytes)
    KeySym(At, String),
}

/// Global counter for generating unique element IDs.
static NEXT_ELEMENT_ID: AtomicU32 = AtomicU32::new(0);

/// Generate a unique element ID with the given prefix.
///
/// Used for form element associations (label ↔ input).
///
/// # Example
///
/// ```ignore
/// let id = generate_element_id("field_");
/// el(El::Label).attr("for", &id)
/// el(El::Input).attr("id", &id)
/// ```
pub fn generate_element_id(prefix: &str) -> String {
    let id = NEXT_ELEMENT_ID.fetch_add(1, Ordering::Relaxed);
    format!("{}{}", prefix, id)
}

/// Extract all synced renderers from an element tree recursively.
///
/// This is used during capsule generation to discover all renderer types
/// so we can create default state instances for proper tree walking during
/// symbol collection.
///
/// Returns a vector of cloned renderers found in the tree.
pub fn extract_renderers(el: &ElementBuilder) -> Vec<Box<dyn SyncedRenderer>> {
    let mut renderers = Vec::new();

    // Check if this element has a synced renderer
    if let Some(renderer) = &el.synced {
        renderers.push(renderer.clone_box());
    }

    // Recursively extract from children
    for child in &el.children {
        renderers.extend(extract_renderers(child));
    }

    renderers
}

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
/// A stable per-sibling identity for keyed morphing (see [`ElementBuilder::key`]).
/// Strings hash with FNV-1a (32-bit); integers use their value (folded to 32
/// bits) — distinct ids stay distinct.
pub trait ElementKey {
    fn key_code(&self) -> u32;
}

fn fnv1a32(bytes: &[u8]) -> u32 {
    let mut h: u32 = 0x811C_9DC5;
    for &b in bytes {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

impl ElementKey for &str {
    fn key_code(&self) -> u32 {
        fnv1a32(self.as_bytes())
    }
}
impl ElementKey for String {
    fn key_code(&self) -> u32 {
        fnv1a32(self.as_bytes())
    }
}
impl ElementKey for u32 {
    fn key_code(&self) -> u32 {
        *self
    }
}
impl ElementKey for u64 {
    fn key_code(&self) -> u32 {
        (*self ^ (*self >> 32)) as u32
    }
}
impl ElementKey for usize {
    fn key_code(&self) -> u32 {
        (*self as u64).key_code()
    }
}

pub trait SyncedRenderer: Send + Sync {
    /// Render with the given state, returning a new ElementBuilder.
    fn render_with_state(&self, state: &dyn Any) -> Option<ElementBuilder>;
    /// Clone this renderer into a boxed trait object.
    fn clone_box(&self) -> Box<dyn SyncedRenderer>;
    /// Get the TypeId of the state type this renderer expects.
    fn state_type_id(&self) -> TypeId;
    /// Create a default state instance for this renderer's state type.
    fn create_default_state(&self) -> Box<dyn Any + Send + Sync>;
    /// Storage class of this renderer's state (default: memory).
    fn storage_type(&self) -> StorageType {
        StorageType::Memory
    }
    /// Cache-key base for shared/persisted state (default: none).
    fn table_name(&self) -> Option<&'static str> {
        None
    }
    /// Get the dependency information for this renderer.
    fn deps(&self) -> RendererDeps;
}

/// Implementation of SyncedRenderer for a specific state type.
struct SyncedRendererImpl<S: Default + Send + Sync + 'static> {
    render: Renderer<S>,
    deps: RendererDeps,
    /// Storage class of S, so the connection can resolve where its state lives
    /// (per-connection vs shared/persisted cache) even with no handler present.
    storage_type: StorageType,
    /// Cache-key base for shared/persisted state (the State `TABLE_NAME`).
    table_name: Option<&'static str>,
}

impl<S: Default + Send + Sync + 'static> SyncedRenderer for SyncedRendererImpl<S> {
    fn render_with_state(&self, state: &dyn Any) -> Option<ElementBuilder> {
        state.downcast_ref::<S>().map(|s| (self.render)(s))
    }

    fn clone_box(&self) -> Box<dyn SyncedRenderer> {
        Box::new(SyncedRendererImpl {
            render: self.render,
            deps: self.deps,
            storage_type: self.storage_type,
            table_name: self.table_name,
        })
    }

    fn state_type_id(&self) -> TypeId {
        TypeId::of::<S>()
    }

    fn create_default_state(&self) -> Box<dyn Any + Send + Sync> {
        Box::new(S::default())
    }

    fn storage_type(&self) -> StorageType {
        self.storage_type
    }

    fn table_name(&self) -> Option<&'static str> {
        self.table_name
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

/// A binding from an element to a Target's boolean state.
#[derive(Clone, Debug)]
struct TargetBinding {
    type_id: TypeId,
    st: u16,
    invert: bool,
    default: bool,
}

/// A trigger that toggles a Target on an event.
#[derive(Clone, Debug)]
struct TargetToggle {
    ev: Ev,
    type_id: TypeId,
}

/// A binding from an element to a Selector variant match.
#[derive(Clone, Debug)]
struct SelectorBinding {
    type_id: TypeId,
    match_val: u8,
    st: u16,
    default_val: u8,
}

/// A trigger that sets a Selector value on an event.
#[derive(Clone, Debug)]
struct SelectorSet {
    ev: Ev,
    type_id: TypeId,
    val: u8,
}

/// A trigger that sets a Target to true on event, then reverts to false after a delay.
/// Repeated events restart the timer.
#[derive(Clone, Debug)]
struct TimedTargetToggle {
    ev: Ev,
    type_id: TypeId,
    delay_ms: u16,
}

/// A delayed toggle that flips a Target boolean once after a delay from mount.
#[derive(Clone, Debug)]
struct AutoToggle {
    type_id: TypeId,
    delay_ms: u16,
}

/// Builder for constructing DOM elements with a fluent API.
#[derive(Clone)]
pub struct ElementBuilder {
    el_type: El,
    text: Option<String>,
    class: Option<String>,
    attrs: Vec<(String, String)>,
    /// Binary-encoded typed attributes (At/Av enums)
    typed_attrs: Vec<TypedAttr>,
    events: Vec<(Ev, HandlerSpec)>,
    /// Morph key (`__k`): sibling-local identity for keyed reordering.
    key: Option<u32>,
    children: Vec<ElementBuilder>,
    synced: Option<Box<dyn SyncedRenderer>>,
    /// Binary-encoded style utility tokens (compact 1-byte each)
    style_utils: Vec<u16>,
    /// Binary-encoded style property+value pairs (2 bytes each)
    style_props: Vec<(u8, u8)>,
    /// Pseudo-class/pseudo-element groups: (Pc code, St tokens)
    pseudo_groups: Vec<(u8, Vec<u16>)>,
    /// Responsive breakpoint groups: (Bp code, St tokens)
    breakpoint_groups: Vec<(u8, Vec<u16>)>,
    /// Target bindings: (TypeId, St code, invert, default)
    target_bindings: Vec<TargetBinding>,
    /// Target toggles: (Ev, TypeId)
    target_toggles: Vec<TargetToggle>,
    /// Selector bindings: (TypeId, match_val, St code, default_val)
    selector_bindings: Vec<SelectorBinding>,
    /// Selector sets: (Ev, TypeId, val)
    selector_sets: Vec<SelectorSet>,
    /// Timed target toggles: set true on event, revert after delay
    timed_toggles: Vec<TimedTargetToggle>,
    /// Auto toggles: flip target after delay from mount
    auto_toggles: Vec<AutoToggle>,
}

impl ElementBuilder {
    /// Create a new element builder with the given element type.
    pub fn new(el_type: El) -> Self {
        Self {
            el_type,
            text: None,
            class: None,
            attrs: Vec::new(),
            typed_attrs: Vec::new(),
            events: Vec::new(),
            key: None,
            children: Vec::new(),
            synced: None,
            style_utils: Vec::new(),
            style_props: Vec::new(),
            pseudo_groups: Vec::new(),
            breakpoint_groups: Vec::new(),
            target_bindings: Vec::new(),
            target_toggles: Vec::new(),
            selector_bindings: Vec::new(),
            selector_sets: Vec::new(),
            timed_toggles: Vec::new(),
            auto_toggles: Vec::new(),
        }
    }

    /// Create a synced element that re-renders on any state change (legacy).
    ///
    /// Prefer `synced_with_deps` for fine-grained re-render filtering.
    pub fn synced<S: Default + Send + Sync + 'static>(render: Renderer<S>) -> Self {
        Self::synced_with_deps::<S>(render, RendererDeps::always())
    }

    /// Create a memory-state synced element with explicit dependency tracking.
    ///
    /// Used for framework internals (e.g. Theme) and types that impl only the
    /// legacy `MemoryState` marker.
    pub fn synced_with_deps<S: Default + Send + Sync + 'static>(
        render: Renderer<S>,
        deps: RendererDeps,
    ) -> Self {
        Self::synced_from(Box::new(SyncedRendererImpl {
            render,
            deps,
            storage_type: StorageType::Memory,
            table_name: None,
        }))
    }

    /// Create a synced element carrying its state's storage class.
    ///
    /// This is what the `#[renderer]` macro emits. Reading `S::STORAGE_TYPE` and
    /// `S::TABLE_NAME` lets the connection resolve where a renderer's state lives
    /// (per-connection memory vs. shared/persisted cache) even when no handler
    /// references that state.
    pub fn synced_with_storage<S: State + Default>(
        render: Renderer<S>,
        deps: RendererDeps,
    ) -> Self {
        let table_name = if S::TABLE_NAME.is_empty() {
            None
        } else {
            Some(S::TABLE_NAME)
        };
        Self::synced_from(Box::new(SyncedRendererImpl {
            render,
            deps,
            storage_type: S::STORAGE_TYPE,
            table_name,
        }))
    }

    /// Wrap a boxed synced renderer in a placeholder element.
    pub(crate) fn synced_from(synced: Box<dyn SyncedRenderer>) -> Self {
        Self {
            el_type: El::Div, // Placeholder, will be replaced by rendered content
            text: None,
            class: None,
            attrs: Vec::new(),
            typed_attrs: Vec::new(),
            events: Vec::new(),
            key: None,
            children: Vec::new(),
            synced: Some(synced),
            style_utils: Vec::new(),
            style_props: Vec::new(),
            pseudo_groups: Vec::new(),
            breakpoint_groups: Vec::new(),
            target_bindings: Vec::new(),
            target_toggles: Vec::new(),
            selector_bindings: Vec::new(),
            selector_sets: Vec::new(),
            timed_toggles: Vec::new(),
            auto_toggles: Vec::new(),
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

    /// Set the `id` attribute on this element.
    pub fn id(self, id: &str) -> Self {
        self.attr("id", id)
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

    /// Set a typed attribute with enum key + enum value.
    ///
    /// Uses binary encoding: 4 bytes on wire (SET_ATTR_ENUM opcode).
    /// Much more compact than string-based `.attr()` for common attributes.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rwire::{el, El, At, Av};
    ///
    /// el(El::Button).at(At::Type, Av::Button)
    /// el(El::Input).at(At::Type, Av::Email)
    /// ```
    pub fn at(mut self, key: At, value: Av) -> Self {
        self.typed_attrs.push(TypedAttr::Enum(key, value));
        self
    }

    /// Set a boolean attribute (presence-only, no value).
    ///
    /// Uses binary encoding: 3 bytes on wire (SET_ATTR_BOOL opcode).
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rwire::{el, El, At};
    ///
    /// el(El::Button).bool_attr(At::Disabled)
    /// el(El::Input).bool_attr(At::Required)
    /// ```
    pub fn bool_attr(mut self, key: At) -> Self {
        self.typed_attrs.push(TypedAttr::Bool(key));
        self
    }

    /// Set a typed attribute with enum key + string value.
    ///
    /// Uses binary encoding: 4-5 bytes on wire (SET_ATTR_KEY_SYM opcode).
    /// The key is a binary enum, the value goes through the symbol table.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rwire::{el, El, At};
    ///
    /// el(El::Button).at_str(At::AriaLabel, "Close dialog")
    /// el(El::Path).at_str(At::D, "M6 9l6 6 6-6")
    /// ```
    pub fn at_str(mut self, key: At, value: &str) -> Self {
        self.typed_attrs
            .push(TypedAttr::KeySym(key, value.to_string()));
        self
    }

    /// Set inline style on this element, merging with any existing inline style.
    ///
    /// A second `.style(...)` augments the first rather than replacing it, so a
    /// selection ring layered onto a card that already set `flex`/`background`
    /// keeps both (later declarations win per CSS property).
    pub fn style(mut self, style: crate::style::Style) -> Self {
        let css = style.to_css();
        if css.is_empty() {
            return self;
        }
        if let Some((_, existing)) = self.attrs.iter_mut().find(|(k, _)| k == "style") {
            if !existing.is_empty() && !existing.trim_end().ends_with(';') {
                existing.push(';');
            }
            existing.push_str(&css);
            self
        } else {
            self.attr("style", &css)
        }
    }

    /// Collect every utility/pseudo/breakpoint `StyleKey` this tree references, so a
    /// static page can emit their CSS rules up front (the live capsule delivers them
    /// lazily over the wire instead). Recurses into children.
    pub fn collect_style_keys(&self, keys: &mut BTreeSet<StyleKey>) {
        for &code in &self.style_utils {
            keys.insert(StyleKey::Util(code));
        }
        for (pc, codes) in &self.pseudo_groups {
            for &st in codes {
                keys.insert(StyleKey::Pseudo(*pc, st));
            }
        }
        for (bp, codes) in &self.breakpoint_groups {
            for &st in codes {
                keys.insert(StyleKey::Breakpoint(*bp, st));
            }
        }
        for child in &self.children {
            child.collect_style_keys(keys);
        }
    }

    /// Serialize this element tree to a self-contained HTML string, for pages
    /// rendered *before* the WebSocket capsule exists (e.g. the auth login).
    ///
    /// Structure, text, attributes, and inline styles (`.style(..)` -> the `style`
    /// attribute) are emitted directly. `.st(..)` utility tokens become `u<code>`
    /// classes (and pseudo/breakpoint variants); pair this with
    /// `capsule_gen::render_static_page`, which inlines the matching CSS so the
    /// tokens resolve. Event handlers, synced renderers, and client-action bindings
    /// are runtime-only and omitted.
    pub fn to_static_html(&self) -> String {
        let mut out = String::new();
        self.write_static_html(&mut out);
        out
    }

    fn write_static_html(&self, out: &mut String) {
        let tag = self.el_type.name();
        out.push('<');
        out.push_str(tag);

        let mut classes: Vec<String> = Vec::new();
        if let Some(c) = &self.class {
            if !c.is_empty() {
                classes.push(c.clone());
            }
        }
        for u in &self.style_utils {
            classes.push(format!("u{u}"));
        }
        for (pc, codes) in &self.pseudo_groups {
            for u in codes {
                classes.push(format!("h{pc}u{u}"));
            }
        }
        for (bp, codes) in &self.breakpoint_groups {
            for u in codes {
                classes.push(format!("b{bp}u{u}"));
            }
        }
        if !classes.is_empty() {
            out.push_str(" class=\"");
            push_attr_escaped(out, &classes.join(" "));
            out.push('"');
        }

        for (key, value) in &self.attrs {
            out.push(' ');
            out.push_str(key);
            out.push_str("=\"");
            push_attr_escaped(out, value);
            out.push('"');
        }
        for ta in &self.typed_attrs {
            match ta {
                TypedAttr::Enum(at, av) => {
                    out.push(' ');
                    out.push_str(at.name());
                    out.push_str("=\"");
                    push_attr_escaped(out, av.value());
                    out.push('"');
                }
                TypedAttr::KeySym(at, value) => {
                    out.push(' ');
                    out.push_str(at.name());
                    out.push_str("=\"");
                    push_attr_escaped(out, value);
                    out.push('"');
                }
                TypedAttr::Bool(at) => {
                    out.push(' ');
                    out.push_str(at.name());
                }
            }
        }
        out.push('>');

        if is_void_element(tag) {
            return;
        }

        if let Some(text) = &self.text {
            push_text_escaped(out, text);
        }
        for child in &self.children {
            child.write_static_html(out);
        }

        out.push_str("</");
        out.push_str(tag);
        out.push('>');
    }

    /// Apply a binary-encoded style utility token.
    ///
    /// Style utilities are pre-combined property+value pairs encoded as single bytes.
    /// This is more compact than string-based styles for common patterns.
    ///
    /// Wire format: [STYLE_UTIL, ref, util_byte] = 3 bytes
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rwire::St;
    ///
    /// el(El::Div).st([St::BgApp, St::FlexCenter])
    /// ```
    pub fn style_util(mut self, util: crate::style_tokens::St) -> Self {
        self.style_utils.push(util.as_u16());
        self
    }

    /// Apply a binary-encoded style property+value pair.
    ///
    /// More flexible than utility tokens, allowing any property+value combination
    /// from the predefined sets.
    ///
    /// Wire format: [STYLE_PROP, ref, prop_byte, value_byte] = 4 bytes
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rwire::style_tokens::{StyleProp, StyleValue};
    ///
    /// el(El::Div)
    ///     .style_prop(StyleProp::Gap, StyleValue::Space4)
    ///     .style_prop(StyleProp::Padding, StyleValue::Space2)
    /// ```
    pub fn style_prop(
        mut self,
        prop: crate::style_tokens::StyleProp,
        value: crate::style_tokens::StyleValue,
    ) -> Self {
        self.style_props.push((prop.as_u8(), value.as_u8()));
        self
    }

    /// Apply style utilities (short form).
    ///
    /// The preferred way to apply typed styles to elements.
    /// Uses semantic tokens that adapt to light/dark theme.
    ///
    /// Wire format: [STYLE_MULTI, ref, count, util1, util2, ...] = 3+n bytes
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rwire::{el, El, St};
    ///
    /// el(El::Div).st([
    ///     St::BgApp,
    ///     St::MinHScreen,
    ///     St::FlexCenter,
    /// ])
    /// ```
    pub fn st<I>(mut self, utils: I) -> Self
    where
        I: IntoIterator<Item = crate::style_tokens::St>,
    {
        self.style_utils
            .extend(utils.into_iter().map(|u| u.as_u16()));
        self
    }

    /// Apply pseudo-class style tokens (hover, focus, disabled, etc.)
    ///
    /// Pseudo tokens generate CSS class rules with pseudo-selectors.
    /// Unlike `.st()` which sets base visual styles, `.ps()` handles
    /// interactive state changes.
    ///
    /// # Example
    ///
    /// ```ignore
    /// el(El::Button)
    ///     .st([St::BgAccent, St::TextOnAccent])
    ///     .hover([St::BgAccentHover])
    ///     .focus_visible([St::OutlineAccent, St::OutlineOffset2])
    /// ```
    pub fn pseudo<I>(mut self, pc: crate::style_tokens::Pc, tokens: I) -> Self
    where
        I: IntoIterator<Item = crate::style_tokens::St>,
    {
        let st_codes: Vec<u16> = tokens.into_iter().map(|s| s.as_u16()).collect();
        if !st_codes.is_empty() {
            self.pseudo_groups.push((pc.as_u8(), st_codes));
        }
        self
    }

    /// Apply hover styles.
    pub fn hover<I: IntoIterator<Item = crate::style_tokens::St>>(self, tokens: I) -> Self {
        self.pseudo(crate::style_tokens::Pc::Hover, tokens)
    }

    /// Apply focus styles.
    pub fn focus<I: IntoIterator<Item = crate::style_tokens::St>>(self, tokens: I) -> Self {
        self.pseudo(crate::style_tokens::Pc::Focus, tokens)
    }

    /// Apply focus-visible styles.
    pub fn focus_visible<I: IntoIterator<Item = crate::style_tokens::St>>(self, tokens: I) -> Self {
        self.pseudo(crate::style_tokens::Pc::FocusVisible, tokens)
    }

    /// Apply active styles.
    pub fn active<I: IntoIterator<Item = crate::style_tokens::St>>(self, tokens: I) -> Self {
        self.pseudo(crate::style_tokens::Pc::Active, tokens)
    }

    /// Apply disabled styles.
    pub fn disabled_style<I: IntoIterator<Item = crate::style_tokens::St>>(
        self,
        tokens: I,
    ) -> Self {
        self.pseudo(crate::style_tokens::Pc::Disabled, tokens)
    }

    /// Apply checked styles.
    pub fn checked<I: IntoIterator<Item = crate::style_tokens::St>>(self, tokens: I) -> Self {
        self.pseudo(crate::style_tokens::Pc::Checked, tokens)
    }

    /// Apply placeholder styles.
    pub fn placeholder_style<I: IntoIterator<Item = crate::style_tokens::St>>(
        self,
        tokens: I,
    ) -> Self {
        self.pseudo(crate::style_tokens::Pc::Placeholder, tokens)
    }

    /// Apply ::after pseudo-element styles.
    pub fn after<I: IntoIterator<Item = crate::style_tokens::St>>(self, tokens: I) -> Self {
        self.pseudo(crate::style_tokens::Pc::After, tokens)
    }

    /// Apply ::before pseudo-element styles.
    pub fn before<I: IntoIterator<Item = crate::style_tokens::St>>(self, tokens: I) -> Self {
        self.pseudo(crate::style_tokens::Pc::Before, tokens)
    }

    /// Apply :last-child styles.
    pub fn last_child<I: IntoIterator<Item = crate::style_tokens::St>>(self, tokens: I) -> Self {
        self.pseudo(crate::style_tokens::Pc::LastChild, tokens)
    }

    /// Apply :nth-child(even) styles.
    pub fn nth_even<I: IntoIterator<Item = crate::style_tokens::St>>(self, tokens: I) -> Self {
        self.pseudo(crate::style_tokens::Pc::NthEven, tokens)
    }

    /// Apply :not(:last-child) styles.
    pub fn not_last_child<I: IntoIterator<Item = crate::style_tokens::St>>(
        self,
        tokens: I,
    ) -> Self {
        self.pseudo(crate::style_tokens::Pc::NotLastChild, tokens)
    }

    /// Apply :checked::after styles.
    pub fn checked_after<I: IntoIterator<Item = crate::style_tokens::St>>(self, tokens: I) -> Self {
        self.pseudo(crate::style_tokens::Pc::CheckedAfter, tokens)
    }

    /// Get the pseudo-class groups.
    pub fn get_pseudo_groups(&self) -> &[(u8, Vec<u16>)] {
        &self.pseudo_groups
    }

    /// Apply responsive breakpoint style tokens (mobile-first, min-width).
    ///
    /// Breakpoint tokens generate `@media(min-width:...)` CSS rules.
    /// Styles are applied at the specified breakpoint and above.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rwire::{el, El, St};
    /// use rwire::style_tokens::Bp;
    ///
    /// el(El::Div)
    ///     .st([St::FlexCol, St::GapMd])       // mobile: column layout
    ///     .md([St::FlexRow])                    // 768px+: row layout
    ///     .lg([St::GridCols3])                  // 1024px+: 3-column grid
    /// ```
    pub fn breakpoint<I>(mut self, bp: crate::style_tokens::Bp, tokens: I) -> Self
    where
        I: IntoIterator<Item = crate::style_tokens::St>,
    {
        let st_codes: Vec<u16> = tokens.into_iter().map(|s| s.as_u16()).collect();
        if !st_codes.is_empty() {
            self.breakpoint_groups.push((bp.as_u8(), st_codes));
        }
        self
    }

    /// Apply styles at the `sm` breakpoint (640px+).
    pub fn sm<I: IntoIterator<Item = crate::style_tokens::St>>(self, tokens: I) -> Self {
        self.breakpoint(crate::style_tokens::Bp::Sm, tokens)
    }

    /// Apply styles at the `md` breakpoint (768px+).
    pub fn md<I: IntoIterator<Item = crate::style_tokens::St>>(self, tokens: I) -> Self {
        self.breakpoint(crate::style_tokens::Bp::Md, tokens)
    }

    /// Apply styles at the `lg` breakpoint (1024px+).
    pub fn lg<I: IntoIterator<Item = crate::style_tokens::St>>(self, tokens: I) -> Self {
        self.breakpoint(crate::style_tokens::Bp::Lg, tokens)
    }

    /// Apply styles at the `xl` breakpoint (1280px+).
    pub fn xl<I: IntoIterator<Item = crate::style_tokens::St>>(self, tokens: I) -> Self {
        self.breakpoint(crate::style_tokens::Bp::Xl, tokens)
    }

    /// Get the breakpoint groups.
    pub fn get_breakpoint_groups(&self) -> &[(u8, Vec<u16>)] {
        &self.breakpoint_groups
    }

    // ========================================================================
    // Client Actions (Targets & Selectors)
    // ========================================================================

    /// Bind a CSS class to a Target's boolean state.
    ///
    /// When the target is `true`, adds the `St` class to this element.
    /// Use with `.st([St::DisplayNone])` for hide-by-default patterns.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[derive(Target)]
    /// struct ModalOpen;
    ///
    /// el(El::Div)
    ///     .when::<ModalOpen>(St::DisplayFlex)
    ///     .st([St::DisplayNone])  // hidden by default
    /// ```
    pub fn when<F: Target>(mut self, st: crate::style_tokens::St) -> Self {
        self.target_bindings.push(TargetBinding {
            type_id: TypeId::of::<F>(),
            st: st.as_u16(),
            invert: false,
            default: F::default_value(),
        });
        self
    }

    /// Bind a CSS class to a Target's inverse state.
    ///
    /// When the target is `false`, adds the `St` class to this element.
    ///
    /// # Example
    ///
    /// ```ignore
    /// el(El::Div).unless::<SidebarOpen>(St::DisplayNone)
    /// ```
    pub fn unless<F: Target>(mut self, st: crate::style_tokens::St) -> Self {
        self.target_bindings.push(TargetBinding {
            type_id: TypeId::of::<F>(),
            st: st.as_u16(),
            invert: true,
            default: F::default_value(),
        });
        self
    }

    /// Toggle a Target on an event (e.g., click toggles modal open/close).
    ///
    /// # Example
    ///
    /// ```ignore
    /// el(El::Button)
    ///     .text("Open Modal")
    ///     .toggle::<ModalOpen>(Ev::Click)
    /// ```
    pub fn toggle<F: Target>(mut self, ev: Ev) -> Self {
        self.target_toggles.push(TargetToggle {
            ev,
            type_id: TypeId::of::<F>(),
        });
        self
    }

    /// Bind a CSS class to a Selector variant match.
    ///
    /// When the selector's value equals this variant, adds the `St` class.
    ///
    /// # Example
    ///
    /// ```ignore
    /// el(El::Div).when_eq(ActiveTab::Home, St::DisplayBlock)
    /// ```
    pub fn when_eq<S: Selector>(mut self, variant: S, st: crate::style_tokens::St) -> Self {
        self.selector_bindings.push(SelectorBinding {
            type_id: TypeId::of::<S>(),
            match_val: variant.variant_value(),
            st: st.as_u16(),
            default_val: S::default_value(),
        });
        self
    }

    /// Set a Selector value on an event (e.g., click activates a tab).
    ///
    /// # Example
    ///
    /// ```ignore
    /// el(El::Button).select(ActiveTab::Settings, Ev::Click)
    /// ```
    pub fn select<S: Selector>(mut self, variant: S, ev: Ev) -> Self {
        self.selector_sets.push(SelectorSet {
            ev,
            type_id: TypeId::of::<S>(),
            val: variant.variant_value(),
        });
        self
    }

    /// Timed toggle: on event, set target to true, then revert to false after delay.
    /// Repeated events restart the timer.
    ///
    /// # Example
    ///
    /// ```ignore
    /// el(El::Button).toggle_timed::<CopyFeedback>(Ev::Click, 2000)
    /// ```
    pub fn toggle_timed<F: Target>(mut self, ev: Ev, delay_ms: u16) -> Self {
        self.timed_toggles.push(TimedTargetToggle {
            ev,
            type_id: TypeId::of::<F>(),
            delay_ms,
        });
        self
    }

    /// Auto toggle: flip target boolean once after delay from mount.
    ///
    /// # Example
    ///
    /// ```ignore
    /// el(El::Div)
    ///     .when::<ToastDismiss>(St::DisplayNone)
    ///     .auto_toggle::<ToastDismiss>(5000)
    /// ```
    pub fn auto_toggle<F: Target>(mut self, delay_ms: u16) -> Self {
        self.auto_toggles.push(AutoToggle {
            type_id: TypeId::of::<F>(),
            delay_ms,
        });
        self
    }

    /// Bind an event handler to this element.
    ///
    /// The handler function will be called when the event occurs.
    /// The event triggers a server round-trip where the handler runs.
    pub fn on(mut self, ev: Ev, handler: HandlerSpec) -> Self {
        self.events.push((ev, handler));
        self
    }

    /// Bind a debounced event handler to this element.
    ///
    /// The handler will only fire after `delay_ms` milliseconds of inactivity.
    /// Useful for search inputs to avoid sending a server request on every keystroke.
    pub fn on_debounced(mut self, ev: Ev, handler: HandlerSpec, delay_ms: u16) -> Self {
        let mut h = handler;
        h.debounce_ms = delay_ms;
        self.events.push((ev, h));
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

    /// Bind a one-shot visibility sentinel to this element (infinite scroll /
    /// content streaming). When the element nears the viewport, `handler`
    /// fires once with `next` as its param — read it via `ctx.item_index()`
    /// and ignore stale values. Each render must pass the new `next`, which
    /// re-keys the binding so the morph installs a fresh observer; one
    /// request in flight is therefore structural, not a convention.
    ///
    /// ```ignore
    /// el(El::Div).on_visible(load_more(), state.delivered as u32)
    /// ```
    pub fn on_visible(mut self, handler: HandlerSpec, next: u32) -> Self {
        let mut param_bytes = Vec::new();
        crate::item_ref::ItemRef::<()>::new(next as usize).encode(&mut param_bytes);
        let handler_with_params = handler.with_param_bytes(param_bytes);
        self.events.push((Ev::Visible, handler_with_params));
        self
    }

    /// Give this element a stable identity among its siblings, so list
    /// reorders morph by identity instead of positionally — the moved DOM
    /// nodes (with their input values, scroll, and focus) travel with their
    /// items. Strings hash (FNV-1a, 32-bit); integers are used directly. Use
    /// your domain id (`todo.id`, message id), NOT the list index — an index
    /// is exactly the positional identity keying exists to replace.
    ///
    /// ```ignore
    /// state.items.iter_with_ref().map(|(item_ref, item)| {
    ///     el(El::Li).key(item.id).text(&item.text)
    /// })
    /// ```
    pub fn key<K: ElementKey>(mut self, key: K) -> Self {
        self.key = Some(key.key_code());
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

    /// Get the style utility tokens.
    pub fn get_style_utils(&self) -> &[u16] {
        &self.style_utils
    }

    /// Get the style property tokens.
    pub fn get_style_props(&self) -> &[(u8, u8)] {
        &self.style_props
    }

    /// Compute a content hash of the element tree for render dedup.
    ///
    /// Hashes the deterministic visual content: element type, class, text,
    /// attributes, style tokens, event types, and children (recursive).
    /// Skips: synced renderer boxes, handler closures.
    pub fn content_hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash_content(&mut hasher);
        hasher.finish()
    }

    /// Hash the visual content of this element into the given hasher.
    fn hash_content(&self, hasher: &mut impl Hasher) {
        self.el_type.as_u8().hash(hasher);
        self.class.hash(hasher);
        self.text.hash(hasher);
        self.attrs.hash(hasher);
        // Hash typed attrs
        for ta in &self.typed_attrs {
            match ta {
                TypedAttr::Enum(k, v) => {
                    0u8.hash(hasher);
                    k.as_u8().hash(hasher);
                    v.as_u8().hash(hasher);
                }
                TypedAttr::Bool(k) => {
                    1u8.hash(hasher);
                    k.as_u8().hash(hasher);
                }
                TypedAttr::KeySym(k, v) => {
                    2u8.hash(hasher);
                    k.as_u8().hash(hasher);
                    v.hash(hasher);
                }
            }
        }
        self.style_utils.hash(hasher);
        self.style_props.hash(hasher);
        self.pseudo_groups.hash(hasher);
        self.breakpoint_groups.hash(hasher);
        for (ev, _) in &self.events {
            ev.as_u8().hash(hasher);
        }
        // Hash target/selector bindings
        for tb in &self.target_bindings {
            tb.type_id.hash(hasher);
            tb.st.hash(hasher);
            tb.invert.hash(hasher);
        }
        for tt in &self.target_toggles {
            tt.ev.as_u8().hash(hasher);
            tt.type_id.hash(hasher);
        }
        for sb in &self.selector_bindings {
            sb.type_id.hash(hasher);
            sb.match_val.hash(hasher);
            sb.st.hash(hasher);
        }
        for ss in &self.selector_sets {
            ss.ev.as_u8().hash(hasher);
            ss.type_id.hash(hasher);
            ss.val.hash(hasher);
        }
        for tt in &self.timed_toggles {
            tt.ev.as_u8().hash(hasher);
            tt.type_id.hash(hasher);
            tt.delay_ms.hash(hasher);
        }
        for at in &self.auto_toggles {
            at.type_id.hash(hasher);
            at.delay_ms.hash(hasher);
        }
        for child in &self.children {
            child.hash_content(hasher);
        }
    }
}

/// Represents how a text string should be encoded.
#[derive(Clone, Debug)]
pub enum TextEncoding {
    /// Use symbol table (traditional approach)
    Symbol(u32),
    /// Use word indices (space-separated words)
    Words(Vec<u8>),
    /// Use integer encoding (for pure numeric strings)
    Int(i32),
}

/// Context for building the DOM tree with state support.
pub struct BuildContext {
    buf: OpcodeBuffer,
    symbols: Vec<String>,
    symbol_map: HashMap<String, u32>,
    /// Remote handlers keyed by stable handler id (see [`crate::stable_handler_id`]).
    handlers: HashMap<u32, HandlerFn>,
    synced_elements: Vec<SyncedElement>,
    next_synced_id: u32,
    /// The synced region currently being emitted into, so nested regions record their
    /// owning parent (used to prune a swapped-out view's regions). Saved/restored
    /// around each synced region's content emission.
    current_synced_parent: Option<u32>,
    /// Word frequency counts (built during collect_symbols)
    word_counts: HashMap<String, usize>,
    /// Word table: word -> index (built after collect_symbols, before emit)
    word_indices: HashMap<String, u8>,
    /// Ordered word list (most frequent first)
    words: Vec<String>,
    /// Text encoding decisions (text -> encoding)
    text_encodings: HashMap<String, TextEncoding>,
    /// Composite style table (pre-analyzed patterns for compression)
    composite_table: crate::style_groups::CompositeTable,
    /// Cache for synced element renders (single-render path).
    /// Populated during collect_symbols, consumed during emit.
    /// This avoids rendering synced elements twice, which would produce
    /// different generate_element_id() values between passes.
    synced_render_cache: HashMap<u32, ElementBuilder>,
    /// Mapping from target TypeId to target index (u8)
    target_indices: HashMap<TypeId, u8>,
    /// Next available target index
    next_target_idx: u8,
    /// Mapping from selector TypeId to selector index (u8)
    selector_indices: HashMap<TypeId, u8>,
    /// Next available selector index
    next_selector_idx: u8,
    /// Whether any targets or selectors are used (for capsule tree-shaking)
    has_client_actions: bool,
    /// Target defaults: target_idx → default_value (for INIT_TARGET emission)
    target_defaults: HashMap<u8, bool>,
    /// Selector defaults: selector_idx → default_value (for INIT_SELECTOR emission)
    selector_defaults: HashMap<u8, u8>,
}

/// Information about a synced element for later updates.
pub struct SyncedElement {
    /// Unique ID for this synced element (used in __synced_N wrapper IDs).
    pub id: u32,
    pub(crate) renderer: Box<dyn SyncedRenderer>,
    pub(crate) state_type_id: TypeId,
    /// Dependency information for fine-grained updates.
    pub deps: RendererDeps,
    /// The enclosing synced region's id, or `None` for a top-level region. Lets a
    /// region's whole subtree be pruned when its parent swaps it out (router outlet).
    pub(crate) parent: Option<u32>,
}

impl Clone for SyncedElement {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            renderer: self.renderer.clone_box(),
            state_type_id: self.state_type_id,
            deps: self.deps,
            parent: self.parent,
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
            parent: None,
        }
    }

    /// Get the TypeId of the state type this element renders from.
    pub fn state_type_id(&self) -> TypeId {
        self.state_type_id
    }

    /// The enclosing synced region's id, or `None` for a top-level region.
    pub fn parent(&self) -> Option<u32> {
        self.parent
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
            handlers: HashMap::new(),
            synced_elements: Vec::new(),
            next_synced_id: 0,
            current_synced_parent: None,
            word_counts: HashMap::new(),
            word_indices: HashMap::new(),
            words: Vec::new(),
            text_encodings: HashMap::new(),
            composite_table: crate::style_groups::CompositeTable::new(),
            synced_render_cache: HashMap::new(),
            target_indices: HashMap::new(),
            next_target_idx: 0,
            selector_indices: HashMap::new(),
            next_selector_idx: 0,
            has_client_actions: false,
            target_defaults: HashMap::new(),
            selector_defaults: HashMap::new(),
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
        word_freq.sort_by_key(|entry| std::cmp::Reverse(entry.1));

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

    /// Analyze style patterns in the element tree and build composite table.
    ///
    /// Call this after collect_symbols and before emit for optimal compression.
    pub fn analyze_style_patterns(&mut self, root: &ElementBuilder) {
        let collector = crate::style_groups::collect_patterns(root);
        let analysis = crate::style_groups::analyze_patterns(&collector);
        self.composite_table = analysis.composite_table;
    }

    /// Get the composite table for capsule generation.
    pub fn composite_table(&self) -> &crate::style_groups::CompositeTable {
        &self.composite_table
    }

    /// Set the composite table directly (reuse startup table for per-connection rendering).
    pub fn set_composite_table(&mut self, table: crate::style_groups::CompositeTable) {
        self.composite_table = table;
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

    /// Maximum number of unique symbols per render pass.
    /// Varint supports more, but this prevents unbounded memory growth from buggy renderers.
    const MAX_SYMBOLS: usize = 16_384;

    fn intern(&mut self, s: &str) -> u32 {
        if let Some(&idx) = self.symbol_map.get(s) {
            return idx;
        }
        assert!(
            self.symbols.len() < Self::MAX_SYMBOLS,
            "symbol table overflow: exceeded {} unique symbols",
            Self::MAX_SYMBOLS
        );
        // Symbol indices start at 0x80 and can grow with varint encoding
        let idx = 0x80 + self.symbols.len() as u32;
        self.symbols.push(s.to_string());
        self.symbol_map.insert(s.to_string(), idx);
        idx
    }

    /// Get a symbol from the map, or intern it if not found.
    /// This handles dynamically generated strings that weren't collected during collect_symbols.
    fn get_or_intern_symbol(&mut self, s: &str) -> u32 {
        if let Some(&idx) = self.symbol_map.get(s) {
            idx
        } else {
            // Symbol wasn't collected - intern it now
            // This can happen with dynamically generated IDs
            self.intern(s)
        }
    }

    /// Register a handler under its stable id (idempotent), returning the id
    /// to emit in the bind opcode.
    fn register_remote_handler(&mut self, spec: &HandlerSpec, handler: &HandlerFn) -> u32 {
        let id = wire_handler_id(spec, handler);
        self.handlers.entry(id).or_insert_with(|| handler.clone());
        id
    }

    /// Collect all symbols from an element tree (first pass).
    /// Also tracks used element and event types for tree shaking.
    ///
    /// Note: Synced element IDs are no longer interned as symbols - they use
    /// dedicated CREATE_SYNCED/GET_SYNCED opcodes with varint encoding.
    pub fn collect_symbols(&mut self, el: &ElementBuilder, state: &dyn Any) {
        // If this is a synced element, render it first with state
        if let Some(renderer) = &el.synced {
            if let Some(rendered) = renderer.render_with_state(state) {
                self.collect_symbols(&rendered, state);
            }
            return;
        }

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
        // Intern string values in typed attrs (Enum/Bool need no interning)
        for ta in &el.typed_attrs {
            if let TypedAttr::KeySym(_, value) = ta {
                self.intern(value);
            }
        }
        // Register target/selector types and client actions
        self.register_client_actions(el);
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
        // If this is a synced element, render it once and cache for emit pass
        if let Some(renderer) = &el.synced {
            let synced_id = self.next_synced_id;
            self.next_synced_id += 1;

            // Render once, cache result for emit pass (single-render path)
            let state_type_id = renderer.state_type_id();
            if let Some(state) = states.get(&state_type_id) {
                if let Some(rendered) = renderer.render_with_state(*state) {
                    self.collect_symbols_multi(&rendered, states);
                    self.synced_render_cache.insert(synced_id, rendered);
                }
            }
            return;
        }

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
        // Intern string values in typed attrs (Enum/Bool need no interning)
        for ta in &el.typed_attrs {
            if let TypedAttr::KeySym(_, value) = ta {
                self.intern(value);
            }
        }
        // Register target/selector types and client actions
        self.register_client_actions(el);
        // Process children
        for child in &el.children {
            self.collect_symbols_multi(child, states);
        }
    }

    /// Emit opcodes for an element tree (second pass).
    pub fn emit(&mut self, el: &ElementBuilder, state: &dyn Any) -> u32 {
        // Reset synced_id counter - we increment again during emit
        self.next_synced_id = 0;

        // Build word table and decide text encodings
        self.build_word_table();

        // Emit symbol table first (only on first call)
        if !self.symbols.is_empty() {
            self.buf.begin_symbols(self.symbols.len() as u32);
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

        // Emit composite table if we have composites
        if !self.composite_table.is_empty() {
            self.buf.composite_table(&self.composite_table);
        }

        // Emit INIT_TARGET and INIT_SELECTOR for all registered types
        if self.has_client_actions {
            self.emit_client_action_inits();
        }

        self.emit_element(el, None, state)
    }

    /// Emit opcodes for an element tree with multi-state support.
    pub fn emit_multi(&mut self, el: &ElementBuilder) -> u32 {
        // Reset synced_id counter - we increment again during emit
        self.next_synced_id = 0;

        // Build word table and decide text encodings
        self.build_word_table();

        // Emit symbol table first (only on first call)
        if !self.symbols.is_empty() {
            self.buf.begin_symbols(self.symbols.len() as u32);
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

        // Emit composite table if we have composites
        if !self.composite_table.is_empty() {
            self.buf.composite_table(&self.composite_table);
        }

        // Emit INIT_TARGET and INIT_SELECTOR for all registered types
        if self.has_client_actions {
            self.emit_client_action_inits();
        }

        self.emit_element_multi(el, None)
    }

    /// Emit text content using the best encoding.
    fn emit_text(&mut self, ref_idx: u32, text: &str) {
        match self.text_encodings.get(text) {
            Some(TextEncoding::Int(n)) => {
                self.buf.set_text_int(ref_idx, *n);
            }
            Some(TextEncoding::Words(indices)) => {
                self.buf.set_text_words(ref_idx, indices);
            }
            _ => {
                // Fall back to symbol encoding
                let sym = self.get_or_intern_symbol(text);
                self.buf.set_text(ref_idx, sym);
            }
        }
    }

    fn emit_element(
        &mut self,
        el: &ElementBuilder,
        parent_ref: Option<u32>,
        state: &dyn Any,
    ) -> u32 {
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
                parent: self.current_synced_parent,
            });

            if let Some(rendered) = renderer.render_with_state(state) {
                // Use CREATE_SYNCED opcode - more compact than CREATE span + SET_ATTR id
                let ref_idx = self.buf.create_synced(synced_id);

                // Emit the rendered content as a child, tagging nested regions with this
                // region as their parent.
                let saved_parent = self.current_synced_parent;
                self.current_synced_parent = Some(synced_id);
                self.emit_element(&rendered, Some(ref_idx), state);
                self.current_synced_parent = saved_parent;

                // Append wrapper to parent
                if let Some(parent) = parent_ref {
                    self.buf.append(parent, ref_idx);
                } else {
                    self.buf.append_to_body(ref_idx);
                }

                return ref_idx;
            }
        }

        let ref_idx = self.buf.create(el.el_type.as_u8());

        if let Some(ref class) = el.class {
            let sym = self.get_or_intern_symbol(class);
            self.buf.set_class(ref_idx, sym);
        }

        if let Some(ref text) = el.text {
            self.emit_text(ref_idx, text);
        }

        for (key, value) in &el.attrs {
            let key_sym = self.get_or_intern_symbol(key);
            let val_sym = self.get_or_intern_symbol(value);
            self.buf.set_attr(ref_idx, key_sym, val_sym);
        }

        // Emit typed attributes (binary-encoded)
        for ta in &el.typed_attrs {
            match ta {
                TypedAttr::Enum(key, value) => {
                    self.buf.set_attr_enum(ref_idx, key.as_u8(), value.as_u8());
                }
                TypedAttr::Bool(key) => {
                    self.buf.set_attr_bool(ref_idx, key.as_u8());
                }
                TypedAttr::KeySym(key, value) => {
                    let val_sym = self.get_or_intern_symbol(value);
                    self.buf.set_attr_key_sym(ref_idx, key.as_u8(), val_sym);
                }
            }
        }

        // Emit style tokens (binary-encoded styles)
        if !el.style_utils.is_empty() {
            // Check if this pattern has a composite
            if let Some(composite_id) = self.composite_table.get_composite_id(&el.style_utils) {
                self.buf.style_composite(ref_idx, composite_id);
            } else if el.style_utils.len() >= 3 {
                // Use STYLE_MULTI for 3+ utilities
                self.buf.style_multi(ref_idx, &el.style_utils);
            } else {
                // Individual STYLE_UTIL for 1-2 utilities
                for &util in &el.style_utils {
                    self.buf.style_util(ref_idx, util);
                }
            }
        }

        // Emit style property+value pairs
        for &(prop, value) in &el.style_props {
            self.buf.style_prop(ref_idx, prop, value);
        }

        // Emit pseudo-class groups
        for (pc_code, st_codes) in &el.pseudo_groups {
            self.buf.style_pseudo(ref_idx, *pc_code, st_codes);
        }

        // Emit breakpoint groups
        for (bp_code, st_codes) in &el.breakpoint_groups {
            self.buf.style_breakpoint(ref_idx, *bp_code, st_codes);
        }

        // Emit client action bindings (targets & selectors)
        self.emit_client_action_bindings(ref_idx, el);
        // Morph key for keyed reordering
        if let Some(k) = el.key {
            self.buf.set_key(ref_idx, k);
        }
        // Bind events
        for (ev, handler_spec) in &el.events {
            if let Some(handler) = &handler_spec.remote_handler {
                let handler_idx = self.register_remote_handler(handler_spec, handler);

                if *ev == Ev::Visible {
                    let empty = Vec::new();
                    let params = handler_spec.param_bytes.as_ref().unwrap_or(&empty);
                    self.buf.bind_sentinel(ref_idx, handler_idx, params);
                } else if let Some(param_bytes) = &handler_spec.param_bytes {
                    self.buf
                        .bind_remote_param(ref_idx, ev.as_u8(), handler_idx, param_bytes);
                } else if handler_spec.debounce_ms > 0 {
                    self.buf.bind_debounced(
                        ref_idx,
                        ev.as_u8(),
                        handler_idx,
                        handler_spec.debounce_ms,
                    );
                } else {
                    self.buf.bind_remote(ref_idx, ev.as_u8(), handler_idx);
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
    fn emit_element_multi(&mut self, el: &ElementBuilder, parent_ref: Option<u32>) -> u32 {
        // If this is a synced element, use the cached render from collect_symbols_multi
        if let Some(renderer) = &el.synced {
            let synced_id = self.next_synced_id;
            self.next_synced_id += 1;

            // Store the synced element info for later updates
            self.synced_elements.push(SyncedElement {
                id: synced_id,
                state_type_id: renderer.state_type_id(),
                renderer: renderer.clone_box(),
                deps: renderer.deps(),
                parent: self.current_synced_parent,
            });

            // Use cached render from collect_symbols_multi (single-render path)
            if let Some(rendered) = self.synced_render_cache.remove(&synced_id) {
                let ref_idx = self.buf.create_synced(synced_id);
                let saved_parent = self.current_synced_parent;
                self.current_synced_parent = Some(synced_id);
                self.emit_element_multi(&rendered, Some(ref_idx));
                self.current_synced_parent = saved_parent;

                if let Some(parent) = parent_ref {
                    self.buf.append(parent, ref_idx);
                } else {
                    self.buf.append_to_body(ref_idx);
                }

                return ref_idx;
            }
            return 0;
        }

        let ref_idx = self.buf.create(el.el_type.as_u8());

        if let Some(ref class) = el.class {
            let sym = self.get_or_intern_symbol(class);
            self.buf.set_class(ref_idx, sym);
        }

        if let Some(ref text) = el.text {
            self.emit_text(ref_idx, text);
        }

        for (key, value) in &el.attrs {
            let key_sym = self.get_or_intern_symbol(key);
            let val_sym = self.get_or_intern_symbol(value);
            self.buf.set_attr(ref_idx, key_sym, val_sym);
        }

        // Emit typed attributes (binary-encoded)
        for ta in &el.typed_attrs {
            match ta {
                TypedAttr::Enum(key, value) => {
                    self.buf.set_attr_enum(ref_idx, key.as_u8(), value.as_u8());
                }
                TypedAttr::Bool(key) => {
                    self.buf.set_attr_bool(ref_idx, key.as_u8());
                }
                TypedAttr::KeySym(key, value) => {
                    let val_sym = self.get_or_intern_symbol(value);
                    self.buf.set_attr_key_sym(ref_idx, key.as_u8(), val_sym);
                }
            }
        }

        // Emit style tokens (binary-encoded styles)
        if !el.style_utils.is_empty() {
            // Check if this pattern has a composite
            if let Some(composite_id) = self.composite_table.get_composite_id(&el.style_utils) {
                self.buf.style_composite(ref_idx, composite_id);
            } else if el.style_utils.len() >= 3 {
                // Use STYLE_MULTI for 3+ utilities
                self.buf.style_multi(ref_idx, &el.style_utils);
            } else {
                // Individual STYLE_UTIL for 1-2 utilities
                for &util in &el.style_utils {
                    self.buf.style_util(ref_idx, util);
                }
            }
        }

        // Emit style property+value pairs
        for &(prop, value) in &el.style_props {
            self.buf.style_prop(ref_idx, prop, value);
        }

        // Emit pseudo-class groups
        for (pc_code, st_codes) in &el.pseudo_groups {
            self.buf.style_pseudo(ref_idx, *pc_code, st_codes);
        }

        // Emit breakpoint groups
        for (bp_code, st_codes) in &el.breakpoint_groups {
            self.buf.style_breakpoint(ref_idx, *bp_code, st_codes);
        }

        // Emit client action bindings (targets & selectors)
        self.emit_client_action_bindings(ref_idx, el);
        // Morph key for keyed reordering
        if let Some(k) = el.key {
            self.buf.set_key(ref_idx, k);
        }
        // Bind events
        for (ev, handler_spec) in &el.events {
            if let Some(handler) = &handler_spec.remote_handler {
                let handler_idx = self.register_remote_handler(handler_spec, handler);

                if *ev == Ev::Visible {
                    let empty = Vec::new();
                    let params = handler_spec.param_bytes.as_ref().unwrap_or(&empty);
                    self.buf.bind_sentinel(ref_idx, handler_idx, params);
                } else if let Some(param_bytes) = &handler_spec.param_bytes {
                    self.buf
                        .bind_remote_param(ref_idx, ev.as_u8(), handler_idx, param_bytes);
                } else if handler_spec.debounce_ms > 0 {
                    self.buf.bind_debounced(
                        ref_idx,
                        ev.as_u8(),
                        handler_idx,
                        handler_spec.debounce_ms,
                    );
                } else {
                    self.buf.bind_remote(ref_idx, ev.as_u8(), handler_idx);
                }
            }
        }

        // Emit children
        for child in &el.children {
            self.emit_element_multi(child, Some(ref_idx));
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

    /// The class-referenced style rules emitted so far (for lazy CSS delivery).
    pub fn referenced_styles(&self) -> &BTreeSet<StyleKey> {
        self.buf.referenced_styles()
    }

    /// Finish building, prepending a `STYLE_DEF` block for any class-referenced
    /// rules not yet sent to this connection (`sent`). Used for the initial DOM so
    /// the styles it uses are delivered alongside it. See
    /// `docs/tree-shaking-redesign.md` (Phase 2).
    pub fn finish_with_style_defs(
        mut self,
        sent: &mut HashSet<StyleKey>,
        sent_maps: &mut HashSet<(u8, u8)>,
    ) -> Bytes {
        self.buf.end();
        // Names (MAP_DEF) before CSS (STYLE_DEF); both land before the body that uses them.
        let mut prefix = map_def_prefix(self.buf.referenced_names(), sent_maps);
        prefix.extend_from_slice(&style_def_prefix(self.buf.referenced_styles(), sent));
        let body = self.buf.finish();
        prepend(prefix, body)
    }

    /// Get the registered remote handlers, keyed by stable handler id.
    pub fn handlers(&self) -> &HashMap<u32, HandlerFn> {
        &self.handlers
    }

    /// Take the synced elements.
    pub fn take_synced_elements(&mut self) -> Vec<SyncedElement> {
        std::mem::take(&mut self.synced_elements)
    }

    /// Get the symbol map for tracking sent symbols.
    ///
    /// This returns a clone of the symbol map after rendering, which can be
    /// used to track which symbols were sent to the client for incremental
    /// symbol updates.
    pub fn take_symbol_map(&self) -> HashMap<String, u32> {
        self.symbol_map.clone()
    }

    /// Snapshot the target/selector index assignments made during this render.
    ///
    /// Synced updates re-render regions with a free `emit_update_element` that has
    /// no `BuildContext`, so the connection keeps this snapshot to re-emit
    /// `BIND_TARGET`/`BIND_SELECTOR`/etc against the same client-side slots.
    pub fn client_action_indices(&self) -> ClientActionIndices {
        ClientActionIndices {
            targets: self.target_indices.clone(),
            selectors: self.selector_indices.clone(),
        }
    }

    /// Get or assign a u8 index for a target TypeId.
    fn get_or_create_target_idx(&mut self, type_id: TypeId, default: bool) -> u8 {
        if let Some(&idx) = self.target_indices.get(&type_id) {
            return idx;
        }
        let idx = self.next_target_idx;
        self.next_target_idx += 1;
        self.target_indices.insert(type_id, idx);
        self.target_defaults.insert(idx, default);
        self.has_client_actions = true;
        idx
    }

    /// Get or assign a u8 index for a selector TypeId.
    fn get_or_create_selector_idx(&mut self, type_id: TypeId, default_val: u8) -> u8 {
        if let Some(&idx) = self.selector_indices.get(&type_id) {
            return idx;
        }
        let idx = self.next_selector_idx;
        self.next_selector_idx += 1;
        self.selector_indices.insert(type_id, idx);
        self.selector_defaults.insert(idx, default_val);
        self.has_client_actions = true;
        idx
    }

    /// Register all target/selector types from an element's bindings.
    fn register_client_actions(&mut self, el: &ElementBuilder) {
        for tb in &el.target_bindings {
            self.get_or_create_target_idx(tb.type_id, tb.default);
        }
        for tt in &el.target_toggles {
            // Ensure the target type is registered even if only toggles exist
            self.get_or_create_target_idx(tt.type_id, false);
        }
        for sb in &el.selector_bindings {
            self.get_or_create_selector_idx(sb.type_id, sb.default_val);
        }
        for ss in &el.selector_sets {
            // Ensure the selector type is registered even if only sets exist
            self.get_or_create_selector_idx(ss.type_id, 0);
        }
        for tt in &el.timed_toggles {
            self.get_or_create_target_idx(tt.type_id, false);
        }
        for at in &el.auto_toggles {
            self.get_or_create_target_idx(at.type_id, false);
        }
    }

    /// Emit INIT_TARGET and INIT_SELECTOR opcodes for all registered types.
    /// Called once at the start of the opcode stream.
    fn emit_client_action_inits(&mut self) {
        for (&idx, &default) in &self.target_defaults.clone() {
            self.buf.init_target(idx, default);
        }
        for (&idx, &default) in &self.selector_defaults.clone() {
            self.buf.init_selector(idx, default);
        }
    }

    /// Emit target/selector binding and trigger opcodes for an element.
    fn emit_client_action_bindings(&mut self, ref_idx: u32, el: &ElementBuilder) {
        for tb in &el.target_bindings {
            let idx = self.target_indices[&tb.type_id];
            self.buf.bind_target(ref_idx, idx, tb.st, tb.invert);
        }
        for tt in &el.target_toggles {
            let idx = self.target_indices[&tt.type_id];
            self.buf.bind_toggle(ref_idx, tt.ev.as_u8(), idx);
        }
        for sb in &el.selector_bindings {
            let idx = self.selector_indices[&sb.type_id];
            self.buf.bind_selector(ref_idx, idx, sb.match_val, sb.st);
        }
        for ss in &el.selector_sets {
            let idx = self.selector_indices[&ss.type_id];
            self.buf.bind_select(ref_idx, ss.ev.as_u8(), idx, ss.val);
        }
        for tt in &el.timed_toggles {
            let idx = self.target_indices[&tt.type_id];
            self.buf
                .bind_timed_toggle(ref_idx, tt.ev.as_u8(), idx, tt.delay_ms);
        }
        for at in &el.auto_toggles {
            let idx = self.target_indices[&at.type_id];
            self.buf.auto_toggle(idx, at.delay_ms);
        }
    }
}

impl Default for BuildContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Target/selector index assignments captured from the initial render.
///
/// Client-side actions (`.target()`, `.toggle()`, `.selector()`, `.timed_toggle()`,
/// auto-toggles) reference a per-`TypeId` `u8` slot that is `INIT`-ed once in the
/// initial DOM. A synced re-render regenerates the elements carrying those
/// bindings, so the connection holds this snapshot to re-emit the per-element
/// `BIND_*` opcodes against the same slots. (The `INIT_*` opcodes are not
/// repeated — the client retains the slots from initial render.)
#[derive(Default, Clone)]
pub struct ClientActionIndices {
    /// TypeId → target slot index.
    pub targets: HashMap<TypeId, u8>,
    /// TypeId → selector slot index.
    pub selectors: HashMap<TypeId, u8>,
}

/// Type alias for the optional per-connection set of CSS rules already delivered.
pub type SentCss<'a> = Option<&'a mut HashSet<StyleKey>>;

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
    handlers: &mut std::collections::HashMap<u32, HandlerFn>,
    changes: ChangeSet,
) -> Bytes {
    build_synced_update_with_known_symbols(
        synced, states, handlers, changes, None, None, None, None, None, None, 0, None,
    )
}

/// Build an update for synced elements with incremental symbol support.
///
/// This is the same as `build_synced_update_multi` but supports tracking which
/// symbols have already been sent to the client. If `known_symbols` is provided:
/// - Only new symbols are sent using SYMBOLS_EXTEND
/// - Known symbols use their existing indices
/// - `known_symbols` is updated with any new symbols after this call
///
/// **TypeId filtering**: If `changed_state_type_id` is provided, only synced elements
/// bound to that state type are re-rendered. This skips irrelevant renderers when
/// a handler modifies only one state type.
///
/// **Render hash dedup**: If `prev_hashes` is provided, the content hash of each
/// rendered element is compared with the previous hash. If identical, the element
/// is skipped (no opcodes emitted). This avoids redundant DOM updates.
///
/// This can reduce update message sizes by 50-90% for repeated updates.
#[allow(clippy::too_many_arguments)]
pub fn build_synced_update_with_known_symbols(
    synced: &[SyncedElement],
    states: &HashMap<TypeId, &(dyn Any + Send + Sync)>,
    handlers: &mut std::collections::HashMap<u32, HandlerFn>,
    changes: ChangeSet,
    known_symbols: Option<&mut HashMap<String, u32>>,
    changed_state_type_id: Option<TypeId>,
    mut prev_hashes: Option<&mut HashMap<u32, u64>>,
    sent_css: SentCss<'_>,
    sent_maps: SentMaps<'_>,
    discovered_out: Option<&mut Vec<SyncedElement>>,
    synced_id_floor: u32,
    client_actions: Option<&ClientActionIndices>,
) -> Bytes {
    let mut buf = OpcodeBuffer::new();

    // Collect all symbols first (but NOT synced element IDs - those use GET_SYNCED opcode)
    let mut new_symbols: Vec<String> = Vec::new();

    // Work directly on the connection's known-symbol map when present, rather than
    // cloning the whole table in (and writing it back out) on every event. New
    // strings are interned into this map in place and the delta tracked in
    // `new_symbols` for the SYMBOLS_EXTEND opcode; the connection's map is thus
    // updated as a side effect, exactly as before but without the O(N) clones.
    // Without a known map (full render) a local map backs the same logic.
    let had_known = known_symbols.is_some();
    let mut local_map: HashMap<String, u32> = HashMap::new();
    let symbol_map: &mut HashMap<String, u32> = match known_symbols {
        Some(known) => known,
        None => &mut local_map,
    };

    // New symbols are assigned indices immediately after the ones already known.
    let next_symbol_idx: u32 = 0x80u32 + symbol_map.len() as u32;
    let mut current_next_idx = next_symbol_idx;

    // Single-render path: render each synced element ONCE and cache the result.
    // This avoids the double-render problem where non-deterministic values
    // (like generate_element_id()) produce different outputs between symbol
    // collection and emission passes.
    //
    // The counter starts at `synced_id_floor` so newly-introduced regions get ids
    // above any the client may still hold from a just-removed region (a router view
    // swap) — reusing an id would make the client morph preserve the stale span.
    let mut synced_counter: u32 = synced_id_floor;
    let mut has_updates = false;
    let mut rendered_cache: HashMap<u32, ElementBuilder> = HashMap::new();

    let trace = std::env::var_os("RWIRE_TRACE").is_some();
    for se in synced {
        // Track the highest synced ID to know where nested ones start
        if se.id >= synced_counter {
            synced_counter = se.id + 1;
        }

        // Layer 1: Skip elements bound to a different state type
        if let Some(changed_id) = changed_state_type_id {
            if se.state_type_id != changed_id {
                if trace {
                    eprintln!("[rwire-trace] se={} skip: state-type", se.id);
                }
                continue;
            }
        }

        // Skip elements that don't need updating (bitmask check)
        if !se.deps.needs_update(changes) {
            if trace {
                eprintln!(
                    "[rwire-trace] se={} skip: deps mask={:#x} always={} changes={:?}",
                    se.id, se.deps.mask, se.deps.always, changes
                );
            }
            continue;
        }

        // Render once, use for both symbol collection and emission
        if let Some(state) = states.get(&se.state_type_id) {
            if let Some(rendered) = se.renderer.render_with_state(*state) {
                // Layer 2: Skip if output hash matches previous
                if let Some(ref mut hashes) = prev_hashes {
                    let hash = rendered.content_hash();
                    if hashes.get(&se.id) == Some(&hash) {
                        if trace {
                            eprintln!("[rwire-trace] se={} skip: hash unchanged", se.id);
                        }
                        continue; // Output unchanged, skip emission
                    }
                    hashes.insert(se.id, hash);
                }
                if trace {
                    eprintln!("[rwire-trace] se={} RENDER", se.id);
                }

                collect_symbols_recursive_with_known(
                    &rendered,
                    &mut new_symbols,
                    symbol_map,
                    &mut current_next_idx,
                    &mut synced_counter,
                    states,
                );
                rendered_cache.insert(se.id, rendered);
                has_updates = true;
            }
        }
    }

    // Early return if no updates needed
    if !has_updates {
        return Bytes::new();
    }

    // Emit symbol table if we have any NEW symbols
    if !new_symbols.is_empty() {
        if had_known {
            // Use SYMBOLS_EXTEND for incremental update
            buf.begin_symbols_extend(new_symbols.len() as u32, next_symbol_idx);
        } else {
            // Use regular SYMBOLS for full table
            buf.begin_symbols(new_symbols.len() as u32);
        }
        for sym in &new_symbols {
            buf.add_symbol(sym);
        }
    }

    // (No write-back needed: when `known_symbols` was supplied we interned new
    // strings directly into it above, so the connection's map is already current.)

    // Emit pass: use cached renders (no second render call).
    //
    // Each region is emitted at most once (the cache entry is taken below). Nested
    // synced regions are NOT folded into their parent's emission: the parent emits a
    // CREATE_SYNCED placeholder (so the morph preserves the live span) while the
    // nested region's own entry here emits its standalone GET_SYNCED + rebuild.

    // Single O(synced) pass: index every region's children by (parent, state type)
    // and find the highest existing id. This replaces both the per-region child
    // rescan below (which was O(regions × synced)) and a separate max() over all ids.
    // Children stay in `synced` order within each bucket, matching id reuse order.
    let mut children_by_parent: HashMap<u32, HashMap<TypeId, Vec<u32>>> = HashMap::new();
    let mut emit_synced_counter: u32 = synced_id_floor;
    for s in synced {
        if s.id >= emit_synced_counter {
            emit_synced_counter = s.id + 1;
        }
        if let Some(parent) = s.parent {
            children_by_parent
                .entry(parent)
                .or_default()
                .entry(s.state_type_id)
                .or_default()
                .push(s.id);
        }
    }
    let no_children: HashMap<TypeId, Vec<u32>> = HashMap::new();

    // Nested regions encountered while re-rendering each region this pass, tagged with
    // their owning parent — the caller reconciles registrations from this.
    let mut discovered: Vec<SyncedElement> = Vec::new();

    for se in synced {
        // Use the cached render result (rendered exactly once above; taking the
        // entry guarantees each region is emitted at most once).
        if let Some(rendered) = rendered_cache.remove(&se.id) {
            // Match nested regions only against THIS region's own children, so they
            // reuse their previous ids — while a sibling region's same-typed regions
            // (e.g. after a router view swap) are treated as new rather than hijacking
            // another region's live spans.
            let ids_by_type = children_by_parent.get(&se.id).unwrap_or(&no_children);
            let mut next_idx_by_type: HashMap<TypeId, usize> = HashMap::new();

            let wrapper_ref = buf.get_synced(se.id);
            buf.clear_children(wrapper_ref);

            emit_update_element(
                &rendered,
                wrapper_ref,
                &mut buf,
                &*symbol_map,
                handlers,
                ids_by_type,
                &mut next_idx_by_type,
                &mut emit_synced_counter,
                states,
                Some(se.id),
                &mut discovered,
                client_actions,
            );
        }
    }

    if let Some(out) = discovered_out {
        *out = discovered;
    }

    buf.end();

    // Lazy delivery: prepend MAP_DEF (element/event/attr/style-token names) then STYLE_DEF
    // (CSS rules) for anything this batch references that the connection hasn't received yet.
    // Both land before the body opcodes that use them.
    let mut prefix = match sent_maps {
        Some(sent) => map_def_prefix(buf.referenced_names(), sent),
        None => BytesMut::new(),
    };
    let style_prefix = match sent_css {
        Some(sent) => style_def_prefix(buf.referenced_styles(), sent),
        None => BytesMut::new(),
    };
    prefix.extend_from_slice(&style_prefix);
    let body = buf.finish();
    prepend(prefix, body)
}

/// The stable wire id for a handler binding.
///
/// Prefers the macro-assigned [`HandlerSpec::handler_id`]; for handlers built
/// without the macro (id `0`) it folds the runtime fn-pointer id to a non-zero
/// `u32` (stable within a process run, which is sufficient — a reconnecting
/// client re-renders and re-binds from scratch).
fn wire_handler_id(spec: &HandlerSpec, handler: &HandlerFn) -> u32 {
    let id = spec.handler_id();
    if id != 0 {
        return id;
    }
    let raw = handler.fn_id() as u64;
    let folded = ((raw ^ (raw >> 32)) as u32) & crate::state::HANDLER_ID_MAX;
    if folded == 0 {
        1
    } else {
        folded
    }
}

/// Emit an element and its children during a synced update.
///
/// This creates new DOM elements and appends them to the parent.
/// Event handlers are rebound using existing handler indices, or new handlers
/// are registered if they weren't present during initial render.
///
/// For an EXISTING nested synced element, this emits a CREATE_SYNCED placeholder
/// only (the morph preserves the live span; the region's own standalone update owns
/// its children). A genuinely new nested region is built inline.
#[allow(clippy::too_many_arguments)]
fn emit_update_element(
    el: &ElementBuilder,
    parent_ref: u32,
    buf: &mut OpcodeBuffer,
    symbol_map: &HashMap<String, u32>,
    handlers: &mut std::collections::HashMap<u32, HandlerFn>,
    ids_by_type: &HashMap<TypeId, Vec<u32>>,
    next_idx_by_type: &mut HashMap<TypeId, usize>,
    synced_counter: &mut u32,
    states: &HashMap<TypeId, &(dyn Any + Send + Sync)>,
    enclosing_synced: Option<u32>,
    discovered: &mut Vec<SyncedElement>,
    client_actions: Option<&ClientActionIndices>,
) -> u32 {
    // Handle nested synced elements
    if let Some(renderer) = &el.synced {
        let state_type_id = renderer.state_type_id();

        // Resolve the wire id (matched to initial-render ids by state-type + order)
        // and whether this nested region already exists on the client.
        let (synced_id, is_existing) = if let Some(ids) = ids_by_type.get(&state_type_id) {
            let idx = next_idx_by_type.entry(state_type_id).or_insert(0);
            if *idx < ids.len() {
                let id = ids[*idx];
                *idx += 1;
                (id, true)
            } else {
                // More nested elements than at initial render - a new region.
                let id = *synced_counter;
                *synced_counter += 1;
                (id, false)
            }
        } else {
            // Truly new synced element type, assign new ID
            let id = *synced_counter;
            *synced_counter += 1;
            (id, false)
        };

        // Record every nested region encountered (existing or new) with its owning
        // parent, so the caller can reconcile a swapped view's registrations.
        discovered.push(SyncedElement {
            id: synced_id,
            state_type_id,
            renderer: renderer.clone_box(),
            deps: renderer.deps(),
            parent: enclosing_synced,
        });

        let wrapper_ref = buf.create_synced(synced_id);

        if is_existing {
            // The region already exists on the client. Emit ONLY the wrapper as a
            // placeholder: the parent morph preserves the live span (the client's
            // `me` skips recursing into `__synced_` regions), and the region's OWN
            // standalone update reconciles its children - including removals.
            //
            // We deliberately do NOT mark it rendered, so the main emit loop still
            // emits its standalone CLEAR_CHILDREN + rebuild when its deps changed.
            // Rebuilding its content here instead would be discarded by that same
            // client short-circuit, leaving stale / append-only content.
        } else if let Some(state) = states.get(&state_type_id) {
            // A genuinely new nested region: no standalone update covers it, so
            // build its content inline. It has no prior children, so its own nested
            // regions match against an empty scope (all fresh) rather than the
            // enclosing region's children.
            if let Some(rendered) = renderer.render_with_state(*state) {
                let child_ids: HashMap<TypeId, Vec<u32>> = HashMap::new();
                let mut child_next_idx: HashMap<TypeId, usize> = HashMap::new();
                emit_update_element(
                    &rendered,
                    wrapper_ref,
                    buf,
                    symbol_map,
                    handlers,
                    &child_ids,
                    &mut child_next_idx,
                    synced_counter,
                    states,
                    Some(synced_id),
                    discovered,
                    client_actions,
                );
            }
        }

        buf.append(parent_ref, wrapper_ref);
        return wrapper_ref;
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

    // Set typed attributes (binary-encoded)
    for ta in &el.typed_attrs {
        match ta {
            TypedAttr::Enum(key, value) => {
                buf.set_attr_enum(ref_idx, key.as_u8(), value.as_u8());
            }
            TypedAttr::Bool(key) => {
                buf.set_attr_bool(ref_idx, key.as_u8());
            }
            TypedAttr::KeySym(key, value) => {
                if let Some(&val_sym) = symbol_map.get(value) {
                    buf.set_attr_key_sym(ref_idx, key.as_u8(), val_sym);
                }
            }
        }
    }

    // Morph key for keyed reordering
    if let Some(k) = el.key {
        buf.set_key(ref_idx, k);
    }

    // Emit style tokens (binary-encoded styles)
    if !el.style_utils.is_empty() {
        if el.style_utils.len() >= 3 {
            buf.style_multi(ref_idx, &el.style_utils);
        } else {
            for &util in &el.style_utils {
                buf.style_util(ref_idx, util);
            }
        }
    }

    // Emit style property+value pairs
    for &(prop, value) in &el.style_props {
        buf.style_prop(ref_idx, prop, value);
    }

    // Emit pseudo-class groups
    for (pc_code, st_codes) in &el.pseudo_groups {
        buf.style_pseudo(ref_idx, *pc_code, st_codes);
    }

    // Emit breakpoint groups
    for (bp_code, st_codes) in &el.breakpoint_groups {
        buf.style_breakpoint(ref_idx, *bp_code, st_codes);
    }

    // Bind events - look up handler index from existing handlers by function pointer
    // If handler not found, register it as a new handler
    for (ev, spec) in &el.events {
        if let Some(handler) = &spec.remote_handler {
            // Register under the stable handler id (idempotent across renders).
            let handler_id = wire_handler_id(spec, handler);
            handlers
                .entry(handler_id)
                .or_insert_with(|| handler.clone());

            // Use BIND_REMOTE_PARAM if we have param bytes,
            // BIND_DEBOUNCED if debounced, otherwise BIND_REMOTE
            if *ev == Ev::Visible {
                let empty = Vec::new();
                let params = spec.param_bytes.as_ref().unwrap_or(&empty);
                buf.bind_sentinel(ref_idx, handler_id, params);
            } else if let Some(param_bytes) = &spec.param_bytes {
                buf.bind_remote_param(ref_idx, ev.as_u8(), handler_id, param_bytes);
            } else if spec.debounce_ms > 0 {
                buf.bind_debounced(ref_idx, ev.as_u8(), handler_id, spec.debounce_ms);
            } else {
                buf.bind_remote(ref_idx, ev.as_u8(), handler_id);
            }
        }
    }

    // Re-emit client-side action bindings (target/selector/toggle) against the
    // index slots captured at initial render, so elements regenerated by this
    // synced update stay wired to the same client-side target/selector state.
    // INIT_TARGET/INIT_SELECTOR are NOT repeated (the client keeps the slots from
    // initial render); only the per-element BIND_* opcodes are re-emitted. A type
    // never registered at initial render has no client slot, so it is skipped.
    if let Some(ca) = client_actions {
        for tb in &el.target_bindings {
            if let Some(&idx) = ca.targets.get(&tb.type_id) {
                buf.bind_target(ref_idx, idx, tb.st, tb.invert);
            }
        }
        for tt in &el.target_toggles {
            if let Some(&idx) = ca.targets.get(&tt.type_id) {
                buf.bind_toggle(ref_idx, tt.ev.as_u8(), idx);
            }
        }
        for sb in &el.selector_bindings {
            if let Some(&idx) = ca.selectors.get(&sb.type_id) {
                buf.bind_selector(ref_idx, idx, sb.match_val, sb.st);
            }
        }
        for ss in &el.selector_sets {
            if let Some(&idx) = ca.selectors.get(&ss.type_id) {
                buf.bind_select(ref_idx, ss.ev.as_u8(), idx, ss.val);
            }
        }
        for tt in &el.timed_toggles {
            if let Some(&idx) = ca.targets.get(&tt.type_id) {
                buf.bind_timed_toggle(ref_idx, tt.ev.as_u8(), idx, tt.delay_ms);
            }
        }
        for at in &el.auto_toggles {
            if let Some(&idx) = ca.targets.get(&at.type_id) {
                buf.auto_toggle(idx, at.delay_ms);
            }
        }
    }

    // Recursively emit children (same enclosing region — regular elements are not a
    // synced boundary).
    for child in &el.children {
        emit_update_element(
            child,
            ref_idx,
            buf,
            symbol_map,
            handlers,
            ids_by_type,
            next_idx_by_type,
            synced_counter,
            states,
            enclosing_synced,
            discovered,
            client_actions,
        );
    }

    // Append to parent
    buf.append(parent_ref, ref_idx);

    ref_idx
}

/// Collect symbols recursively with support for incremental symbol updates.
/// Only new symbols (not in the initial symbol_map) are added to new_symbols.
fn collect_symbols_recursive_with_known(
    el: &ElementBuilder,
    new_symbols: &mut Vec<String>,
    symbol_map: &mut HashMap<String, u32>,
    next_idx: &mut u32,
    synced_counter: &mut u32,
    states: &HashMap<TypeId, &(dyn Any + Send + Sync)>,
) {
    fn intern_with_known(
        s: &str,
        new_symbols: &mut Vec<String>,
        symbol_map: &mut HashMap<String, u32>,
        next_idx: &mut u32,
    ) {
        if symbol_map.contains_key(s) {
            return;
        }
        let idx = *next_idx;
        *next_idx += 1;
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
    // Intern string values in typed attrs
    for ta in &el.typed_attrs {
        if let TypedAttr::KeySym(_, value) = ta {
            intern_with_known(value, new_symbols, symbol_map, next_idx);
        }
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

#[cfg(test)]
mod map_def_tests {
    use super::*;
    use std::collections::{BTreeSet, HashSet};

    #[test]
    fn map_def_prefix_encodes_names_and_dedups_per_connection() {
        // Element code 0x00 = "div", wire kind 0.
        let mut referenced = BTreeSet::new();
        referenced.insert((NAME_ELEMENT, 0u8));
        let mut sent: HashSet<(u8, u8)> = HashSet::new();

        let bytes = map_def_prefix(&referenced, &mut sent);
        assert_eq!(bytes[0], MAP_DEF);
        assert_eq!(bytes[1], 1, "count = 1 entry (varint)");
        assert_eq!(bytes[2], 0, "kind = element");
        assert_eq!(bytes[3], 0, "code = 0 (div)");
        assert_eq!(bytes[4], 3, "name length (varint)");
        assert_eq!(&bytes[5..8], b"div");

        // Same connection + same code → already delivered → nothing re-sent.
        assert!(map_def_prefix(&referenced, &mut sent).is_empty());
    }

    #[test]
    fn map_def_prefix_tags_svg_elements_with_kind_6() {
        // 0x18 is an SVG element code, so it is emitted with wire kind 6 — the client
        // sets both E[code] and SE[code]=1.
        let mut referenced = BTreeSet::new();
        referenced.insert((NAME_ELEMENT, 0x18u8));
        let mut sent: HashSet<(u8, u8)> = HashSet::new();
        let bytes = map_def_prefix(&referenced, &mut sent);
        assert_eq!(bytes[0], MAP_DEF);
        assert_eq!(bytes[2], 6, "svg element uses kind 6");
        assert_eq!(bytes[3], 0x18);
    }
}

#[cfg(test)]
mod client_action_update_tests {
    use super::*;
    use crate::action::Target;
    use crate::protocol::opcodes::BIND_TOGGLE;

    struct TestTarget;
    impl Target for TestTarget {}

    /// Drive a single element carrying a `.toggle()` binding through the synced
    /// update emitter and return the produced opcode bytes.
    fn emit_toggle_update(client_actions: Option<&ClientActionIndices>) -> Vec<u8> {
        let el = el(El::Button).text("x").toggle::<TestTarget>(Ev::Click);

        let mut buf = OpcodeBuffer::new();
        let mut handlers: HashMap<u32, HandlerFn> = HashMap::new();
        let ids_by_type: HashMap<TypeId, Vec<u32>> = HashMap::new();
        let mut next_idx_by_type: HashMap<TypeId, usize> = HashMap::new();
        let mut synced_counter = 0u32;
        let states: HashMap<TypeId, &(dyn Any + Send + Sync)> = HashMap::new();
        let mut discovered: Vec<SyncedElement> = Vec::new();

        emit_update_element(
            &el,
            0,
            &mut buf,
            &HashMap::new(),
            &mut handlers,
            &ids_by_type,
            &mut next_idx_by_type,
            &mut synced_counter,
            &states,
            None,
            &mut discovered,
            client_actions,
        );
        buf.end();
        buf.finish().to_vec()
    }

    #[test]
    fn synced_update_reemits_client_action_bindings() {
        // Regression for M1: a `.toggle()` inside a synced region must keep its
        // client-side binding after the region re-renders. With the index map the
        // update stream re-emits BIND_TOGGLE; without it (legacy behavior) it does not.
        let mut indices = ClientActionIndices::default();
        indices.targets.insert(TypeId::of::<TestTarget>(), 0);

        let with = emit_toggle_update(Some(&indices));
        assert!(
            with.contains(&BIND_TOGGLE),
            "expected BIND_TOGGLE (0x{BIND_TOGGLE:02x}) to be re-emitted on synced update"
        );

        // No index map → no client-action binding emitted (documents the old gap).
        let without = emit_toggle_update(None);
        assert!(!without.contains(&BIND_TOGGLE));

        // An unknown target type (never registered at init) is skipped, not panicked.
        let empty = ClientActionIndices::default();
        let unknown = emit_toggle_update(Some(&empty));
        assert!(!unknown.contains(&BIND_TOGGLE));
    }
}
