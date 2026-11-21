//! Standard Web Push for rwire: notify subscribers without a third-party
//! service (VAPID, RFC 8030, 8291 and 8292).
//!
//! Three parts, each where it belongs:
//!
//! - **The service worker** (in `rwire` itself, behind an option): with
//!   `Pwa::push_notifications(true)` it shows a notification for every push
//!   (`title`, `body`, `url`) and focuses or opens the app at that URL on a
//!   click.
//! - **The browser side** (the `push` runtime extension, served by
//!   [`ServePush::serve_push`]): an element made with
//!   [`PushExt::push_subscribe`] asks for the permission *inside the click*,
//!   the user gesture iOS requires, subscribes, and reports the outcome
//!   over the WebSocket through the [`report_input`] the page carries.
//! - **The server side** ([`Push`]): VAPID keys, the subscriptions (memory
//!   and an atomic JSON file), [`Push::send_push`] to every subscriber with
//!   `aes128gcm` encryption and a VAPID JWT, and the purge of the
//!   subscriptions the push service says are gone (404, 410).
//!
//! ```ignore
//! use rwire_push::{report_input, Notification, Push, PushExt, ServePush, VapidKey};
//!
//! static PUSH: OnceLock<Push> = OnceLock::new();
//!
//! #[handler]
//! fn on_report(me: &mut Me, ctx: &EventContext) {
//!     me.push = PUSH.get().unwrap().handle_report(ctx.text().unwrap_or(""));
//! }
//!
//! fn page(me: &Me) -> ElementBuilder {
//!     el(El::Div).append([
//!         Button::primary("Notify me").build().push_subscribe(PUSH.get().unwrap()),
//!         report_input("push-report", on_report()),
//!     ])
//! }
//!
//! let push = Push::new(VapidKey::load_or_generate("data/vapid.key")?, "mailto:ops@example.org")
//!     .store("data/push-subscriptions.json")?;
//! Server::bind(addr)?.root(root)
//!     .capsule_config(CapsuleConfig::new().pwa(Pwa::new("Blog").push_notifications(true)))
//!     .serve_push().run().await
//! // later, for a new post:
//! push.send_push(&Notification::new("New post").body("…").url("/posts/42")).await;
//! ```

use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures::stream::{self, StreamExt};
use rwire::{el, El, ElementBuilder, Ev, HandlerSpec, ServerWithRoot};
use serde::{Deserialize, Serialize};

mod ece;
mod subscription;
mod vapid;

pub use ece::{encrypt, EncryptError, MAX_PAYLOAD};
pub use subscription::{Keys, Subscription, SubscriptionError, KNOWN_PUSH_HOSTS};
pub use vapid::{VapidError, VapidKey, TOKEN_LIFETIME_SECS};

use subscription::{EndpointPolicy, Store};

/// The extension, minified: built from `runtime/src/ext/push.ts` by the
/// runtime's `npm run sync`.
pub const PUSH_JS: &str = include_str!("../assets/push.min.js");
/// Where the runtime loader asks for it once a batch hints `ext("push")`.
pub const PUSH_JS_PATH: &str = "/_rw/ext/push.js";

/// The default cap on stored subscriptions (each report may add one).
pub const DEFAULT_CAPACITY: usize = 10_000;

/// base64url without padding, the encoding of every Web Push key.
pub(crate) mod b64 {
    use base64::engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD};
    use base64::Engine;

    pub fn encode(b: &[u8]) -> String {
        URL_SAFE_NO_PAD.encode(b)
    }

    /// Decode, padding tolerated.
    pub fn decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
        if s.ends_with('=') {
            URL_SAFE.decode(s)
        } else {
            URL_SAFE_NO_PAD.decode(s)
        }
    }
}

// ============================================================================
// Browser side: builder helpers and the extension
// ============================================================================

/// Elements that drive the `push` extension.
pub trait PushExt {
    /// A click on this element subscribes the browser to `push`'s server
    /// (permission prompt first, inside the click).
    fn push_subscribe(self, push: &Push) -> Self;
    /// A click on this element drops the browser's subscription.
    fn push_unsubscribe(self) -> Self;
}

impl PushExt for ElementBuilder {
    fn push_subscribe(self, push: &Push) -> Self {
        self.ext("push")
            .attr("data-push", "subscribe")
            .attr("data-push-key", push.public_key())
    }

    fn push_unsubscribe(self) -> Self {
        self.ext("push").attr("data-push", "unsubscribe")
    }
}

/// The hidden input the extension reports through (one per page):
/// `handler` is bound on its `input` event and passes `ctx.text()` to
/// [`Push::handle_report`]. It also gets the browser's current state when
/// it first appears.
pub fn report_input(id: &str, handler: HandlerSpec) -> ElementBuilder {
    el(El::Input)
        .ext("push")
        .attr("id", id)
        .attr("data-push-report", "1")
        .attr("type", "hidden")
        .on(Ev::Input, handler)
}

/// The server that ships the extension.
pub trait ServePush {
    fn serve_push(self) -> Self;
}

impl<F> ServePush for ServerWithRoot<F>
where
    F: Fn() -> ElementBuilder + Send + Sync + Clone + 'static,
{
    fn serve_push(self) -> Self {
        self.asset(
            PUSH_JS_PATH,
            "text/javascript; charset=utf-8",
            Cow::Borrowed(PUSH_JS.as_bytes()),
        )
    }
}

/// What the browser reported, after [`Push::handle_report`] applied it:
/// what the page shows next to its button.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PushState {
    /// Nothing reported yet.
    #[default]
    Unknown,
    /// The browser is subscribed, and the server holds its subscription.
    Subscribed,
    /// The browser is not subscribed (or just unsubscribed).
    Unsubscribed,
    /// The user refused notifications; only the browser settings undo it.
    Denied,
    /// No Push API here: an old browser, or iOS outside an installed PWA.
    Unsupported,
    /// The browser or the server refused (the reason, for a log or a tooltip).
    Failed(String),
}

#[derive(Deserialize)]
struct Report {
    state: String,
    subscription: Option<Subscription>,
    endpoint: Option<String>,
    error: Option<String>,
}

// ============================================================================
// Server side
// ============================================================================

/// A notification, as the service worker shows it.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Notification {
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub body: String,
    /// Opened on a click (same origin only; the start URL otherwise).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub url: Option<String>,
    /// An icon URL (the app's first icon otherwise).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub icon: Option<String>,
    /// A newer notification with the same tag replaces the older one.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tag: Option<String>,
}

impl Notification {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }
    pub fn body(mut self, s: impl Into<String>) -> Self {
        self.body = s.into();
        self
    }
    pub fn url(mut self, s: impl Into<String>) -> Self {
        self.url = Some(s.into());
        self
    }
    pub fn icon(mut self, s: impl Into<String>) -> Self {
        self.icon = Some(s.into());
        self
    }
    pub fn tag(mut self, s: impl Into<String>) -> Self {
        self.tag = Some(s.into());
        self
    }
}

/// How urgent a message is (RFC 8030 §5.3): push services may hold
/// low-urgency messages to spare the battery.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Urgency {
    VeryLow,
    Low,
    #[default]
    Normal,
    High,
}

impl Urgency {
    fn as_str(self) -> &'static str {
        match self {
            Urgency::VeryLow => "very-low",
            Urgency::Low => "low",
            Urgency::Normal => "normal",
            Urgency::High => "high",
        }
    }
}

/// What happened to one message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Delivery {
    /// The push service accepted it (2xx).
    Accepted,
    /// The subscription is gone (404 or 410): it was removed.
    Gone,
    /// Anything else: an HTTP status and its body, or a transport error.
    Failed(String),
}

/// The outcome of a [`Push::send_push`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SendReport {
    /// Messages the push services accepted.
    pub accepted: usize,
    /// Subscriptions removed because the push service said they are gone.
    pub removed: usize,
    /// `(endpoint, reason)` of the others; their subscriptions are kept.
    pub failed: Vec<(String, String)>,
}

/// Why a send could not start.
#[derive(Debug)]
pub enum SendError {
    /// The payload does not fit in one message.
    Encrypt(EncryptError),
    /// The payload could not be serialized.
    Json(serde_json::Error),
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SendError::Encrypt(e) => e.fmt(f),
            SendError::Json(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for SendError {}

struct Inner {
    vapid: VapidKey,
    subject: String,
    store: Store,
    policy: EndpointPolicy,
    client: reqwest::Client,
    ttl: u32,
    urgency: Urgency,
    concurrency: usize,
}

/// The Web Push service of an application: its VAPID key, its
/// subscriptions, and the sending. Cheap to clone (shared).
#[derive(Clone)]
pub struct Push {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for Push {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Push")
            .field("public_key", &self.public_key())
            .field("subscriptions", &self.inner.store.len())
            .finish_non_exhaustive()
    }
}

impl Push {
    /// A service signing with `vapid`, naming its operator `subject`
    /// (`mailto:you@example.org` or an `https:` URL: push services use it
    /// to reach you about your traffic). Subscriptions are kept in memory
    /// until [`Push::store`] gives them a file.
    pub fn new(vapid: VapidKey, subject: impl Into<String>) -> Self {
        Self::build(
            vapid,
            subject.into(),
            Store::memory(DEFAULT_CAPACITY),
            EndpointPolicy::default(),
        )
    }

    fn build(vapid: VapidKey, subject: String, store: Store, policy: EndpointPolicy) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("reqwest client");
        Self {
            inner: Arc::new(Inner {
                vapid,
                subject,
                store,
                policy,
                client,
                ttl: 7 * 24 * 3600,
                urgency: Urgency::Normal,
                concurrency: 16,
            }),
        }
    }

    fn rebuild(self, f: impl FnOnce(&mut Inner)) -> Self {
        let mut inner = Arc::try_unwrap(self.inner)
            .unwrap_or_else(|_| panic!("configure Push before cloning it"));
        f(&mut inner);
        Self {
            inner: Arc::new(inner),
        }
    }

    /// Keep the subscriptions in the JSON file at `path` (loaded now,
    /// rewritten atomically on every change, mode 0600: it holds secrets).
    pub fn store(self, path: impl Into<PathBuf>) -> std::io::Result<Self> {
        let store = Store::open(path.into(), self.inner.store.capacity)?;
        Ok(self.rebuild(|i| i.store = store))
    }

    /// Cap the stored subscriptions (default [`DEFAULT_CAPACITY`]).
    pub fn capacity(self, n: usize) -> Self {
        self.rebuild(|i| i.store.capacity = n)
    }

    /// Also accept subscriptions whose endpoint has this exact origin
    /// (`scheme://host[:port]`): a self-hosted push service, or a local
    /// test double. By default only the browsers' own push services
    /// ([`KNOWN_PUSH_HOSTS`], https) are accepted, so a client cannot make
    /// the server POST into its own network.
    pub fn allow_endpoint_origin(self, origin: impl Into<String>) -> Self {
        let o = origin.into();
        self.rebuild(|i| i.policy.extra_origins.push(o))
    }

    /// How long a push service keeps a message for an offline device
    /// (seconds, default one week).
    pub fn ttl(self, secs: u32) -> Self {
        self.rebuild(|i| i.ttl = secs)
    }

    /// The urgency sent with each message (default [`Urgency::Normal`]).
    pub fn urgency(self, u: Urgency) -> Self {
        self.rebuild(|i| i.urgency = u)
    }

    /// The VAPID public key (base64url) browsers subscribe with.
    pub fn public_key(&self) -> &str {
        self.inner.vapid.public_key()
    }

    /// The stored subscriptions.
    pub fn subscriptions(&self) -> Vec<Subscription> {
        self.inner.store.list()
    }

    /// Store `sub` after checking its endpoint and keys; whether it is new
    /// or changed.
    pub fn subscribe(&self, sub: Subscription) -> Result<bool, SubscriptionError> {
        self.inner.policy.check(&sub.endpoint)?;
        sub.key_bytes()?;
        self.inner.store.upsert(sub)
    }

    /// Forget the subscription at `endpoint`; whether there was one.
    pub fn unsubscribe(&self, endpoint: &str) -> bool {
        self.inner.store.remove(&[endpoint]) > 0
    }

    /// Apply what the extension reported (the JSON `ctx.text()` of the
    /// [`report_input`] handler): store or forget the subscription, and
    /// say what the page should show.
    pub fn handle_report(&self, json: &str) -> PushState {
        let r: Report = match serde_json::from_str(json) {
            Ok(r) => r,
            Err(e) => return PushState::Failed(format!("bad report: {e}")),
        };
        match r.state.as_str() {
            "subscribed" => match r.subscription.map(|s| self.subscribe(s)) {
                Some(Ok(_)) => PushState::Subscribed,
                Some(Err(e)) => PushState::Failed(e.to_string()),
                None => PushState::Failed("subscribed without a subscription".into()),
            },
            "unsubscribed" => {
                if let Some(e) = r.endpoint {
                    self.unsubscribe(&e);
                }
                PushState::Unsubscribed
            }
            "denied" => PushState::Denied,
            "unsupported" => PushState::Unsupported,
            "error" => PushState::Failed(r.error.unwrap_or_default()),
            other => PushState::Failed(format!("unknown state {other:?}")),
        }
    }

    /// Send `notification` to every subscriber, a few at a time; remove
    /// the subscriptions the push service says are gone.
    pub async fn send_push(&self, notification: &Notification) -> Result<SendReport, SendError> {
        let payload = serde_json::to_vec(notification).map_err(SendError::Json)?;
        if payload.len() > MAX_PAYLOAD {
            return Err(SendError::Encrypt(EncryptError::TooLarge(payload.len())));
        }
        let subs = self.subscriptions();
        let outcomes: Vec<(String, Delivery)> = stream::iter(subs)
            .map(|s| {
                let payload = &payload;
                async move {
                    let d = self.deliver(&s, payload).await;
                    (s.endpoint, d)
                }
            })
            .buffer_unordered(self.inner.concurrency)
            .collect()
            .await;

        let mut report = SendReport::default();
        let mut gone = Vec::new();
        for (endpoint, d) in outcomes {
            match d {
                Delivery::Accepted => report.accepted += 1,
                Delivery::Gone => gone.push(endpoint),
                Delivery::Failed(why) => report.failed.push((endpoint, why)),
            }
        }
        let gone: Vec<&str> = gone.iter().map(String::as_str).collect();
        report.removed = self.inner.store.remove(&gone);
        Ok(report)
    }

    /// Send raw `payload` bytes to one subscription (no purge).
    pub async fn deliver(&self, sub: &Subscription, payload: &[u8]) -> Delivery {
        let url = match self.inner.policy.check(&sub.endpoint) {
            Ok(u) => u,
            Err(e) => return Delivery::Failed(e.to_string()),
        };
        let body = match sub
            .key_bytes()
            .map_err(|e| e.to_string())
            .and_then(|(p, a)| encrypt(payload, &p, &a).map_err(|e| e.to_string()))
        {
            Ok(b) => b,
            Err(e) => return Delivery::Failed(e),
        };
        let audience = url.origin().ascii_serialization();
        let res = self
            .inner
            .client
            .post(url)
            .header("TTL", self.inner.ttl.to_string())
            .header("Urgency", self.inner.urgency.as_str())
            .header("Content-Encoding", "aes128gcm")
            .header("Content-Type", "application/octet-stream")
            .header(
                "Authorization",
                self.inner
                    .vapid
                    .authorization(&audience, &self.inner.subject),
            )
            .body(body)
            .send()
            .await;
        match res {
            Ok(r) if r.status().is_success() => Delivery::Accepted,
            Ok(r) if r.status().as_u16() == 404 || r.status().as_u16() == 410 => Delivery::Gone,
            Ok(r) => {
                let status = r.status();
                let text = r.text().await.unwrap_or_default();
                let text: String = text.chars().take(200).collect();
                Delivery::Failed(format!("{status}: {text}"))
            }
            Err(e) => Delivery::Failed(e.to_string()),
        }
    }

    /// Write the subscriptions to their file now and wait for it (before
    /// a shutdown; changes are otherwise saved in the background).
    pub async fn flush(&self) -> std::io::Result<()> {
        self.inner.store.flush().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UA_PUBLIC: &str =
        "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4";
    const AUTH: &str = "BTBZMqHH6r4Tts7J_aSIgg";

    fn report(endpoint: &str) -> String {
        serde_json::json!({
            "state": "subscribed",
            "permission": "granted",
            "subscription": {
                "endpoint": endpoint,
                "expirationTime": null,
                "keys": { "p256dh": UA_PUBLIC, "auth": AUTH }
            }
        })
        .to_string()
    }

    fn push() -> Push {
        Push::new(VapidKey::generate(), "mailto:ops@example.org")
    }

    #[test]
    fn a_subscribed_report_is_stored_and_an_unsubscribed_one_forgets_it() {
        let p = push();
        let ep = "https://fcm.googleapis.com/fcm/send/abc";
        assert_eq!(p.handle_report(&report(ep)), PushState::Subscribed);
        assert_eq!(p.subscriptions().len(), 1);
        assert_eq!(p.subscriptions()[0].keys.auth, AUTH);
        let gone = serde_json::json!({"state":"unsubscribed","endpoint":ep,"permission":"granted"});
        assert_eq!(p.handle_report(&gone.to_string()), PushState::Unsubscribed);
        assert!(p.subscriptions().is_empty());
    }

    #[test]
    fn a_report_with_a_foreign_endpoint_is_refused() {
        let p = push();
        let st = p.handle_report(&report("http://10.0.0.1/admin"));
        assert!(
            matches!(st, PushState::Failed(ref m) if m.contains("not an allowed")),
            "{st:?}"
        );
        assert!(p.subscriptions().is_empty(), "nothing stored");
    }

    #[test]
    fn the_other_states_map_through() {
        let p = push();
        assert_eq!(p.handle_report(r#"{"state":"denied"}"#), PushState::Denied);
        assert_eq!(
            p.handle_report(r#"{"state":"unsupported"}"#),
            PushState::Unsupported
        );
        assert_eq!(
            p.handle_report(r#"{"state":"error","error":"AbortError"}"#),
            PushState::Failed("AbortError".into())
        );
        assert!(matches!(p.handle_report("nope"), PushState::Failed(_)));
        assert!(matches!(
            p.handle_report(r#"{"state":"subscribed"}"#),
            PushState::Failed(_)
        ));
    }

    #[test]
    fn the_notification_payload_is_compact_json() {
        let n = Notification::new("New post").body("Hello").url("/posts/1");
        assert_eq!(
            serde_json::to_string(&n).unwrap(),
            r#"{"title":"New post","body":"Hello","url":"/posts/1"}"#
        );
    }

    #[tokio::test]
    async fn an_oversized_notification_is_refused_before_any_request() {
        let p = push();
        let n = Notification::new("x").body("a".repeat(MAX_PAYLOAD));
        assert!(matches!(
            p.send_push(&n).await,
            Err(SendError::Encrypt(EncryptError::TooLarge(_)))
        ));
    }

    #[test]
    fn the_helpers_hint_the_extension() {
        let p = push();
        let html = el(El::Button).push_subscribe(&p).to_static_html();
        assert!(html.contains("data-push=\"subscribe\""), "{html}");
        assert!(html.contains(&format!("data-push-key=\"{}\"", p.public_key())));
        let html = el(El::Button).push_unsubscribe().to_static_html();
        assert!(html.contains("data-push=\"unsubscribe\""), "{html}");
        assert!(PUSH_JS.contains("data-push-report"), "vendored extension");
    }
}
