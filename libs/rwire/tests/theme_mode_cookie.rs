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
    let mut stream = TcpStream::connect(("127.0.0.1", PORT)).await.unwrap();
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
    assert!(page_with("rw_mode=sepia").await.contains("prefers-color-scheme"));
}
