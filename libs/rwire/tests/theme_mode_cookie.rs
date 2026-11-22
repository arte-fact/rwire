//! A theme mode the visitor picked (the `rw_mode` cookie the runtime keeps)
//! reaches the first paint: the page's theme CSS is already in that mode.

use std::time::Duration;

use rwire::theme::{Theme, ThemeMode};
use rwire::{el, theme, CapsuleConfig, El, ElementBuilder, Server};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const PORT: u16 = 19230;

#[theme]
fn app_theme() -> Theme {
    Theme::new().mode(ThemeMode::System)
}

fn page() -> ElementBuilder {
    el(El::P).text("themed")
}

async fn wait_up(port: u16) {
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the server on {port} never started");
}

/// The page served for `GET /` with `cookie` (no gzip).
async fn page_with(cookie: &str) -> String {
    page_on(PORT, cookie).await
}

/// The response to `GET /` on `port` with `cookie` (no gzip), headers included.
async fn page_on(port: u16, cookie: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let cookie = if cookie.is_empty() {
        String::new()
    } else {
        format!("Cookie: {cookie}\r\n")
    };
    stream
        .write_all(format!("GET / HTTP/1.1\r\nHost: localhost\r\n{cookie}\r\n").as_bytes())
        .await
        .unwrap();
    let mut raw = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut raw))
        .await
        .expect("no response")
        .unwrap();
    String::from_utf8_lossy(&raw).into_owned()
}

#[tokio::test]
async fn the_first_paint_is_in_the_mode_the_visitor_picked() {
    tokio::spawn(async {
        let _ = Server::bind(&format!("127.0.0.1:{PORT}"))
            .unwrap()
            .root(page)
            .capsule_config(CapsuleConfig::new())
            .theme(app_theme())
            .run()
            .await;
    });
    wait_up(PORT).await;

    // The app's mode: both blocks, following the OS.
    let system = page_with("").await;
    assert!(system.contains("@media (prefers-color-scheme:dark){:root{color-scheme:dark;"));
    assert!(system.contains(":root{color-scheme:light;"));

    // A forced pick: that mode only.
    let dark = page_with("rwire_session=x; rw_mode=dark").await;
    assert!(dark.contains(":root{color-scheme:dark;"), "dark page");
    assert!(!dark.contains("prefers-color-scheme"), "dark is forced");
    assert!(!dark.contains("color-scheme:light"));
    let light = page_with("rw_mode=light").await;
    assert!(light.contains(":root{color-scheme:light;"));
    assert!(!light.contains("prefers-color-scheme"));

    // An unknown value is ignored.
    assert!(page_with("rw_mode=sepia")
        .await
        .contains("prefers-color-scheme"));
}

/// The page depends on the `rw_mode` cookie, so a cache must key it on cookies.
#[tokio::test]
async fn the_page_varies_on_cookies() {
    let response = page_on_started(PORT_VARY, None).await;
    let head = &response[..response.find("\r\n\r\n").unwrap()];
    let vary = head
        .lines()
        .find(|l| l.to_ascii_lowercase().starts_with("vary:"))
        .expect("a Vary header");
    assert!(vary.contains("Cookie"), "{vary}");
    assert!(vary.contains("Accept-Encoding"), "{vary}");
}

/// Behind a `base_path` (the proxy strips the prefix), the server still reads
/// the cookie the runtime scoped to the mount.
#[tokio::test]
async fn the_mode_cookie_is_read_behind_a_base_path() {
    let page = page_on_started(PORT_BASE, Some("/preview/x")).await;
    assert!(page.contains("/preview/x"), "the capsule knows its mount");
    let dark = page_on(PORT_BASE, "rw_mode=dark").await;
    assert!(dark.contains(":root{color-scheme:dark;"), "dark page");
    assert!(!dark.contains("prefers-color-scheme"));
}

const PORT_VARY: u16 = 19231;
const PORT_BASE: u16 = 19232;

/// Start a themed server on `port` (optionally under `base`), then GET `/`.
async fn page_on_started(port: u16, base: Option<&'static str>) -> String {
    tokio::spawn(async move {
        let mut server = Server::bind(&format!("127.0.0.1:{port}"))
            .unwrap()
            .root(page)
            .capsule_config(CapsuleConfig::new())
            .theme(app_theme());
        if let Some(base) = base {
            server = server.base_path(base);
        }
        let _ = server.run().await;
    });
    wait_up(port).await;
    page_on(port, "").await
}
