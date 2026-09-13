//! Integration tests for the rwire server.

use async_std::io::ReadExt;
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::task;
use rwire::{
    el, El, ElementBuilder, Ev, HandlerSpec, MemoryState, ProxyResolver, Server, ServerConfig,
    State, StorageType,
};
use std::time::Duration;

/// Read a full HTTP response by parsing Content-Length and reading until complete.
async fn read_full_http_response(stream: &mut TcpStream) -> String {
    let mut buffer = Vec::with_capacity(16384);
    let mut temp = [0u8; 4096];

    // Read until we have the full response
    loop {
        let n = stream.read(&mut temp).await.unwrap();
        if n == 0 {
            break;
        }
        buffer.extend_from_slice(&temp[..n]);

        // Check if we have the full response
        let response_str = String::from_utf8_lossy(&buffer);
        if let Some(header_end) = response_str.find("\r\n\r\n") {
            // Parse Content-Length
            if let Some(cl_line) = response_str
                .lines()
                .find(|l| l.starts_with("Content-Length:"))
            {
                if let Ok(content_length) = cl_line
                    .split(':')
                    .nth(1)
                    .unwrap_or("0")
                    .trim()
                    .parse::<usize>()
                {
                    let body_start = header_end + 4;
                    let body_len = buffer.len() - body_start;
                    if body_len >= content_length {
                        break; // We have the full response
                    }
                }
            } else {
                // No Content-Length, assume response is complete after headers
                break;
            }
        }

        // Safety timeout - don't loop forever
        if buffer.len() > 65536 {
            break;
        }
    }

    String::from_utf8_lossy(&buffer).to_string()
}

// Test state
#[derive(Default)]
struct Counter {
    count: i32,
}

impl MemoryState for Counter {}
impl State for Counter {
    const STORAGE_TYPE: StorageType = StorageType::Memory;
}

fn increment(state: &mut Counter) {
    state.count += 1;
}

fn build_counter() -> ElementBuilder {
    el(El::Div).class("counter").append([
        el(El::Button).text("-"),
        el(El::Span).text("0"),
        el(El::Button)
            .text("+")
            .on(Ev::Click, HandlerSpec::memory(increment)),
    ])
}

fn build_simple() -> ElementBuilder {
    el(El::Div).class("simple").text("Hello")
}

/// Test that server binds and accepts connections
#[async_std::test]
async fn test_server_accepts_http() {
    // Start server in background
    let server_task = task::spawn(async {
        let _ = Server::bind("127.0.0.1:19001")
            .unwrap()
            .root(build_simple)
            .run()
            .await;
    });

    // Give server time to start
    task::sleep(Duration::from_millis(100)).await;

    // Connect and send HTTP request
    let mut stream = TcpStream::connect("127.0.0.1:19001").await.unwrap();
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .unwrap();

    // Read full response
    let response_str = read_full_http_response(&mut stream).await;

    // Verify HTTP response
    assert!(response_str.contains("HTTP/1.1 200 OK"));
    assert!(response_str.contains("text/html"));
    assert!(response_str.contains("<!DOCTYPE html>"));

    // Name maps ship empty; entries are delivered lazily over the wire via MAP_DEF.
    assert!(
        response_str.contains("__rwx"),
        "runtime artifact must be embedded"
    );

    drop(stream);
    server_task.cancel().await;
}

/// The element/event/attribute/style-token name maps ship empty: each entry is
/// delivered lazily over the wire via `MAP_DEF` the first time a code is referenced
/// (the name-map analogue of lazy CSS). So the served capsule inlines no names — and a
/// token reached only through a plain helper can never be missing, since its name is
/// sent exactly when its opcode is.
#[async_std::test]
async fn test_capsule_ships_empty_name_maps() {
    let server_task = task::spawn(async {
        let _ = Server::bind("127.0.0.1:19002")
            .unwrap()
            .root(build_simple)
            .run()
            .await;
    });

    task::sleep(Duration::from_millis(100)).await;

    let mut stream = TcpStream::connect("127.0.0.1:19002").await.unwrap();
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .unwrap();

    let response_str = read_full_http_response(&mut stream).await;

    // Maps ship empty; no element names are inlined into the capsule.
    assert!(
        response_str.contains("__rwx"),
        "runtime artifact must be embedded"
    );
    assert!(
        !response_str.contains("0:'div'"),
        "names must not be inlined into the capsule"
    );

    drop(stream);
    server_task.cancel().await;
}

/// A minimal upstream that echoes the request path it received, so a proxy test can assert the
/// prefix was stripped. Responds `Connection: close` so the reader sees EOF.
async fn spawn_echo_upstream(addr: &'static str) -> task::JoinHandle<()> {
    let listener = TcpListener::bind(addr).await.unwrap();
    task::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let mut buf = [0u8; 1024];
            let n = sock.read(&mut buf).await.unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]);
            let path = req
                .lines()
                .next()
                .and_then(|l| l.split(' ').nth(1))
                .unwrap_or("?")
                .to_string();
            let body = format!("UPSTREAM saw {path}");
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = sock.write_all(resp.as_bytes()).await;
            let _ = sock.flush().await;
        }
    })
}

/// The reverse proxy forwards a matched path to the upstream with its prefix stripped, and leaves
/// unmatched paths to the app itself — the same-origin preview mechanism (claw's P2 CD flow).
#[async_std::test]
async fn proxy_forwards_matched_paths_and_strips_prefix() {
    let upstream = spawn_echo_upstream("127.0.0.1:19010").await;
    let server = task::spawn(async {
        let _ = Server::bind("127.0.0.1:19011")
            .unwrap()
            .root(build_simple)
            .config(ServerConfig::new().proxy(ProxyResolver::new(|path| {
                path.starts_with("/preview/x")
                    .then(|| (19010u16, "/preview/x".to_string()))
            })))
            .run()
            .await;
    });
    task::sleep(Duration::from_millis(150)).await;

    // A proxied path reaches the upstream with `/preview/x` stripped off.
    let mut proxied = TcpStream::connect("127.0.0.1:19011").await.unwrap();
    proxied
        .write_all(b"GET /preview/x/dash?y=1 HTTP/1.1\r\nHost: h\r\n\r\n")
        .await
        .unwrap();
    let proxied_resp = read_full_http_response(&mut proxied).await;
    assert!(
        proxied_resp.contains("UPSTREAM saw /dash?y=1"),
        "proxy must strip the prefix and forward: {proxied_resp}"
    );

    // A non-matching path is served by the app itself, not proxied.
    let mut direct = TcpStream::connect("127.0.0.1:19011").await.unwrap();
    direct
        .write_all(b"GET / HTTP/1.1\r\nHost: h\r\n\r\n")
        .await
        .unwrap();
    let direct_resp = read_full_http_response(&mut direct).await;
    assert!(
        direct_resp.contains("<!DOCTYPE html>") && !direct_resp.contains("UPSTREAM"),
        "unmatched paths bypass the proxy: {direct_resp}"
    );

    drop(proxied);
    drop(direct);
    upstream.cancel().await;
    server.cancel().await;
}

/// Test that counter app capsule has correct elements
#[async_std::test]
async fn test_counter_capsule() {
    let server_task = task::spawn(async {
        let _ = Server::bind("127.0.0.1:19003")
            .unwrap()
            .root(build_counter)
            .run()
            .await;
    });

    task::sleep(Duration::from_millis(100)).await;

    let mut stream = TcpStream::connect("127.0.0.1:19003").await.unwrap();
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .unwrap();

    let response_str = read_full_http_response(&mut stream).await;

    // Element/event names are delivered lazily over the wire (MAP_DEF), not inlined.
    assert!(
        response_str.contains("__rwx"),
        "runtime artifact must be embedded"
    );
    assert!(!response_str.contains("0:'div'"));
    assert!(!response_str.contains("1:'click'"));

    drop(stream);
    server_task.cancel().await;
}

/// Test WebSocket upgrade detection
#[async_std::test]
async fn test_websocket_upgrade() {
    let server_task = task::spawn(async {
        let _ = Server::bind("127.0.0.1:19004")
            .unwrap()
            .root(build_simple)
            .run()
            .await;
    });

    task::sleep(Duration::from_millis(100)).await;

    let mut stream = TcpStream::connect("127.0.0.1:19004").await.unwrap();

    // Send WebSocket upgrade request
    let request = "GET / HTTP/1.1\r\n\
Host: localhost\r\n\
Upgrade: websocket\r\n\
Connection: Upgrade\r\n\
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
Sec-WebSocket-Version: 13\r\n\r\n";

    stream.write_all(request.as_bytes()).await.unwrap();

    let mut response = vec![0u8; 1024];
    let n = stream.read(&mut response).await.unwrap();
    let response_str = String::from_utf8_lossy(&response[..n]);

    // Should get WebSocket upgrade response
    let response_lower = response_str.to_lowercase();
    assert!(
        response_str.contains("101 Switching Protocols"),
        "Expected 101 response, got: {}",
        response_str
    );
    assert!(
        response_lower.contains("upgrade: websocket"),
        "Expected Upgrade header, got: {}",
        response_str
    );

    drop(stream);
    server_task.cancel().await;
}

/// Test multiple concurrent connections
#[async_std::test]
async fn test_concurrent_connections() {
    let server_task = task::spawn(async {
        let _ = Server::bind("127.0.0.1:19005")
            .unwrap()
            .root(build_simple)
            .run()
            .await;
    });

    task::sleep(Duration::from_millis(100)).await;

    // Spawn multiple concurrent requests
    let handles: Vec<_> = (0..5)
        .map(|_| {
            task::spawn(async {
                let mut stream = TcpStream::connect("127.0.0.1:19005").await.unwrap();
                stream
                    .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
                    .await
                    .unwrap();

                let mut response = vec![0u8; 8192];
                let n = stream.read(&mut response).await.unwrap();
                let response_str = String::from_utf8_lossy(&response[..n]);

                assert!(response_str.contains("HTTP/1.1 200 OK"));
                true
            })
        })
        .collect();

    // Wait for all requests
    for handle in handles {
        assert!(handle.await);
    }

    server_task.cancel().await;
}

/// Test capsule content-length header
#[async_std::test]
async fn test_content_length() {
    let server_task = task::spawn(async {
        let _ = Server::bind("127.0.0.1:19006")
            .unwrap()
            .root(build_simple)
            .run()
            .await;
    });

    task::sleep(Duration::from_millis(100)).await;

    let mut stream = TcpStream::connect("127.0.0.1:19006").await.unwrap();
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .unwrap();

    let mut response = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = stream.read(&mut buf).await.unwrap();
        if n == 0 {
            break;
        }
        response.extend_from_slice(&buf[..n]);
        // Check if we've received the full response (headers + body)
        if let Some(header_end) = response.windows(4).position(|w| w == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&response[..header_end]);
            if let Some(cl_line) = headers.lines().find(|l| l.starts_with("Content-Length:")) {
                if let Ok(cl) = cl_line.split(':').nth(1).unwrap().trim().parse::<usize>() {
                    if response.len() >= header_end + 4 + cl {
                        break;
                    }
                }
            }
        }
    }
    let response_str = String::from_utf8_lossy(&response);

    // Extract Content-Length
    let content_length: usize = response_str
        .lines()
        .find(|l| l.starts_with("Content-Length:"))
        .unwrap()
        .split(':')
        .nth(1)
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    // Find body (after \r\n\r\n)
    let body_start = response_str.find("\r\n\r\n").unwrap() + 4;
    let body = &response_str[body_start..];

    // Content-Length should match actual body length
    assert_eq!(content_length, body.len());

    drop(stream);
    server_task.cancel().await;
}

/// T2: a browser cross-origin WebSocket handshake is refused with 403; the
/// same-origin one (and a configured extra origin) upgrade normally.
#[async_std::test]
async fn test_websocket_origin_gate() {
    use rwire::ServerConfig;
    let server_task = task::spawn(async {
        let _ = Server::bind("127.0.0.1:19021")
            .unwrap()
            .root(build_simple)
            .config(ServerConfig::new().allow_origin("https://embed.example.com"))
            .run()
            .await;
    });
    task::sleep(Duration::from_millis(100)).await;

    async fn handshake(origin: Option<&str>) -> String {
        let mut stream = TcpStream::connect("127.0.0.1:19021").await.unwrap();
        let origin_line = origin
            .map(|o| format!("Origin: {o}\r\n"))
            .unwrap_or_default();
        let request = format!(
            "GET / HTTP/1.1\r\nHost: 127.0.0.1:19021\r\n{origin_line}Upgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = vec![0u8; 1024];
        let n = stream.read(&mut response).await.unwrap();
        String::from_utf8_lossy(&response[..n]).to_string()
    }

    // Cross-origin: refused before the upgrade.
    let r = handshake(Some("http://evil.example")).await;
    assert!(r.contains("403 Forbidden"), "expected 403, got: {r}");
    assert!(r.contains("cross_origin"), "expected reason, got: {r}");

    // Same-origin: upgrades.
    let r = handshake(Some("http://127.0.0.1:19021")).await;
    assert!(
        r.contains("101 Switching Protocols"),
        "expected 101, got: {r}"
    );

    // Allowlisted extra origin: upgrades.
    let r = handshake(Some("https://embed.example.com")).await;
    assert!(
        r.contains("101 Switching Protocols"),
        "expected 101, got: {r}"
    );

    // No Origin header (non-browser client): upgrades.
    let r = handshake(None).await;
    assert!(
        r.contains("101 Switching Protocols"),
        "expected 101, got: {r}"
    );

    server_task.cancel().await;
}
