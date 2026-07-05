//! Server configuration for connection limits and timeouts.

use std::sync::Arc;
use std::time::Duration;

/// Resolves a request path to a reverse-proxy upstream. Returns `(port, prefix)` when the path is a
/// proxied route — `port` is the local upstream to forward to on `127.0.0.1`, and `prefix` is the
/// leading path segment to strip so the upstream sees a root-relative path — or `None` for a normal
/// request the server should handle itself.
///
/// The mechanism lives here; the *policy* (which paths map to which ports) is the host app's
/// closure. It's how one origin can front several sibling servers behind a single auth gate — e.g.
/// a preview supervisor mapping `/preview/<id>/…` to a pooled port (claw's P2 CD flow).
/// The boxed closure a [`ProxyResolver`] holds: a request path → `(upstream port, prefix to strip)`.
type ProxyFn = dyn Fn(&str) -> Option<(u16, String)> + Send + Sync;

#[derive(Clone)]
pub struct ProxyResolver(Arc<ProxyFn>);

impl ProxyResolver {
    /// Wrap a path→`(port, prefix)` resolver.
    pub fn new(resolve: impl Fn(&str) -> Option<(u16, String)> + Send + Sync + 'static) -> Self {
        Self(Arc::new(resolve))
    }

    /// Resolve `path` to an upstream, or `None` to handle it normally.
    #[must_use]
    pub fn resolve(&self, path: &str) -> Option<(u16, String)> {
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
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            max_connections: 10_000,
            max_connections_per_ip: 100,
            idle_timeout: Duration::from_secs(300),
            state_memory_limit: 1024 * 1024, // 1MB
            secure_cookies: false,
            proxy: None,
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

    /// Set the maximum connections per IP address.
    pub fn max_connections_per_ip(mut self, limit: usize) -> Self {
        self.max_connections_per_ip = limit;
        self
    }

    /// Set the idle timeout duration.
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
