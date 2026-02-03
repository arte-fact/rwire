//! Minimal HTML capsule for RustWire runtime.
//!
//! Serves a dynamically generated HTML page that contains only what's needed to:
//! - Connect to WebSocket
//! - Execute DOM opcodes
//! - Send events back to server
//!
//! The capsule is tree-shaken to include only the element types and event types
//! actually used by the application.

use async_std::io::WriteExt;
use async_std::net::TcpStream;
use std::time::Duration;

use crate::session::{SessionId, COOKIE_MAX_AGE_SECS};

/// Serve a pre-generated capsule HTML over the TCP stream.
///
/// If `session_id` is provided and `is_new_session` is true, sets a session cookie.
pub async fn serve(
    mut stream: TcpStream,
    capsule: &str,
    session_id: Option<&SessionId>,
    is_new_session: bool,
) -> std::io::Result<()> {
    // Build HTTP response headers
    let mut headers = String::from("HTTP/1.1 200 OK\r\n");
    headers.push_str("Content-Type: text/html; charset=utf-8\r\n");
    headers.push_str("Connection: close\r\n");
    headers.push_str("Cache-Control: no-cache\r\n");

    // Set session cookie if this is a new session
    if let Some(sid) = session_id {
        if is_new_session {
            let cookie = sid.to_cookie(Some(Duration::from_secs(COOKIE_MAX_AGE_SECS)));
            headers.push_str(&format!("Set-Cookie: {}\r\n", cookie));
        }
    }

    headers.push_str(&format!("Content-Length: {}\r\n", capsule.len()));
    headers.push_str("\r\n");

    // Write response
    stream.write_all(headers.as_bytes()).await?;
    stream.write_all(capsule.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

/// Check if the HTTP request is a WebSocket upgrade request.
pub fn is_websocket_upgrade(headers: &str) -> bool {
    headers.to_lowercase().contains("upgrade: websocket")
}
