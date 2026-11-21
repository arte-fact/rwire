//! `asset_dir()`: a folder served from disk on each request.
//!
//! Layout of each test's folder (under the system temp dir):
//!
//! ```text
//! base/
//!   secret.txt              outside the served folder
//!   media/                  served under /media
//!     cover.png
//!     .hidden
//!     sub/inner.txt
//!     inside -> cover.png   link inside the folder: served
//!     outside -> ../secret.txt
//!     escape -> ..          link to a directory outside
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use rwire::{el, AuthRequest, DelegatedAuth, El, ElementBuilder, Identity, Server};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const SECRET: &str = "top secret: outside the media folder";
const COVER: &[u8] = b"\x89PNG fake cover bytes";

fn page() -> ElementBuilder {
    el(El::P).text("the app page")
}

/// A fresh `base/` for `name`, filled as in the module docs.
fn layout(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("rwire-asset-dir-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let media = base.join("media");
    std::fs::create_dir_all(media.join("sub")).unwrap();
    std::fs::write(base.join("secret.txt"), SECRET).unwrap();
    std::fs::write(media.join("cover.png"), COVER).unwrap();
    std::fs::write(media.join(".hidden"), SECRET).unwrap();
    std::fs::write(media.join("sub/inner.txt"), "inner text").unwrap();
    std::os::unix::fs::symlink("cover.png", media.join("inside")).unwrap();
    std::os::unix::fs::symlink("../secret.txt", media.join("outside")).unwrap();
    std::os::unix::fs::symlink("..", media.join("escape")).unwrap();
    base
}

enum Gate {
    Open,
    /// `auth_with`: only the `sid=valid` cookie gets in.
    Delegated,
    Form,
}

async fn start(port: u16, media: &Path, public: bool, gate: Gate) {
    let mut server = Server::bind(&format!("127.0.0.1:{port}"))
        .unwrap()
        .root(page);
    server = if public {
        server.public_asset_dir("/media", media)
    } else {
        server.asset_dir("/media/", media)
    };
    match gate {
        Gate::Open => {}
        Gate::Form => server = server.auth("admin", "secret"),
        Gate::Delegated => {
            server = server.auth_with(DelegatedAuth::new("sid", |req: AuthRequest| {
                let valid = req.cookie("sid") == Some("valid");
                async move { valid.then(|| Identity::new("reader")) }
            }))
        }
    }
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

struct Response {
    head: String,
    body: Vec<u8>,
}

impl Response {
    fn status(&self) -> &str {
        self.head.lines().next().unwrap_or("")
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.head.lines().skip(1).find_map(|line| {
            let (n, v) = line.split_once(':')?;
            n.trim().eq_ignore_ascii_case(name).then(|| v.trim())
        })
    }

    fn leaks(&self) -> bool {
        self.body
            .windows(SECRET.len())
            .any(|w| w == SECRET.as_bytes())
    }
}

/// `method target` with extra header lines (each ending in `\r\n`), sent raw:
/// the target reaches the server exactly as written.
async fn request(port: u16, method: &str, target: &str, headers: &str) -> Response {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    stream
        .write_all(
            format!("{method} {target} HTTP/1.1\r\nHost: localhost\r\n{headers}\r\n").as_bytes(),
        )
        .await
        .unwrap();
    let mut raw = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut raw))
        .await
        .expect("no response")
        .unwrap();
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("no head");
    Response {
        head: String::from_utf8_lossy(&raw[..split]).into_owned(),
        body: raw[split + 4..].to_vec(),
    }
}

async fn get(port: u16, target: &str) -> Response {
    request(port, "GET", target, "").await
}

/// Every way out of the folder, as sent on the request line.
const TRAVERSALS: &[&str] = &[
    "/media/../secret.txt",
    "/media/%2e%2e/secret.txt",
    "/media/%2E%2E/secret.txt",
    "/media/.%2e/secret.txt",
    "/media/%252e%252e/secret.txt",
    "/media/..%2fsecret.txt",
    "/media/..%5csecret.txt",
    "/media/sub/../../secret.txt",
    "/media//secret.txt",
    "/media/outside",
    "/media/escape/secret.txt",
    "/media/.hidden",
    "/media/%2ehidden",
    "/media/cover.png%00.txt",
];

#[tokio::test]
async fn serves_files_added_after_start() {
    let base = layout("added");
    let media = base.join("media");
    start(19111, &media, false, Gate::Open).await;

    let r = get(19111, "/media/cover.png").await;
    assert_eq!(r.status(), "HTTP/1.1 200 OK", "{}", r.head);
    assert_eq!(r.body, COVER);
    assert_eq!(r.header("content-type"), Some("image/png"));
    assert_eq!(r.header("x-content-type-options"), Some("nosniff"));
    assert!(r
        .header("content-security-policy")
        .unwrap()
        .contains("sandbox"));
    assert_eq!(r.header("cache-control"), Some("private, no-cache"));

    // Added while the server runs, in a new sub-folder, name percent-encoded.
    let r = get(19111, "/media/2026/new%20photo.webp").await;
    assert_eq!(r.status(), "HTTP/1.1 404 Not Found");
    std::fs::create_dir_all(media.join("2026")).unwrap();
    std::fs::write(media.join("2026/new photo.webp"), b"webp bytes").unwrap();
    let r = get(19111, "/media/2026/new%20photo.webp?v=2").await;
    assert_eq!(r.status(), "HTTP/1.1 200 OK", "{}", r.head);
    assert_eq!(r.body, b"webp bytes");
    assert_eq!(r.header("content-type"), Some("image/webp"));

    // A link that stays inside the folder is fine; so is a nested file.
    assert_eq!(get(19111, "/media/inside").await.body, COVER);
    assert_eq!(get(19111, "/media/sub/inner.txt").await.body, b"inner text");

    // Outside the allowlist: served as bytes, never as a page or a script.
    std::fs::write(media.join("page.html"), "<script>alert(1)</script>").unwrap();
    let r = get(19111, "/media/page.html").await;
    assert_eq!(r.header("content-type"), Some("application/octet-stream"));

    // Missing file, folder, folder root: 404, no listing.
    for target in [
        "/media/missing.png",
        "/media/sub",
        "/media/sub/",
        "/media/",
        "/media",
    ] {
        let r = get(19111, target).await;
        assert_eq!(r.status(), "HTTP/1.1 404 Not Found", "{target}");
        assert!(
            !String::from_utf8_lossy(&r.body).contains("inner.txt"),
            "{target}"
        );
    }
    // The prefix is a whole segment.
    let r = get(19111, "/mediax/cover.png").await;
    assert_ne!(r.body, COVER);
}

#[tokio::test]
async fn refuses_every_traversal() {
    let base = layout("traversal");
    start(19112, &base.join("media"), false, Gate::Open).await;
    for target in TRAVERSALS {
        let r = get(19112, target).await;
        assert!(!r.leaks(), "{target} leaked the secret:\n{}", r.head);
        assert_eq!(r.status(), "HTTP/1.1 404 Not Found", "{target}");
    }
}

/// Counter-check: the secret is readable when it *is* in the served folder,
/// so the 404s above come from the refusals, not from a broken setup.
#[tokio::test]
async fn the_traversal_targets_exist() {
    let base = layout("counter");
    start(19113, &base, false, Gate::Open).await;
    let r = get(19113, "/media/secret.txt").await;
    assert_eq!(r.status(), "HTTP/1.1 200 OK");
    assert!(r.leaks());
    // Through the link, now inside the folder.
    assert!(get(19113, "/media/media/outside").await.leaks());
}

#[tokio::test]
async fn revalidates_with_304() {
    let base = layout("revalidate");
    let media = base.join("media");
    start(19114, &media, false, Gate::Open).await;

    let r = get(19114, "/media/cover.png").await;
    let etag = r.header("etag").unwrap().to_string();
    let modified = r.header("last-modified").unwrap().to_string();
    assert!(modified.ends_with(" GMT"), "{modified}");

    let r = request(
        19114,
        "GET",
        "/media/cover.png",
        &format!("If-None-Match: {etag}\r\n"),
    )
    .await;
    assert_eq!(r.status(), "HTTP/1.1 304 Not Modified");
    assert!(r.body.is_empty());
    let r = request(
        19114,
        "GET",
        "/media/cover.png",
        &format!("If-Modified-Since: {modified}\r\n"),
    )
    .await;
    assert_eq!(r.status(), "HTTP/1.1 304 Not Modified");

    // Rewritten on disk: a new validator, the new bytes.
    std::fs::write(media.join("cover.png"), b"a new, longer cover").unwrap();
    let r = request(
        19114,
        "GET",
        "/media/cover.png",
        &format!("If-None-Match: {etag}\r\n"),
    )
    .await;
    assert_eq!(r.status(), "HTTP/1.1 200 OK");
    assert_eq!(r.body, b"a new, longer cover");
    assert_ne!(r.header("etag"), Some(etag.as_str()));
}

#[tokio::test]
async fn ranges_head_and_large_files() {
    let base = layout("range");
    let media = base.join("media");
    // 3 MB, streamed.
    let video: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
    std::fs::write(media.join("clip.mp4"), &video).unwrap();
    start(19115, &media, false, Gate::Open).await;

    let r = get(19115, "/media/clip.mp4").await;
    assert_eq!(r.header("content-length"), Some("3000000"));
    assert_eq!(r.header("accept-ranges"), Some("bytes"));
    assert!(r.body == video, "the large file arrived altered");

    let r = request(
        19115,
        "GET",
        "/media/clip.mp4",
        "Range: bytes=1000-1999\r\n",
    )
    .await;
    assert_eq!(r.status(), "HTTP/1.1 206 Partial Content");
    assert_eq!(r.header("content-range"), Some("bytes 1000-1999/3000000"));
    assert_eq!(r.body, &video[1000..2000]);

    let r = request(19115, "GET", "/media/clip.mp4", "Range: bytes=-10\r\n").await;
    assert_eq!(r.body, &video[video.len() - 10..]);

    let r = request(19115, "GET", "/media/clip.mp4", "Range: bytes=5000000-\r\n").await;
    assert_eq!(r.status(), "HTTP/1.1 416 Range Not Satisfiable");
    assert_eq!(r.header("content-range"), Some("bytes */3000000"));

    // An If-Range for another version gets the whole file.
    let r = request(
        19115,
        "GET",
        "/media/clip.mp4",
        "Range: bytes=0-9\r\nIf-Range: W/\"old\"\r\n",
    )
    .await;
    assert_eq!(r.status(), "HTTP/1.1 200 OK");

    let r = request(19115, "HEAD", "/media/clip.mp4", "").await;
    assert_eq!(r.status(), "HTTP/1.1 200 OK");
    assert_eq!(r.header("content-length"), Some("3000000"));
    assert!(r.body.is_empty());
}

#[tokio::test]
async fn a_folder_stays_behind_the_auth_gate() {
    let base = layout("gated");
    start(19116, &base.join("media"), false, Gate::Delegated).await;

    let r = get(19116, "/media/cover.png").await;
    assert_ne!(r.body, COVER, "served without a session:\n{}", r.head);
    let r = request(19116, "GET", "/media/cover.png", "Cookie: sid=forged\r\n").await;
    assert_ne!(r.body, COVER);
    let r = request(19116, "HEAD", "/media/cover.png", "").await;
    assert!(!r.status().contains("200"), "{}", r.head);

    let r = request(19116, "GET", "/media/cover.png", "Cookie: sid=valid\r\n").await;
    assert_eq!(r.status(), "HTTP/1.1 200 OK");
    assert_eq!(r.body, COVER);
    // Traversals stay refused for an authenticated visitor.
    for target in TRAVERSALS {
        let r = request(19116, "GET", target, "Cookie: sid=valid\r\n").await;
        assert!(!r.leaks(), "{target} leaked the secret");
    }
}

#[tokio::test]
async fn a_folder_stays_behind_the_login_form() {
    let base = layout("form");
    start(19117, &base.join("media"), false, Gate::Form).await;
    let r = get(19117, "/media/cover.png").await;
    assert_ne!(r.body, COVER, "served without a session:\n{}", r.head);
}

#[tokio::test]
async fn a_public_folder_is_served_before_the_gate() {
    let base = layout("public");
    start(19118, &base.join("media"), true, Gate::Delegated).await;
    let r = get(19118, "/media/cover.png").await;
    assert_eq!(r.status(), "HTTP/1.1 200 OK");
    assert_eq!(r.body, COVER);
    assert_eq!(r.header("cache-control"), Some("public, no-cache"));
    // The page itself is still gated.
    let r = get(19118, "/").await;
    assert!(!String::from_utf8_lossy(&r.body).contains("the app page"));
    for target in TRAVERSALS {
        assert!(
            !get(19118, target).await.leaks(),
            "{target} leaked the secret"
        );
    }
}
