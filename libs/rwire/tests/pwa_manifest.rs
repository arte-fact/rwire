//! The PWA manifest behind an auth gate.
//!
//! - The page's `<link rel="manifest">` asks for credentials, so a browser
//!   behind an HTTP auth in front of the app (a proxy's basic auth) sends them.
//! - rwire's own gates serve the manifest, the service worker and the icons
//!   before the gate: they load with or without a session, while the page
//!   itself stays gated.

use std::time::Duration;

use rwire::{
    el, AuthRequest, CapsuleConfig, DelegatedAuth, El, ElementBuilder, Identity, Pwa, Server,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

fn page() -> ElementBuilder {
    el(El::P).text("the app page")
}

enum Gate {
    Form,
    Delegated,
}

async fn start(port: u16, pwa: Pwa, gate: Gate) {
    let mut server = Server::bind(&format!("127.0.0.1:{port}"))
        .unwrap()
        .root(page)
        .capsule_config(CapsuleConfig::new().pwa(pwa));
    server = match gate {
        Gate::Form => server.auth("admin", "secret"),
        Gate::Delegated => server.auth_with(DelegatedAuth::new("sid", |req: AuthRequest| {
            let valid = req.cookie("sid") == Some("valid");
            async move { valid.then(|| Identity::new("reader")) }
        })),
    };
    tokio::spawn(async move {
        let _ = server.run().await;
    });
    for _ in 0..100 {
        if TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .is_ok()
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the server on {port} never started");
}

/// `GET path` with extra header lines; the whole response.
async fn get(port: u16, path: &str, headers: &str) -> String {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    stream
        .write_all(format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n{headers}\r\n").as_bytes())
        .await
        .unwrap();
    let mut response = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut response))
        .await
        .expect("no response")
        .unwrap();
    String::from_utf8_lossy(&response).into_owned()
}

const LINK: &str =
    "<link rel=\"manifest\" href=\"/manifest.webmanifest\" crossorigin=\"use-credentials\">";

#[tokio::test]
async fn the_page_asks_for_the_manifest_with_credentials() {
    start(19121, Pwa::new("News"), Gate::Delegated).await;
    let page = get(19121, "/", "Cookie: sid=valid\r\n").await;
    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert!(page.contains(LINK), "manifest link without credentials");

    start(
        19122,
        Pwa::new("News").manifest_credentials(false),
        Gate::Delegated,
    )
    .await;
    let page = get(19122, "/", "Cookie: sid=valid\r\n").await;
    assert!(page.contains("<link rel=\"manifest\" href=\"/manifest.webmanifest\">"));
    assert!(!page.contains("crossorigin=\"use-credentials\""));
}

/// With and without a session, behind both gates: the PWA files load, the
/// page does not.
#[tokio::test]
async fn pwa_files_load_with_and_without_a_session() {
    start(19123, Pwa::new("News"), Gate::Delegated).await;
    start(19124, Pwa::new("News"), Gate::Form).await;
    for (port, cookie) in [
        (19123, ""),
        (19123, "Cookie: sid=forged\r\n"),
        (19123, "Cookie: sid=valid\r\n"),
        (19124, ""),
    ] {
        let manifest = get(port, "/manifest.webmanifest", cookie).await;
        assert!(
            manifest.starts_with("HTTP/1.1 200 OK"),
            "{port} {cookie:?}:\n{manifest}"
        );
        assert!(manifest.contains("\"name\":\"News\""), "{manifest}");
        let sw = get(port, "/sw.js", cookie).await;
        assert!(
            sw.starts_with("HTTP/1.1 200 OK"),
            "{port} {cookie:?}:\n{sw}"
        );
        let icon = get(port, "/pwa/icon-512.png", cookie).await;
        assert!(icon.starts_with("HTTP/1.1 200 OK"), "{port} {cookie:?}");
    }
    // The page itself stays behind the gate.
    for port in [19123, 19124] {
        let page = get(port, "/", "").await;
        assert!(
            !page.contains(LINK),
            "{port}: the gated page was served:\n{page}"
        );
    }
}
