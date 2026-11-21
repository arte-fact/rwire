//! The first render of a routed view must paint the current shared state.
//!
//! A `#[storage(shared)]` state filled before anyone connects (by a background
//! task, through `update_shared_if`) is read by a region inside the router's
//! `outlet()`, on a route the shell itself never renders. The first render of
//! that view, live over the WebSocket or static with `ssr(true)`, must show the
//! data, not the state's empty default:
//!
//! - opening the route directly (the client reports its path on open): the
//!   first frame is the route's view, never `/`'s;
//! - navigating to it from another route;
//! - with the shared region nested inside a memory-state region;
//! - `GET /posts/x` with `ssr(true)` serves the requested route, not `/`
//!   (also in the absolute form a proxy sends), stays behind the auth gate,
//!   is off behind it unless allowed, and skips the excluded routes;
//! - a path no route matches is a `404`.

use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use rwire::{
    el, outlet, renderer, AuthRequest, CapsuleConfig, ChangeSet, DelegatedAuth, El, ElementBuilder,
    Identity, Router, Server, SharedServerState, State,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;

/// Content published by the application, the same for every visitor.
#[derive(State, Default)]
#[storage(shared)]
struct Blog {
    title: String,
    revision: u32,
}

/// Per-connection UI state wrapping a shared region (nested regions).
#[derive(State, Default)]
#[storage(memory)]
struct Frame {
    expanded: bool,
}

#[renderer]
fn post(blog: &Blog) -> ElementBuilder {
    el(El::H1).text(&format!("post: {} (rev {})", blog.title, blog.revision))
}

#[renderer]
fn frame(state: &Frame) -> ElementBuilder {
    el(El::Article)
        .text(if state.expanded { "expanded" } else { "folded" })
        .append([post()])
}

/// Per-connection state read only by a region nested in a shared region.
#[derive(State, Default)]
#[storage(memory)]
struct Badge {
    seen: bool,
}

#[renderer]
fn badge(state: &Badge) -> ElementBuilder {
    el(El::Span).text(if state.seen {
        "badge: seen"
    } else {
        "badge: unread"
    })
}

/// Shared state nobody has published yet: it renders its default.
#[derive(State, Default)]
#[storage(shared)]
struct Drafts {
    count: u32,
}

#[renderer]
fn drafts(state: &Drafts) -> ElementBuilder {
    el(El::P).text(&format!("drafts: {}", state.count))
}

#[renderer]
fn badged_post(blog: &Blog) -> ElementBuilder {
    el(El::Div).append([el(El::H2).text(&blog.title), badge(), drafts()])
}

fn router() -> Router {
    Router::new()
        .page("/", |_| el(El::P).text("home page"))
        .page("/about", |_| el(El::P).text("about page"))
        .page("/posts/:slug", |_| el(El::Section).append([post()]))
        .page("/framed", |_| el(El::Section).append([frame()]))
        .page("/badged", |_| el(El::Section).append([badged_post()]))
}

/// The shell never reads `Blog` itself: only routed views do.
fn shell() -> ElementBuilder {
    el(El::Div).append([
        el(El::Header).text("the shell"),
        el(El::Main).append([outlet()]),
    ])
}

const USER: &str = "admin";
const PASSWORD: &str = "secret";
const COOKIE: &str = "hub_session";

/// Which auth gate, if any, guards the page.
enum Gate {
    Open,
    /// `.auth(user, password)`: the form login.
    Form,
    /// `.auth_with(..)`: only the `hub_session=valid` cookie is accepted.
    Delegated,
}

/// Start a server on `port` whose `Blog` is published before it runs, and
/// return its shared state once it accepts connections.
async fn start(port: u16, capsule: Option<CapsuleConfig>, gate: Gate) -> Arc<SharedServerState> {
    let mut server = Server::bind(&format!("127.0.0.1:{port}"))
        .unwrap()
        .root(shell)
        .routes(router());
    if let Some(config) = capsule {
        server = server.capsule_config(config);
    }
    match gate {
        Gate::Open => {}
        Gate::Form => server = server.auth(USER, PASSWORD),
        Gate::Delegated => {
            server = server.auth_with(DelegatedAuth::new(COOKIE, |req: AuthRequest| {
                let valid = req.cookie(COOKIE) == Some("valid");
                async move { valid.then(|| Identity::new("reader")) }
            }))
        }
    }
    let shared = server.shared_state();
    shared.update_shared_if::<Blog>(|blog| {
        blog.title = "Hello rwire".to_string();
        blog.revision = 1;
        Some(ChangeSet::all())
    });
    tokio::spawn(async move {
        let _ = server.run().await;
    });
    for _ in 0..100 {
        if TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .is_ok()
        {
            return shared;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the server on {port} never started");
}

type Ws = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>;

async fn connect(port: u16) -> Ws {
    let (ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}/"))
        .await
        .unwrap();
    ws
}

/// Read frames until one contains `needle`. Fails on any of `forbidden` (a
/// frame that painted the empty default, or the wrong view) or after 5 s.
async fn expect_frame(ws: &mut Ws, needle: &str, forbidden: &[&str]) {
    let found = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(Ok(message)) = ws.next().await {
            if let Message::Binary(data) = message {
                for bad in forbidden {
                    assert!(
                        !contains(&data, bad),
                        "a frame painted {bad:?} before {needle:?}"
                    );
                }
                if contains(&data, needle) {
                    return true;
                }
            }
        }
        false
    })
    .await;
    assert_eq!(found, Ok(true), "{needle:?} never reached the client");
}

fn contains(data: &[u8], needle: &str) -> bool {
    data.windows(needle.len()).any(|w| w == needle.as_bytes())
}

/// What the runtime sends on open and on every navigation.
fn route(path: &str) -> Message {
    Message::Text(format!("R{path}"))
}

const EMPTY_POST: &str = "post:  (rev 0)";
const POST: &str = "post: Hello rwire (rev 1)";

#[tokio::test]
async fn opening_a_route_directly_paints_the_shared_state() {
    let shared = start(19091, None, Gate::Open).await;
    let mut ws = connect(19091).await;
    // As the runtime does: report the path as soon as the socket opens. The
    // initial DOM is the route's view, with no `/` frame before it.
    ws.send(route("/posts/hello")).await.unwrap();
    expect_frame(&mut ws, POST, &[EMPTY_POST, "home page"]).await;

    // And the view is subscribed: a later publish reaches it.
    shared.update_shared_changed::<Blog>(ChangeSet::all(), |blog| {
        blog.title = "Second post".to_string();
        blog.revision = 2;
    });
    expect_frame(&mut ws, "post: Second post (rev 2)", &[]).await;
}

#[tokio::test]
async fn navigating_to_a_route_paints_the_shared_state() {
    let _shared = start(19092, None, Gate::Open).await;
    let mut ws = connect(19092).await;
    expect_frame(&mut ws, "home page", &[]).await;
    ws.send(route("/about")).await.unwrap();
    expect_frame(&mut ws, "about page", &[]).await;
    ws.send(route("/posts/hello")).await.unwrap();
    expect_frame(&mut ws, POST, &[EMPTY_POST]).await;
}

#[tokio::test]
async fn a_shared_region_nested_in_a_routed_region_paints_the_shared_state() {
    let shared = start(19093, None, Gate::Open).await;
    let mut ws = connect(19093).await;
    expect_frame(&mut ws, "home page", &[]).await;
    ws.send(route("/framed")).await.unwrap();
    expect_frame(&mut ws, POST, &[EMPTY_POST]).await;

    shared.update_shared_changed::<Blog>(ChangeSet::all(), |blog| {
        blog.revision = 3;
    });
    expect_frame(&mut ws, "post: Hello rwire (rev 3)", &[]).await;
}

#[tokio::test]
async fn regions_nested_in_a_routed_region_render_states_the_page_never_read() {
    let _shared = start(19097, None, Gate::Open).await;
    let mut ws = connect(19097).await;
    expect_frame(&mut ws, "home page", &[]).await;
    ws.send(route("/badged")).await.unwrap();
    expect_frame(&mut ws, "badge: unread", &[]).await;

    let mut ws = connect(19097).await;
    expect_frame(&mut ws, "home page", &[]).await;
    ws.send(route("/badged")).await.unwrap();
    expect_frame(&mut ws, "drafts: 0", &[]).await;
}

/// `GET path` and the whole response (the server closes after it), minus the
/// inlined runtime script so a failure prints the page, not 40 KB of JS.
async fn get(port: u16, path: &str) -> String {
    get_with(port, path, "").await
}

/// `GET path` with extra header lines (each ending in `\r\n`).
async fn get_with(port: u16, path: &str, headers: &str) -> String {
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
    let page = String::from_utf8_lossy(&response).into_owned();
    match page.find("<script>") {
        Some(script) => page[..script].to_string(),
        None => page,
    }
}

#[tokio::test]
async fn ssr_renders_the_requested_route_with_the_shared_state() {
    let shared = start(19094, Some(ssr()), Gate::Open).await;

    let page = get(19094, "/posts/hello").await;
    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert!(page.contains(POST), "the post is missing:\n{page}");
    assert!(!page.contains("home page"), "served the / view:\n{page}");
    assert!(page.contains("the shell"), "the shell is missing:\n{page}");

    // The query string is not part of the route.
    let page = get(19094, "/framed?ref=feed").await;
    assert!(page.contains(POST), "the nested post is missing:\n{page}");
    assert!(page.contains("folded"), "memory state renders its default");

    // Rendered per request: a later publish shows on the next load.
    shared.update_shared_changed::<Blog>(ChangeSet::all(), |blog| {
        blog.revision = 4;
    });
    let page = get(19094, "/posts/hello").await;
    assert!(page.contains("post: Hello rwire (rev 4)"), "{page}");

    let page = get(19094, "/").await;
    assert!(page.contains("home page"), "{page}");
    assert!(!page.contains("post:"), "{page}");

    // The absolute form a proxy may send names the same route.
    let page = get(19094, "http://localhost/posts/hello?ref=feed").await;
    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert!(page.contains("post: Hello rwire"), "{page}");
    assert!(!page.contains("home page"), "{page}");
}

fn ssr() -> CapsuleConfig {
    CapsuleConfig::new().ssr(true)
}

/// The mount div of a page: the static paint, if any.
fn mount(page: &str) -> &str {
    let start = page.find("<div id=\"rw\">").expect("no mount div");
    let rest = &page[start..];
    &rest[..rest
        .find("</div>")
        .map_or(rest.len(), |end| end + "</div>".len())]
}

#[tokio::test]
async fn ssr_skips_excluded_routes() {
    let _shared = start(19098, Some(ssr().ssr_exclude("/posts/:slug")), Gate::Open).await;
    let page = get(19098, "/posts/hello").await;
    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert_eq!(mount(&page), "<div id=\"rw\"></div>", "{page}");

    // The other routes keep their paint.
    let page = get(19098, "/framed").await;
    assert!(page.contains(POST), "{page}");
}

#[tokio::test]
async fn ssr_is_off_behind_auth_unless_allowed() {
    let _shared = start(19099, Some(ssr()), Gate::Delegated).await;
    let page = get_with(
        19099,
        "/posts/hello",
        &format!("Cookie: {COOKIE}=valid\r\n"),
    )
    .await;
    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert_eq!(mount(&page), "<div id=\"rw\"></div>", "{page}");
}

#[tokio::test]
async fn a_path_no_route_matches_is_a_404() {
    let _shared = start(19100, Some(ssr()), Gate::Open).await;
    let page = get(19100, "/favicon.ico").await;
    assert!(page.starts_with("HTTP/1.1 404 Not Found"), "{page}");
    // The page still loads, with the router's not-found view.
    assert!(page.contains("the shell"), "{page}");

    let page = get(19100, "/about").await;
    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
}

#[tokio::test]
async fn ssr_stays_behind_the_auth_gate() {
    let _shared = start(19095, Some(ssr().ssr_behind_auth(true)), Gate::Form).await;
    let page = get(19095, "/posts/hello").await;
    assert!(
        !page.contains("Hello rwire"),
        "shared state leaked to an unauthenticated visitor:\n{page}"
    );
    assert!(!page.contains("the shell"), "{page}");
}

#[tokio::test]
async fn ssr_stays_behind_delegated_auth() {
    let _shared = start(19096, Some(ssr().ssr_behind_auth(true)), Gate::Delegated).await;
    let page = get(19096, "/posts/hello").await;
    assert!(
        !page.contains("Hello rwire"),
        "shared state leaked to an unauthenticated visitor:\n{page}"
    );
    let page = get_with(
        19096,
        "/posts/hello",
        &format!("Cookie: {COOKIE}=forged\r\n"),
    )
    .await;
    assert!(!page.contains("Hello rwire"), "{page}");

    let page = get_with(
        19096,
        "/posts/hello",
        &format!("Cookie: {COOKIE}=valid\r\n"),
    )
    .await;
    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert!(page.contains(POST), "the post is missing:\n{page}");
}
