//! Message encryption for Web Push (RFC 8291) in the `aes128gcm` content
//! coding (RFC 8188), one record.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes128Gcm, Nonce};
use hkdf::Hkdf;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::{PublicKey, SecretKey};
use rand_core::{OsRng, RngCore};
use sha2::Sha256;

/// The record size written in the header: one record carries the whole
/// message, and push services must accept 4096-octet bodies (RFC 8030 §7.2).
const RS: u32 = 4096;
/// Salt (16) + record size (4) + key id length (1) + key id (65).
const HEADER_LEN: usize = 86;
/// The AES-GCM tag.
const TAG_LEN: usize = 16;

/// The largest payload one message carries: 4096 octets of body, less the
/// header, the tag and the padding delimiter.
pub const MAX_PAYLOAD: usize = RS as usize - HEADER_LEN - TAG_LEN - 1;

/// Why a payload could not be encrypted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncryptError {
    /// The payload exceeds [`MAX_PAYLOAD`] octets.
    TooLarge(usize),
    /// The subscription's `p256dh` is not a P-256 point.
    BadKey,
    /// The subscription's `auth` secret is not 16 octets.
    BadAuth,
}

impl std::fmt::Display for EncryptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptError::TooLarge(n) => {
                write!(f, "payload of {n} bytes exceeds {MAX_PAYLOAD}")
            }
            EncryptError::BadKey => f.write_str("subscription key is not a P-256 point"),
            EncryptError::BadAuth => f.write_str("subscription auth secret is not 16 bytes"),
        }
    }
}

impl std::error::Error for EncryptError {}

/// Encrypt `payload` for the user agent holding `ua_public` (65-octet
/// uncompressed point) and `auth_secret` (16 octets): a fresh key pair and
/// salt per message.
pub fn encrypt(
    payload: &[u8],
    ua_public: &[u8],
    auth_secret: &[u8],
) -> Result<Vec<u8>, EncryptError> {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    encrypt_with(
        payload,
        ua_public,
        auth_secret,
        &SecretKey::random(&mut OsRng),
        &salt,
    )
}

/// [`encrypt`] with the application server's key pair and the salt given:
/// the deterministic core, checked against RFC 8291 appendix A.
pub(crate) fn encrypt_with(
    payload: &[u8],
    ua_public: &[u8],
    auth_secret: &[u8],
    as_secret: &SecretKey,
    salt: &[u8; 16],
) -> Result<Vec<u8>, EncryptError> {
    if payload.len() > MAX_PAYLOAD {
        return Err(EncryptError::TooLarge(payload.len()));
    }
    if auth_secret.len() != 16 {
        return Err(EncryptError::BadAuth);
    }
    let ua = PublicKey::from_sec1_bytes(ua_public).map_err(|_| EncryptError::BadKey)?;
    let ua_point = ua.to_encoded_point(false);
    let as_point = as_secret.public_key().to_encoded_point(false);

    // RFC 8291 §3.3: combine the ECDH secret with the auth secret.
    let ecdh = p256::ecdh::diffie_hellman(as_secret.to_nonzero_scalar(), ua.as_affine());
    let mut key_info = Vec::with_capacity(14 + 65 + 65);
    key_info.extend_from_slice(b"WebPush: info\0");
    key_info.extend_from_slice(ua_point.as_bytes());
    key_info.extend_from_slice(as_point.as_bytes());
    let mut ikm = [0u8; 32];
    Hkdf::<Sha256>::new(Some(auth_secret), ecdh.raw_secret_bytes())
        .expand(&key_info, &mut ikm)
        .expect("32 bytes is a valid HKDF-SHA256 length");

    // RFC 8188 §2.2–2.3: the content encryption key and nonce.
    let prk = Hkdf::<Sha256>::new(Some(salt), &ikm);
    let mut cek = [0u8; 16];
    let mut nonce = [0u8; 12];
    prk.expand(b"Content-Encoding: aes128gcm\0", &mut cek)
        .expect("16 bytes is a valid HKDF-SHA256 length");
    prk.expand(b"Content-Encoding: nonce\0", &mut nonce)
        .expect("12 bytes is a valid HKDF-SHA256 length");

    // One record: the payload and the last-record delimiter, no padding.
    let mut plain = Vec::with_capacity(payload.len() + 1);
    plain.extend_from_slice(payload);
    plain.push(2);
    let sealed = Aes128Gcm::new_from_slice(&cek)
        .expect("16-byte key")
        .encrypt(Nonce::from_slice(&nonce), plain.as_slice())
        .expect("AES-GCM encryption of a bounded buffer cannot fail");

    let mut out = Vec::with_capacity(HEADER_LEN + sealed.len());
    out.extend_from_slice(salt);
    out.extend_from_slice(&RS.to_be_bytes());
    out.push(65);
    out.extend_from_slice(as_point.as_bytes());
    out.extend_from_slice(&sealed);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::b64;

    /// RFC 8291 appendix A, whitespace removed.
    const PLAINTEXT: &str = "V2hlbiBJIGdyb3cgdXAsIEkgd2FudCB0byBiZSBhIHdhdGVybWVsb24";
    const AS_PUBLIC: &str =
        "BP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8";
    const AS_PRIVATE: &str = "yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw";
    const UA_PUBLIC: &str =
        "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4";
    const SALT: &str = "DGv6ra1nlYgDCS1FRnbzlw";
    const AUTH: &str = "BTBZMqHH6r4Tts7J_aSIgg";
    const HEADER: &str = "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8";
    const CIPHERTEXT: &str =
        "8pfeW0KbunFT06SuDKoJH9Ql87S1QUrdirN6GcG7sFz1y1sqLgVi1VhjVkHsUoEsbI_0LpXMuGvnzQ";

    fn vector() -> Result<Vec<u8>, EncryptError> {
        let as_secret = SecretKey::from_slice(&b64::decode(AS_PRIVATE).unwrap()).unwrap();
        assert_eq!(
            b64::encode(as_secret.public_key().to_encoded_point(false).as_bytes()),
            AS_PUBLIC,
            "the vector's key pair"
        );
        let salt: [u8; 16] = b64::decode(SALT).unwrap().try_into().unwrap();
        encrypt_with(
            &b64::decode(PLAINTEXT).unwrap(),
            &b64::decode(UA_PUBLIC).unwrap(),
            &b64::decode(AUTH).unwrap(),
            &as_secret,
            &salt,
        )
    }

    #[test]
    fn rfc8291_appendix_a() {
        let out = vector().unwrap();
        let header = b64::decode(HEADER).unwrap();
        assert_eq!(header.len(), HEADER_LEN);
        assert_eq!(&out[..HEADER_LEN], header.as_slice(), "86-octet header");
        assert_eq!(
            b64::encode(&out[HEADER_LEN..]),
            CIPHERTEXT,
            "ciphertext and tag"
        );
    }

    #[test]
    fn a_fresh_key_and_salt_per_message() {
        let ua = b64::decode(UA_PUBLIC).unwrap();
        let auth = b64::decode(AUTH).unwrap();
        let a = encrypt(b"same", &ua, &auth).unwrap();
        let b = encrypt(b"same", &ua, &auth).unwrap();
        assert_ne!(a[..16], b[..16], "salt");
        assert_ne!(a[21..86], b[21..86], "ephemeral key");
        assert_eq!(a.len(), HEADER_LEN + 4 + 1 + TAG_LEN);
    }

    #[test]
    fn bounds_and_bad_keys_are_refused() {
        let ua = b64::decode(UA_PUBLIC).unwrap();
        let auth = b64::decode(AUTH).unwrap();
        assert_eq!(MAX_PAYLOAD, 3993);
        assert!(encrypt(&vec![0; MAX_PAYLOAD], &ua, &auth).is_ok());
        assert_eq!(
            encrypt(&vec![0; MAX_PAYLOAD + 1], &ua, &auth),
            Err(EncryptError::TooLarge(MAX_PAYLOAD + 1))
        );
        assert_eq!(encrypt(b"x", &ua[..64], &auth), Err(EncryptError::BadKey));
        let mut off_curve = ua.clone();
        off_curve[64] ^= 1;
        assert_eq!(encrypt(b"x", &off_curve, &auth), Err(EncryptError::BadKey));
        assert_eq!(encrypt(b"x", &ua, &auth[..15]), Err(EncryptError::BadAuth));
    }
}
