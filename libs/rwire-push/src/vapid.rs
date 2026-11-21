//! VAPID (RFC 8292): the application server's P-256 key pair and the
//! ES256-signed JWT that identifies it to push services.

use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::SecretKey;
use rand_core::OsRng;

use crate::b64;

/// How long a signed token stays valid. RFC 8292 caps `exp` at 24 hours.
pub const TOKEN_LIFETIME_SECS: u64 = 12 * 3600;

/// The application server's VAPID key pair.
///
/// Its text form, the one [`VapidKey::from_base64url`] reads and
/// [`VapidKey::to_base64url`] writes, is the 32-byte private scalar in
/// base64url without padding: the "private key" printed by the usual
/// `web-push generate-vapid-keys` tools.
#[derive(Clone)]
pub struct VapidKey {
    secret: SecretKey,
    public: String,
}

impl std::fmt::Debug for VapidKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the private half.
        f.debug_struct("VapidKey")
            .field("public", &self.public)
            .finish_non_exhaustive()
    }
}

/// Why a VAPID key could not be had.
#[derive(Debug)]
pub enum VapidError {
    /// The text is not a base64url P-256 private scalar.
    Invalid,
    /// The environment variable is unset or not unicode.
    MissingEnv(String),
    /// Reading or writing the key file failed.
    Io(std::io::Error),
}

impl std::fmt::Display for VapidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VapidError::Invalid => f.write_str("not a VAPID private key (32 bytes, base64url)"),
            VapidError::MissingEnv(v) => write!(f, "environment variable {v} is not set"),
            VapidError::Io(e) => write!(f, "VAPID key file: {e}"),
        }
    }
}

impl std::error::Error for VapidError {}

impl From<std::io::Error> for VapidError {
    fn from(e: std::io::Error) -> Self {
        VapidError::Io(e)
    }
}

impl VapidKey {
    fn from_secret(secret: SecretKey) -> Self {
        let public = b64::encode(secret.public_key().to_encoded_point(false).as_bytes());
        Self { secret, public }
    }

    /// A fresh random key pair.
    pub fn generate() -> Self {
        Self::from_secret(SecretKey::random(&mut OsRng))
    }

    /// The key whose private scalar is `s` (base64url, padding and
    /// surrounding whitespace tolerated).
    pub fn from_base64url(s: &str) -> Result<Self, VapidError> {
        let bytes = b64::decode(s.trim()).map_err(|_| VapidError::Invalid)?;
        if bytes.len() != 32 {
            return Err(VapidError::Invalid);
        }
        SecretKey::from_slice(&bytes)
            .map(Self::from_secret)
            .map_err(|_| VapidError::Invalid)
    }

    /// The key held by the environment variable `var`.
    pub fn from_env(var: &str) -> Result<Self, VapidError> {
        let v = std::env::var(var).map_err(|_| VapidError::MissingEnv(var.to_string()))?;
        Self::from_base64url(&v)
    }

    /// The key stored at `path`, or a new one written there (mode 0600 on
    /// unix) if the file does not exist yet. Keep the file: a new key
    /// orphans every subscription made under the old one.
    pub fn load_or_generate(path: impl AsRef<Path>) -> Result<Self, VapidError> {
        let path = path.as_ref();
        match std::fs::read_to_string(path) {
            Ok(s) => Self::from_base64url(&s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let key = Self::generate();
                if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
                    std::fs::create_dir_all(dir)?;
                }
                let mut f = private_file(path)?;
                f.write_all(key.to_base64url().as_bytes())?;
                f.write_all(b"\n")?;
                f.sync_all()?;
                Ok(key)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// The private scalar, base64url: what [`VapidKey::from_base64url`] reads.
    pub fn to_base64url(&self) -> String {
        b64::encode(&self.secret.to_bytes())
    }

    /// The public key (65-octet uncompressed point, base64url): the
    /// `applicationServerKey` browsers subscribe with, and the `k` of the
    /// `Authorization` header.
    pub fn public_key(&self) -> &str {
        &self.public
    }

    /// A signed JWT for the push service at `audience` (its origin, e.g.
    /// `https://fcm.googleapis.com`), naming the operator `subject`
    /// (`mailto:` or `https:`), valid until `exp` (Unix seconds).
    pub fn token(&self, audience: &str, subject: &str, exp: u64) -> String {
        let header = b64::encode(br#"{"typ":"JWT","alg":"ES256"}"#);
        let claims = serde_json::json!({ "aud": audience, "exp": exp, "sub": subject });
        let signing_input = format!("{header}.{}", b64::encode(claims.to_string().as_bytes()));
        let sig: Signature = SigningKey::from(&self.secret).sign(signing_input.as_bytes());
        format!("{signing_input}.{}", b64::encode(&sig.to_bytes()))
    }

    /// The `Authorization` header value for `audience` (RFC 8292 §3): a
    /// token valid [`TOKEN_LIFETIME_SECS`] from now.
    pub fn authorization(&self, audience: &str, subject: &str) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!(
            "vapid t={}, k={}",
            self.token(audience, subject, now + TOKEN_LIFETIME_SECS),
            self.public
        )
    }
}

/// A new file only the owner may read.
pub(crate) fn private_file(path: &Path) -> std::io::Result<std::fs::File> {
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::signature::Verifier;
    use p256::ecdsa::VerifyingKey;

    /// Verify a JWT with the public key alone, as a push service does.
    fn verify(token: &str, public_b64: &str) -> serde_json::Value {
        let (input, sig) = token.rsplit_once('.').unwrap();
        let vk = VerifyingKey::from_sec1_bytes(&b64::decode(public_b64).unwrap()).unwrap();
        let sig = Signature::from_slice(&b64::decode(sig).unwrap()).unwrap();
        vk.verify(input.as_bytes(), &sig).expect("ES256 signature");
        let (header, claims) = input.split_once('.').unwrap();
        let header: serde_json::Value =
            serde_json::from_slice(&b64::decode(header).unwrap()).unwrap();
        assert_eq!(header["alg"], "ES256");
        assert_eq!(header["typ"], "JWT");
        serde_json::from_slice(&b64::decode(claims).unwrap()).unwrap()
    }

    #[test]
    fn the_token_verifies_with_the_public_key() {
        let k = VapidKey::generate();
        let t = k.token(
            "https://push.example",
            "mailto:ops@example.org",
            1_700_000_000,
        );
        let claims = verify(&t, k.public_key());
        assert_eq!(claims["aud"], "https://push.example");
        assert_eq!(claims["sub"], "mailto:ops@example.org");
        assert_eq!(claims["exp"], 1_700_000_000);
    }

    #[test]
    #[should_panic(expected = "ES256 signature")]
    fn another_key_does_not_verify_it() {
        let t = VapidKey::generate().token("https://push.example", "mailto:a@b", 1);
        verify(&t, VapidKey::generate().public_key());
    }

    #[test]
    #[should_panic(expected = "ES256 signature")]
    fn a_tampered_claim_does_not_verify() {
        let k = VapidKey::generate();
        let t = k.token("https://push.example", "mailto:a@b", 1);
        let forged_claims =
            b64::encode(br#"{"aud":"https://evil.example","exp":1,"sub":"mailto:a@b"}"#);
        let mut parts: Vec<&str> = t.split('.').collect();
        parts[1] = &forged_claims;
        verify(&parts.join("."), k.public_key());
    }

    #[test]
    fn the_authorization_header_carries_token_and_key() {
        let k = VapidKey::generate();
        let h = k.authorization("https://push.example", "mailto:a@b");
        let rest = h.strip_prefix("vapid t=").unwrap();
        let (t, key) = rest.split_once(", k=").unwrap();
        assert_eq!(key, k.public_key());
        let exp = verify(t, key)["exp"].as_u64().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(
            exp > now && exp <= now + 24 * 3600,
            "RFC 8292: at most 24 h"
        );
    }

    #[test]
    fn the_text_form_round_trips_and_bad_text_is_refused() {
        let k = VapidKey::generate();
        let back = VapidKey::from_base64url(&format!(" {}\n", k.to_base64url())).unwrap();
        assert_eq!(back.public_key(), k.public_key());
        assert_eq!(b64::decode(k.public_key()).unwrap().len(), 65);
        assert!(VapidKey::from_base64url("short").is_err());
        assert!(
            VapidKey::from_base64url(&b64::encode(&[0u8; 32])).is_err(),
            "zero scalar"
        );
        assert!(
            !format!("{k:?}").contains(&k.to_base64url()),
            "Debug hides the secret"
        );
    }

    #[test]
    fn a_key_file_is_created_once_then_reused() {
        let dir = std::env::temp_dir().join(format!("rwire-push-vapid-{}", std::process::id()));
        let path = dir.join("vapid.key");
        let _ = std::fs::remove_dir_all(&dir);
        let a = VapidKey::load_or_generate(&path).unwrap();
        let b = VapidKey::load_or_generate(&path).unwrap();
        assert_eq!(a.public_key(), b.public_key());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        std::fs::write(&path, "garbage").unwrap();
        assert!(matches!(
            VapidKey::load_or_generate(&path),
            Err(VapidError::Invalid)
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_env_form() {
        let var = format!("RWIRE_PUSH_TEST_VAPID_{}", std::process::id());
        assert!(matches!(
            VapidKey::from_env(&var),
            Err(VapidError::MissingEnv(_))
        ));
        let k = VapidKey::generate();
        // A variable private to this test (process id in its name).
        std::env::set_var(&var, k.to_base64url());
        assert_eq!(
            VapidKey::from_env(&var).unwrap().public_key(),
            k.public_key()
        );
    }
}
