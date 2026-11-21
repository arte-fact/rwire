//! A router with a declared not-found view keeps the page for an unknown path
//! (404 + the capsule, so the view renders). In its own binary: the installed
//! router is global to the process.

use std::time::Duration;

use rwire::{el, El, Router, Server};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

async fn get(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    stream
        .write_all(format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes())
        .await
        .unwrap();
    let mut raw = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut raw))
        .await
        .expect("no response")
        .unwrap();
    String::from_utf8_lossy(&raw).into_owned()
}

/// With the router: a declared not-found view keeps the page (404); without
/// one, a short 404.
#[tokio::test]
async fn the_router_not_found_view_keeps_the_page() {
    tokio::spawn(async {
        let _ = Server::bind("127.0.0.1:19183")
            .unwrap()
            .root(|| el(El::Main).append([rwire::outlet()]))
            .routes(
                Router::new()
                    .page("/", |_| el(El::P).text("home"))
                    .not_found(|_| el(El::P).text("lost")),
            )
            .run()
            .await;
    });
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", 19183)).await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let r = get(19183, "/nowhere").await;
    assert!(r.starts_with("HTTP/1.1 404 Not Found"), "{r}");
    assert!(
        r.contains("<script>"),
        "the page must load to render the view"
    );
    let r = get(19183, "/").await;
    assert!(r.starts_with("HTTP/1.1 200 OK"), "{r}");
}
