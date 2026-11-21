//! Server configuration for connection limits and timeouts.

use std::sync::Arc;
use std::time::Duration;

/// One reverse-proxy routing decision for a request path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyDecision {
    /// Forward to a local upstream on `127.0.0.1:port`, stripping the leading `prefix` path
    /// segment so the upstream sees a root-relative path.
    Forward { port: u16, prefix: String },
    /// The path belongs to the proxy's namespace but has no live upstream. The server answers
    /// with a small human-facing 503 page carrying `message` — it must NOT fall through to the
    /// app: a stale deep link (e.g. a dead preview bookmarked or held open in an installed PWA)
    /// would otherwise open the app shell where the linked service was expected, which reads as
    /// the app mysteriously relaunching.
    Gone { message: String },
}

/// Resolves a request path to a reverse-proxy decision — or `None` for a normal request the
/// server should handle itself.
///
/// The mechanism lives here; the *policy* (which paths map to which upstreams) is the host app's
/// closure. It's how one origin can front several sibling servers behind a single auth gate — e.g.
/// a preview supervisor mapping `/preview/<id>/…` to a pooled port (claw's P2 CD flow).
type ProxyFn = dyn Fn(&str) -> Option<ProxyDecision> + Send + Sync;

#[derive(Clone)]
pub struct ProxyResolver(Arc<ProxyFn>);

impl ProxyResolver {
    /// Wrap a path→[`ProxyDecision`] resolver.
    pub fn new(resolve: impl Fn(&str) -> Option<ProxyDecision> + Send + Sync + 'static) -> Self {
        Self(Arc::new(resolve))
    }

    /// Resolve `path` to a decision, or `None` to handle it normally.
    #[must_use]
    pub fn resolve(&self, path: &str) -> Option<ProxyDecision> {
        (self.0)(path)
    }
}

impl std::fmt::Debug for ProxyResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProxyResolver(..)")
    }
}

/// Configuration for the rwire server.
///
/// Controls connection limits, timeouts, and resource constraints.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Maximum total concurrent connections. Default: 10,000
    pub max_connections: usize,

    /// Maximum connections from a single IP address. Default: 100
    pub max_connections_per_ip: usize,

    /// Disconnect clients after this duration of inactivity. Default: 5 minutes
    pub idle_timeout: Duration,

    /// Maximum memory per connection state in bytes. Default: 1MB
    pub state_memory_limit: usize,

    /// Extra origins allowed to open WebSocket connections, as full origin
    /// strings (e.g. `https://app.example.com`). Same-origin requests (the
    /// `Origin` header's host matching the request `Host`) are always allowed,
    /// and non-browser clients without an `Origin` header are not blocked —
    /// this list is for legitimate cross-origin setups (a page on another
    /// domain embedding this app). Default: empty.
    pub allowed_origins: Vec<String>,

    /// Force the `Secure` attribute on the session cookie regardless of the
    /// request scheme. Default: false.
    ///
    /// Normally this is unnecessary: the server auto-detects HTTPS from the
    /// proxy's `X-Forwarded-Proto` header and marks the cookie `Secure` then,
    /// while leaving it off for plain-HTTP dev (where a `Secure` cookie would be
    /// dropped by the browser). Set this only to force `Secure` on in a setup
    /// that doesn't send that header.
    pub secure_cookies: bool,

    /// Reverse-proxy resolver, consulted for every authenticated request (after the auth gate, so
    /// proxied upstreams inherit it). `None` (default) disables proxying.
    pub proxy: Option<ProxyResolver>,

    /// Serve `/metrics` before the auth gate even when one is set. Default:
    /// false, so with `auth()` / `auth_with()` the metrics need the same
    /// session as the page (without a gate they are public). `/health` and
    /// `/ready` are always public.
    pub public_metrics: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            max_connections: 10_000,
            max_connections_per_ip: 100,
            idle_timeout: Duration::from_secs(300),
            state_memory_limit: 1024 * 1024, // 1MB
            allowed_origins: Vec::new(),
            secure_cookies: false,
            proxy: None,
            public_metrics: false,
        }
    }
}

impl ServerConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum total concurrent connections.
    pub fn max_connections(mut self, limit: usize) -> Self {
        self.max_connections = limit;
        self
    }

    /// Serve `/metrics` to anyone even behind `auth()` / `auth_with()` (for a
    /// scraper that can't log in). Off by default: behind a gate the metrics
    /// need a session. Prefer exposing them only on a private network.
    pub fn public_metrics(mut self, public: bool) -> Self {
        self.public_metrics = public;
        self
    }

    /// Set the maximum connections per IP address.
    pub fn max_connections_per_ip(mut self, limit: usize) -> Self {
        self.max_connections_per_ip = limit;
        self
    }

    /// Set the idle timeout duration.
    /// Allow an extra cross-origin `Origin` to open WebSocket connections
    /// (full origin string, e.g. `https://app.example.com`). Same-origin
    /// connections are always allowed; call repeatedly for several origins.
    pub fn allow_origin(mut self, origin: impl Into<String>) -> Self {
        self.allowed_origins.push(origin.into());
        self
    }

    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

    /// Set the maximum memory per connection state.
    pub fn state_memory_limit(mut self, limit: usize) -> Self {
        self.state_memory_limit = limit;
        self
    }

    /// Force the `Secure` attribute on the session cookie even without an
    /// `X-Forwarded-Proto: https` header (which is otherwise auto-detected).
    pub fn secure_cookies(mut self, secure: bool) -> Self {
        self.secure_cookies = secure;
        self
    }

    /// Install a reverse-proxy resolver (see [`ProxyResolver`]). Matched paths are forwarded to a
    /// local port after the auth gate, so proxied upstreams inherit authentication.
    pub fn proxy(mut self, resolver: ProxyResolver) -> Self {
        self.proxy = Some(resolver);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.max_connections, 10_000);
        assert_eq!(config.max_connections_per_ip, 100);
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
        assert_eq!(config.state_memory_limit, 1024 * 1024);
    }

    #[test]
    fn test_builder_pattern() {
        let config = ServerConfig::new()
            .max_connections(5000)
            .max_connections_per_ip(50)
            .idle_timeout(Duration::from_secs(120))
            .state_memory_limit(512 * 1024);

        assert_eq!(config.max_connections, 5000);
        assert_eq!(config.max_connections_per_ip, 50);
        assert_eq!(config.idle_timeout, Duration::from_secs(120));
        assert_eq!(config.state_memory_limit, 512 * 1024);
    }
}
