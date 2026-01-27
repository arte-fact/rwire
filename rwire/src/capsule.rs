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

/// HTTP response headers for the capsule.
const HTTP_RESPONSE_HEADER: &str = "HTTP/1.1 200 OK\r\n\
Content-Type: text/html; charset=utf-8\r\n\
Connection: close\r\n\
Cache-Control: no-cache\r\n";

/// Serve a pre-generated capsule HTML over the TCP stream.
pub async fn serve(mut stream: TcpStream, capsule: &str) -> std::io::Result<()> {
    // Write HTTP response
    let response = format!(
        "{}Content-Length: {}\r\n\r\n{}",
        HTTP_RESPONSE_HEADER,
        capsule.len(),
        capsule
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

/// Check if the HTTP request is a WebSocket upgrade request.
pub fn is_websocket_upgrade(headers: &str) -> bool {
    headers.to_lowercase().contains("upgrade: websocket")
}
