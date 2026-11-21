//! Integration tests for the rwire server.

use rwire::{
    el, El, ElementBuilder, Ev, HandlerSpec, MemoryState, ProxyResolver, Server, ServerConfig,
    State, StorageType,
};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

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
#[tokio::test]
async fn test_server_accepts_http() {
    // Start server in background
    let (listener, addr) = free_listener();
    let server_task = tokio::spawn(async move {
        let _ = Server::bind("127.0.0.1:0")
            .unwrap()
            .root(build_simple)
            .run_on(listener)
            .await;
    });

    // Connect and send HTTP request
    let mut stream = TcpStream::connect(addr).await.unwrap();
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
    server_task.abort();
}

/// The element/event/attribute/style-token name maps ship empty: each entry is
/// delivered lazily over the wire via `MAP_DEF` the first time a code is referenced
/// (the name-map analogue of lazy CSS). So the served capsule inlines no names — and a
/// token reached only through a plain helper can never be missing, since its name is
/// sent exactly when its opcode is.
#[tokio::test]
async fn test_capsule_ships_empty_name_maps() {
    let (listener, addr) = free_listener();
    let server_task = tokio::spawn(async move {
        let _ = Server::bind("127.0.0.1:0")
            .unwrap()
            .root(build_simple)
            .run_on(listener)
            .await;
    });

    let mut stream = TcpStream::connect(addr).await.unwrap();
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
    server_task.abort();
}

/// A minimal upstream that echoes the request path it received, so a proxy test can assert the
/// prefix was stripped. Responds `Connection: close` so the reader sees EOF.
async fn spawn_echo_upstream() -> (tokio::task::JoinHandle<()>, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let task = tokio::spawn(async move {
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
    });
    (task, port)
}

/// The reverse proxy forwards a matched path to the upstream with its prefix stripped, and leaves
/// unmatched paths to the app itself — the same-origin preview mechanism (claw's P2 CD flow).
#[tokio::test]
async fn proxy_forwards_matched_paths_and_strips_prefix() {
    let (upstream, upstream_port) = spawn_echo_upstream().await;
    let (listener, addr) = free_listener();
    let server = tokio::spawn(async move {
        let _ = Server::bind("127.0.0.1:0")
            .unwrap()
            .root(build_simple)
            .config(ServerConfig::new().proxy(ProxyResolver::new(move |path| {
                if path.starts_with("/preview/x") {
                    Some(rwire::ProxyDecision::Forward {
                        port: upstream_port,
                        prefix: "/preview/x".to_string(),
                    })
                } else if path.starts_with("/preview/") {
                    Some(rwire::ProxyDecision::Gone {
                        message: "this preview isn't running".to_string(),
                    })
                } else {
                    None
                }
            })))
            .run_on(listener)
            .await;
    });

    // A proxied path reaches the upstream with `/preview/x` stripped off.
    let mut proxied = TcpStream::connect(addr).await.unwrap();
    proxied
        .write_all(b"GET /preview/x/dash?y=1 HTTP/1.1\r\nHost: h\r\n\r\n")
        .await
        .unwrap();
    let proxied_resp = read_full_http_response(&mut proxied).await;
    assert!(
        proxied_resp.contains("UPSTREAM saw /dash?y=1"),
        "proxy must strip the prefix and forward: {proxied_resp}"
    );

    // A claimed path with no live upstream gets the human 503 page — never the app shell
    // (a stale preview link must not open a second copy of the app).
    let mut gone = TcpStream::connect(addr).await.unwrap();
    gone.write_all(b"GET /preview/dead/whatever HTTP/1.1\r\nHost: h\r\n\r\n")
        .await
        .unwrap();
    let gone_resp = read_full_http_response(&mut gone).await;
    assert!(
        gone_resp.contains("503")
            && gone_resp.contains("this preview isn't running")
            && !gone_resp.contains("__rwx"),
        "a Gone path serves the 503 page, not the capsule: {gone_resp}"
    );

    // A non-matching path is served by the app itself, not proxied.
    let mut direct = TcpStream::connect(addr).await.unwrap();
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
    drop(gone);
    drop(direct);
    upstream.abort();
    server.abort();
}

/// Test that counter app capsule has correct elements
#[tokio::test]
async fn test_counter_capsule() {
    let (listener, addr) = free_listener();
    let server_task = tokio::spawn(async move {
        let _ = Server::bind("127.0.0.1:0")
            .unwrap()
            .root(build_counter)
            .run_on(listener)
            .await;
    });

    let mut stream = TcpStream::connect(addr).await.unwrap();
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
    server_task.abort();
}

/// Test WebSocket upgrade detection
#[tokio::test]
async fn test_websocket_upgrade() {
    let (listener, addr) = free_listener();
    let server_task = tokio::spawn(async move {
        let _ = Server::bind("127.0.0.1:0")
            .unwrap()
            .root(build_simple)
            .run_on(listener)
            .await;
    });

    let mut stream = TcpStream::connect(addr).await.unwrap();

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
    server_task.abort();
}

/// Test multiple concurrent connections
#[tokio::test]
async fn test_concurrent_connections() {
    let (listener, addr) = free_listener();
    let server_task = tokio::spawn(async move {
        let _ = Server::bind("127.0.0.1:0")
            .unwrap()
            .root(build_simple)
            .run_on(listener)
            .await;
    });

    // Spawn multiple concurrent requests
    let handles: Vec<_> = (0..5)
        .map(|_| {
            tokio::spawn(async move {
                let mut stream = TcpStream::connect(addr).await.unwrap();
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
        assert!(handle.await.unwrap());
    }

    server_task.abort();
}

/// Test capsule content-length header
#[tokio::test]
async fn test_content_length() {
    let (listener, addr) = free_listener();
    let server_task = tokio::spawn(async move {
        let _ = Server::bind("127.0.0.1:0")
            .unwrap()
            .root(build_simple)
            .run_on(listener)
            .await;
    });

    let mut stream = TcpStream::connect(addr).await.unwrap();
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
    server_task.abort();
}

/// T2: a browser cross-origin WebSocket handshake is refused with 403; the
/// same-origin one (and a configured extra origin) upgrade normally.
#[tokio::test]
async fn test_websocket_origin_gate() {
    use rwire::ServerConfig;
    let (listener, addr) = free_listener();
    let server_task = tokio::spawn(async move {
        let _ = Server::bind("127.0.0.1:0")
            .unwrap()
            .root(build_simple)
            .config(ServerConfig::new().allow_origin("https://embed.example.com"))
            .run_on(listener)
            .await;
    });

    async fn handshake(addr: std::net::SocketAddr, origin: Option<&str>) -> String {
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let origin_line = origin
            .map(|o| format!("Origin: {o}\r\n"))
            .unwrap_or_default();
        let request = format!(
            "GET / HTTP/1.1\r\nHost: {addr}\r\n{origin_line}Upgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = vec![0u8; 1024];
        let n = stream.read(&mut response).await.unwrap();
        String::from_utf8_lossy(&response[..n]).to_string()
    }

    // Cross-origin: refused before the upgrade.
    let r = handshake(addr, Some("http://evil.example")).await;
    assert!(r.contains("403 Forbidden"), "expected 403, got: {r}");
    assert!(r.contains("cross_origin"), "expected reason, got: {r}");

    // Same-origin: upgrades.
    let r = handshake(addr, Some(&format!("http://{addr}"))).await;
    assert!(
        r.contains("101 Switching Protocols"),
        "expected 101, got: {r}"
    );

    // Allowlisted extra origin: upgrades.
    let r = handshake(addr, Some("https://embed.example.com")).await;
    assert!(
        r.contains("101 Switching Protocols"),
        "expected 101, got: {r}"
    );

    // No Origin header (non-browser client): upgrades.
    let r = handshake(addr, None).await;
    assert!(
        r.contains("101 Switching Protocols"),
        "expected 101, got: {r}"
    );

    server_task.abort();
}

/// A listener on a free port (port 0), and its address. The server takes it
/// with `run_on`: connections queue from now on, so no startup sleep.
fn free_listener() -> (std::net::TcpListener, std::net::SocketAddr) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    (listener, addr)
}
