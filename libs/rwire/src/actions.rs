//! Async-action lifecycle — a keyless `loading → done | errored` primitive.
//!
//! A handler that hands work to a background thread calls [`EventContext::defer`], which marks
//! its action `Loading` (broadcast to the client, so the triggering control spins) and returns a
//! [`Resolver`]. When the work finishes, the background code calls [`Resolver::ok`] (the control
//! returns to idle) or [`Resolver::err`] (the message surfaces on the control). No client event is
//! involved in the resolution — it rides the shared-state broadcast, exactly like any other
//! `#[storage(shared)]` change.
//!
//! **Keyless.** An action's identity is `(session, handler_id, param)` — all three already travel
//! with the event binding (`handler_id` is the `stable_handler_id`, i.e. the wire `handler_idx`;
//! `param` is the control's `BIND_REMOTE_PARAM` bytes). The developer writes no correlation key.
//!
//! **`Ready` is not stored.** Success *removes* the entry, so the registry only ever holds
//! in-flight `Loading` and unacknowledged `Errored` — which keeps it naturally bounded and means
//! "no entry" is the single idle/done state a control renders by default.
//!
//! [`EventContext::defer`]: crate::EventContext::defer

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::builder::{ElementBuilder, SyncedRenderer};
use crate::server::SharedServerState;
use crate::state::{RendererDeps, State, StorageType};
use crate::style_tokens::St;

/// How long an action may sit `Loading` before the backstop fails it, so a worker that crashed
/// or forgot to resolve never leaves a control spinning forever.
pub(crate) const LOADING_TIMEOUT: Duration = Duration::from_secs(30);

/// Identity of one action instance. Every part is known both at dispatch (decoded from the
/// `ClientEvent`) and at render (from the control's binding), so resolution and reflection agree
/// without a developer-authored key.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ActionKey {
    /// The connection's session id — scopes an action to one user.
    pub session: String,
    /// The handler's `stable_handler_id` (module + name), equal to the wire `handler_idx`.
    pub handler_id: u32,
    /// The control's `BIND_REMOTE_PARAM` bytes (empty for param-less controls). Distinguishes
    /// N instances of the same handler — one per list row.
    pub param: Vec<u8>,
}

impl ActionKey {
    /// Assemble a key from its three parts.
    pub fn new(session: String, handler_id: u32, param: Vec<u8>) -> Self {
        Self {
            session,
            handler_id,
            param,
        }
    }
}

/// The lifecycle phase of a deferred action. Absence from the registry is the implicit idle/done
/// state — success is represented by *removing* the entry, not by a `Ready` variant.
#[derive(Clone, PartialEq, Debug)]
pub enum Phase {
    /// Work is in flight; the triggering control shows a spinner.
    Loading,
    /// Work failed; the `String` is the message to surface on the control / error slot.
    Errored(String),
}

/// Process-global registry of in-flight and failed deferred actions. One shared instance
/// (`TABLE_NAME = "__actions__"`, deliberately distinct from any app state so their global cache
/// keys never collide); every connection subscribes and filters to its own session when
/// reflecting.
#[derive(Default)]
pub struct Actions {
    entries: HashMap<ActionKey, Phase>,
}

impl State for Actions {
    const STORAGE_TYPE: StorageType = StorageType::Shared;
    const TABLE_NAME: &'static str = "__actions__";
}

impl Actions {
    /// The phase of one exact action, or `None` when idle/done.
    pub fn phase(&self, key: &ActionKey) -> Option<&Phase> {
        self.entries.get(key)
    }

    /// The phase for a `(handler_id, param)` pair, ignoring session.
    ///
    /// Reflection runs in a render context that has no session (renderers take only `&State`),
    /// so it matches session-agnostically. This is correct for single-session deployments; true
    /// per-session isolation needs the session threaded into the render path — a later refinement.
    pub fn any_phase(&self, handler_id: u32, param: &[u8]) -> Option<&Phase> {
        self.entries
            .iter()
            .find_map(|(k, v)| (k.handler_id == handler_id && k.param == param).then_some(v))
    }

    /// Number of tracked actions (in-flight + failed). Test/introspection aid.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn set(&mut self, key: ActionKey, phase: Phase) {
        self.entries.insert(key, phase);
    }

    pub(crate) fn clear(&mut self, key: &ActionKey) {
        self.entries.remove(key);
    }

    /// Drop every entry for a session — called when its connection ends.
    pub fn drop_session(&mut self, session: &str) {
        self.entries.retain(|k, _| k.session != session);
    }
}

/// A handle to resolve a deferred action later, from a background thread or task.
///
/// Obtained from [`EventContext::defer`](crate::EventContext::defer). `Send + 'static`, so it moves
/// freely into `std::thread::spawn` or an async task. A `Resolver` that is dropped without being
/// resolved leaves the action `Loading` until the timeout backstop fails it.
pub struct Resolver {
    // `None` when produced by a context with no action slot (synthetic/route/test dispatch); its
    // methods then no-op rather than panic.
    inner: Option<(Arc<SharedServerState>, ActionKey)>,
}

impl Resolver {
    pub(crate) fn new(shared: Arc<SharedServerState>, key: ActionKey) -> Self {
        Self {
            inner: Some((shared, key)),
        }
    }

    pub(crate) fn noop() -> Self {
        Self { inner: None }
    }

    /// Mark the action successful: remove it, so the reflecting control returns to idle.
    pub fn ok(self) {
        if let Some((shared, key)) = self.inner {
            shared.resolve_action(key, None);
        }
    }

    /// Mark the action failed: `message` surfaces on the control (and any error slot).
    pub fn err(self, message: impl Into<String>) {
        if let Some((shared, key)) = self.inner {
            shared.resolve_action(key, Some(message.into()));
        }
    }
}

/// A [`SyncedRenderer`] that reflects one control's action phase: it reads [`Actions`], looks up
/// its `(handler_id, param)`, and renders the idle / loading / errored variant. Because it captures
/// per-control identity, it can't be the macro's fn-pointer renderer — it holds cloneable
/// `Arc<dyn Fn>` factories instead. Built via [`reflect_action`].
struct ActionReflector {
    handler_id: u32,
    param: Vec<u8>,
    idle: Arc<dyn Fn() -> ElementBuilder + Send + Sync>,
    loading: Arc<dyn Fn() -> ElementBuilder + Send + Sync>,
    errored: Arc<dyn Fn(&str) -> ElementBuilder + Send + Sync>,
}

impl SyncedRenderer for ActionReflector {
    fn render_with_state(&self, state: &dyn Any) -> Option<ElementBuilder> {
        let actions = state.downcast_ref::<Actions>()?;
        Some(match actions.any_phase(self.handler_id, &self.param) {
            Some(Phase::Loading) => (self.loading)(),
            Some(Phase::Errored(msg)) => (self.errored)(msg),
            None => (self.idle)(),
        })
    }

    fn clone_box(&self) -> Box<dyn SyncedRenderer> {
        Box::new(ActionReflector {
            handler_id: self.handler_id,
            param: self.param.clone(),
            idle: Arc::clone(&self.idle),
            loading: Arc::clone(&self.loading),
            errored: Arc::clone(&self.errored),
        })
    }

    fn state_type_id(&self) -> TypeId {
        TypeId::of::<Actions>()
    }

    fn create_default_state(&self) -> Box<dyn Any + Send + Sync> {
        Box::new(Actions::default())
    }

    fn storage_type(&self) -> StorageType {
        StorageType::Shared
    }

    fn table_name(&self) -> Option<&'static str> {
        Some(Actions::TABLE_NAME)
    }

    fn deps(&self) -> RendererDeps {
        // Any change to the registry may flip this control — always re-render on a broadcast.
        RendererDeps::always()
    }
}

/// Build a synced region that reflects an action's phase for the given `(handler_id, param)`.
///
/// The region subscribes to the shared [`Actions`] registry and re-renders on every change: it
/// shows `idle()` when the action is idle/done, `loading()` while it is in flight, and
/// `errored(message)` when it failed. This is the generic engine the action-aware components build
/// on; callers supply the three renderings so the reflection stays layout- and component-agnostic.
pub fn reflect_action(
    handler_id: u32,
    param: Vec<u8>,
    idle: impl Fn() -> ElementBuilder + Send + Sync + 'static,
    loading: impl Fn() -> ElementBuilder + Send + Sync + 'static,
    errored: impl Fn(&str) -> ElementBuilder + Send + Sync + 'static,
) -> ElementBuilder {
    ElementBuilder::synced_from(Box::new(ActionReflector {
        handler_id,
        param,
        idle: Arc::new(idle),
        loading: Arc::new(loading),
        errored: Arc::new(errored),
    }))
    // `display:contents`: the synced wrapper adds no box of its own, so reflecting a control is
    // layout-transparent — the rendered phase participates in the parent layout as if unwrapped.
    // This is what lets an ordinary `Button` become action-aware without shifting anything.
    .st([St::DisplayContents])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(session: &str, handler_id: u32, param: &[u8]) -> ActionKey {
        ActionKey::new(session.to_string(), handler_id, param.to_vec())
    }

    #[test]
    fn absent_key_is_idle() {
        let actions = Actions::default();
        assert!(actions.phase(&key("s", 1, b"")).is_none());
        assert!(actions.is_empty());
    }

    #[test]
    fn set_and_read_phase() {
        let mut actions = Actions::default();
        actions.set(key("s", 1, b""), Phase::Loading);
        assert_eq!(actions.phase(&key("s", 1, b"")), Some(&Phase::Loading));
        actions.set(key("s", 1, b""), Phase::Errored("boom".into()));
        assert_eq!(
            actions.phase(&key("s", 1, b"")),
            Some(&Phase::Errored("boom".into()))
        );
    }

    #[test]
    fn clear_returns_to_idle() {
        let mut actions = Actions::default();
        let k = key("s", 7, b"row-3");
        actions.set(k.clone(), Phase::Loading);
        actions.clear(&k);
        assert!(actions.phase(&k).is_none());
    }

    #[test]
    fn param_distinguishes_instances_of_one_handler() {
        let mut actions = Actions::default();
        actions.set(key("s", 9, b"a"), Phase::Loading);
        assert_eq!(actions.any_phase(9, b"a"), Some(&Phase::Loading));
        // A different param of the same handler is a different, still-idle action.
        assert!(actions.any_phase(9, b"b").is_none());
    }

    #[test]
    fn any_phase_ignores_session() {
        let mut actions = Actions::default();
        actions.set(key("session-xyz", 4, b""), Phase::Loading);
        assert_eq!(actions.any_phase(4, b""), Some(&Phase::Loading));
    }

    #[test]
    fn drop_session_clears_only_that_session() {
        let mut actions = Actions::default();
        actions.set(key("a", 1, b""), Phase::Loading);
        actions.set(key("b", 1, b""), Phase::Errored("x".into()));
        actions.drop_session("a");
        assert!(actions.phase(&key("a", 1, b"")).is_none());
        assert_eq!(
            actions.phase(&key("b", 1, b"")),
            Some(&Phase::Errored("x".into()))
        );
    }

    #[test]
    fn noop_resolver_does_not_panic() {
        Resolver::noop().ok();
        Resolver::noop().err("ignored");
    }
}
