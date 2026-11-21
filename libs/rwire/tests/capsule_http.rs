//! What the capsule's HTTP side answers: short 404s, the favicon, the
//! `<title>`, and gzip. (The router's not-found view is in
//! `capsule_not_found_view.rs`: the router is global to a test binary.)

use std::time::Duration;

use rwire::{el, handler, CapsuleConfig, El, ElementBuilder, Pwa, Server, State};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(State, Default)]
#[storage(memory)]
struct Nav {
    visits: u32,
}

#[handler]
fn on_nav(nav: &mut Nav) {
    nav.visits += 1;
}

fn page() -> ElementBuilder {
    el(El::P).text("the app page")
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

/// `GET path` with extra header lines; the raw response.
async fn get(port: u16, path: &str, headers: &str) -> Vec<u8> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    stream
        .write_all(format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n{headers}\r\n").as_bytes())
        .await
        .unwrap();
    let mut raw = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut raw))
        .await
        .expect("no response")
        .unwrap();
    raw
}

fn text(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw).into_owned()
}

/// Without a router or `on_route`, the app has one page, at `/`.
#[tokio::test]
async fn an_app_without_routes_has_one_page() {
    tokio::spawn(async {
        let _ = Server::bind("127.0.0.1:19181")
            .unwrap()
            .root(page)
            .run()
            .await;
    });
    wait_up(19181).await;
    for ok in ["/", "/?utm=x"] {
        let r = text(&get(19181, ok, "").await);
        assert!(r.starts_with("HTTP/1.1 200 OK"), "{ok}: {r}");
        assert!(r.contains("<script>"), "{ok}: not the page");
    }
    for miss in ["/favicon.ico", "/wp-login.php", "/a/b"] {
        let r = get(19181, miss, "").await;
        let t = text(&r);
        assert!(t.starts_with("HTTP/1.1 404 Not Found"), "{miss}: {t}");
        assert!(r.len() < 300, "{miss}: {} bytes", r.len());
        assert!(!t.contains("<script>"), "{miss}: the page was sent");
    }
}

/// With `on_route`, the app routes every path itself.
#[tokio::test]
async fn an_app_with_on_route_routes_every_path() {
    tokio::spawn(async {
        let _ = Server::bind("127.0.0.1:19182")
            .unwrap()
            .root(page)
            .on_route(on_nav())
            .run()
            .await;
    });
    wait_up(19182).await;
    let r = text(&get(19182, "/inbox/42", "").await);
    assert!(r.starts_with("HTTP/1.1 200 OK"), "{r}");
    assert!(r.contains("<script>"));
}

const ICON: &[u8] = b"\x89PNG fake icon";

/// The response body (after the blank line).
fn body(raw: &[u8]) -> &[u8] {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("no head");
    &raw[split + 4..]
}

/// `CapsuleConfig::favicon` serves `/favicon.ico` (before the auth gate,
/// like the other fixed files) and links it, once, even with a PWA.
#[tokio::test]
async fn the_favicon_is_served_and_linked() {
    tokio::spawn(async {
        let _ = Server::bind("127.0.0.1:19184")
            .unwrap()
            .root(page)
            .capsule_config(
                CapsuleConfig::new()
                    .favicon(ICON, "image/png")
                    .pwa(Pwa::new("Office")),
            )
            .auth("admin", "secret")
            .run()
            .await;
    });
    wait_up(19184).await;
    let r = get(19184, "/favicon.ico", "").await;
    assert!(text(&r).starts_with("HTTP/1.1 200 OK"), "{}", text(&r));
    assert!(text(&r).contains("Content-Type: image/png\r\n"));
    assert_eq!(body(&r), ICON);

    // The page (behind the login here, so read it from an open twin).
    tokio::spawn(async {
        let _ = Server::bind("127.0.0.1:19185")
            .unwrap()
            .root(page)
            .capsule_config(
                CapsuleConfig::new()
                    .favicon(ICON, "image/png")
                    .pwa(Pwa::new("Office")),
            )
            .run()
            .await;
    });
    wait_up(19185).await;
    let page = text(&get(19185, "/", "").await);
    assert_eq!(page.matches("rel=\"icon\"").count(), 1, "{page}");
    assert!(
        page.contains("<link rel=\"icon\" href=\"/favicon.ico\" type=\"image/png\">"),
        "{page}"
    );
    // The PWA keeps its home-screen icon.
    assert!(page.contains("apple-touch-icon"));
}
