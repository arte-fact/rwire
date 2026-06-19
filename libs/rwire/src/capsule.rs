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
/// `brand`, when set, is shown (with a glyph) as the form heading.
pub async fn serve_login(
    mut stream: TcpStream,
    error: bool,
    brand: Option<&str>,
) -> std::io::Result<()> {
    let html = login_html(error, brand);
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
fn login_html(error: bool, brand: Option<&str>) -> String {
    let err = if error {
        "<p class=\"err\">Incorrect username or password.</p>"
    } else {
        ""
    };
    let title = brand.unwrap_or("Sign in");
    let glyph = if brand.is_some() {
        "<svg width=\"18\" height=\"18\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"#88c0d0\" \
stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" style=\"flex:0 0 auto\">\
<path d=\"M4 17l6-5-6-5M12 19h8\"/></svg>"
    } else {
        ""
    };
    // Self-contained (no capsule runtime/CSS): flat, hairline, mono, Nord — the
    // terminal look. Inputs/buttons take an accent border on focus/hover.
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{title}</title>\
<style>*{{box-sizing:border-box}}\
body{{margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;\
background:#2e3440;color:#d8dee9;\
font-family:'Fira Code',ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;font-size:.85rem}}\
form{{background:#2e3440;border:1px solid #434c5e;border-radius:3px;padding:1.25rem;\
width:min(22rem,90vw);display:flex;flex-direction:column;gap:.7rem}}\
.brand{{display:flex;align-items:center;gap:.5rem;margin-bottom:.25rem}}\
.brand b{{color:#eceff4;font-size:1.05rem}}\
.field{{display:flex;flex-direction:column;gap:.25rem}}\
label{{font-size:.7rem;color:#81a1c1;text-transform:uppercase;letter-spacing:.05em}}\
input{{width:100%;padding:.5rem .6rem;border:1px solid #434c5e;border-radius:3px;\
background:#3b4252;color:#eceff4;font:inherit;outline:none}}\
input::placeholder{{color:#4c566a}}\
input:focus{{border-color:#88c0d0}}\
button{{margin-top:.25rem;padding:.55rem;border:1px solid #434c5e;border-radius:3px;\
background:#3b4252;color:#d8dee9;font:inherit;cursor:pointer}}\
button:hover{{border-color:#88c0d0;color:#eceff4}}\
.err{{color:#bf616a;font-size:.8rem;margin:0}}</style></head>\
<body><form method=\"POST\" action=\"/login\">\
<div class=\"brand\">{glyph}<b>{title}</b></div>{err}\
<div class=\"field\"><label>Username</label>\
<input name=\"username\" autofocus autocomplete=\"username\"></div>\
<div class=\"field\"><label>Password</label>\
<input name=\"password\" type=\"password\" autocomplete=\"current-password\"></div>\
<button type=\"submit\">Sign in</button></form></body></html>"
    )
}

/// Check if the HTTP request is a WebSocket upgrade request.
pub fn is_websocket_upgrade(headers: &str) -> bool {
    headers.to_lowercase().contains("upgrade: websocket")
}
