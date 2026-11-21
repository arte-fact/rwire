//! A fake push service on a local port: it receives what `send_push` posts,
//! checks the headers, verifies the VAPID JWT with the key the request
//! carries, and decrypts the body with the subscriber's private key — a
//! decryption written here, independently of the crate's encryption. Its
//! paths choose its answer (`/ok`, `/gone` → 410, `/missing` → 404,
//! `/err` → 500), so the purge is observed end to end.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes128Gcm, Nonce};
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use hkdf::Hkdf;
use p256::ecdsa::signature::Verifier;
use p256::ecdsa::{Signature, VerifyingKey};
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::{PublicKey, SecretKey};
use rand_core::{OsRng, RngCore};
use rwire_push::{Notification, Push, PushState, SendReport, VapidKey};
use sha2::Sha256;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A browser's side of a subscription: the keys it keeps.
struct Ua {
    secret: SecretKey,
    auth: [u8; 16],
}

impl Ua {
    fn new() -> Self {
        let mut auth = [0u8; 16];
        OsRng.fill_bytes(&mut auth);
        Self {
            secret: SecretKey::random(&mut OsRng),
            auth,
        }
    }

    fn public(&self) -> Vec<u8> {
        self.secret
            .public_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec()
    }

    /// The report the extension would send for this subscription.
    fn report(&self, endpoint: &str) -> String {
        serde_json::json!({
            "state": "subscribed",
            "permission": "granted",
            "subscription": {
                "endpoint": endpoint,
                "expirationTime": null,
                "keys": { "p256dh": B64.encode(self.public()), "auth": B64.encode(self.auth) }
            }
        })
        .to_string()
    }

    /// RFC 8291 decryption of one aes128gcm record.
    fn decrypt(&self, body: &[u8]) -> Result<Vec<u8>, String> {
        if body.len() < 21 {
            return Err("short body".into());
        }
        let salt = &body[..16];
        let rs = u32::from_be_bytes(body[16..20].try_into().unwrap());
        let idlen = body[20] as usize;
        if rs < 18 || idlen != 65 || body.len() < 21 + idlen {
            return Err(format!("bad header rs={rs} idlen={idlen}"));
        }
        let as_public = &body[21..21 + idlen];
        let ciphertext = &body[21 + idlen..];
        if ciphertext.len() > rs as usize {
            return Err("more than one record".into());
        }
        let as_key = PublicKey::from_sec1_bytes(as_public).map_err(|e| e.to_string())?;
        let ecdh = p256::ecdh::diffie_hellman(self.secret.to_nonzero_scalar(), as_key.as_affine());
        let mut info = b"WebPush: info\0".to_vec();
        info.extend(self.public());
        info.extend(as_public);
        let mut ikm = [0u8; 32];
        Hkdf::<Sha256>::new(Some(&self.auth), ecdh.raw_secret_bytes())
            .expand(&info, &mut ikm)
            .unwrap();
        let prk = Hkdf::<Sha256>::new(Some(salt), &ikm);
        let (mut cek, mut nonce) = ([0u8; 16], [0u8; 12]);
        prk.expand(b"Content-Encoding: aes128gcm\0", &mut cek)
            .unwrap();
        prk.expand(b"Content-Encoding: nonce\0", &mut nonce)
            .unwrap();
        let mut plain = Aes128Gcm::new_from_slice(&cek)
            .unwrap()
            .decrypt(Nonce::from_slice(&nonce), ciphertext)
            .map_err(|_| "AES-GCM tag mismatch".to_string())?;
        // Strip the padding: zeros after the 0x02 last-record delimiter.
        while plain.last() == Some(&0) {
            plain.pop();
        }
        match plain.pop() {
            Some(2) => Ok(plain),
            other => Err(format!("bad delimiter {other:?}")),
        }
    }
}

/// Verify `Authorization: vapid t=…, k=…` for `audience`; the key.
fn verify_vapid(header: &str, audience: &str) -> Result<String, String> {
    let rest = header
        .strip_prefix("vapid t=")
        .ok_or("not a vapid header")?;
    let (token, k) = rest.split_once(", k=").ok_or("no k=")?;
    let (input, sig) = token.rsplit_once('.').ok_or("no signature")?;
    let vk = VerifyingKey::from_sec1_bytes(&B64.decode(k).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let sig = Signature::from_slice(&B64.decode(sig).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    vk.verify(input.as_bytes(), &sig)
        .map_err(|_| "bad JWT signature")?;
    let claims = input.split_once('.').ok_or("no claims")?.1;
    let claims: serde_json::Value =
        serde_json::from_slice(&B64.decode(claims).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if claims["aud"] != audience {
        return Err(format!("aud {} != {audience}", claims["aud"]));
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let exp = claims["exp"].as_u64().ok_or("no exp")?;
    if exp <= now || exp > now + 24 * 3600 {
        return Err("exp out of range".into());
    }
    if !claims["sub"]
        .as_str()
        .is_some_and(|s| s.starts_with("mailto:"))
    {
        return Err("no sub".into());
    }
    Ok(k.to_string())
}

/// What the fake service saw: per path, the decrypted payload or the error.
type Seen = Arc<Mutex<Vec<(String, Result<(String, Vec<u8>), String>)>>>;

/// Serve one request per connection: check it, record it, answer.
async fn fake_service(uas: Arc<HashMap<String, Ua>>) -> (String, Seen) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let seen: Seen = Arc::default();
    let (seen2, origin2) = (seen.clone(), origin.clone());
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = listener.accept().await.unwrap();
            let (uas, seen, origin) = (uas.clone(), seen2.clone(), origin2.clone());
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                let (head, mut body) = loop {
                    let n = sock.read(&mut chunk).await.unwrap();
                    assert!(n > 0, "connection closed mid-request");
                    buf.extend_from_slice(&chunk[..n]);
                    if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        break (
                            String::from_utf8(buf[..i].to_vec()).unwrap(),
                            buf[i + 4..].to_vec(),
                        );
                    }
                };
                let mut lines = head.lines();
                let request_line = lines.next().unwrap().to_string();
                let headers: HashMap<String, String> = lines
                    .filter_map(|l| l.split_once(':'))
                    .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
                    .collect();
                let len: usize = headers["content-length"].parse().unwrap();
                while body.len() < len {
                    let n = sock.read(&mut chunk).await.unwrap();
                    body.extend_from_slice(&chunk[..n]);
                }
                let path = request_line.split(' ').nth(1).unwrap().to_string();
                let checked = (|| {
                    if !request_line.starts_with("POST ") {
                        return Err(format!("method: {request_line}"));
                    }
                    if headers.get("content-encoding").map(String::as_str) != Some("aes128gcm") {
                        return Err("content-encoding".into());
                    }
                    if !headers.contains_key("ttl") {
                        return Err("TTL header is required (RFC 8030 §5.2)".into());
                    }
                    let k = verify_vapid(&headers["authorization"], &origin)?;
                    let ua = uas.get(&path).ok_or("unknown subscription")?;
                    Ok((k, ua.decrypt(&body)?))
                })();
                let status = match (&checked, path.split('/').nth(1)) {
                    (Err(_), _) => "400 Bad Request",
                    (Ok(_), Some("gone")) => "410 Gone",
                    (Ok(_), Some("missing")) => "404 Not Found",
                    (Ok(_), Some("err")) => "500 Internal Server Error",
                    (Ok(_), _) => "201 Created",
                };
                seen.lock().unwrap().push((path, checked));
                let resp =
                    format!("HTTP/1.1 {status}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
                sock.write_all(resp.as_bytes()).await.unwrap();
            });
        }
    });
    (origin, seen)
}

#[tokio::test(flavor = "multi_thread")]
async fn send_push_delivers_decryptable_signed_messages_and_purges_gone_subscriptions() {
    let paths = ["/ok/1", "/ok/2", "/gone/3", "/missing/4", "/err/5"];
    let uas: HashMap<String, Ua> = paths.iter().map(|p| (p.to_string(), Ua::new())).collect();
    let uas = Arc::new(uas);
    let (origin, seen) = fake_service(uas.clone()).await;

    let dir = std::env::temp_dir().join(format!("rwire-push-it-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join("subs.json");
    let vapid = VapidKey::generate();
    let push = Push::new(vapid.clone(), "mailto:ops@example.org")
        .allow_endpoint_origin(&origin)
        .store(&file)
        .unwrap();
    for p in paths {
        let state = push.handle_report(&uas[p].report(&format!("{origin}{p}")));
        assert_eq!(state, PushState::Subscribed, "{p}");
    }

    let n = Notification::new("New post").body("Hello").url("/posts/1");
    let report = push.send_push(&n).await.unwrap();

    assert_eq!(report.accepted, 2, "{report:?}");
    assert_eq!(report.removed, 2, "410 and 404 are purged: {report:?}");
    assert_eq!(report.failed.len(), 1, "{report:?}");
    assert!(report.failed[0].0.ends_with("/err/5"));
    assert!(report.failed[0].1.starts_with("500"), "{report:?}");

    {
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 5, "one request per subscriber");
        for (path, checked) in seen.iter() {
            let (k, plain) = checked.as_ref().unwrap_or_else(|e| panic!("{path}: {e}"));
            assert_eq!(k, push.public_key(), "{path}: k= is the server's key");
            let got: Notification = serde_json::from_slice(plain).unwrap();
            assert_eq!(got, n, "{path}");
        }
    }

    let mut left: Vec<String> = push
        .subscriptions()
        .into_iter()
        .map(|s| s.endpoint)
        .collect();
    left.sort();
    assert_eq!(
        left,
        [
            format!("{origin}/err/5"),
            format!("{origin}/ok/1"),
            format!("{origin}/ok/2")
        ]
    );

    // The purge reached the file.
    push.flush().await.unwrap();
    let reopened = Push::new(vapid, "mailto:ops@example.org")
        .store(&file)
        .unwrap();
    assert_eq!(reopened.subscriptions().len(), 3);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Counter-proof: the fake service does reject what it cannot decrypt —
/// a subscription stored with another browser's keys is not delivered,
/// and a failure is not a purge.
#[tokio::test(flavor = "multi_thread")]
async fn the_fake_service_rejects_a_message_for_other_keys() {
    let uas: HashMap<String, Ua> = [("/ok/1".to_string(), Ua::new())].into();
    let (origin, seen) = fake_service(Arc::new(uas)).await;
    let push =
        Push::new(VapidKey::generate(), "mailto:ops@example.org").allow_endpoint_origin(&origin);
    // Another browser's keys under the endpoint the service knows.
    push.handle_report(&Ua::new().report(&format!("{origin}/ok/1")));

    let report = push.send_push(&Notification::new("x")).await.unwrap();
    assert_eq!(
        report,
        SendReport {
            accepted: 0,
            removed: 0,
            failed: vec![(format!("{origin}/ok/1"), "400 Bad Request: ".into())],
        }
    );
    let seen = seen.lock().unwrap();
    assert_eq!(seen[0].1.as_ref().unwrap_err(), "AES-GCM tag mismatch");
    assert_eq!(push.subscriptions().len(), 1, "kept");
}

/// Counter-proof: without `allow_endpoint_origin`, a local endpoint is
/// refused at subscription time, so the server never POSTs to it.
#[tokio::test(flavor = "multi_thread")]
async fn a_local_endpoint_is_refused_by_default() {
    let uas: HashMap<String, Ua> = [("/ok/1".to_string(), Ua::new())].into();
    let (origin, seen) = fake_service(Arc::new(uas)).await;
    let push = Push::new(VapidKey::generate(), "mailto:ops@example.org");
    let state = push.handle_report(&Ua::new().report(&format!("{origin}/ok/1")));
    assert!(matches!(state, PushState::Failed(_)), "{state:?}");
    let report = push.send_push(&Notification::new("x")).await.unwrap();
    assert_eq!(report, SendReport::default());
    assert!(
        seen.lock().unwrap().is_empty(),
        "no request left the server"
    );
}

/// The test's own decryption agrees with RFC 8291 §5: the published
/// message decrypts with the published user-agent key.
#[test]
fn the_test_decryptor_reads_the_rfc_message() {
    let d = |s: &str| B64.decode(s).unwrap();
    let ua = Ua {
        secret: SecretKey::from_slice(&d("q1dXpw3UpT5VOmu_cf_v6ih07Aems3njxI-JWgLcM94")).unwrap(),
        auth: d("BTBZMqHH6r4Tts7J_aSIgg").try_into().unwrap(),
    };
    let mut msg = d("DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8");
    msg.extend(d(
        "8pfeW0KbunFT06SuDKoJH9Ql87S1QUrdirN6GcG7sFz1y1sqLgVi1VhjVkHsUoEsbI_0LpXMuGvnzQ",
    ));
    assert_eq!(
        ua.decrypt(&msg).unwrap(),
        b"When I grow up, I want to be a watermelon"
    );
}
