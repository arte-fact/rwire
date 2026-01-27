//! Client state management for rwire.
//!
//! This module provides traits and types for managing per-connection state
//! with reactive rendering.

use std::any::{Any, TypeId};

use crate::builder::ElementBuilder;

/// Marker trait for client state types.
///
/// Implement this trait (via `#[derive(ClientState)]`) on types that represent
/// per-connection state. State types must also implement `Default` for
/// automatic initialization.
///
/// # Example
///
/// ```ignore
/// #[derive(ClientState, Default)]
/// struct Counter {
///     count: i32,
/// }
/// ```
pub trait ClientState: Default + Send + Sync + 'static {}

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
    fn clone_box(&self) -> Box<dyn HandlerInner>;
}

struct TypedHandler<S: ClientState> {
    handler: fn(&mut S),
}

impl<S: ClientState> HandlerInner for TypedHandler<S> {
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

    fn clone_box(&self) -> Box<dyn HandlerInner> {
        Box::new(TypedHandler { handler: self.handler })
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
    /// Create a new handler for the given state type.
    pub fn new<S: ClientState>(handler: fn(&mut S)) -> Self {
        Self {
            inner: Box::new(TypedHandler { handler }),
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

    /// Call the handler with the given state.
    pub fn call(&self, state: &mut dyn Any) {
        self.inner.call(state);
    }
}

impl std::fmt::Debug for HandlerFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerFn")
            .field("state_type_id", &self.state_type_id())
            .finish()
    }
}
