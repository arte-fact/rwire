//! Delegated authentication end to end, against a fake identity hub: a tiny local
//! HTTP server answering `GET /admin/session` with `200 <user>` for a known
//! Bearer token and `401` otherwise — the shape of the mcp-agents hub.

use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use rwire::delegated_auth::{CLOSE_HUB_UNAVAILABLE, CLOSE_UNAUTHORIZED};
use rwire::{
    el, handler, renderer, AuthRequest, DelegatedAuth, El, ElementBuilder, Ev, Identity, Server,
    ServerConfig, State, Validation, AUTH_CONFLICT,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::Message;

const COOKIE: &str = "hub_session";
const TOKEN: &str = "tok-ada";
const LOGIN: &str = "https://hub.example/login";

// ── Fake hub ────────────────────────────────────────────────────────────────

/// Valid tokens and a count of `/admin/session` calls. While `down`, the hub
/// drops every connection without answering.
#[derive(Clone, Default)]
struct Hub {
    valid: Arc<Mutex<HashSet<String>>>,
    calls: Arc<AtomicUsize>,
    down: Arc<std::sync::atomic::AtomicBool>,
}

impl Hub {
    async fn start(port: u16) -> Self {
        let hub = Hub::default();
        hub.valid.lock().unwrap().insert(TOKEN.to_string());
        let listener = TcpListener::bind(("127.0.0.1", port)).await.unwrap();
        let served = hub.clone();
        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let hub = served.clone();
                tokio::spawn(async move {
                    let head = read_head(&mut stream).await;
                    hub.calls.fetch_add(1, Ordering::SeqCst);
                    if hub.down.load(Ordering::SeqCst) {
                        return;
                    }
                    let token = head
                        .lines()
                        .find_map(|l| l.strip_prefix("Authorization: Bearer "))
                        .unwrap_or("");
                    let ok = hub.valid.lock().unwrap().contains(token);
                    let response = if ok {
                        "HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nada"
                    } else {
                        "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    };
                    let _ = stream.write_all(response.as_bytes()).await;
                });
            }
        });
        hub
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn revoke(&self, token: &str) {
        self.valid.lock().unwrap().remove(token);
    }

    fn set_down(&self, down: bool) {
        self.down.store(down, Ordering::SeqCst);
    }
}

async fn read_head(stream: &mut TcpStream) -> String {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte).await {
            Ok(1) => head.push(byte[0]),
            _ => break,
        }
    }
    String::from_utf8_lossy(&head).into_owned()
}

/// The validator an application would write: ask the hub with the cookie as a
/// Bearer token; keep the token in the identity for later calls on the user's behalf.
fn hub_auth(hub_port: u16) -> DelegatedAuth {
    DelegatedAuth::new(COOKIE, move |req: AuthRequest| {
        let token = req.cookie(COOKIE).map(str::to_string);
        async move {
            let token = token?;
            let mut stream = TcpStream::connect(("127.0.0.1", hub_port)).await.ok()?;
            let ask = format!(
                "GET /admin/session HTTP/1.1\r\nHost: hub\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
            );
            stream.write_all(ask.as_bytes()).await.ok()?;
            let mut answer = String::new();
            stream.read_to_string(&mut answer).await.ok()?;
            if status_code(&answer) != Some(200) {
                return None;
            }
            let user = answer.split("\r\n\r\n").nth(1)?.to_string();
            Some(Identity::new(user).with_data(token))
        }
    })
    .login_url(LOGIN)
}

/// The same hub client, telling a revocation (401/403) from an outage (no
/// connection, no answer, 5xx).
fn hub_validation(hub_port: u16) -> DelegatedAuth {
    DelegatedAuth::with_validation(COOKIE, move |req: AuthRequest| {
        let token = req.cookie(COOKIE).map(str::to_string);
        async move {
            let Some(token) = token else {
                return Validation::Revoked;
            };
            let Ok(mut stream) = TcpStream::connect(("127.0.0.1", hub_port)).await else {
                return Validation::Unavailable;
            };
            let ask = format!(
                "GET /admin/session HTTP/1.1\r\nHost: hub\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
            );
            let mut answer = String::new();
            if stream.write_all(ask.as_bytes()).await.is_err()
                || stream.read_to_string(&mut answer).await.is_err()
            {
                return Validation::Unavailable;
            }
            match status_code(&answer) {
                Some(200) => match answer.split("\r\n\r\n").nth(1) {
                    Some(user) => Validation::Valid(Identity::new(user).with_data(token)),
                    None => Validation::Unavailable,
                },
                Some(401 | 403) => Validation::Revoked,
                _ => Validation::Unavailable,
            }
        }
    })
    .login_url(LOGIN)
}

/// The status code of an HTTP response, whatever its version.
fn status_code(response: &str) -> Option<u16> {
    let line = response.lines().next()?;
    let mut parts = line.split(' ');
    parts.next().filter(|v| v.starts_with("HTTP/"))?;
    parts.next()?.parse().ok()
}

// ── App ─────────────────────────────────────────────────────────────────────

#[derive(State, Default)]
#[storage(memory)]
struct Who {
    line: String,
}

/// Reads the identity the handshake established.
#[handler]
fn whoami(who: &mut Who, ctx: &EventContext) {
    who.line = match ctx.identity() {
        Some(identity) => {
            let token_ok = identity.data::<String>().map(String::as_str) == Some(TOKEN);
            format!("who: {} token:{token_ok}", identity.id())
        }
        None => "who: anonymous".to_string(),
    };
}

#[renderer]
fn who_view(who: &Who) -> ElementBuilder {
    el(El::P).text(if who.line.is_empty() {
        "who: not asked"
    } else {
        &who.line
    })
}

fn app() -> ElementBuilder {
    el(El::Main).append([
        who_view(),
        el(El::Button).text("who").on(Ev::Click, whoami()),
    ])
}

async fn start(port: u16, auth: DelegatedAuth) {
    start_with(port, auth, ServerConfig::new()).await;
}

async fn start_with(port: u16, auth: DelegatedAuth, config: ServerConfig) {
    tokio::spawn(async move {
        let _ = Server::bind(&format!("127.0.0.1:{port}"))
            .unwrap()
            .root(app)
            .auth_with(auth)
            .config(config)
            .run()
            .await;
    });
    tokio::time::sleep(Duration::from_millis(150)).await;
}

// ── HTTP helpers ────────────────────────────────────────────────────────────

async fn get(port: u16, path: &str, cookie: Option<&str>) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let cookie = cookie
        .map(|c| format!("Cookie: {COOKIE}={c}\r\n"))
        .unwrap_or_default();
    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n{cookie}\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut head = vec![0u8; 512];
    let n = stream.read(&mut head).await.unwrap();
    String::from_utf8_lossy(&head[..n]).into_owned()
}

/// The whole response to a `method` request, read until the server closes.
async fn request(port: u16, method: &str, path: &str, cookie: Option<&str>) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let cookie = cookie
        .map(|c| format!("Cookie: {COOKIE}={c}\r\n"))
        .unwrap_or_default();
    let request = format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\n{cookie}\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response).await;
    String::from_utf8_lossy(&response).into_owned()
}

async fn ws_handshake(port: u16, cookie: Option<&str>) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let cookie = cookie
        .map(|c| format!("Cookie: {COOKIE}={c}\r\n"))
        .unwrap_or_default();
    let request = format!(
        "GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{cookie}Upgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = vec![0u8; 512];
    let n = stream.read(&mut response).await.unwrap();
    String::from_utf8_lossy(&response[..n]).into_owned()
}

type Ws = tokio_tungstenite::WebSocketStream<TcpStream>;

async fn ws_connect(port: u16, cookie: &str) -> Ws {
    let mut request = format!("ws://127.0.0.1:{port}/")
        .into_client_request()
        .unwrap();
    request.headers_mut().insert(
        "Cookie",
        HeaderValue::from_str(&format!("{COOKIE}={cookie}")).unwrap(),
    );
    let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let (ws, _) = tokio_tungstenite::client_async(request, stream)
        .await
        .expect("handshake");
    ws
}

/// Read until a binary frame contains `needle`.
async fn wait_for(ws: &mut Ws, needle: &str) {
    let found = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(Ok(message)) = ws.next().await {
            if let Message::Binary(data) = message {
                if data.windows(needle.len()).any(|w| w == needle.as_bytes()) {
                    return true;
                }
            }
        }
        false
    })
    .await;
    assert_eq!(found, Ok(true), "{needle:?} never reached the client");
}

fn click(handler_id: u32) -> Message {
    let mut frame = vec![0x00];
    rwire::protocol::varint::write_varint(&mut frame, handler_id);
    frame.extend_from_slice(&[Ev::Click.as_u8(), 0, 0]);
    Message::Binary(frame)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn valid_cookie_serves_the_page_and_handlers_see_the_identity() {
    let hub = Hub::start(19161).await;
    start(19061, hub_auth(19161)).await;

    for _ in 0..5 {
        let response = get(19061, "/", Some(TOKEN)).await;
        assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    }

    let mut ws = ws_connect(19061, TOKEN).await;
    wait_for(&mut ws, "who: not asked").await;
    ws.send(click(whoami().handler_id())).await.unwrap();
    wait_for(&mut ws, "who: ada token:true").await;

    assert_eq!(
        hub.calls(),
        1,
        "five pages and a handshake within the cache TTL: one hub call"
    );
}

#[tokio::test]
async fn anonymous_or_invalid_page_is_sent_to_login_with_next() {
    let hub = Hub::start(19162).await;
    start(19062, hub_auth(19162)).await;
    let location = format!("Location: {LOGIN}?next=%2Finbox%3Ftab%3D2\r\n");

    let anonymous = get(19062, "/inbox?tab=2", None).await;
    assert!(anonymous.starts_with("HTTP/1.1 303"), "{anonymous}");
    assert!(anonymous.contains(&location), "{anonymous}");
    assert_eq!(hub.calls(), 0, "no cookie: the hub is not asked");

    for _ in 1..=2 {
        let invalid = get(19062, "/inbox?tab=2", Some("forged")).await;
        assert!(invalid.starts_with("HTTP/1.1 303"), "{invalid}");
        assert!(invalid.contains(&location), "{invalid}");
        // A rejection is remembered briefly (never as a success).
        assert_eq!(hub.calls(), 1, "the rejection is remembered");
    }
}

#[tokio::test]
async fn websocket_without_a_valid_cookie_is_refused() {
    let hub = Hub::start(19163).await;
    start(19063, hub_auth(19163)).await;

    let anonymous = ws_handshake(19063, None).await;
    assert!(anonymous.starts_with("HTTP/1.1 401"), "{anonymous}");
    let invalid = ws_handshake(19063, Some("forged")).await;
    assert!(invalid.starts_with("HTTP/1.1 401"), "{invalid}");

    let valid = ws_handshake(19063, Some(TOKEN)).await;
    assert!(valid.contains("101 Switching Protocols"), "{valid}");
    assert_eq!(
        hub.calls(),
        2,
        "the forged and the valid handshakes; not the anonymous one"
    );
}

#[tokio::test]
async fn a_revoked_identity_closes_the_open_websocket() {
    let hub = Hub::start(19164).await;
    start(19064, hub_auth(19164).cache_ttl(Duration::from_millis(200))).await;

    let mut ws = ws_connect(19064, TOKEN).await;
    wait_for(&mut ws, "who: not asked").await;
    hub.revoke(TOKEN);

    let closed = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(message) = ws.next().await {
            if let Ok(Message::Close(Some(frame))) = message {
                return Some(frame.code);
            }
        }
        None
    })
    .await
    .expect("the socket stayed open after revocation");
    assert_eq!(closed, Some(CloseCode::from(CLOSE_UNAUTHORIZED)));

    let reconnect = ws_handshake(19064, Some(TOKEN)).await;
    assert!(reconnect.starts_with("HTTP/1.1 401"), "{reconnect}");
}

#[tokio::test]
async fn form_auth_and_delegated_auth_are_exclusive() {
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        Server::bind("127.0.0.1:19065")
            .unwrap()
            .root(app)
            .auth("admin", "secret")
            .auth_with(hub_auth(19165))
            .run(),
    )
    .await
    .expect("run() kept serving instead of refusing");
    let err = result.expect_err("both gates configured must not start");
    assert_eq!(err.to_string(), AUTH_CONFLICT);
}

/// Renderers have no context, so the identity reaches them through state: the
/// `on_route` handler, which runs on connect, sees it and copies it in.
#[handler]
fn on_connect(who: &mut Who, ctx: &EventContext) {
    if let Some(identity) = ctx.identity() {
        who.line = format!("routed as {}", identity.id());
    }
}

#[tokio::test]
async fn on_route_sees_the_identity_so_renderers_can_show_it() {
    let _hub = Hub::start(19166).await;
    tokio::spawn(async move {
        let _ = Server::bind("127.0.0.1:19066")
            .unwrap()
            .root(app)
            .on_route(on_connect())
            .auth_with(hub_auth(19166))
            .run()
            .await;
    });
    tokio::time::sleep(Duration::from_millis(150)).await;

    let mut ws = ws_connect(19066, TOKEN).await;
    // What the browser runtime sends on every open.
    ws.send(Message::Text("R/".to_string())).await.unwrap();
    wait_for(&mut ws, "routed as ada").await;
}

/// A burst of requests with one forged cookie asks the hub once: concurrent
/// ones share the validation in flight, later ones hit the remembered
/// rejection.
#[tokio::test]
async fn a_burst_of_forged_cookies_asks_the_hub_once() {
    let hub = Hub::start(19167).await;
    start(19067, hub_auth(19167)).await;
    let tasks: Vec<_> = (0..40)
        .map(|_| tokio::spawn(async { get(19067, "/", Some("forged")).await }))
        .collect();
    for task in tasks {
        let response = task.await.unwrap();
        assert!(response.starts_with("HTTP/1.1 303"), "{response}");
    }
    for _ in 0..10 {
        assert!(get(19067, "/", Some("forged"))
            .await
            .starts_with("HTTP/1.1 303"));
    }
    assert_eq!(hub.calls(), 1, "50 requests, one hub call");
}

/// A handshake over the admission caps is refused before its cookie reaches
/// the hub.
#[tokio::test]
async fn a_handshake_over_the_caps_never_reaches_the_hub() {
    let hub = Hub::start(19168).await;
    start_with(
        19068,
        hub_auth(19168),
        ServerConfig::new().max_connections_per_ip(0),
    )
    .await;
    for cookie in ["forged-1", "forged-2", TOKEN] {
        let response = ws_handshake(19068, Some(cookie)).await;
        assert!(response.starts_with("HTTP/1.1 503"), "{response}");
    }
    assert_eq!(hub.calls(), 0);
}

/// Wait for the socket's close frame (or `None` if it stays open `for_`).
async fn close_code(ws: &mut Ws, for_: Duration) -> Option<CloseCode> {
    tokio::time::timeout(for_, async {
        while let Some(message) = ws.next().await {
            match message {
                Ok(Message::Close(Some(frame))) => return Some(frame.code),
                Ok(Message::Close(None)) | Err(_) => return Some(CloseCode::Abnormal),
                _ => {}
            }
        }
        Some(CloseCode::Abnormal)
    })
    .await
    .ok()
    .flatten()
}

/// A hub outage is not a revocation: the open socket stays up through it,
/// new connections are refused meanwhile, and once the hub is back a
/// revocation closes the socket with 4401.
#[tokio::test]
async fn an_open_websocket_survives_a_hub_outage_but_not_a_revocation() {
    let hub = Hub::start(19169).await;
    let auth = hub_validation(19169)
        .cache_ttl(Duration::from_millis(100))
        .outage_grace(Duration::from_secs(30));
    start(19069, auth).await;
    let mut ws = ws_connect(19069, TOKEN).await;
    wait_for(&mut ws, "who: not asked").await;

    hub.set_down(true);
    let before = hub.calls();
    assert_eq!(close_code(&mut ws, Duration::from_millis(700)).await, None);
    assert!(
        hub.calls() >= before + 3,
        "the socket was re-validated during the outage"
    );
    let refused = ws_handshake(19069, Some("another-token")).await;
    assert!(
        refused.starts_with("HTTP/1.1 401"),
        "fail closed: {refused}"
    );

    hub.set_down(false);
    assert_eq!(close_code(&mut ws, Duration::from_millis(300)).await, None);
    hub.revoke(TOKEN);
    assert_eq!(
        close_code(&mut ws, Duration::from_secs(5)).await,
        Some(CloseCode::from(CLOSE_UNAUTHORIZED))
    );
}

/// Past the grace period, an outage closes the socket with 1013.
#[tokio::test]
async fn an_outage_longer_than_the_grace_closes_with_1013() {
    let hub = Hub::start(19170).await;
    let auth = hub_validation(19170)
        .cache_ttl(Duration::from_millis(100))
        .outage_grace(Duration::from_millis(500));
    start(19070, auth).await;
    let mut ws = ws_connect(19070, TOKEN).await;
    wait_for(&mut ws, "who: not asked").await;
    hub.set_down(true);
    assert_eq!(close_code(&mut ws, Duration::from_millis(300)).await, None);
    assert_eq!(
        close_code(&mut ws, Duration::from_secs(5)).await,
        Some(CloseCode::from(CLOSE_HUB_UNAVAILABLE))
    );
}

/// `/metrics` needs the session behind a gate; `/health` and `/ready` don't.
#[tokio::test]
async fn metrics_stay_behind_the_gate_unless_made_public() {
    let hub = Hub::start(19171).await;
    start(19072, hub_auth(19171)).await;
    let anonymous = get(19072, "/metrics", None).await;
    assert!(anonymous.starts_with("HTTP/1.1 303"), "{anonymous}");
    assert!(!anonymous.contains("rwire_"), "{anonymous}");
    let forged = get(19072, "/metrics", Some("forged")).await;
    assert!(!forged.contains("rwire_"), "{forged}");
    let session = get(19072, "/metrics", Some(TOKEN)).await;
    assert!(session.starts_with("HTTP/1.1 200"), "{session}");
    assert!(session.contains("rwire_"), "{session}");
    // With a query or as `HEAD`, the same gate and the same export.
    let query = get(19072, "/metrics?name=x", None).await;
    assert!(query.starts_with("HTTP/1.1 303"), "{query}");
    let query = get(19072, "/metrics?name=x", Some(TOKEN)).await;
    assert!(query.contains("rwire_"), "{query}");
    let head = request(19072, "HEAD", "/metrics", None).await;
    assert!(head.starts_with("HTTP/1.1 303"), "{head}");
    let head = request(19072, "HEAD", "/metrics", Some(TOKEN)).await;
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(head.contains("Content-Length: "), "{head}");
    assert!(head.ends_with("\r\n\r\n"), "HEAD has no body: {head}");
    for probe in ["/health", "/ready"] {
        let open = get(19072, probe, None).await;
        assert!(open.starts_with("HTTP/1.1 200"), "{probe}: {open}");
    }

    start_with(
        19073,
        hub_auth(19171),
        ServerConfig::new().public_metrics(true),
    )
    .await;
    let public = get(19073, "/metrics", None).await;
    assert!(public.starts_with("HTTP/1.1 200"), "{public}");
    assert!(public.contains("rwire_"), "{public}");
    let public = get(19073, "/metrics?name=x", None).await;
    assert!(public.contains("rwire_"), "{public}");
    let head = request(19073, "HEAD", "/metrics", None).await;
    assert!(
        head.starts_with("HTTP/1.1 200") && head.ends_with("\r\n\r\n"),
        "{head}"
    );
    // The forged and the valid cookie were checked; the public endpoint asks nothing.
    assert_eq!(hub.calls(), 2);
}

/// Without a gate, `/metrics` stays public, as before.
#[tokio::test]
async fn metrics_are_public_without_a_gate() {
    tokio::spawn(async {
        let _ = Server::bind("127.0.0.1:19074")
            .unwrap()
            .root(app)
            .run()
            .await;
    });
    tokio::time::sleep(Duration::from_millis(150)).await;
    let open = get(19074, "/metrics", None).await;
    assert!(open.starts_with("HTTP/1.1 200"), "{open}");
    assert!(open.contains("rwire_"), "{open}");
}

/// A login page served by the app itself, behind the gate, is not redirected
/// to itself (no 303 loop); other pages still are.
#[tokio::test]
async fn a_same_origin_login_page_is_not_redirected_to_itself() {
    let hub = Hub::start(19172).await;
    start(19075, hub_auth(19172).login_url("/login")).await;
    let login = get(19075, "/login?next=%2Finbox", None).await;
    assert!(login.starts_with("HTTP/1.1 401"), "{login}");
    let inbox = get(19075, "/inbox", None).await;
    assert!(inbox.starts_with("HTTP/1.1 303"), "{inbox}");
    assert!(
        inbox.contains("Location: /login?next=%2Finbox\r\n"),
        "{inbox}"
    );
    assert_eq!(hub.calls(), 0);
}

/// The hub client reads the status code, whatever the HTTP version.
#[test]
fn status_codes_are_read_from_any_http_version() {
    assert_eq!(status_code("HTTP/1.1 200 OK\r\n\r\nada"), Some(200));
    assert_eq!(status_code("HTTP/1.0 200 OK\r\n\r\nada"), Some(200));
    assert_eq!(status_code("HTTP/2 401\r\n\r\n"), Some(401));
    assert_eq!(status_code("garbage"), None);
    assert_eq!(status_code(""), None);
}

/// Behind a proxy that strips `/rwire`, the login redirect's `next` carries
/// the prefix, so the user comes back to the page they asked for.
#[tokio::test]
async fn the_login_next_includes_the_base_path() {
    let hub = Hub::start(19173).await;
    let auth = hub_auth(19173).login_url("/login?next=rwire");
    tokio::spawn(async move {
        let _ = Server::bind("127.0.0.1:19076")
            .unwrap()
            .base_path("/rwire")
            .root(app)
            .auth_with(auth)
            .run()
            .await;
    });
    tokio::time::sleep(Duration::from_millis(150)).await;
    // What the proxy forwards for `/rwire/s/office/x`.
    let page = get(19076, "/s/office/x", None).await;
    assert!(page.starts_with("HTTP/1.1 303"), "{page}");
    assert!(
        page.contains("Location: /login?next=rwire&next=%2Frwire%2Fs%2Foffice%2Fx\r\n"),
        "{page}"
    );
    assert_eq!(hub.calls(), 0);
}

/// `serve_metrics(false)` turns `/metrics` off, even with a session.
#[tokio::test]
async fn metrics_can_be_turned_off() {
    let hub = Hub::start(19174).await;
    start_with(
        19077,
        hub_auth(19174),
        ServerConfig::new()
            .serve_metrics(false)
            .public_metrics(true),
    )
    .await;
    for cookie in [None, Some(TOKEN)] {
        let response = get(19077, "/metrics", cookie).await;
        assert!(!response.contains("# HELP"), "{cookie:?}: {response}");
    }
    let health = get(19077, "/health", None).await;
    assert!(health.starts_with("HTTP/1.1 200"), "{health}");
    let _ = hub;
}
