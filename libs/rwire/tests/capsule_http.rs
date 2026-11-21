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

/// `CapsuleConfig::title` is the page's one `<title>`, in the SSR paint too.
#[tokio::test]
async fn the_title_is_set_with_and_without_ssr() {
    for (port, ssr) in [(19186, false), (19187, true)] {
        tokio::spawn(async move {
            let _ = Server::bind(&format!("127.0.0.1:{port}"))
                .unwrap()
                .root(page)
                .capsule_config(CapsuleConfig::new().title("Office — agents").ssr(ssr))
                .run()
                .await;
        });
        wait_up(port).await;
        let page = text(&get(port, "/", "").await);
        assert_eq!(page.matches("<title>").count(), 1, "{page}");
        assert!(page.contains("<title>Office — agents</title>"), "{page}");
        if ssr {
            assert!(page.contains("the app page"), "no SSR paint");
        }
    }
}

fn header(raw: &[u8], name: &str) -> Option<String> {
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n")?;
    String::from_utf8_lossy(&raw[..split])
        .lines()
        .skip(1)
        .find_map(|line| {
            let (n, v) = line.split_once(':')?;
            n.trim()
                .eq_ignore_ascii_case(name)
                .then(|| v.trim().to_string())
        })
}

fn gunzip(bytes: &[u8]) -> Vec<u8> {
    use std::io::Read;
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(bytes)
        .read_to_end(&mut out)
        .expect("not gzip");
    out
}

const GZIP: &str = "Accept-Encoding: gzip, deflate, br\r\n";

/// The page, a text asset and the bundled extension module are gzipped for a
/// client that takes gzip, with `Vary: Accept-Encoding`; the bytes are the
/// same once decompressed. Images and the short 404 are sent as they are.
#[tokio::test]
async fn text_responses_are_gzipped_when_accepted() {
    let script = "export function i(doc) { doc.title = 'x'; }\n".repeat(80);
    let leaked: &'static str = Box::leak(script.clone().into_boxed_str());
    tokio::spawn(async move {
        let _ = Server::bind("127.0.0.1:19188")
            .unwrap()
            .root(page)
            .capsule_config(CapsuleConfig::new().title("Office"))
            .asset("/app.js", "text/javascript", leaked.as_bytes())
            .asset("/logo.png", "image/png", &b"\x89PNG not compressed"[..])
            .run()
            .await;
    });
    wait_up(19188).await;

    let plain = get(19188, "/", "").await;
    let packed = get(19188, "/", GZIP).await;
    assert_eq!(header(&plain, "content-encoding"), None);
    assert_eq!(header(&plain, "vary").as_deref(), Some("Accept-Encoding"));
    assert_eq!(header(&packed, "content-encoding").as_deref(), Some("gzip"));
    assert_eq!(header(&packed, "vary").as_deref(), Some("Accept-Encoding"));
    assert_eq!(gunzip(body(&packed)), body(&plain));
    eprintln!(
        "capsule: {} bytes plain, {} gzipped",
        body(&plain).len(),
        body(&packed).len()
    );
    assert!(body(&packed).len() * 2 < body(&plain).len());

    let js = get(19188, "/app.js", GZIP).await;
    assert_eq!(header(&js, "content-encoding").as_deref(), Some("gzip"));
    assert_eq!(gunzip(body(&js)), script.as_bytes());
    let js = get(19188, "/app.js", "Accept-Encoding: gzip;q=0\r\n").await;
    assert_eq!(header(&js, "content-encoding"), None);
    assert_eq!(body(&js), script.as_bytes());

    let png = get(19188, "/logo.png", GZIP).await;
    assert_eq!(header(&png, "content-encoding"), None);
    assert_eq!(header(&png, "vary"), None);

    let vim = get(19188, "/_rw/ext/vim.js", GZIP).await;
    assert_eq!(header(&vim, "content-encoding").as_deref(), Some("gzip"));
    let plain_vim = get(19188, "/_rw/ext/vim.js", "").await;
    assert_eq!(gunzip(body(&vim)), body(&plain_vim));

    let miss = get(19188, "/nope", GZIP).await;
    assert_eq!(header(&miss, "content-encoding"), None);
}

/// The per-request `ssr(true)` page is gzipped as it is served.
#[tokio::test]
async fn the_ssr_page_is_gzipped_per_request() {
    tokio::spawn(async {
        let _ = Server::bind("127.0.0.1:19189")
            .unwrap()
            .root(page)
            .capsule_config(CapsuleConfig::new().ssr(true))
            .run()
            .await;
    });
    wait_up(19189).await;
    let packed = get(19189, "/", GZIP).await;
    assert_eq!(header(&packed, "content-encoding").as_deref(), Some("gzip"));
    let html = String::from_utf8(gunzip(body(&packed))).unwrap();
    assert!(
        html.contains("<div id=\"rw\"><p>the app page</p></div>"),
        "{html}"
    );
    assert_eq!(html.as_bytes(), body(&get(19189, "/", "").await));
}
