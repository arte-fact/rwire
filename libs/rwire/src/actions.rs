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
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use crate::builder::{ElementBuilder, SyncedRenderer};
use crate::server::SharedServerState;
use crate::state::{RendererDeps, State, StorageType};
use crate::style_tokens::St;

/// How long an action may sit `Loading` before the backstop fails it, so a worker that crashed
/// or forgot to resolve never leaves a control spinning forever.
pub(crate) const LOADING_TIMEOUT: Duration = Duration::from_secs(30);

/// Factory rendering an idle/loading phase of a reflected control.
type PhaseRender = Arc<dyn Fn() -> ElementBuilder + Send + Sync>;
/// Factory rendering the errored phase of a reflected control, given the message.
type ErrorRender = Arc<dyn Fn(&str) -> ElementBuilder + Send + Sync>;
/// Factory rendering a page-level error slot, given every current message.
type ErrorsRender = Arc<dyn Fn(&[String]) -> ElementBuilder + Send + Sync>;

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

    /// Every current error message (for a page-level error slot). Session-agnostic, like
    /// [`Self::any_phase`], for the same render-context reason.
    pub fn errors(&self) -> Vec<String> {
        self.entries
            .values()
            .filter_map(|p| match p {
                Phase::Errored(message) => Some(message.clone()),
                Phase::Loading => None,
            })
            .collect()
    }

    /// Drop every entry for a session — called when its connection ends.
    pub fn drop_session(&mut self, session: &str) {
        self.entries.retain(|k, _| k.session != session);
    }
}

/// Where a deferred action stands relative to its handler: the server marks it `Loading` only
/// after the handler returns (a `&mut SharedState` handler holds the shared-cache lock that
/// marking re-enters), so a resolution made before that is kept here and applied instead.
#[derive(Debug, Default)]
pub(crate) enum Settlement {
    /// The handler is still running; nothing resolved yet.
    #[default]
    Pending,
    /// Resolved before the handler returned: `None` success, `Some(message)` failure.
    Early(Option<String>),
    /// The handler returned; resolutions go straight to the registry.
    Armed,
}

/// Shared by an action's [`Resolver`]s and the server for one dispatch.
pub(crate) type SettlementCell = Arc<Mutex<Settlement>>;

/// After the handler returned: apply a resolution made in it, or mark the action `Loading` and
/// arm the timeout backstop. Holding the cell while marking means a resolver racing on another
/// thread waits and lands after `Loading`, never under it.
pub(crate) fn settle_after_handler(
    cell: &SettlementCell,
    shared: &Arc<SharedServerState>,
    key: ActionKey,
) {
    let mut settlement = cell.lock().unwrap_or_else(PoisonError::into_inner);
    match std::mem::replace(&mut *settlement, Settlement::Armed) {
        Settlement::Early(error) => {
            // No spinner: success clears any entry a previous run left, failure shows now.
            if error.is_some() || shared.action_phase(&key).is_some() {
                shared.resolve_action(key, error);
            }
        }
        Settlement::Pending | Settlement::Armed => {
            shared.set_action_loading(key.clone());
            crate::server::spawn_action_timeout(Arc::clone(shared), key);
        }
    }
}

/// A handle to resolve a deferred action later, from a background thread or task.
///
/// Obtained from [`EventContext::defer`](crate::EventContext::defer). `Send + 'static`, so it moves
/// freely into `std::thread::spawn` or an async task. A `Resolver` that is dropped without being
/// resolved leaves the action `Loading` until the timeout backstop fails it.
///
/// It may also be resolved before the handler returns (inline, or by work the handler waited
/// for): the action then settles as soon as the handler returns, without a loading phase.
pub struct Resolver {
    // `None` when produced by a context with no action slot (synthetic/route/test dispatch); its
    // methods then no-op rather than panic.
    inner: Option<(Arc<SharedServerState>, ActionKey, SettlementCell)>,
}

impl Resolver {
    pub(crate) fn new(
        shared: Arc<SharedServerState>,
        key: ActionKey,
        cell: SettlementCell,
    ) -> Self {
        Self {
            inner: Some((shared, key, cell)),
        }
    }

    fn resolve(self, error: Option<String>) {
        let Some((shared, key, cell)) = self.inner else {
            return;
        };
        let mut settlement = cell.lock().unwrap_or_else(PoisonError::into_inner);
        match *settlement {
            // Still inside the handler (which may hold the shared-cache lock): keep it for
            // `settle_after_handler`, touching no shared state here.
            Settlement::Pending | Settlement::Early(_) => *settlement = Settlement::Early(error),
            Settlement::Armed => shared.resolve_action(key, error),
        }
    }

    pub(crate) fn noop() -> Self {
        Self { inner: None }
    }

    /// Mark the action successful: remove it, so the reflecting control returns to idle.
    pub fn ok(self) {
        self.resolve(None);
    }

    /// Mark the action failed: `message` surfaces on the control (and any error slot).
    pub fn err(self, message: impl Into<String>) {
        self.resolve(Some(message.into()));
    }
}

/// A [`SyncedRenderer`] that reflects one control's action phase: it reads [`Actions`], looks up
/// its `(handler_id, param)`, and renders the idle / loading / errored variant. Because it captures
/// per-control identity, it can't be the macro's fn-pointer renderer — it holds cloneable
/// `Arc<dyn Fn>` factories instead. Built via [`reflect_action`].
struct ActionReflector {
    handler_id: u32,
    param: Vec<u8>,
    idle: PhaseRender,
    loading: PhaseRender,
    errored: ErrorRender,
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

    fn identity(&self) -> Option<u64> {
        // The reflected `(handler_id, param)` is the region's stable identity: when a parent
        // re-renders, this wrapper keeps its wire id however its siblings changed, so the live
        // span (and any in-flight phase) stays attached to the right control.
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.handler_id.hash(&mut hasher);
        self.param.hash(&mut hasher);
        Some(hasher.finish())
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
    .boxless("an action-reflection wrapper (`Button::on_click`, `Button::submit`, a reflected send button)")
}

/// A [`SyncedRenderer`] over every current error message — the engine behind a page-level error
/// slot. Re-renders whenever the registry changes; `render` receives all messages currently
/// `Errored` (empty when there are none).
struct ActionErrorsReflector {
    render: ErrorsRender,
}

impl SyncedRenderer for ActionErrorsReflector {
    fn render_with_state(&self, state: &dyn Any) -> Option<ElementBuilder> {
        let messages = match state.downcast_ref::<Actions>() {
            Some(actions) => actions.errors(),
            None => Vec::new(),
        };
        Some((self.render)(&messages))
    }

    fn clone_box(&self) -> Box<dyn SyncedRenderer> {
        Box::new(ActionErrorsReflector {
            render: Arc::clone(&self.render),
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
        RendererDeps::always()
    }
}

/// Build a synced region that renders every current action error — a page-level slot for errors
/// whose triggering control may be off-screen. `render` is called with all current messages
/// (empty slice when there are none, so it can render nothing). An error clears the moment its
/// action is retried or resolved.
pub fn reflect_action_errors(
    render: impl Fn(&[String]) -> ElementBuilder + Send + Sync + 'static,
) -> ElementBuilder {
    ElementBuilder::synced_from(Box::new(ActionErrorsReflector {
        render: Arc::new(render),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A context for `key`'s action, as the server builds it at dispatch.
    fn dispatch(shared: &Arc<SharedServerState>, key: &ActionKey) -> crate::EventContext {
        crate::EventContext::new_with_params(Vec::new(), Vec::new())
            .with_action(Arc::clone(shared), key.clone())
    }

    #[tokio::test]
    async fn a_resolution_inside_the_handler_is_applied_when_it_returns() {
        let shared = SharedServerState::new(Duration::from_millis(100));
        let ok = ActionKey::new("s".into(), 1, Vec::new());
        let ctx = dispatch(&shared, &ok);
        ctx.defer().ok();
        assert_eq!(
            shared.action_phase(&ok),
            None,
            "nothing touched in the handler"
        );
        ctx.settle_deferred();
        assert_eq!(shared.action_phase(&ok), None, "success: idle, no spinner");

        let failed = ActionKey::new("s".into(), 2, Vec::new());
        let ctx = dispatch(&shared, &failed);
        ctx.defer().err("nope");
        ctx.settle_deferred();
        assert_eq!(
            shared.action_phase(&failed),
            Some(Phase::Errored("nope".into()))
        );

        // A success in the handler also clears an error a previous run left.
        let ctx = dispatch(&shared, &failed);
        ctx.defer().ok();
        ctx.settle_deferred();
        assert_eq!(shared.action_phase(&failed), None);
    }

    #[tokio::test]
    async fn a_resolution_after_the_handler_follows_loading() {
        let shared = SharedServerState::new(Duration::from_millis(100));
        let key = ActionKey::new("s".into(), 3, Vec::new());
        let ctx = dispatch(&shared, &key);
        let resolver = ctx.defer();
        ctx.settle_deferred();
        assert_eq!(shared.action_phase(&key), Some(Phase::Loading));
        resolver.err("later");
        assert_eq!(
            shared.action_phase(&key),
            Some(Phase::Errored("later".into()))
        );

        // Never resolved: stays `Loading` for the backstop.
        let key = ActionKey::new("s".into(), 4, Vec::new());
        let ctx = dispatch(&shared, &key);
        drop(ctx.defer());
        ctx.settle_deferred();
        assert_eq!(shared.action_phase(&key), Some(Phase::Loading));
    }

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

    fn wrapper() -> ElementBuilder {
        use crate::builder::el;
        use crate::protocol::El;
        reflect_action(
            1,
            Vec::new(),
            || el(El::Button),
            || el(El::Button),
            |_| el(El::Button),
        )
    }

    #[test]
    #[cfg(debug_assertions)]
    fn setters_on_the_reflection_wrapper_panic_with_the_method_name() {
        use crate::attr_tokens::At;
        type Setter = Box<dyn Fn(ElementBuilder) -> ElementBuilder>;
        let cases: Vec<(&str, Setter)> = vec![
            ("attr", Box::new(|b| b.attr("title", "x"))),
            ("attr", Box::new(|b| b.data("id", "1"))),
            ("at_str", Box::new(|b| b.at_str(At::Title, "x"))),
            ("bool_attr", Box::new(|b| b.bool_attr(At::Disabled))),
            ("class", Box::new(|b| b.class("c"))),
            ("text", Box::new(|b| b.text("t"))),
            ("st", Box::new(|b| b.st([St::BgApp]))),
            ("pseudo", Box::new(|b| b.hover([St::BgApp]))),
        ];
        for (method, f) in cases {
            let Err(err) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(wrapper())))
            else {
                panic!("`.{method}()` on the wrapper did not panic");
            };
            let msg = err.downcast_ref::<String>().cloned().unwrap_or_default();
            assert!(msg.contains(&format!("`.{method}()`")), "{msg}");
            assert!(msg.contains("action-reflection wrapper"), "{msg}");
        }
    }

    #[test]
    fn events_and_ordinary_elements_are_not_guarded() {
        use crate::builder::el;
        use crate::protocol::El;
        #[derive(Default)]
        struct S;
        impl crate::state::MemoryState for S {}
        fn noop(_: &mut S) {}
        // Binding on the wrapper still works (events bubble through display:contents).
        let _ = wrapper().on(
            crate::protocol::Ev::Click,
            crate::state::HandlerSpec::memory::<S>(noop),
        );
        let _ = el(El::Div).attr("title", "x").st([St::BgApp]).text("ok");
    }
}
