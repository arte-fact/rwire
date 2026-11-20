//! Delegated authentication end to end, against a fake identity hub: a tiny local
//! HTTP server answering `GET /admin/session` with `200 <user>` for a known
//! Bearer token and `401` otherwise — the shape of the mcp-agents hub.

use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_std::io::{ReadExt, WriteExt};
use async_std::net::{TcpListener, TcpStream};
use async_std::task;
use async_tungstenite::tungstenite::client::IntoClientRequest;
use async_tungstenite::tungstenite::http::HeaderValue;
use async_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use async_tungstenite::tungstenite::Message;
use futures::{SinkExt, StreamExt};
use rwire::delegated_auth::CLOSE_UNAUTHORIZED;
use rwire::{
    el, handler, renderer, AuthRequest, DelegatedAuth, El, ElementBuilder, Ev, Identity, Server,
    State, AUTH_CONFLICT,
};

const COOKIE: &str = "hub_session";
const TOKEN: &str = "tok-ada";
const LOGIN: &str = "https://hub.example/login";

// ── Fake hub ────────────────────────────────────────────────────────────────

/// Valid tokens and a count of `/admin/session` calls.
#[derive(Clone, Default)]
struct Hub {
    valid: Arc<Mutex<HashSet<String>>>,
    calls: Arc<AtomicUsize>,
}

impl Hub {
    async fn start(port: u16) -> Self {
        let hub = Hub::default();
        hub.valid.lock().unwrap().insert(TOKEN.to_string());
        let listener = TcpListener::bind(("127.0.0.1", port)).await.unwrap();
        let served = hub.clone();
        task::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let hub = served.clone();
                task::spawn(async move {
                    let head = read_head(&mut stream).await;
                    hub.calls.fetch_add(1, Ordering::SeqCst);
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
            if !answer.starts_with("HTTP/1.1 200") {
                return None;
            }
            let user = answer.split("\r\n\r\n").nth(1)?.to_string();
            Some(Identity::new(user).with_data(token))
        }
    })
    .login_url(LOGIN)
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
    task::spawn(async move {
        let _ = Server::bind(&format!("127.0.0.1:{port}"))
            .unwrap()
            .root(app)
            .auth_with(auth)
            .run()
            .await;
    });
    task::sleep(Duration::from_millis(150)).await;
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

type Ws = async_tungstenite::WebSocketStream<TcpStream>;

async fn ws_connect(port: u16, cookie: &str) -> Ws {
    let mut request = format!("ws://127.0.0.1:{port}/")
        .into_client_request()
        .unwrap();
    request.headers_mut().insert(
        "Cookie",
        HeaderValue::from_str(&format!("{COOKIE}={cookie}")).unwrap(),
    );
    let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let (ws, _) = async_tungstenite::client_async(request, stream)
        .await
        .expect("handshake");
    ws
}

/// Read until a binary frame contains `needle`.
async fn wait_for(ws: &mut Ws, needle: &str) {
    let found = async_std::future::timeout(Duration::from_secs(5), async {
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

#[async_std::test]
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

#[async_std::test]
async fn anonymous_or_invalid_page_is_sent_to_login_with_next() {
    let hub = Hub::start(19162).await;
    start(19062, hub_auth(19162)).await;
    let location = format!("Location: {LOGIN}?next=%2Finbox%3Ftab%3D2\r\n");

    let anonymous = get(19062, "/inbox?tab=2", None).await;
    assert!(anonymous.starts_with("HTTP/1.1 303"), "{anonymous}");
    assert!(anonymous.contains(&location), "{anonymous}");
    assert_eq!(hub.calls(), 0, "no cookie: the hub is not asked");

    for attempt in 1..=2 {
        let invalid = get(19062, "/inbox?tab=2", Some("forged")).await;
        assert!(invalid.starts_with("HTTP/1.1 303"), "{invalid}");
        assert!(invalid.contains(&location), "{invalid}");
        assert_eq!(hub.calls(), attempt, "a failure is never cached");
    }
}

#[async_std::test]
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

#[async_std::test]
async fn a_revoked_identity_closes_the_open_websocket() {
    let hub = Hub::start(19164).await;
    start(19064, hub_auth(19164).cache_ttl(Duration::from_millis(200))).await;

    let mut ws = ws_connect(19064, TOKEN).await;
    wait_for(&mut ws, "who: not asked").await;
    hub.revoke(TOKEN);

    let closed = async_std::future::timeout(Duration::from_secs(5), async {
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

#[async_std::test]
async fn form_auth_and_delegated_auth_are_exclusive() {
    let result = async_std::future::timeout(
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

#[async_std::test]
async fn on_route_sees_the_identity_so_renderers_can_show_it() {
    let _hub = Hub::start(19166).await;
    task::spawn(async move {
        let _ = Server::bind("127.0.0.1:19066")
            .unwrap()
            .root(app)
            .on_route(on_connect())
            .auth_with(hub_auth(19166))
            .run()
            .await;
    });
    task::sleep(Duration::from_millis(150)).await;

    let mut ws = ws_connect(19066, TOKEN).await;
    // What the browser runtime sends on every open.
    ws.send(Message::Text("R/".to_string())).await.unwrap();
    wait_for(&mut ws, "routed as ada").await;
}
