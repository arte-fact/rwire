//! Delegated authentication: an external authority (an identity hub, an SSO
//! gateway…) decides who a request belongs to, instead of rwire's own
//! user/password form ([`ServerWithRoot::auth`](crate::ServerWithRoot::auth)).
//!
//! The application supplies an async validator through [`DelegatedAuth::new`]
//! and installs it with [`ServerWithRoot::auth_with`](crate::ServerWithRoot::auth_with).
//! rwire calls it for every page request and WebSocket handshake that carries the
//! session cookie, caches **successes only** for a short time, and:
//!
//! - serves the page / upgrades the socket when it returns an [`Identity`], which
//!   handlers then read with [`EventContext::identity`](crate::EventContext::identity);
//! - otherwise redirects a page request to [`DelegatedAuth::login_url`] (with
//!   `?next=<path>`) and refuses a WebSocket handshake with `401`;
//! - re-validates open WebSockets every [`DelegatedAuth::cache_ttl`], closing one
//!   whose identity is revoked (close code [`CLOSE_UNAUTHORIZED`]). When the
//!   validator can't be reached ([`Validation::Unavailable`]), an open socket
//!   stays up for [`DelegatedAuth::outage_grace`], then closes with
//!   [`CLOSE_HUB_UNAVAILABLE`]; new requests are refused meanwhile (fail closed).
//!
//! The validator is a plain `Future + Send` polled on rwire's tokio runtime, so
//! it can use tokio-based clients (reqwest, a hub SDK) directly. It runs once
//! per uncached credential and must not block the thread: do blocking work in
//! `tokio::task::spawn_blocking`.

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// WebSocket close code sent when a re-validation finds the identity revoked
/// (the 4000–4999 range is reserved for applications; 4401 mirrors HTTP 401).
pub const CLOSE_UNAUTHORIZED: u16 = 4401;

/// WebSocket close code sent when the validator stayed unreachable for longer
/// than [`DelegatedAuth::outage_grace`]: 1013, "try again later".
pub const CLOSE_HUB_UNAVAILABLE: u16 = 1013;

/// Who a connection belongs to, as established by the validator.
///
/// `id` is the user (or account) identifier. `data` is opaque to rwire: put there
/// what handlers need to act on the user's behalf, typically the token itself so
/// a handler can call the identity hub with it. `Debug` never prints `data`.
#[derive(Clone)]
pub struct Identity {
    id: String,
    data: Option<Arc<dyn Any + Send + Sync>>,
}

impl Identity {
    /// An identity with no attached data.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            data: None,
        }
    }

    /// Attach opaque data (a token, a claims struct…), read back with [`Self::data`].
    pub fn with_data<T: Any + Send + Sync>(mut self, data: T) -> Self {
        self.data = Some(Arc::new(data));
        self
    }

    /// The identifier the validator returned.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The attached data, if any and if it is a `T`.
    pub fn data<T: Any>(&self) -> Option<&T> {
        self.data.as_deref().and_then(|d| d.downcast_ref::<T>())
    }
}

impl fmt::Debug for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Identity")
            .field("id", &self.id)
            .field("data", &self.data.as_ref().map(|_| "<opaque>"))
            .finish()
    }
}

/// The request a validator judges: method, path and headers of the page request
/// or WebSocket handshake. Owned, so the validator's future can be `'static`.
#[derive(Clone)]
pub struct AuthRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
}

impl AuthRequest {
    /// Parse the head of a raw HTTP request (anything after the blank line is ignored).
    pub(crate) fn from_head(head: &str) -> Self {
        let head = head.split("\r\n\r\n").next().unwrap_or(head);
        let mut lines = head.split("\r\n");
        let mut first = lines.next().unwrap_or("").split(' ');
        let method = first.next().unwrap_or("").to_string();
        let path = first.next().unwrap_or("").to_string();
        let headers = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.trim().to_string(), value.trim().to_string()))
            .collect();
        Self {
            method,
            path,
            headers,
        }
    }

    /// `GET`, `POST`…
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Path and query, as requested (`/inbox?tab=2`).
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The first header with this name (case-insensitive).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// Every header, in request order.
    pub fn headers(&self) -> impl Iterator<Item = (&str, &str)> {
        self.headers.iter().map(|(n, v)| (n.as_str(), v.as_str()))
    }

    /// A cookie's value by name, across every `Cookie` header.
    pub fn cookie(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .filter(|(n, _)| n.eq_ignore_ascii_case("cookie"))
            .flat_map(|(_, v)| v.split(';'))
            .filter_map(|pair| pair.trim().split_once('='))
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.trim())
    }
}

impl fmt::Debug for AuthRequest {
    /// Header names only: cookies and `Authorization` carry credentials.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthRequest")
            .field("method", &self.method)
            .field("path", &self.path)
            .field(
                "headers",
                &self.headers.iter().map(|(n, _)| n).collect::<Vec<_>>(),
            )
            .finish()
    }
}

/// A validator's verdict, for [`DelegatedAuth::with_validation`].
#[derive(Clone, Debug)]
pub enum Validation {
    /// The credential belongs to this identity.
    Valid(Identity),
    /// The authority answered no: unknown, expired or revoked credential. An
    /// open WebSocket closes with [`CLOSE_UNAUTHORIZED`].
    Revoked,
    /// The authority could not be asked (network error, timeout, 5xx): the
    /// credential is neither confirmed nor revoked. New requests are refused;
    /// an open WebSocket gets [`DelegatedAuth::outage_grace`].
    Unavailable,
}

/// `Some` is valid, `None` is revoked: the [`DelegatedAuth::new`] contract.
impl From<Option<Identity>> for Validation {
    fn from(identity: Option<Identity>) -> Self {
        identity.map_or(Validation::Revoked, Validation::Valid)
    }
}

type ValidateFuture = Pin<Box<dyn Future<Output = Validation> + Send>>;
type ValidateFn = dyn Fn(AuthRequest) -> ValidateFuture + Send + Sync;

/// Default lifetime of a cached successful validation.
pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(30);
/// Default bound on cached validations (successes and rejections, each).
pub const DEFAULT_CACHE_CAPACITY: usize = 1024;
/// Default time a validation may take before it counts as a failure.
pub const DEFAULT_VALIDATE_TIMEOUT: Duration = Duration::from_secs(5);
/// Default lifetime of a remembered rejection.
pub const DEFAULT_NEGATIVE_TTL: Duration = Duration::from_secs(5);
/// Default bound on validations running at once.
pub const DEFAULT_MAX_CONCURRENT_VALIDATIONS: usize = 16;
/// Default time an open WebSocket survives an unreachable validator.
pub const DEFAULT_OUTAGE_GRACE: Duration = Duration::from_secs(300);

/// A cache key: the SHA-256 of the credential, never the credential itself,
/// so a memory dump or a stray `Debug` of the cache shows no token.
type Key = [u8; 32];

fn key(token: &str) -> Key {
    use sha2::{Digest, Sha256};
    Sha256::digest(token.as_bytes()).into()
}

/// Validations remembered for a while, keyed by [`Key`].
struct Cache {
    /// Successful validations.
    valid: HashMap<Key, (Identity, Instant)>,
    /// Rejections, kept [`DelegatedAuth::negative_ttl`]: a forged cookie sent
    /// in a loop asks the validator once per that time. Timeouts are never
    /// remembered.
    rejected: HashMap<Key, Instant>,
    /// When expired entries were last swept.
    swept: Instant,
}

type Flight = futures::future::Shared<futures::future::BoxFuture<'static, Validation>>;

/// The state validations share; behind an `Arc` so a validation in flight
/// outlives the request that started it.
struct State {
    cache: Mutex<Cache>,
    /// One validation per credential at a time: concurrent requests with the
    /// same cookie await the same answer.
    in_flight: Mutex<HashMap<Key, Flight>>,
    /// Bounds the validations running at once, across credentials.
    permits: tokio::sync::Semaphore,
}

impl State {
    fn new(max_concurrent: usize) -> Arc<Self> {
        Arc::new(Self {
            cache: Mutex::new(Cache {
                valid: HashMap::new(),
                rejected: HashMap::new(),
                swept: Instant::now(),
            }),
            in_flight: Mutex::new(HashMap::new()),
            permits: tokio::sync::Semaphore::new(max_concurrent),
        })
    }
}

/// The TTLs and bound a cache operation needs.
#[derive(Clone, Copy)]
struct Limits {
    ttl: Duration,
    negative_ttl: Duration,
    capacity: usize,
}

impl Cache {
    /// Sweep expired entries, once per TTL (the shorter of the two): they
    /// don't linger until the cache happens to fill up.
    fn sweep_if_due(&mut self, limits: Limits) {
        let every = if limits.negative_ttl.is_zero() {
            limits.ttl
        } else {
            limits.ttl.min(limits.negative_ttl)
        };
        if self.swept.elapsed() >= every {
            self.valid.retain(|_, (_, at)| at.elapsed() < limits.ttl);
            self.rejected
                .retain(|_, at| at.elapsed() < limits.negative_ttl);
            self.swept = Instant::now();
        }
    }

    fn lookup(&self, key: &Key, limits: Limits) -> Option<Validation> {
        if let Some((identity, at)) = self.valid.get(key) {
            if at.elapsed() < limits.ttl {
                return Some(Validation::Valid(identity.clone()));
            }
        }
        match self.rejected.get(key) {
            Some(at) if at.elapsed() < limits.negative_ttl => Some(Validation::Revoked),
            _ => None,
        }
    }

    fn record(&mut self, key: Key, outcome: &Validation, limits: Limits) {
        match outcome {
            Validation::Valid(identity) => {
                self.rejected.remove(&key);
                insert_bounded(
                    &mut self.valid,
                    key,
                    (identity.clone(), Instant::now()),
                    limits.capacity,
                    |(_, at)| *at,
                );
            }
            Validation::Revoked if !limits.negative_ttl.is_zero() => {
                // A rejection never overrides a success still in the cache.
                self.valid.remove(&key);
                insert_bounded(
                    &mut self.rejected,
                    key,
                    Instant::now(),
                    limits.capacity,
                    |at| *at,
                );
            }
            Validation::Revoked | Validation::Unavailable => {}
        }
    }
}

/// Insert into a map bounded to `capacity`, dropping the oldest entry first.
fn insert_bounded<V>(
    map: &mut HashMap<Key, V>,
    key: Key,
    value: V,
    capacity: usize,
    at: impl Fn(&V) -> Instant,
) {
    if map.len() >= capacity && !map.contains_key(&key) {
        if let Some(oldest) = map.iter().min_by_key(|(_, v)| at(v)).map(|(k, _)| *k) {
            map.remove(&oldest);
        }
    }
    map.insert(key, value);
}

/// Delegated authentication settings and cache. See the [module docs](self).
pub struct DelegatedAuth {
    cookie: String,
    validate: Arc<ValidateFn>,
    login_url: Option<String>,
    cache_ttl: Duration,
    cache_capacity: usize,
    timeout: Duration,
    negative_ttl: Duration,
    max_concurrent: usize,
    outage_grace: Duration,
    state: Arc<State>,
}

impl DelegatedAuth {
    /// Delegate authentication of the session cookie `cookie` to `validate`.
    ///
    /// `validate` receives the whole request head (cookies, `Authorization`,
    /// forwarded headers…) and returns the [`Identity`] or `None`. A request
    /// without the cookie is anonymous: it never reaches the validator.
    ///
    /// `None` counts as a revocation, so a validator that returns `None` when
    /// the authority is down closes open WebSockets at the next check. Use
    /// [`Self::with_validation`] to tell the two apart.
    pub fn new<V, Fut>(cookie: impl Into<String>, validate: V) -> Self
    where
        V: Fn(AuthRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Option<Identity>> + Send + 'static,
    {
        use futures::FutureExt;
        Self::with_validation(cookie, move |request| {
            validate(request).map(Validation::from)
        })
    }

    /// Like [`Self::new`], with a validator that tells a revoked credential
    /// ([`Validation::Revoked`]) from an authority it couldn't reach
    /// ([`Validation::Unavailable`]): an open WebSocket then survives an outage
    /// for [`Self::outage_grace`] instead of closing at the next check.
    pub fn with_validation<V, Fut>(cookie: impl Into<String>, validate: V) -> Self
    where
        V: Fn(AuthRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Validation> + Send + 'static,
    {
        Self {
            cookie: cookie.into(),
            validate: Arc::new(move |request| Box::pin(validate(request)) as ValidateFuture),
            login_url: None,
            cache_ttl: DEFAULT_CACHE_TTL,
            cache_capacity: DEFAULT_CACHE_CAPACITY,
            timeout: DEFAULT_VALIDATE_TIMEOUT,
            negative_ttl: DEFAULT_NEGATIVE_TTL,
            max_concurrent: DEFAULT_MAX_CONCURRENT_VALIDATIONS,
            outage_grace: DEFAULT_OUTAGE_GRACE,
            state: State::new(DEFAULT_MAX_CONCURRENT_VALIDATIONS),
        }
    }

    /// Where a page request without a valid identity is sent, with a `303`.
    ///
    /// rwire appends `next=<path>` to it (after `?`, or `&` if the URL already
    /// has a query), the requested path and query percent-encoded: for
    /// `login_url("https://hub.example/login")`, a request to `/inbox?tab=2`
    /// goes to `https://hub.example/login?next=%2Finbox%3Ftab%3D2`. `next` is a
    /// path on *this* app, not a full URL: a login page on another origin sends
    /// the user back to the app's own origin plus `next`.
    ///
    /// The URL may be absolute (another origin) or a path on this app
    /// (`/login`). In the latter case the login page itself is answered `401`
    /// instead of being redirected to itself.
    ///
    /// When the app is mounted under a prefix (`base_path`, a proxy strips it),
    /// `next` carries the prefix: a request for `/rwire/s/x` sends
    /// `next=%2Frwire%2Fs%2Fx`, the path the browser must come back to.
    ///
    /// Without a login URL, such requests get a plain `401`. WebSocket
    /// handshakes always get `401`.
    pub fn login_url(mut self, url: impl Into<String>) -> Self {
        self.login_url = Some(url.into());
        self
    }

    /// How long a successful validation is reused (default 30 s), which is also
    /// the interval at which open WebSockets are re-validated.
    pub fn cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// The most validations kept at once (default 1024), for successes and for
    /// rejections each. Expired entries are swept once per TTL; when the cache
    /// is still full, the oldest goes.
    pub fn cache_capacity(mut self, capacity: usize) -> Self {
        self.cache_capacity = capacity.max(1);
        self
    }

    /// How long a validation may take before it counts as a failure (default
    /// 5 s), waiting for a free slot ([`Self::max_concurrent_validations`])
    /// included. A timeout is never remembered.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// How long a rejection is remembered (default 5 s): a forged or revoked
    /// cookie sent again within that time is refused without asking the
    /// validator. `Duration::ZERO` asks every time. A rejection never
    /// replaces a success, and a success clears it.
    pub fn negative_ttl(mut self, ttl: Duration) -> Self {
        self.negative_ttl = ttl;
        self
    }

    /// The most validations running at once (default 16), across all
    /// credentials. Requests beyond it wait for a slot within
    /// [`Self::timeout`], then fail closed: a flood of forged cookies can't
    /// multiply calls to the validator.
    pub fn max_concurrent_validations(mut self, max: usize) -> Self {
        self.max_concurrent = max.max(1);
        self.state = State::new(self.max_concurrent);
        self
    }

    /// How long an open WebSocket stays up while the validator can't be reached
    /// (default 5 min), counted from its last confirmed validation. Past it, the
    /// socket closes with [`CLOSE_HUB_UNAVAILABLE`]. A revocation closes it at
    /// once whatever this is; new requests are refused during an outage.
    pub fn outage_grace(mut self, grace: Duration) -> Self {
        self.outage_grace = grace;
        self
    }

    pub(crate) fn outage_grace_period(&self) -> Duration {
        self.outage_grace
    }

    pub(crate) fn revalidate_every(&self) -> Duration {
        self.cache_ttl.max(Duration::from_millis(100))
    }

    fn limits(&self) -> Limits {
        Limits {
            ttl: self.cache_ttl,
            negative_ttl: self.negative_ttl,
            capacity: self.cache_capacity,
        }
    }

    /// The identity of the request whose head is `head`, from the cache or the
    /// validator. Never logs the cookie.
    pub(crate) async fn identify(&self, head: &str) -> Option<Identity> {
        match self.check(AuthRequest::from_head(head)).await {
            Validation::Valid(identity) => Some(identity),
            Validation::Revoked | Validation::Unavailable => None,
        }
    }

    /// The part of a handshake kept for re-validating its WebSocket: the method,
    /// the path and the session cookie only. Other cookies, `Authorization` and
    /// the rest of the head are not kept for the socket's lifetime.
    pub(crate) fn revalidation_head(&self, head: &str) -> String {
        let request = AuthRequest::from_head(head);
        let cookie = request
            .cookie(&self.cookie)
            .map(|token| format!("Cookie: {}={token}\r\n", self.cookie))
            .unwrap_or_default();
        format!(
            "{} {} HTTP/1.1\r\n{cookie}\r\n",
            request.method(),
            request.path()
        )
    }

    /// Whether the request whose head is `head` carries the session cookie
    /// (an anonymous request is not a failed one).
    pub(crate) fn has_credential(&self, head: &str) -> bool {
        AuthRequest::from_head(head)
            .cookie(&self.cookie)
            .is_some_and(|t| !t.is_empty())
    }

    /// Re-validate an open WebSocket's credentials (`head` is its handshake):
    /// the verdict, so the caller can tell a revocation from an outage.
    pub(crate) async fn revalidate(&self, head: &str) -> Validation {
        self.check(AuthRequest::from_head(head)).await
    }

    async fn check(&self, request: AuthRequest) -> Validation {
        let Some(token) = request.cookie(&self.cookie).filter(|t| !t.is_empty()) else {
            return Validation::Revoked;
        };
        let key = key(token);
        let limits = self.limits();
        {
            let mut cache = self
                .state
                .cache
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            cache.sweep_if_due(limits);
            if let Some(known) = cache.lookup(&key, limits) {
                return known;
            }
        }
        let flight = {
            let mut in_flight = self
                .state
                .in_flight
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            in_flight
                .entry(key)
                .or_insert_with(|| self.flight(key, request, limits))
                .clone()
        };
        flight.await
    }

    /// The one validation of `key`, shared by every request that asks while it
    /// runs: it takes a slot, asks the validator, remembers the answer and
    /// leaves the in-flight table.
    fn flight(&self, key: Key, request: AuthRequest, limits: Limits) -> Flight {
        use futures::FutureExt;
        let state = Arc::clone(&self.state);
        let validate = Arc::clone(&self.validate);
        let timeout = self.timeout;
        async move {
            let deadline = tokio::time::Instant::now() + timeout;
            let answer = tokio::time::timeout_at(deadline, async {
                let _slot = state.permits.acquire().await.ok()?;
                // A slot freed by a validation that just timed out wakes this
                // waiter before its own deadline is checked: re-check it, or
                // every waiter of a flood would call the validator in turn.
                if tokio::time::Instant::now() >= deadline {
                    return None;
                }
                Some(validate(request).await)
            })
            .await;
            let outcome = match answer {
                Ok(Some(verdict)) => verdict,
                Ok(None) | Err(_) => Validation::Unavailable,
            };
            state
                .cache
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .record(key, &outcome, limits);
            state
                .in_flight
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&key);
            outcome
        }
        .boxed()
        .shared()
    }

    #[cfg(test)]
    fn cache_len(&self) -> usize {
        self.state
            .cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .valid
            .len()
    }

    #[cfg(test)]
    fn cache_keys(&self) -> Vec<Key> {
        let cache = self
            .state
            .cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        cache
            .valid
            .keys()
            .chain(cache.rejected.keys())
            .copied()
            .collect()
    }

    /// The redirect target for an anonymous page request to `path`, if a login
    /// URL is configured and is not the requested page itself: a login URL on
    /// this app's origin (a path, or an absolute URL whose host is `host`, the
    /// request's `Host`) that names `path` would redirect to itself forever.
    ///
    /// `base` is the URL prefix the app is mounted under (`/rwire`, or empty):
    /// a proxy strips it before rwire sees `path`, so it is put back, both in
    /// `next` (where the browser must return) and for the loop check (the
    /// login page's path is a path on the origin, not on the app).
    pub(crate) fn login_redirect(
        &self,
        path: &str,
        host: Option<&str>,
        base: &str,
    ) -> Option<String> {
        let path = format!("{base}{path}");
        let path = path.as_str();
        let url = self.login_url.as_deref()?;
        let login_path = match url.split_once("://") {
            Some((_, rest)) => {
                let (authority, tail) =
                    rest.split_at(rest.find(['/', '?', '#']).unwrap_or(rest.len()));
                let same_origin = host.is_some_and(|h| h.eq_ignore_ascii_case(authority));
                same_origin.then_some(if tail.is_empty() { "/" } else { tail })
            }
            None => Some(url),
        };
        let bare = |p: &str| p.split(['?', '#']).next().unwrap_or("").to_string();
        if login_path.is_some_and(|login| bare(login) == bare(path)) {
            return None;
        }
        let sep = if url.contains('?') { '&' } else { '?' };
        Some(format!("{url}{sep}next={}", percent_encode(path)))
    }
}

impl fmt::Debug for DelegatedAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DelegatedAuth")
            .field("cookie", &self.cookie)
            .field("login_url", &self.login_url)
            .field("cache_ttl", &self.cache_ttl)
            .field("cache_capacity", &self.cache_capacity)
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
}

/// Percent-encode everything but RFC 3986 unreserved characters.
fn percent_encode(input: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn head(cookie: &str) -> String {
        format!("GET /inbox?tab=2 HTTP/1.1\r\nHost: x\r\nCookie: other=1; sid={cookie}\r\n\r\n")
    }

    fn counting(calls: Arc<AtomicUsize>) -> DelegatedAuth {
        DelegatedAuth::new("sid", move |req: AuthRequest| {
            calls.fetch_add(1, Ordering::SeqCst);
            let ok = req.cookie("sid") == Some("good");
            async move { ok.then(|| Identity::new("ada").with_data("good".to_string())) }
        })
    }

    #[test]
    fn request_head_is_parsed() {
        let req = AuthRequest::from_head(&head("good"));
        assert_eq!(req.method(), "GET");
        assert_eq!(req.path(), "/inbox?tab=2");
        assert_eq!(req.header("HOST"), Some("x"));
        assert_eq!(req.cookie("sid"), Some("good"));
        assert_eq!(req.cookie("other"), Some("1"));
        assert_eq!(req.cookie("missing"), None);
    }

    #[test]
    fn debug_output_never_shows_credentials() {
        let req = AuthRequest::from_head(&head("s3cret"));
        let identity = Identity::new("ada").with_data("s3cret".to_string());
        let printed = format!("{req:?} {identity:?}");
        assert!(!printed.contains("s3cret"), "{printed}");
        assert!(printed.contains("ada"));
    }

    #[tokio::test]
    async fn successes_are_cached_failures_are_not() {
        let calls = Arc::new(AtomicUsize::new(0));
        let auth = counting(Arc::clone(&calls)).negative_ttl(Duration::ZERO);
        for _ in 0..5 {
            let identity = auth.identify(&head("good")).await.expect("valid");
            assert_eq!(identity.id(), "ada");
            assert_eq!(identity.data::<String>().map(String::as_str), Some("good"));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1, "one call for N requests");

        for _ in 0..3 {
            assert!(auth.identify(&head("bad")).await.is_none());
        }
        assert_eq!(calls.load(Ordering::SeqCst), 4, "every failure re-asks");
    }

    /// A rejection is remembered for the negative TTL, then asked again.
    #[tokio::test]
    async fn rejections_are_remembered_briefly() {
        let calls = Arc::new(AtomicUsize::new(0));
        let auth = counting(Arc::clone(&calls)).negative_ttl(Duration::from_millis(50));
        for _ in 0..10 {
            assert!(auth.identify(&head("forged")).await.is_none());
        }
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "one call for 10 rejections"
        );
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert!(auth.identify(&head("forged")).await.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 2, "asked again after the TTL");
        // A valid cookie is not affected.
        assert!(auth.identify(&head("good")).await.is_some());
    }

    /// A validator that answers after `delay`, counting calls and the most
    /// running at once.
    fn slow(delay: Duration, calls: Arc<AtomicUsize>, peak: Arc<AtomicUsize>) -> DelegatedAuth {
        let running = Arc::new(AtomicUsize::new(0));
        DelegatedAuth::new("sid", move |req: AuthRequest| {
            calls.fetch_add(1, Ordering::SeqCst);
            let now = running.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(now, Ordering::SeqCst);
            let running = Arc::clone(&running);
            let ok = req.cookie("sid") == Some("good");
            async move {
                tokio::time::sleep(delay).await;
                running.fetch_sub(1, Ordering::SeqCst);
                ok.then(|| Identity::new("ada"))
            }
        })
    }

    #[tokio::test]
    async fn concurrent_requests_with_one_cookie_share_one_validation() {
        let (calls, peak) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let auth = Arc::new(slow(Duration::from_millis(50), calls.clone(), peak));
        for cookie in ["forged", "good"] {
            let tasks: Vec<_> = (0..50)
                .map(|_| {
                    let auth = Arc::clone(&auth);
                    tokio::spawn(async move { auth.identify(&head(cookie)).await.is_some() })
                })
                .collect();
            for task in tasks {
                assert_eq!(task.await.unwrap(), cookie == "good");
            }
        }
        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "one call per cookie, not 100"
        );
    }

    #[tokio::test]
    async fn validations_run_at_most_max_concurrent_at_once() {
        let (calls, peak) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let auth = Arc::new(
            slow(Duration::from_millis(20), calls.clone(), peak.clone())
                .max_concurrent_validations(4),
        );
        let tasks: Vec<_> = (0..40)
            .map(|i| {
                let auth = Arc::clone(&auth);
                tokio::spawn(async move { auth.identify(&head(&format!("forged-{i}"))).await })
            })
            .collect();
        for task in tasks {
            assert!(task.await.unwrap().is_none());
        }
        assert_eq!(
            calls.load(Ordering::SeqCst),
            40,
            "distinct cookies are each asked"
        );
        assert!(
            peak.load(Ordering::SeqCst) <= 4,
            "peak {}",
            peak.load(Ordering::SeqCst)
        );
    }

    /// A flood of distinct forged cookies against a stalled validator: the
    /// excess times out waiting for a slot (fail closed) and never calls it.
    #[tokio::test]
    async fn a_flood_fails_closed_without_calling_the_validator_more() {
        let (calls, peak) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let auth = Arc::new(
            slow(Duration::from_secs(10), calls.clone(), peak)
                .max_concurrent_validations(2)
                .timeout(Duration::from_millis(100)),
        );
        let tasks: Vec<_> = (0..30)
            .map(|i| {
                let auth = Arc::clone(&auth);
                tokio::spawn(async move { auth.identify(&head(&format!("f{i}"))).await })
            })
            .collect();
        for task in tasks {
            assert!(task.await.unwrap().is_none());
        }
        assert!(
            calls.load(Ordering::SeqCst) <= 2,
            "{} calls",
            calls.load(Ordering::SeqCst)
        );
    }

    /// `with_validation`: an unreachable authority is never remembered, a
    /// revocation is; `revalidate` reports which one it was.
    #[tokio::test]
    async fn unavailable_is_not_a_revocation() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&calls);
        let auth = DelegatedAuth::with_validation("sid", move |req: AuthRequest| {
            counted.fetch_add(1, Ordering::SeqCst);
            let verdict = match req.cookie("sid") {
                Some("good") => Validation::Valid(Identity::new("ada")),
                Some("down") => Validation::Unavailable,
                _ => Validation::Revoked,
            };
            async move { verdict }
        });
        for _ in 0..3 {
            assert!(matches!(
                auth.revalidate(&head("down")).await,
                Validation::Unavailable
            ));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 3, "an outage is asked again");
        for _ in 0..3 {
            assert!(matches!(
                auth.revalidate(&head("gone")).await,
                Validation::Revoked
            ));
        }
        assert_eq!(
            calls.load(Ordering::SeqCst),
            4,
            "a revocation is remembered"
        );
        assert!(matches!(
            auth.revalidate(&head("good")).await,
            Validation::Valid(_)
        ));
        assert!(auth.identify(&head("down")).await.is_none(), "fail closed");
    }

    #[test]
    fn option_validators_keep_their_meaning() {
        assert!(matches!(Validation::from(None), Validation::Revoked));
        assert!(matches!(
            Validation::from(Some(Identity::new("ada"))),
            Validation::Valid(i) if i.id() == "ada"
        ));
    }

    /// A timeout is not a rejection: the next request asks again.
    #[tokio::test]
    async fn timeouts_are_never_remembered() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&calls);
        let auth = DelegatedAuth::new("sid", move |_req: AuthRequest| {
            let first = counted.fetch_add(1, Ordering::SeqCst) == 0;
            async move {
                if first {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
                Some(Identity::new("ada"))
            }
        })
        .timeout(Duration::from_millis(30));
        assert!(auth.identify(&head("good")).await.is_none());
        assert!(auth.identify(&head("good")).await.is_some());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn expired_success_is_validated_again() {
        let calls = Arc::new(AtomicUsize::new(0));
        let auth = counting(Arc::clone(&calls)).cache_ttl(Duration::from_millis(30));
        assert!(auth.identify(&head("good")).await.is_some());
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert!(auth.identify(&head("good")).await.is_some());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn missing_cookie_never_reaches_the_validator() {
        let calls = Arc::new(AtomicUsize::new(0));
        let auth = counting(Arc::clone(&calls));
        let anonymous = "GET / HTTP/1.1\r\nHost: x\r\n\r\n";
        let empty = "GET / HTTP/1.1\r\nCookie: sid=\r\n\r\n";
        assert!(auth.identify(anonymous).await.is_none());
        assert!(auth.identify(empty).await.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn cache_is_bounded() {
        let auth = DelegatedAuth::new("sid", |req: AuthRequest| {
            let id = req.cookie("sid").unwrap_or("").to_string();
            async move { Some(Identity::new(id)) }
        })
        .cache_capacity(3);
        for i in 0..10 {
            assert!(auth.identify(&head(&format!("t{i}"))).await.is_some());
        }
        assert_eq!(auth.cache_len(), 3);
    }

    /// The cache holds SHA-256 digests, never the token; nothing of it shows
    /// in `Debug` either.
    #[tokio::test]
    async fn cache_keys_are_hashes_of_the_token() {
        let token = "s3cret-session-token";
        let auth = DelegatedAuth::new("sid", |_req: AuthRequest| async {
            Some(Identity::new("ada"))
        });
        assert!(auth.identify(&head(token)).await.is_some());
        let keys = auth.cache_keys();
        assert_eq!(keys, vec![key(token)]);
        // SHA-256 of "abc" (FIPS 180-2 test vector).
        assert_eq!(
            key("abc"),
            [
                0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
                0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
                0xf2, 0x00, 0x15, 0xad
            ]
        );
        let bytes: Vec<u8> = keys.concat();
        assert!(
            !bytes.windows(token.len()).any(|w| w == token.as_bytes()),
            "the token is stored in clear"
        );
        let printed = format!("{auth:?}");
        assert!(!printed.contains(token), "{printed}");
    }

    /// Expired entries go on the next access after a TTL, not only when the
    /// cache is full.
    #[tokio::test]
    async fn expired_entries_are_swept() {
        let auth = DelegatedAuth::new("sid", |req: AuthRequest| {
            let id = req.cookie("sid").unwrap_or("").to_string();
            async move { Some(Identity::new(id)) }
        })
        .cache_ttl(Duration::from_millis(30));
        for i in 0..5 {
            assert!(auth.identify(&head(&format!("t{i}"))).await.is_some());
        }
        assert_eq!(auth.cache_len(), 5);
        tokio::time::sleep(Duration::from_millis(60)).await;
        // Any access sweeps; anonymous requests never touch the cache, so use
        // one token: the four others are gone, the new one is cached.
        assert!(auth.identify(&head("fresh")).await.is_some());
        assert_eq!(auth.cache_len(), 1);
    }

    #[tokio::test]
    async fn slow_validator_counts_as_failure() {
        let auth = DelegatedAuth::new("sid", |_req: AuthRequest| async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            Some(Identity::new("late"))
        })
        .timeout(Duration::from_millis(20));
        assert!(auth.identify(&head("good")).await.is_none());
    }

    #[test]
    fn login_redirect_encodes_next() {
        let auth = counting(Arc::new(AtomicUsize::new(0)));
        assert_eq!(auth.login_redirect("/", None, ""), None);
        let auth = auth.login_url("https://hub.example/login");
        assert_eq!(
            auth.login_redirect("/inbox?tab=2&x=a b", Some("app.example"), "")
                .as_deref(),
            Some("https://hub.example/login?next=%2Finbox%3Ftab%3D2%26x%3Da%20b")
        );
        let auth = counting(Arc::new(AtomicUsize::new(0))).login_url("/login?app=voice");
        assert_eq!(
            auth.login_redirect("/", None, "").as_deref(),
            Some("/login?app=voice&next=%2F")
        );
    }

    /// A login page on this app's origin is not redirected to itself.
    #[test]
    fn login_redirect_never_loops_on_its_own_page() {
        let relative = counting(Arc::new(AtomicUsize::new(0))).login_url("/login?app=voice");
        assert_eq!(relative.login_redirect("/login", None, ""), None);
        assert_eq!(relative.login_redirect("/login?next=%2F", None, ""), None);
        assert!(relative.login_redirect("/inbox", None, "").is_some());

        let absolute =
            counting(Arc::new(AtomicUsize::new(0))).login_url("https://app.example/login");
        assert_eq!(
            absolute.login_redirect("/login", Some("APP.example"), ""),
            None
        );
        assert!(absolute
            .login_redirect("/inbox", Some("app.example"), "")
            .is_some());
        // Another origin's /login is not this app's page.
        assert!(absolute
            .login_redirect("/login", Some("other.example"), "")
            .is_some());
        assert!(absolute.login_redirect("/login", None, "").is_some());
    }

    /// Under a base path, `next` is the path the browser asked for (prefix
    /// included), and only the origin's own login page is not redirected.
    #[test]
    fn login_redirect_puts_the_base_path_back() {
        let auth = counting(Arc::new(AtomicUsize::new(0))).login_url("/login?next=rwire");
        assert_eq!(
            auth.login_redirect("/s/office/x", None, "/rwire")
                .as_deref(),
            Some("/login?next=rwire&next=%2Frwire%2Fs%2Foffice%2Fx")
        );
        assert_eq!(
            auth.login_redirect("/", None, "/rwire").as_deref(),
            Some("/login?next=rwire&next=%2Frwire%2F")
        );
        // The app's own `/login` (`/rwire/login` on the origin) is not the
        // login page: it is redirected; without the prefix it would loop.
        assert!(auth.login_redirect("/login", None, "/rwire").is_some());
        assert_eq!(auth.login_redirect("/login", None, ""), None);
    }

    /// The re-validation head keeps the session cookie, not the rest.
    #[test]
    fn revalidation_keeps_only_the_session_cookie() {
        let auth = counting(Arc::new(AtomicUsize::new(0)));
        let head = "GET /ws HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer hub-secret\r\n\
                    Cookie: tracking=abc; sid=good; theme=dark\r\nUser-Agent: test\r\n\r\n";
        let kept = auth.revalidation_head(head);
        assert_eq!(kept, "GET /ws HTTP/1.1\r\nCookie: sid=good\r\n\r\n");
        let request = AuthRequest::from_head(&kept);
        assert_eq!(request.cookie("sid"), Some("good"));
        assert!(!kept.contains("hub-secret") && !kept.contains("tracking"));
    }
}
