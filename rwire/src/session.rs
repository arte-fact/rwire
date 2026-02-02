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
//! let cookie = "rwire_session=abc123def456";
//! if let Some(id) = SessionId::from_cookie(cookie) {
//!     println!("Session ID: {}", id);
//! }
//! ```

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

    /// Generate a new random session ID.
    pub fn generate() -> Self {
        // Use timestamp + random-ish value for uniqueness
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        // Simple pseudo-random using timestamp mixing
        let random = timestamp.wrapping_mul(0x5851F42D4C957F2D);

        Self(format!("{:016x}{:016x}", timestamp as u64, random as u64))
    }

    /// Parse a session ID from a cookie header value.
    ///
    /// Looks for a cookie named "rwire_session" or the specified name.
    pub fn from_cookie(cookie_header: &str) -> Option<Self> {
        Self::from_cookie_named(cookie_header, "rwire_session")
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

    /// Create a Set-Cookie header value for this session ID.
    pub fn to_cookie(&self, max_age: Option<Duration>) -> String {
        self.to_cookie_named("rwire_session", max_age)
    }

    /// Create a Set-Cookie header value with a custom cookie name.
    pub fn to_cookie_named(&self, name: &str, max_age: Option<Duration>) -> String {
        let mut cookie = format!("{}={}; Path=/; HttpOnly; SameSite=Strict", name, self.0);

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
    fn test_session_id_from_cookie() {
        let cookie = "rwire_session=abc123; other=value";
        let id = SessionId::from_cookie(cookie);

        assert!(id.is_some());
        assert_eq!(id.unwrap().as_str(), "abc123");
    }

    #[test]
    fn test_session_id_from_cookie_custom_name() {
        let cookie = "my_session=xyz789; rwire_session=abc123";
        let id = SessionId::from_cookie_named(cookie, "my_session");

        assert!(id.is_some());
        assert_eq!(id.unwrap().as_str(), "xyz789");
    }

    #[test]
    fn test_session_id_to_cookie() {
        let id = SessionId::new("test123".to_string());
        let cookie = id.to_cookie(Some(Duration::from_secs(3600)));

        assert!(cookie.contains("rwire_session=test123"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("Max-Age=3600"));
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
