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

/// Send a `401 Unauthorized` (used to reject an unauthenticated WebSocket
/// upgrade — the browser can't show a form there, so it just fails and the page
/// reload lands on the login screen).
pub async fn serve_unauthorized(mut stream: TcpStream) -> std::io::Result<()> {
    let body = "Authentication required.";
    let resp = format!(
        "HTTP/1.1 401 Unauthorized\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(resp.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

/// Serve the login page (`200`), showing an error banner when `error` is set.
pub async fn serve_login(mut stream: TcpStream, error: bool) -> std::io::Result<()> {
    let html = login_html(error);
    let resp = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/html; charset=utf-8\r\n\
         Cache-Control: no-store\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n{html}",
        html.len()
    );
    stream.write_all(resp.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

/// Send a `303 See Other` redirect, optionally with a `Set-Cookie` header.
pub async fn serve_redirect(
    mut stream: TcpStream,
    location: &str,
    set_cookie: Option<&str>,
) -> std::io::Result<()> {
    let cookie = set_cookie.map_or_else(String::new, |c| format!("Set-Cookie: {c}\r\n"));
    let resp = format!(
        "HTTP/1.1 303 See Other\r\n\
         Location: {location}\r\n\
         {cookie}\
         Content-Length: 0\r\n\
         Connection: close\r\n\r\n"
    );
    stream.write_all(resp.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

/// Standalone dark-themed login page (self-contained; no capsule runtime/CSS).
fn login_html(error: bool) -> String {
    let err = if error {
        "<p style=\"color:#bf616a;margin:0 0 1rem\">Incorrect username or password.</p>"
    } else {
        ""
    };
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Sign in</title></head>\
<body style=\"margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;\
background:#2e3440;color:#eceff4;font-family:system-ui,sans-serif\">\
<form method=\"POST\" action=\"/login\" style=\"background:#3b4252;padding:2rem;border-radius:12px;\
width:min(20rem,90vw);box-shadow:0 10px 40px rgba(0,0,0,.4)\">\
<h1 style=\"margin:0 0 1.25rem;font-size:1.25rem\">Node Monitor</h1>{err}\
<label style=\"display:block;font-size:.8rem;margin:0 0 .25rem;color:#d8dee9\">Username</label>\
<input name=\"username\" autofocus autocomplete=\"username\" style=\"width:100%;box-sizing:border-box;\
padding:.55rem;margin:0 0 .9rem;border:1px solid #4c566a;border-radius:6px;background:#2e3440;color:#eceff4\">\
<label style=\"display:block;font-size:.8rem;margin:0 0 .25rem;color:#d8dee9\">Password</label>\
<input name=\"password\" type=\"password\" autocomplete=\"current-password\" style=\"width:100%;box-sizing:border-box;\
padding:.55rem;margin:0 0 1.25rem;border:1px solid #4c566a;border-radius:6px;background:#2e3440;color:#eceff4\">\
<button type=\"submit\" style=\"width:100%;padding:.6rem;border:0;border-radius:6px;background:#5e81ac;\
color:#fff;font-size:.95rem;cursor:pointer\">Sign in</button></form></body></html>"
    )
}

/// Check if the HTTP request is a WebSocket upgrade request.
pub fn is_websocket_upgrade(headers: &str) -> bool {
    headers.to_lowercase().contains("upgrade: websocket")
}
