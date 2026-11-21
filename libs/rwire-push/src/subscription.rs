//! Push subscriptions: what a browser hands over, how it is checked, and
//! where it is kept (memory, mirrored to an atomic JSON file).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use p256::PublicKey;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::b64;

/// A browser's push subscription, as `PushSubscription.toJSON()` gives it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    /// The push service URL to POST messages to (a capability: secret).
    pub endpoint: String,
    /// The keys messages are encrypted for.
    pub keys: Keys,
}

/// A subscription's encryption keys (base64url).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keys {
    /// The user agent's P-256 public key (65-octet uncompressed point).
    pub p256dh: String,
    /// The 16-octet authentication secret.
    pub auth: String,
}

/// Why a subscription was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionError {
    /// The endpoint is not a URL, or is too long.
    BadEndpoint,
    /// The endpoint's origin is not an allowed push service.
    EndpointNotAllowed(String),
    /// `p256dh` is not a P-256 point.
    BadKey,
    /// `auth` is not 16 octets.
    BadAuth,
    /// The store holds its maximum number of subscriptions.
    Full,
}

impl std::fmt::Display for SubscriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriptionError::BadEndpoint => f.write_str("endpoint is not a valid URL"),
            SubscriptionError::EndpointNotAllowed(o) => {
                write!(f, "endpoint origin {o} is not an allowed push service")
            }
            SubscriptionError::BadKey => f.write_str("p256dh is not a P-256 public key"),
            SubscriptionError::BadAuth => f.write_str("auth is not a 16-byte secret"),
            SubscriptionError::Full => f.write_str("subscription store is full"),
        }
    }
}

impl std::error::Error for SubscriptionError {}

/// The push services browsers ship with, over https: Chrome and most
/// Chromium browsers (FCM), Firefox (Mozilla autopush), Safari and iOS
/// (Apple), Edge (WNS). A leading dot matches any subdomain.
pub const KNOWN_PUSH_HOSTS: &[&str] = &[
    "fcm.googleapis.com",
    "updates.push.services.mozilla.com",
    "web.push.apple.com",
    ".push.apple.com",
    ".notify.windows.com",
];

/// The longest endpoint accepted.
const MAX_ENDPOINT: usize = 2048;

/// Which endpoints the server will POST to. A subscription arrives from
/// the browser, so its endpoint is attacker-controlled: without this check
/// any client could make the server send requests into its own network.
#[derive(Clone, Debug, Default)]
pub(crate) struct EndpointPolicy {
    /// Extra exact origins (`scheme://host[:port]`), e.g. a self-hosted
    /// push service or a test double.
    pub extra_origins: Vec<String>,
}

impl EndpointPolicy {
    /// The endpoint's origin, if it is allowed.
    pub fn check(&self, endpoint: &str) -> Result<Url, SubscriptionError> {
        if endpoint.len() > MAX_ENDPOINT {
            return Err(SubscriptionError::BadEndpoint);
        }
        let url = Url::parse(endpoint).map_err(|_| SubscriptionError::BadEndpoint)?;
        let origin = url.origin().ascii_serialization();
        if self.extra_origins.contains(&origin) {
            return Ok(url);
        }
        let known = url.scheme() == "https"
            && url.port().is_none()
            && url.username().is_empty()
            && url.password().is_none()
            && url.host_str().is_some_and(|h| {
                KNOWN_PUSH_HOSTS.iter().any(|k| match k.strip_prefix('.') {
                    Some(suffix) => h
                        .strip_suffix(suffix)
                        .is_some_and(|sub| sub.ends_with('.') && sub.len() > 1),
                    None => h == *k,
                })
            });
        if known {
            Ok(url)
        } else {
            Err(SubscriptionError::EndpointNotAllowed(origin))
        }
    }
}

impl Subscription {
    /// The decoded `(p256dh, auth)`, checked.
    pub fn key_bytes(&self) -> Result<(Vec<u8>, Vec<u8>), SubscriptionError> {
        let p = b64::decode(&self.keys.p256dh).map_err(|_| SubscriptionError::BadKey)?;
        PublicKey::from_sec1_bytes(&p).map_err(|_| SubscriptionError::BadKey)?;
        if p.len() != 65 {
            return Err(SubscriptionError::BadKey);
        }
        let a = b64::decode(&self.keys.auth).map_err(|_| SubscriptionError::BadAuth)?;
        if a.len() != 16 {
            return Err(SubscriptionError::BadAuth);
        }
        Ok((p, a))
    }
}

/// The subscriptions, in memory and mirrored to a JSON file.
pub(crate) struct Store {
    subs: Mutex<Vec<Subscription>>,
    file: Option<Arc<FileMirror>>,
    /// The most subscriptions kept; a new one beyond is refused.
    pub capacity: usize,
}

/// The file a store is mirrored to. Writes may run on blocking threads in
/// any order; each carries a generation and an older one never overwrites
/// a newer one.
struct FileMirror {
    path: PathBuf,
    /// The generation last written.
    written: Mutex<u64>,
    /// The generation last taken.
    taken: Mutex<u64>,
}

impl FileMirror {
    fn write(&self, generation: u64, json: &str) -> std::io::Result<()> {
        let mut written = self.written.lock().unwrap_or_else(|e| e.into_inner());
        if generation <= *written {
            return Ok(());
        }
        write_atomic(&self.path, json.as_bytes())?;
        *written = generation;
        Ok(())
    }
}

/// Write `bytes` to `path` through a temporary file and a rename: a crash
/// leaves the old file or the new one, never half of one.
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let mut f = crate::vapid::private_file(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    std::fs::rename(&tmp, path)
}

impl Store {
    pub fn memory(capacity: usize) -> Self {
        Self {
            subs: Mutex::new(Vec::new()),
            file: None,
            capacity,
        }
    }

    /// The subscriptions kept at `path` (none if the file does not exist).
    /// A file that does not parse is an error, not an empty store: the
    /// next write would otherwise erase every subscriber.
    pub fn open(path: PathBuf, capacity: usize) -> std::io::Result<Self> {
        let subs = match std::fs::read(&path) {
            Ok(b) => serde_json::from_slice(&b).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{}: {e}", path.display()),
                )
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)?;
        }
        Ok(Self {
            subs: Mutex::new(subs),
            file: Some(Arc::new(FileMirror {
                path,
                written: Mutex::new(0),
                taken: Mutex::new(0),
            })),
            capacity,
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Subscription>> {
        self.subs.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn list(&self) -> Vec<Subscription> {
        self.lock().clone()
    }

    pub fn len(&self) -> usize {
        self.lock().len()
    }

    /// Add `sub`, or replace the keys of the one with its endpoint.
    /// Whether anything changed.
    pub fn upsert(&self, sub: Subscription) -> Result<bool, SubscriptionError> {
        let mut subs = self.lock();
        let changed = match subs.iter().position(|s| s.endpoint == sub.endpoint) {
            Some(i) if subs[i] == sub => false,
            Some(i) => {
                subs[i] = sub;
                true
            }
            None if subs.len() >= self.capacity => return Err(SubscriptionError::Full),
            None => {
                subs.push(sub);
                true
            }
        };
        if changed {
            self.mirror(&subs);
        }
        Ok(changed)
    }

    /// Remove the subscriptions whose endpoint is in `endpoints`; how many.
    pub fn remove(&self, endpoints: &[&str]) -> usize {
        let mut subs = self.lock();
        let before = subs.len();
        subs.retain(|s| !endpoints.contains(&s.endpoint.as_str()));
        let removed = before - subs.len();
        if removed > 0 {
            self.mirror(&subs);
        }
        removed
    }

    /// A snapshot and its generation, taken under the subscriptions lock so
    /// generations follow the order of the changes.
    fn snapshot(&self, subs: &[Subscription]) -> Option<(Arc<FileMirror>, u64, String)> {
        let file = self.file.clone()?;
        let generation = {
            let mut t = file.taken.lock().unwrap_or_else(|e| e.into_inner());
            *t += 1;
            *t
        };
        let json = serde_json::to_string_pretty(subs).expect("subscriptions serialize");
        Some((file, generation, json))
    }

    /// Mirror a change to the file: on a blocking thread when a tokio
    /// runtime is there (handlers must not block), else right away.
    fn mirror(&self, subs: &[Subscription]) {
        let Some((file, generation, json)) = self.snapshot(subs) else {
            return;
        };
        let write = move || {
            if let Err(e) = file.write(generation, &json) {
                eprintln!("rwire-push: cannot save subscriptions: {e}");
            }
        };
        match tokio::runtime::Handle::try_current() {
            Ok(h) => drop(h.spawn_blocking(write)),
            Err(_) => write(),
        }
    }

    /// Write the current subscriptions now and wait for it.
    pub async fn flush(&self) -> std::io::Result<()> {
        let Some((file, generation, json)) = self.snapshot(&self.lock()) else {
            return Ok(());
        };
        tokio::task::spawn_blocking(move || file.write(generation, &json))
            .await
            .map_err(std::io::Error::other)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UA_PUBLIC: &str =
        "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4";
    const AUTH: &str = "BTBZMqHH6r4Tts7J_aSIgg";

    fn sub(endpoint: &str) -> Subscription {
        Subscription {
            endpoint: endpoint.to_string(),
            keys: Keys {
                p256dh: UA_PUBLIC.to_string(),
                auth: AUTH.to_string(),
            },
        }
    }

    #[test]
    fn known_push_services_are_allowed_and_the_rest_refused() {
        let p = EndpointPolicy::default();
        for ok in [
            "https://fcm.googleapis.com/fcm/send/abc",
            "https://updates.push.services.mozilla.com/wpush/v2/x",
            "https://web.push.apple.com/QGx",
            "https://wns2-par02p.notify.windows.com/w/?token=1",
        ] {
            assert!(p.check(ok).is_ok(), "{ok}");
        }
        for bad in [
            "http://fcm.googleapis.com/fcm/send/abc",    // not https
            "https://fcm.googleapis.com:8443/x",         // other port
            "https://fcm.googleapis.com.evil.example/x", // suffix trick
            "https://evilnotify.windows.com/x",          // not a subdomain
            "https://.notify.windows.com/x",             // empty label
            "https://user@fcm.googleapis.com/x",         // credentials
            "https://127.0.0.1/x",
            "https://localhost/x",
            "https://169.254.169.254/latest/meta-data",
            "file:///etc/passwd",
            "not a url",
        ] {
            assert!(p.check(bad).is_err(), "{bad}");
        }
        assert!(p
            .check(&format!("https://fcm.googleapis.com/{}", "a".repeat(2100)))
            .is_err());
    }

    #[test]
    fn an_extra_origin_is_matched_exactly() {
        let p = EndpointPolicy {
            extra_origins: vec!["http://127.0.0.1:4567".into()],
        };
        assert!(p.check("http://127.0.0.1:4567/push/1").is_ok());
        assert!(p.check("http://127.0.0.1:4568/push/1").is_err());
        assert!(p.check("https://127.0.0.1:4567/push/1").is_err());
    }

    #[test]
    fn keys_are_checked() {
        assert!(sub("x").key_bytes().is_ok());
        let mut s = sub("x");
        s.keys.p256dh = AUTH.into();
        assert_eq!(s.key_bytes(), Err(SubscriptionError::BadKey));
        let mut s = sub("x");
        s.keys.auth = UA_PUBLIC.into();
        assert_eq!(s.key_bytes(), Err(SubscriptionError::BadAuth));
    }

    #[test]
    fn upsert_replaces_by_endpoint_and_respects_capacity() {
        let st = Store::memory(2);
        assert_eq!(st.upsert(sub("a")), Ok(true));
        assert_eq!(st.upsert(sub("a")), Ok(false), "same subscription");
        let mut rekeyed = sub("a");
        rekeyed.keys.auth = "AAAAAAAAAAAAAAAAAAAAAA".into();
        assert_eq!(st.upsert(rekeyed.clone()), Ok(true));
        assert_eq!(st.list(), vec![rekeyed]);
        assert_eq!(st.upsert(sub("b")), Ok(true));
        assert_eq!(st.upsert(sub("c")), Err(SubscriptionError::Full));
        assert_eq!(st.remove(&["a", "zz"]), 1);
        assert_eq!(st.len(), 1);
    }

    #[test]
    fn the_file_survives_a_reopen_and_a_bad_file_is_an_error() {
        let dir = std::env::temp_dir().join(format!("rwire-push-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("subs.json");
        // No runtime here: writes are immediate.
        let st = Store::open(path.clone(), 10).unwrap();
        st.upsert(sub("a")).unwrap();
        st.upsert(sub("b")).unwrap();
        st.remove(&["a"]);
        let again = Store::open(path.clone(), 10).unwrap();
        assert_eq!(again.list(), vec![sub("b")]);
        assert!(
            !dir.join("subs.json.tmp").exists(),
            "temporary file renamed"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "auth secrets are private");
        }
        std::fs::write(&path, "{not json").unwrap();
        assert!(Store::open(path, 10).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_older_generation_never_overwrites_a_newer_one() {
        let dir = std::env::temp_dir().join(format!("rwire-push-gen-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let m = FileMirror {
            path: dir.join("f.json"),
            written: Mutex::new(0),
            taken: Mutex::new(0),
        };
        m.write(2, "new").unwrap();
        m.write(1, "old").unwrap();
        assert_eq!(std::fs::read_to_string(&m.path).unwrap(), "new");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn under_a_runtime_writes_go_to_a_blocking_thread_and_flush_waits() {
        let dir = std::env::temp_dir().join(format!("rwire-push-rt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("subs.json");
        let st = Store::open(path.clone(), 100).unwrap();
        for i in 0..20 {
            st.upsert(sub(&format!("e{i}"))).unwrap();
        }
        st.flush().await.unwrap();
        let back = Store::open(path, 100).unwrap();
        assert_eq!(back.len(), 20);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
