//! Integration tests for the rwire server.

use async_std::io::ReadExt;
use async_std::net::TcpStream;
use async_std::prelude::*;
use async_std::task;
use rwire::{el, El, ElementBuilder, Ev, HandlerSpec, MemoryState, Server, State, StorageType};
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
            if let Some(cl_line) = response_str.lines().find(|l| l.starts_with("Content-Length:"))
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

    // Verify capsule contains our element/event mappings
    assert!(response_str.contains("const E="));
    assert!(response_str.contains("const V="));

    drop(stream);
    server_task.cancel().await;
}

/// Test that capsule contains tree-shaken element types
#[async_std::test]
async fn test_capsule_tree_shaking() {
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

    // Simple component only uses div - should only have div in mappings
    assert!(response_str.contains("0:'div'"));
    // Should NOT contain button since we don't use it
    assert!(!response_str.contains("2:'button'"));

    drop(stream);
    server_task.cancel().await;
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

    // Counter uses div, span, button
    assert!(response_str.contains("0:'div'"));
    assert!(response_str.contains("1:'span'"));
    assert!(response_str.contains("2:'button'"));

    // Counter uses click event
    assert!(response_str.contains("1:'click'"));

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

    let mut response = vec![0u8; 8192];
    let n = stream.read(&mut response).await.unwrap();
    let response_str = String::from_utf8_lossy(&response[..n]);

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
