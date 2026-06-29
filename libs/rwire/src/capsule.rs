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

use crate::capsule_gen::{render_static_page, CapsuleConfig};
use crate::session::{SessionId, COOKIE_MAX_AGE_SECS};
use crate::{el, At, Av, El, ElementBuilder, St, Style};

/// Serve a pre-generated capsule HTML over the TCP stream.
///
/// If `session_id` is provided and `is_new_session` is true, sets a session cookie.
pub async fn serve(
    mut stream: TcpStream,
    capsule: &str,
    session_id: Option<&SessionId>,
    is_new_session: bool,
    secure_cookie: bool,
) -> std::io::Result<()> {
    // Build HTTP response headers
    let mut headers = String::from("HTTP/1.1 200 OK\r\n");
    headers.push_str("Content-Type: text/html; charset=utf-8\r\n");
    headers.push_str("Connection: close\r\n");
    headers.push_str("Cache-Control: no-cache\r\n");

    // Set session cookie if this is a new session
    if let Some(sid) = session_id {
        if is_new_session {
            let cookie = sid.to_cookie(
                Some(Duration::from_secs(COOKIE_MAX_AGE_SECS)),
                secure_cookie,
            );
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
/// `brand`, when set, is shown (with a glyph) as the form heading. `config` carries
/// the app theme + composite classes so the page renders with the design system.
pub async fn serve_login(
    mut stream: TcpStream,
    error: bool,
    brand: Option<&str>,
    config: &CapsuleConfig,
) -> std::io::Result<()> {
    let title = brand.unwrap_or("Sign in");
    let html = render_static_page(config, title, &login_page(error, brand));
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

/// The login page tree, built entirely from the design system: a full-viewport
/// flat card, theme colors and spacing via `.st(..)`, accent border on focus/hover
/// via `.focus`/`.hover`. `render_static_page` inlines the matching CSS, so there's
/// no hand-written stylesheet — the login looks exactly like the app it gates.
fn login_page(error: bool, brand: Option<&str>) -> ElementBuilder {
    el(El::Div)
        .st([
            St::MinHScreen,
            St::DisplayFlex,
            St::ItemsCenter,
            St::JustifyCenter,
            St::BgApp,
            St::TextDefault,
            St::FontMono,
            St::TextSm,
        ])
        .append([login_card(error, brand)])
}

fn login_card(error: bool, brand: Option<&str>) -> ElementBuilder {
    let title = brand.unwrap_or("Sign in");

    let mut brand_row = el(El::Div).st([St::DisplayFlex, St::ItemsCenter, St::GapSm]);
    if brand.is_some() {
        brand_row = brand_row.append([login_glyph()]);
    }
    brand_row = brand_row.append([el(El::Strong)
        .st([St::TextHigh, St::TextLg, St::FontSemibold])
        .text(title)]);

    let mut children: Vec<ElementBuilder> = vec![brand_row];
    if error {
        children.push(
            el(El::P)
                .st([St::TextError, St::TextXs])
                .text("Incorrect username or password."),
        );
    }
    children.push(login_field(
        "Username",
        login_input(
            el(El::Input)
                .attr("name", "username")
                .attr("autofocus", "")
                .attr("autocomplete", "username"),
        ),
    ));
    children.push(login_field(
        "Password",
        login_input(
            el(El::Input)
                .attr("name", "password")
                .at(At::Type, Av::Password)
                .attr("autocomplete", "current-password"),
        ),
    ));
    children.push(
        el(El::Button)
            .at(At::Type, Av::Submit)
            .st([
                St::WFull,
                St::PySm,
                St::BorderDefault,
                St::RoundedSm,
                St::BgSurface,
                St::TextDefault,
                St::CursorPointer,
            ])
            .style(Style::new().set("font", "inherit"))
            .hover([St::BorderAccent, St::TextHigh])
            .text("Sign in"),
    );

    el(El::Form)
        .attr("method", "POST")
        .attr("action", "/login")
        .st([
            St::DisplayFlex,
            St::FlexCol,
            St::GapMd,
            St::PMd,
            St::BgApp,
            St::BorderDefault,
            St::RoundedSm,
        ])
        .style(Style::new().width("min(22rem,90vw)"))
        .append(children)
}

fn login_input(input: ElementBuilder) -> ElementBuilder {
    input
        .st([
            St::WFull,
            St::PxSm,
            St::PySm,
            St::BorderDefault,
            St::RoundedSm,
            St::BgSurface,
            St::TextHigh,
            St::OutlineNone,
        ])
        .style(Style::new().set("font", "inherit"))
        .focus([St::BorderAccent])
}

fn login_field(label: &str, input: ElementBuilder) -> ElementBuilder {
    el(El::Div)
        .st([St::DisplayFlex, St::FlexCol, St::GapXs])
        .append([
            el(El::Label)
                .st([
                    St::TextXs,
                    St::TextMuted,
                    St::TextUppercase,
                    St::TrackingWide,
                ])
                .text(label),
            input,
        ])
}

/// The terminal-prompt brand glyph (matches the in-app wordmark icon); accent color
/// via `currentColor` inherited from the surrounding accent text.
fn login_glyph() -> ElementBuilder {
    el(El::Svg)
        .attr("width", "18")
        .attr("height", "18")
        .attr("viewBox", "0 0 24 24")
        .attr("fill", "none")
        .attr("stroke", "currentColor")
        .attr("stroke-width", "2")
        .attr("stroke-linecap", "round")
        .attr("stroke-linejoin", "round")
        .st([St::TextAccent])
        .style(Style::new().set("flex", "0 0 auto"))
        .append([el(El::Path).attr("d", "M4 17l6-5-6-5M12 19h8")])
}

/// Check if the HTTP request is a WebSocket upgrade request.
pub fn is_websocket_upgrade(headers: &str) -> bool {
    headers.to_lowercase().contains("upgrade: websocket")
}
