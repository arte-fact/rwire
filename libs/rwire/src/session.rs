//! Session management for rwire servers.
//!
//! Provides session ID generation, cookie parsing, and session state management.
//!
//! # Example
//!
//! ```ignore
//! use rwire::session::{Session, SessionId};
//!
//! // Generate a new session ID
//! let session_id = SessionId::generate();
//!
//! // Parse from cookie header
//! let cookie = "rwire_sid=abc123def456";
//! if let Some(id) = SessionId::from_cookie(cookie) {
//!     println!("Session ID: {}", id);
//! }
//! ```

/// Default cookie name for rwire sessions.
pub const COOKIE_NAME: &str = "rwire_sid";

/// Default cookie max age (1 year in seconds).
pub const COOKIE_MAX_AGE_SECS: u64 = 365 * 24 * 60 * 60;

use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A unique session identifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// Create a session ID from a string.
    pub fn new(id: String) -> Self {
        Self(id)
    }

    /// Generate a new session ID with 128 bits of cryptographically secure
    /// randomness from the OS CSPRNG.
    ///
    /// The session ID is a bearer token for the user's persisted state (it keys
    /// `shared_cache_key`), so it must be unguessable. Reading `/dev/urandom`
    /// yields 16 random bytes rendered as 32 hex chars. If the OS RNG is
    /// somehow unavailable we fall back to time-based mixing to avoid panicking,
    /// but that path is not cryptographically secure.
    pub fn generate() -> Self {
        match Self::random_hex() {
            Some(hex) => Self(hex),
            None => Self(Self::weak_timestamp_id()),
        }
    }

    /// Read 16 bytes from the OS CSPRNG and hex-encode them. Returns `None` if
    /// `/dev/urandom` cannot be read (extremely rare; triggers the weak fallback).
    fn random_hex() -> Option<String> {
        use std::fmt::Write as _;
        use std::io::Read as _;

        let mut buf = [0u8; 16];
        std::fs::File::open("/dev/urandom")
            .and_then(|mut f| f.read_exact(&mut buf))
            .ok()?;

        let mut hex = String::with_capacity(32);
        for byte in buf {
            let _ = write!(hex, "{byte:02x}");
        }
        Some(hex)
    }

    /// Non-cryptographic fallback used only when the OS RNG is unavailable.
    fn weak_timestamp_id() -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mixed = timestamp.wrapping_mul(0x5851F42D4C957F2D);
        format!("{:016x}{:016x}", timestamp as u64, mixed as u64)
    }

    /// Parse a session ID from a cookie header value.
    ///
    /// Looks for a cookie named "rwire_sid".
    pub fn from_cookie(cookie_header: &str) -> Option<Self> {
        Self::from_cookie_named(cookie_header, COOKIE_NAME)
    }

    /// Parse a session ID from a cookie header with a custom cookie name.
    pub fn from_cookie_named(cookie_header: &str, cookie_name: &str) -> Option<Self> {
        for part in cookie_header.split(';') {
            let part = part.trim();
            if let Some(rest) = part.strip_prefix(cookie_name) {
                if let Some(value) = rest.strip_prefix('=') {
                    return Some(Self(value.trim().to_string()));
                }
            }
        }
        None
    }

    /// Get the session ID as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// True only if this id has the exact shape [`generate`](Self::generate)
    /// produces: 32 hex characters.
    ///
    /// Used at the trust boundary to reject client-supplied cookie values before
    /// they are used as a persisted-state cache key. Requiring hex rules out a
    /// crafted value containing `:` (which could collide with the `__shared__:`
    /// key namespace) and bounds the length, mitigating session fixation and
    /// cache-key injection.
    pub fn is_valid_format(&self) -> bool {
        self.0.len() == 32 && self.0.bytes().all(|b| b.is_ascii_hexdigit())
    }

    /// Create a Set-Cookie header value for this session ID.
    ///
    /// When `secure` is true the `Secure` attribute is added so the cookie is
    /// only sent over HTTPS — enable it when the server is reachable over TLS
    /// (e.g. behind a TLS-terminating proxy); leave it off for plain-HTTP local
    /// development, where `Secure` would prevent the cookie being sent at all.
    pub fn to_cookie(&self, max_age: Option<Duration>, secure: bool) -> String {
        self.to_cookie_named(COOKIE_NAME, max_age, secure)
    }

    /// Create a Set-Cookie header value with a custom cookie name.
    pub fn to_cookie_named(&self, name: &str, max_age: Option<Duration>, secure: bool) -> String {
        let mut cookie = format!("{}={}; Path=/; HttpOnly; SameSite=Strict", name, self.0);

        if secure {
            cookie.push_str("; Secure");
        }

        if let Some(age) = max_age {
            cookie.push_str(&format!("; Max-Age={}", age.as_secs()));
        }

        cookie
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Session data container.
#[derive(Clone, Debug)]
pub struct Session {
    /// The session ID.
    pub id: SessionId,
    /// When the session was created.
    pub created_at: SystemTime,
    /// When the session was last accessed.
    pub last_accessed: SystemTime,
    /// Custom session data as key-value pairs.
    data: std::collections::HashMap<String, String>,
}

impl Session {
    /// Create a new session with a generated ID.
    pub fn new() -> Self {
        let now = SystemTime::now();
        Self {
            id: SessionId::generate(),
            created_at: now,
            last_accessed: now,
            data: std::collections::HashMap::new(),
        }
    }

    /// Create a session with an existing ID.
    pub fn with_id(id: SessionId) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            created_at: now,
            last_accessed: now,
            data: std::collections::HashMap::new(),
        }
    }

    /// Update the last accessed time.
    pub fn touch(&mut self) {
        self.last_accessed = SystemTime::now();
    }

    /// Get a value from the session.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }

    /// Set a value in the session.
    pub fn set(&mut self, key: &str, value: &str) {
        self.data.insert(key.to_string(), value.to_string());
    }

    /// Remove a value from the session.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.data.remove(key)
    }

    /// Check if the session has expired.
    pub fn is_expired(&self, max_age: Duration) -> bool {
        self.last_accessed
            .elapsed()
            .map(|elapsed| elapsed > max_age)
            .unwrap_or(true)
    }

    /// Get the age of the session since creation.
    pub fn age(&self) -> Duration {
        self.created_at.elapsed().unwrap_or_default()
    }

    /// Get the idle time since last access.
    pub fn idle_time(&self) -> Duration {
        self.last_accessed.elapsed().unwrap_or_default()
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_generate() {
        let id1 = SessionId::generate();
        let id2 = SessionId::generate();

        assert!(!id1.as_str().is_empty());
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_session_id_is_128bit_hex() {
        // 16 random bytes rendered as lowercase hex => 32 chars, all hex digits.
        let id = SessionId::generate();
        assert_eq!(id.as_str().len(), 32, "expected 128-bit hex id");
        assert!(
            id.as_str().bytes().all(|b| b.is_ascii_hexdigit()),
            "session id must be hex: {}",
            id.as_str()
        );
    }

    #[test]
    fn test_session_id_high_entropy_no_collisions() {
        // A predictable/time-derived generator collides under tight loops; a CSPRNG
        // does not. Generate many in quick succession and require all distinct.
        use std::collections::HashSet;
        let ids: HashSet<String> = (0..10_000).map(|_| SessionId::generate().0).collect();
        assert_eq!(ids.len(), 10_000, "session ids must be unique");
    }

    #[test]
    fn test_session_id_from_cookie() {
        let cookie = "rwire_sid=abc123; other=value";
        let id = SessionId::from_cookie(cookie);

        assert!(id.is_some());
        assert_eq!(id.unwrap().as_str(), "abc123");
    }

    #[test]
    fn test_session_id_from_cookie_custom_name() {
        let cookie = "my_session=xyz789; rwire_sid=abc123";
        let id = SessionId::from_cookie_named(cookie, "my_session");

        assert!(id.is_some());
        assert_eq!(id.unwrap().as_str(), "xyz789");
    }

    #[test]
    fn test_session_id_to_cookie() {
        let id = SessionId::new("test123".to_string());
        let cookie = id.to_cookie(Some(Duration::from_secs(3600)), false);

        assert!(cookie.contains("rwire_sid=test123"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("Max-Age=3600"));
        // Secure omitted by default (plain-HTTP dev).
        assert!(!cookie.contains("Secure"));
    }

    #[test]
    fn test_session_id_to_cookie_secure() {
        let id = SessionId::new("test123".to_string());
        let cookie = id.to_cookie(Some(Duration::from_secs(3600)), true);
        assert!(cookie.contains("; Secure"));
    }

    #[test]
    fn test_session_id_is_valid_format() {
        // A freshly generated id passes; crafted/short/`:`-bearing values do not.
        assert!(SessionId::generate().is_valid_format());
        assert!(!SessionId::new("abc123".to_string()).is_valid_format()); // too short
        assert!(!SessionId::new("__shared__:shared_counter".to_string()).is_valid_format());
        assert!(!SessionId::new("z".repeat(32)).is_valid_format()); // non-hex
        assert!(SessionId::new("a".repeat(32)).is_valid_format());
    }

    #[test]
    fn test_session_cookie_roundtrip() {
        let id = SessionId::generate();
        let set_cookie = id.to_cookie(Some(Duration::from_secs(3600)), false);

        // Simulate browser sending cookie back (just the name=value part)
        let cookie_value = format!("{}={}", COOKIE_NAME, id.as_str());
        let parsed = SessionId::from_cookie(&cookie_value);

        assert!(parsed.is_some());
        assert_eq!(parsed.unwrap().as_str(), id.as_str());

        // Also verify the Set-Cookie contains the expected parts
        assert!(set_cookie.contains(&cookie_value));
    }

    #[test]
    fn test_session_data() {
        let mut session = Session::new();

        session.set("user_id", "42");
        assert_eq!(session.get("user_id"), Some("42"));

        session.remove("user_id");
        assert_eq!(session.get("user_id"), None);
    }

    #[test]
    fn test_session_expiry() {
        let session = Session::new();

        // New session should not be expired with a reasonable max age
        assert!(!session.is_expired(Duration::from_secs(3600)));

        // Session should be expired with 0 max age
        assert!(session.is_expired(Duration::from_secs(0)));
    }
}
