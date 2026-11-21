//! End to end: a Markdown page with trapped links, served with `ssr(true)`.
//! The pre-rendered HTML reaches the browser before the WebSocket runtime and
//! its sanitizer, so it must carry no executable URL.

use std::time::Duration;

use rwire::{CapsuleConfig, ElementBuilder, Server};
use rwire_markdown::Markdown;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const PAGE: &str = "# Trapped post

[plain](javascript:alert(1)) [cased](JaVaScRiPt:alert(2))
[entity](&#106;avascript:alert(3)) [colon](javascript&#58;alert(4))
[tab](<java\tscript:alert(5)>) [percent](%6Aavascript:alert(6))
[vb](vbscript:msgbox(7)) [html](data:text/html,<script>alert(8)</script>)
[ref][evil] <javascript:alert(9)>

![svg](data:image/svg+xml,<svg onload=alert(10)>)

[evil]: javascript:alert(11)

Legit: [docs](/docs/intro), [anchor](#install), [site](https://example.com),
[mail](mailto:me@example.com).
";

fn page() -> ElementBuilder {
    Markdown::new(PAGE).build()
}

#[tokio::test]
async fn ssr_serves_no_executable_url() {
    let server = Server::bind("127.0.0.1:19131")
        .unwrap()
        .root(page)
        .capsule_config(CapsuleConfig::new().ssr(true));
    tokio::spawn(async move {
        let _ = server.run().await;
    });
    let mut stream = None;
    for _ in 0..100 {
        if let Ok(s) = TcpStream::connect("127.0.0.1:19131").await {
            stream = Some(s);
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let mut stream = stream.expect("the server never started");
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .unwrap();
    let mut raw = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut raw))
        .await
        .expect("no response")
        .unwrap();
    let page = String::from_utf8_lossy(&raw);
    // The static paint only: the inlined runtime follows it.
    let paint = &page[page.find("<div id=\"rw\">").expect("no mount")..];
    let paint = &paint[..paint.find("<script").unwrap_or(paint.len())];

    assert!(paint.contains("Trapped post"), "no static paint:\n{paint}");
    for scheme in ["javascript", "vbscript", "data:text", "data:image/svg"] {
        assert!(
            !paint.to_lowercase().contains(&format!("=\"{scheme}")),
            "{scheme} URL in the static paint:\n{paint}"
        );
    }
    assert!(!paint.contains("%6Aavascript"), "{paint}");
    assert!(!paint.contains("&amp;#106;avascript"), "{paint}");
    for href in [
        "/docs/intro",
        "#install",
        "https://example.com",
        "mailto:me@example.com",
    ] {
        assert!(
            paint.contains(&format!("href=\"{href}\"")),
            "{href} lost:\n{paint}"
        );
    }
}
