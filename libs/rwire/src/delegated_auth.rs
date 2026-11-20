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
//!   whose identity is gone (close code [`CLOSE_UNAUTHORIZED`]).
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

type ValidateFuture = Pin<Box<dyn Future<Output = Option<Identity>> + Send>>;
type ValidateFn = dyn Fn(AuthRequest) -> ValidateFuture + Send + Sync;

/// Default lifetime of a cached successful validation.
pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(30);
/// Default bound on cached validations.
pub const DEFAULT_CACHE_CAPACITY: usize = 1024;
/// Default time a validation may take before it counts as a failure.
pub const DEFAULT_VALIDATE_TIMEOUT: Duration = Duration::from_secs(5);

/// Delegated authentication settings and cache. See the [module docs](self).
pub struct DelegatedAuth {
    cookie: String,
    validate: Arc<ValidateFn>,
    login_url: Option<String>,
    cache_ttl: Duration,
    cache_capacity: usize,
    timeout: Duration,
    /// Successful validations only, keyed by cookie value.
    cache: Mutex<HashMap<String, (Identity, Instant)>>,
}

impl DelegatedAuth {
    /// Delegate authentication of the session cookie `cookie` to `validate`.
    ///
    /// `validate` receives the whole request head (cookies, `Authorization`,
    /// forwarded headers…) and returns the [`Identity`] or `None`. A request
    /// without the cookie is anonymous: it never reaches the validator.
    pub fn new<V, Fut>(cookie: impl Into<String>, validate: V) -> Self
    where
        V: Fn(AuthRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Option<Identity>> + Send + 'static,
    {
        Self {
            cookie: cookie.into(),
            validate: Arc::new(move |request| Box::pin(validate(request)) as ValidateFuture),
            login_url: None,
            cache_ttl: DEFAULT_CACHE_TTL,
            cache_capacity: DEFAULT_CACHE_CAPACITY,
            timeout: DEFAULT_VALIDATE_TIMEOUT,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Where an anonymous page request is sent (`303`), with `next=<path>` appended
    /// (percent-encoded, after `?` or `&`). The path is relative to this server:
    /// a login page on another origin must resolve it against the app's own URL.
    /// Without a login URL, anonymous page requests get a plain `401`.
    pub fn login_url(mut self, url: impl Into<String>) -> Self {
        self.login_url = Some(url.into());
        self
    }

    /// How long a successful validation is reused (default 30 s), which is also
    /// the interval at which open WebSockets are re-validated. Failures are never
    /// cached.
    pub fn cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// The most validations kept at once (default 1024). When full, expired
    /// entries go first, then the oldest one.
    pub fn cache_capacity(mut self, capacity: usize) -> Self {
        self.cache_capacity = capacity.max(1);
        self
    }

    /// How long a validation may take before it counts as a failure (default 5 s).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub(crate) fn revalidate_every(&self) -> Duration {
        self.cache_ttl.max(Duration::from_millis(100))
    }

    /// The identity of the request whose head is `head`, from the cache or the
    /// validator. Never logs the cookie.
    pub(crate) async fn identify(&self, head: &str) -> Option<Identity> {
        let request = AuthRequest::from_head(head);
        let token = request.cookie(&self.cookie).filter(|t| !t.is_empty())?;
        let token = token.to_string();
        if let Some(identity) = self.cached(&token) {
            return Some(identity);
        }
        let validation = (self.validate)(request);
        let identity = tokio::time::timeout(self.timeout, validation)
            .await
            .ok()
            .flatten()?;
        self.remember(token, identity.clone());
        Some(identity)
    }

    fn cached(&self, token: &str) -> Option<Identity> {
        let cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        cache
            .get(token)
            .filter(|(_, at)| at.elapsed() < self.cache_ttl)
            .map(|(identity, _)| identity.clone())
    }

    fn remember(&self, token: String, identity: Identity) {
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        if cache.len() >= self.cache_capacity && !cache.contains_key(&token) {
            let ttl = self.cache_ttl;
            cache.retain(|_, (_, at)| at.elapsed() < ttl);
            if cache.len() >= self.cache_capacity {
                let oldest = cache
                    .iter()
                    .min_by_key(|(_, (_, at))| *at)
                    .map(|(k, _)| k.clone());
                if let Some(oldest) = oldest {
                    cache.remove(&oldest);
                }
            }
        }
        cache.insert(token, (identity, Instant::now()));
    }

    #[cfg(test)]
    fn cache_len(&self) -> usize {
        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    /// The redirect target for an anonymous page request to `path`, if a login
    /// URL is configured.
    pub(crate) fn login_redirect(&self, path: &str) -> Option<String> {
        let url = self.login_url.as_deref()?;
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
        let auth = counting(Arc::clone(&calls));
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
        assert_eq!(auth.login_redirect("/"), None);
        let auth = auth.login_url("https://hub.example/login");
        assert_eq!(
            auth.login_redirect("/inbox?tab=2&x=a b").as_deref(),
            Some("https://hub.example/login?next=%2Finbox%3Ftab%3D2%26x%3Da%20b")
        );
        let auth = counting(Arc::new(AtomicUsize::new(0))).login_url("/login?app=voice");
        assert_eq!(
            auth.login_redirect("/").as_deref(),
            Some("/login?app=voice&next=%2F")
        );
    }
}
