//! State management for rwire.
//!
//! This module provides traits and types for managing state with different
//! storage strategies:
//!
//! - **LocalState**: Client-side state (no server round-trip)
//! - **MemoryState**: Server memory state (per-connection, default)
//! - **PersistedState**: Database-backed state (survives disconnects)
//!
//! Use `#[derive(State)]` with `#[storage(...)]` attribute to specify storage:
//!
//! ```ignore
//! use rwire::State;
//!
//! #[derive(State, Default)]
//! #[storage(local)]
//! struct UiState { sidebar_open: bool }
//!
//! #[derive(State, Default)]
//! #[storage(memory)]  // or omit for default
//! struct SessionState { user_id: Option<u64> }
//!
//! #[derive(State)]
//! #[storage(persisted, table = "notes")]
//! struct Note { id: u64, content: String }
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::builder::ElementBuilder;
use crate::item_ref::ItemRef;
use crate::protocol::opcodes::{
    MUT_ADD_I32, MUT_ADD_I8, MUT_SET_BOOL, MUT_SET_I32, MUT_SET_STR, MUT_TOGGLE,
};

// ============================================================================
// EventContext - Payload data passed to handlers
// ============================================================================

/// Payload data from a client event.
///
/// Different event types provide different payload types:
/// - `input`/`change` on `<input>`/`<textarea>` → `Text(value)`
/// - `click` on element with `data-*` attrs → `Data(attrs)`
/// - `submit` on `<form>` → `Form(field_values)`
#[derive(Debug, Clone)]
pub enum EventPayload {
    /// Text value from input.value or textarea.value
    Text(String),
    /// Data attributes from the clicked element (data-id="5" → {"id": "5"})
    Data(HashMap<String, String>),
    /// Form field values from form submission
    Form(HashMap<String, String>),
    /// No payload data
    Empty,
}

/// Context passed to handlers with 2-parameter signatures.
///
/// Provides access to event payload data such as input values, data attributes,
/// and form field values. The payload is parsed lazily on first access.
///
/// Also provides access to handler parameter bytes for ItemRef-based handlers.
///
/// # Example
///
/// ```ignore
/// #[handler]
/// fn add_todo(state: &mut TodoState, ctx: &EventContext) {
///     if let Some(text) = ctx.text() {
///         state.items.push(Todo::new(text.to_string()));
///     }
/// }
///
/// #[handler]
/// fn delete_item(state: &mut TodoState, ctx: &EventContext) {
///     if let Some(id) = ctx.data("id").and_then(|s| s.parse().ok()) {
///         state.items.retain(|item| item.id != id);
///     }
/// }
///
/// // ItemRef-based handler (cleaner API)
/// #[handler]
/// fn toggle_item(state: &mut TodoState, ctx: &EventContext) {
///     if let Some(item_ref) = ctx.item_ref::<TodoItem>() {
///         if let Some(item) = item_ref.get_mut(&mut state.items) {
///             item.done = !item.done;
///         }
///     }
/// }
/// ```
pub struct EventContext {
    /// Raw payload bytes from the client (JSON for form/text events)
    raw: Vec<u8>,
    /// Lazily parsed payload
    parsed: OnceLock<EventPayload>,
    /// Handler parameter bytes (for ItemRef-based handlers)
    param_bytes: Vec<u8>,
}

impl EventContext {
    /// Create a new EventContext from raw payload bytes.
    pub fn new(raw: Vec<u8>) -> Self {
        Self {
            raw,
            parsed: OnceLock::new(),
            param_bytes: Vec::new(),
        }
    }

    /// Create a new EventContext with both payload and param bytes.
    pub fn new_with_params(raw: Vec<u8>, param_bytes: Vec<u8>) -> Self {
        Self {
            raw,
            parsed: OnceLock::new(),
            param_bytes,
        }
    }

    /// Create an empty EventContext (no payload).
    pub fn empty() -> Self {
        Self {
            raw: Vec::new(),
            parsed: OnceLock::new(),
            param_bytes: Vec::new(),
        }
    }

    /// Get the parsed payload, parsing lazily if needed.
    pub fn payload(&self) -> &EventPayload {
        self.parsed.get_or_init(|| self.parse_payload())
    }

    /// Parse the raw payload bytes into an EventPayload.
    fn parse_payload(&self) -> EventPayload {
        if self.raw.is_empty() {
            return EventPayload::Empty;
        }

        // Try to parse as JSON
        let json_str = match std::str::from_utf8(&self.raw) {
            Ok(s) => s,
            Err(_) => return EventPayload::Empty,
        };

        // Parse JSON object with type discriminator
        let parsed: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(_) => return EventPayload::Empty,
        };

        let obj = match parsed.as_object() {
            Some(o) => o,
            None => return EventPayload::Empty,
        };

        let payload_type = obj.get("t").and_then(|v| v.as_str()).unwrap_or("");
        let value = obj.get("v");

        match payload_type {
            "text" => {
                if let Some(s) = value.and_then(|v| v.as_str()) {
                    EventPayload::Text(s.to_string())
                } else {
                    EventPayload::Empty
                }
            }
            "data" => {
                if let Some(obj) = value.and_then(|v| v.as_object()) {
                    let map: HashMap<String, String> = obj
                        .iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect();
                    EventPayload::Data(map)
                } else {
                    EventPayload::Empty
                }
            }
            "form" => {
                if let Some(obj) = value.and_then(|v| v.as_object()) {
                    let map: HashMap<String, String> = obj
                        .iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect();
                    EventPayload::Form(map)
                } else {
                    EventPayload::Empty
                }
            }
            _ => EventPayload::Empty,
        }
    }

    /// Get text value if this is a text payload.
    ///
    /// Returns `Some(&str)` for `input` or `change` events on input/textarea elements.
    pub fn text(&self) -> Option<&str> {
        match self.payload() {
            EventPayload::Text(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Get a data attribute value if this is a data payload.
    ///
    /// Returns `Some(&str)` for `click` events on elements with matching data-* attribute.
    ///
    /// # Example
    ///
    /// For an element with `data-id="5"`, `ctx.data("id")` returns `Some("5")`.
    pub fn data(&self, key: &str) -> Option<&str> {
        match self.payload() {
            EventPayload::Data(map) => map.get(key).map(|s| s.as_str()),
            _ => None,
        }
    }

    /// Get a form field value if this is a form payload.
    ///
    /// Returns `Some(&str)` for `submit` events on form elements.
    pub fn field(&self, name: &str) -> Option<&str> {
        match self.payload() {
            EventPayload::Form(map) => map.get(name).map(|s| s.as_str()),
            _ => None,
        }
    }

    /// Check if the payload is empty.
    pub fn is_empty(&self) -> bool {
        matches!(self.payload(), EventPayload::Empty)
    }

    /// Get the raw payload bytes.
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// Get the handler parameter bytes.
    pub fn param_bytes(&self) -> &[u8] {
        &self.param_bytes
    }

    /// Check if this context has parameter bytes (from on_ref binding).
    pub fn has_params(&self) -> bool {
        !self.param_bytes.is_empty()
    }

    /// Extract an ItemRef from the parameter bytes.
    ///
    /// This is used with handlers bound via `on_ref()`. The ItemRef was
    /// encoded when binding the event and sent back with the event.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[handler]
    /// fn toggle_item(state: &mut TodoState, ctx: &EventContext) {
    ///     if let Some(item_ref) = ctx.item_ref::<TodoItem>() {
    ///         if let Some(item) = item_ref.get_mut(&mut state.items) {
    ///             item.done = !item.done;
    ///         }
    ///     }
    /// }
    /// ```
    pub fn item_ref<T: 'static>(&self) -> Option<ItemRef<T>> {
        if self.param_bytes.is_empty() {
            return None;
        }
        ItemRef::decode(&self.param_bytes).map(|(r, _)| r)
    }

    /// Get the item index from the parameter bytes.
    ///
    /// This is a lower-level alternative to `item_ref()` that just returns
    /// the index without type information.
    pub fn item_index(&self) -> Option<usize> {
        self.item_ref::<()>().map(|r| r.index())
    }
}

impl std::fmt::Debug for EventContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventContext")
            .field("payload", self.payload())
            .finish()
    }
}

// ============================================================================
// Storage Type Enum
// ============================================================================

/// Storage type for state, determined at compile time via `#[storage(...)]` attribute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageType {
    /// Client-side state (no server round-trip for mutations)
    Local,
    /// Server memory state (per-connection, default)
    Memory,
    /// Database-backed state (survives disconnects)
    Persisted,
}

// ============================================================================
// Mutation Types (for local state handlers)
// ============================================================================

/// A single field mutation operation for local state.
///
/// These mutations are compiled from handler function bodies and executed
/// entirely on the client side without server round-trips.
#[derive(Clone, Debug, PartialEq)]
pub enum Mutation {
    /// Toggle a boolean field: `state.field = !state.field`
    Toggle { field: u8 },
    /// Add an i8 value to a numeric field: `state.field += n` (where n fits in i8)
    AddI8 { field: u8, value: i8 },
    /// Add an i32 value to a numeric field: `state.field += n`
    AddI32 { field: u8, value: i32 },
    /// Set a boolean field: `state.field = true/false`
    SetBool { field: u8, value: bool },
    /// Set an i32 field: `state.field = n`
    SetI32 { field: u8, value: i32 },
    /// Set a string field: `state.field = "..."`
    SetStr { field: u8, value: String },
}

impl Mutation {
    /// Encode this mutation to bytes for the wire protocol.
    pub fn encode(&self, buf: &mut Vec<u8>) {
        match self {
            Mutation::Toggle { field } => {
                buf.push(MUT_TOGGLE);
                buf.push(*field);
            }
            Mutation::AddI8 { field, value } => {
                buf.push(MUT_ADD_I8);
                buf.push(*field);
                buf.push(*value as u8);
            }
            Mutation::AddI32 { field, value } => {
                buf.push(MUT_ADD_I32);
                buf.push(*field);
                // Big-endian encoding
                buf.push((*value >> 24) as u8);
                buf.push((*value >> 16) as u8);
                buf.push((*value >> 8) as u8);
                buf.push(*value as u8);
            }
            Mutation::SetBool { field, value } => {
                buf.push(MUT_SET_BOOL);
                buf.push(*field);
                buf.push(if *value { 1 } else { 0 });
            }
            Mutation::SetI32 { field, value } => {
                buf.push(MUT_SET_I32);
                buf.push(*field);
                // Big-endian encoding
                buf.push((*value >> 24) as u8);
                buf.push((*value >> 16) as u8);
                buf.push((*value >> 8) as u8);
                buf.push(*value as u8);
            }
            Mutation::SetStr { field, value } => {
                buf.push(MUT_SET_STR);
                buf.push(*field);
                let bytes = value.as_bytes();
                buf.push(bytes.len() as u8);
                buf.extend_from_slice(bytes);
            }
        }
    }

    /// Get the encoded byte length of this mutation.
    pub fn encoded_len(&self) -> usize {
        match self {
            Mutation::Toggle { .. } => 2,
            Mutation::AddI8 { .. } => 3,
            Mutation::AddI32 { .. } => 6,
            Mutation::SetBool { .. } => 3,
            Mutation::SetI32 { .. } => 6,
            Mutation::SetStr { value, .. } => 3 + value.len(),
        }
    }
}

/// Specification for a local handler's mutations.
///
/// This is generated by the `#[handler]` macro for local state types.
/// It contains the state index and a list of mutations to apply.
#[derive(Clone, Debug)]
pub struct LocalMutations {
    /// Index of the state type this handler operates on (assigned at build time)
    pub state_idx: u8,
    /// List of mutations to apply to the state
    pub mutations: Vec<Mutation>,
}

impl LocalMutations {
    /// Create a new LocalMutations with the given mutations.
    /// The state_idx will be set during build context registration.
    pub fn new(mutations: Vec<Mutation>) -> Self {
        Self {
            state_idx: 0, // Will be set by BuildContext
            mutations,
        }
    }

    /// Encode all mutations to bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        for m in &self.mutations {
            m.encode(&mut buf);
        }
        buf
    }
}

// ============================================================================
// State Traits
// ============================================================================

/// Marker trait for local state types (client-side, no server round-trip).
///
/// Local state lives entirely in the browser. Mutations happen without
/// WebSocket communication, making it ideal for UI state like:
/// - Sidebar open/closed
/// - Selected tabs
/// - Form drafts before submission
///
/// # Example
///
/// ```ignore
/// #[derive(State, Default)]
/// #[storage(local)]
/// struct UiState {
///     sidebar_open: bool,
/// }
/// ```
pub trait LocalState: Default + Send + Sync + 'static {
    /// Return the storage type (always Local for this trait).
    fn storage_type() -> StorageType {
        StorageType::Local
    }
}

/// Marker trait for memory state types (server-side, per-connection).
///
/// Memory state lives in server RAM and is the default storage type.
/// State is lost when the connection closes. Ideal for:
/// - Session data
/// - Computed views
/// - Temporary application state
///
/// # Example
///
/// ```ignore
/// #[derive(State, Default)]
/// #[storage(memory)]  // or just omit #[storage] for default
/// struct SessionState {
///     user_id: Option<u64>,
/// }
/// ```
pub trait MemoryState: Default + Send + Sync + 'static {
    /// Return the storage type (always Memory for this trait).
    fn storage_type() -> StorageType {
        StorageType::Memory
    }
}

/// Marker trait for persisted state types (database-backed).
///
/// Persisted state is stored in a database and survives disconnects.
/// Changes are automatically saved after handler execution. Ideal for:
/// - User data
/// - Preferences
/// - Documents
///
/// # Example
///
/// ```ignore
/// #[derive(State)]
/// #[storage(persisted, table = "notes")]
/// struct Note {
///     #[key]
///     id: u64,
///     content: String,
/// }
/// ```
pub trait PersistedState: Send + Sync + 'static {
    /// Return the storage type (always Persisted for this trait).
    fn storage_type() -> StorageType {
        StorageType::Persisted
    }

    /// Return the table name for this persisted state.
    fn table_name() -> &'static str;

    /// Return the key field name for this persisted state.
    fn key_field() -> &'static str;
}

// ============================================================================
// Unified State Trait
// ============================================================================

/// Unified trait for all state types, with storage type as an associated constant.
///
/// This trait is implemented by the `#[derive(State)]` macro based on the
/// `#[storage(...)]` attribute. The `STORAGE_TYPE` constant determines how
/// handlers for this state behave:
///
/// - `Local`: Handlers execute entirely on the client (no server round-trip)
/// - `Memory`: Handlers execute on the server (current behavior)
/// - `Persisted`: Handlers execute on server and persist to database
///
/// # Example
///
/// ```ignore
/// #[derive(State, Default)]
/// #[storage(local)]
/// struct UiState { sidebar_open: bool }
///
/// #[derive(State, Default)]
/// struct Counter { count: i32 }  // Memory is default
/// ```
pub trait State: Send + Sync + 'static {
    /// The storage type for this state, known at compile time.
    const STORAGE_TYPE: StorageType;

    /// Table name for persisted state (empty for non-persisted).
    const TABLE_NAME: &'static str = "";

    /// Key field name for persisted state (empty for non-persisted).
    const KEY_FIELD: &'static str = "";

    /// Serialize this state for sending to the client (local state only).
    fn serialize_for_client(&self) -> Option<Vec<u8>> {
        None
    }

    /// Deserialize state from client bytes (local state only).
    fn deserialize_from_client(_bytes: &[u8]) -> Option<Self>
    where
        Self: Sized,
    {
        None
    }
}

// ============================================================================
// HandlerSpec - Bridge between macros and runtime
// ============================================================================

/// Specification for a handler, determining its execution strategy.
///
/// The `#[handler]` macro generates this instead of `HandlerFn`. The storage
/// type of the state determines whether the handler:
/// - Executes locally on the client (Local storage)
/// - Requires a server round-trip (Memory/Persisted storage)
#[derive(Clone)]
pub struct HandlerSpec {
    /// The storage type of the state this handler operates on.
    pub storage_type: StorageType,
    /// The TypeId of the state this handler operates on.
    pub state_type_id: Option<TypeId>,
    /// The remote handler function (for Memory/Persisted state).
    pub remote_handler: Option<HandlerFn>,
    /// The local mutations (for Local state).
    pub local_mutations: Option<LocalMutations>,
    /// Parameter bytes to send with the event (for ItemRef-based handlers).
    /// These bytes are encoded when binding the event and sent back with the event.
    pub param_bytes: Option<Vec<u8>>,
}

impl HandlerSpec {
    /// Create a HandlerSpec from a handler function and state type.
    ///
    /// This is called by the `#[handler]` macro. The storage type is
    /// determined by `S::STORAGE_TYPE` at compile time.
    pub fn from_fn<S: State + Default + 'static>(handler: fn(&mut S)) -> Self {
        match S::STORAGE_TYPE {
            StorageType::Local => Self {
                storage_type: StorageType::Local,
                state_type_id: Some(TypeId::of::<S>()),
                remote_handler: None,
                local_mutations: None, // Will be set by macro for local handlers
                param_bytes: None,
            },
            StorageType::Memory => Self {
                storage_type: StorageType::Memory,
                state_type_id: Some(TypeId::of::<S>()),
                remote_handler: Some(HandlerFn::new_state::<S>(handler)),
                local_mutations: None,
                param_bytes: None,
            },
            StorageType::Persisted => Self {
                storage_type: StorageType::Persisted,
                state_type_id: Some(TypeId::of::<S>()),
                remote_handler: Some(HandlerFn::new_state::<S>(handler)),
                local_mutations: None,
                param_bytes: None,
            },
        }
    }

    /// Create a HandlerSpec from a handler function with EventContext.
    ///
    /// This is called by the `#[handler]` macro when the handler has a
    /// 2-parameter signature: `fn(state: &mut S, ctx: &EventContext)`.
    pub fn from_fn_with_context<S: State + Default + 'static>(
        handler: fn(&mut S, &EventContext),
    ) -> Self {
        match S::STORAGE_TYPE {
            StorageType::Local => Self {
                storage_type: StorageType::Local,
                state_type_id: Some(TypeId::of::<S>()),
                remote_handler: None,
                local_mutations: None, // Local handlers with context not supported yet
                param_bytes: None,
            },
            StorageType::Memory => Self {
                storage_type: StorageType::Memory,
                state_type_id: Some(TypeId::of::<S>()),
                remote_handler: Some(HandlerFn::new_with_context::<S>(handler)),
                local_mutations: None,
                param_bytes: None,
            },
            StorageType::Persisted => Self {
                storage_type: StorageType::Persisted,
                state_type_id: Some(TypeId::of::<S>()),
                remote_handler: Some(HandlerFn::new_with_context::<S>(handler)),
                local_mutations: None,
                param_bytes: None,
            },
        }
    }

    /// Create a local HandlerSpec with mutations and state type.
    ///
    /// This is called by the `#[handler]` macro for local state types
    /// where the handler body has been analyzed into mutations.
    pub fn local<S: 'static>(mutations: LocalMutations) -> Self {
        Self {
            storage_type: StorageType::Local,
            state_type_id: Some(TypeId::of::<S>()),
            remote_handler: None,
            local_mutations: Some(mutations),
            param_bytes: None,
        }
    }

    /// Create a local HandlerSpec with mutations (without state type).
    ///
    /// Deprecated: Use `local::<S>` with the state type parameter instead.
    pub fn local_untyped(mutations: LocalMutations) -> Self {
        Self {
            storage_type: StorageType::Local,
            state_type_id: None,
            remote_handler: None,
            local_mutations: Some(mutations),
            param_bytes: None,
        }
    }

    /// Create a memory HandlerSpec with a remote handler.
    pub fn memory<S: MemoryState>(handler: fn(&mut S)) -> Self {
        Self {
            storage_type: StorageType::Memory,
            state_type_id: Some(TypeId::of::<S>()),
            remote_handler: Some(HandlerFn::new::<S>(handler)),
            local_mutations: None,
            param_bytes: None,
        }
    }

    /// Add parameter bytes to this handler spec (returns a new spec).
    ///
    /// This is used by `ElementBuilder::on_ref()` to attach item reference
    /// data to the handler.
    pub fn with_param_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.param_bytes = Some(bytes);
        self
    }

    /// Get the state type ID.
    pub fn state_type_id(&self) -> Option<TypeId> {
        self.state_type_id
    }

    /// Check if this is a local handler (no server round-trip).
    pub fn is_local(&self) -> bool {
        self.storage_type == StorageType::Local
    }

    /// Check if this is a remote handler (requires server round-trip).
    pub fn is_remote(&self) -> bool {
        matches!(self.storage_type, StorageType::Memory | StorageType::Persisted)
    }

    /// Get the remote handler, if any.
    pub fn remote_handler(&self) -> Option<&HandlerFn> {
        self.remote_handler.as_ref()
    }

    /// Get the local mutations, if any.
    pub fn local_mutations(&self) -> Option<&LocalMutations> {
        self.local_mutations.as_ref()
    }
}

impl std::fmt::Debug for HandlerSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerSpec")
            .field("storage_type", &self.storage_type)
            .field("has_state_type_id", &self.state_type_id.is_some())
            .field("has_remote_handler", &self.remote_handler.is_some())
            .field("has_local_mutations", &self.local_mutations.is_some())
            .field("has_param_bytes", &self.param_bytes.is_some())
            .finish()
    }
}

// ============================================================================
// ClientState (Deprecated)
// ============================================================================

/// Marker trait for client state types.
///
/// **Deprecated**: Use `#[derive(State)]` with `#[storage(memory)]` instead.
///
/// This trait is kept for backwards compatibility and is an alias for `MemoryState`.
///
/// # Example
///
/// ```ignore
/// // Old way (deprecated):
/// #[derive(ClientState, Default)]
/// struct Counter { count: i32 }
///
/// // New way:
/// #[derive(State, Default)]
/// #[storage(memory)]  // or omit for default
/// struct Counter { count: i32 }
/// ```
#[deprecated(
    since = "0.2.0",
    note = "Use #[derive(State)] with #[storage(memory)] instead"
)]
pub trait ClientState: Default + Send + Sync + 'static {}

// Blanket implementation: any MemoryState is also a ClientState (for backwards compatibility)
#[allow(deprecated)]
impl<T: MemoryState> ClientState for T {}

/// Type alias for stateful handler functions.
pub type StatefulHandler<S> = fn(&mut S);

/// Type alias for renderer functions.
pub type Renderer<S> = fn(&S) -> ElementBuilder;

/// A type-erased handler that can mutate state.
pub struct HandlerFn {
    /// The type-erased handler function
    inner: Box<dyn HandlerInner>,
}

trait HandlerInner: Send + Sync {
    fn state_type_id(&self) -> TypeId;
    fn create_state(&self) -> Box<dyn Any + Send + Sync>;
    fn call(&self, state: &mut dyn Any);
    fn call_with_context(&self, state: &mut dyn Any, ctx: &EventContext);
    fn needs_context(&self) -> bool;
    fn clone_box(&self) -> Box<dyn HandlerInner>;
    /// Get a unique identifier for this handler (function pointer address)
    fn fn_id(&self) -> usize;
}

/// Handler that takes only state (legacy, single-parameter).
struct TypedHandler<S: Default + Send + Sync + 'static> {
    handler: fn(&mut S),
}

/// Handler that takes state and EventContext (new, two-parameter).
struct TypedHandlerWithContext<S: Default + Send + Sync + 'static> {
    handler: fn(&mut S, &EventContext),
}

impl<S: Default + Send + Sync + 'static> HandlerInner for TypedHandler<S> {
    fn state_type_id(&self) -> TypeId {
        TypeId::of::<S>()
    }

    fn create_state(&self) -> Box<dyn Any + Send + Sync> {
        Box::new(S::default())
    }

    fn call(&self, state: &mut dyn Any) {
        if let Some(s) = state.downcast_mut::<S>() {
            (self.handler)(s);
        }
    }

    fn call_with_context(&self, state: &mut dyn Any, _ctx: &EventContext) {
        // Legacy handler ignores context
        self.call(state);
    }

    fn needs_context(&self) -> bool {
        false
    }

    fn clone_box(&self) -> Box<dyn HandlerInner> {
        Box::new(TypedHandler { handler: self.handler })
    }

    fn fn_id(&self) -> usize {
        self.handler as usize
    }
}

impl<S: Default + Send + Sync + 'static> HandlerInner for TypedHandlerWithContext<S> {
    fn state_type_id(&self) -> TypeId {
        TypeId::of::<S>()
    }

    fn create_state(&self) -> Box<dyn Any + Send + Sync> {
        Box::new(S::default())
    }

    fn call(&self, state: &mut dyn Any) {
        // If called without context, use empty context
        self.call_with_context(state, &EventContext::empty());
    }

    fn call_with_context(&self, state: &mut dyn Any, ctx: &EventContext) {
        if let Some(s) = state.downcast_mut::<S>() {
            (self.handler)(s, ctx);
        }
    }

    fn needs_context(&self) -> bool {
        true
    }

    fn clone_box(&self) -> Box<dyn HandlerInner> {
        Box::new(TypedHandlerWithContext { handler: self.handler })
    }

    fn fn_id(&self) -> usize {
        self.handler as usize
    }
}

impl Clone for HandlerFn {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone_box(),
        }
    }
}

impl HandlerFn {
    /// Create a new handler for the given ClientState type (legacy API).
    #[allow(deprecated)]
    pub fn new<S: ClientState>(handler: fn(&mut S)) -> Self {
        Self {
            inner: Box::new(TypedHandler { handler }),
        }
    }

    /// Create a new handler for the given State type.
    pub fn new_state<S: State + Default>(handler: fn(&mut S)) -> Self {
        Self {
            inner: Box::new(TypedHandler { handler }),
        }
    }

    /// Create a new handler with context support for the given State type.
    pub fn new_with_context<S: State + Default>(handler: fn(&mut S, &EventContext)) -> Self {
        Self {
            inner: Box::new(TypedHandlerWithContext { handler }),
        }
    }

    /// Get the TypeId of the state this handler operates on.
    pub fn state_type_id(&self) -> TypeId {
        self.inner.state_type_id()
    }

    /// Create a new default state of the correct type.
    pub fn create_state(&self) -> Box<dyn Any + Send + Sync> {
        self.inner.create_state()
    }

    /// Call the handler with the given state (legacy, no context).
    pub fn call(&self, state: &mut dyn Any) {
        self.inner.call(state);
    }

    /// Call the handler with the given state and event context.
    pub fn call_with_context(&self, state: &mut dyn Any, ctx: &EventContext) {
        self.inner.call_with_context(state, ctx);
    }

    /// Check if this handler needs context (uses 2-parameter signature).
    pub fn needs_context(&self) -> bool {
        self.inner.needs_context()
    }

    /// Get a unique identifier for this handler (function pointer address).
    ///
    /// This is used to match handlers during synced element updates,
    /// where we need to find the correct handler index for rebinding events.
    pub fn fn_id(&self) -> usize {
        self.inner.fn_id()
    }
}

impl std::fmt::Debug for HandlerFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerFn")
            .field("state_type_id", &self.state_type_id())
            .finish()
    }
}
