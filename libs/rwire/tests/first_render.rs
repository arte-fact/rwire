//! The first render of a routed view must paint the current shared state.
//!
//! A `#[storage(shared)]` state filled before anyone connects (by a background
//! task, through `update_shared_if`) is read by a region inside the router's
//! `outlet()`, on a route the shell itself never renders. The first render of
//! that view must show the data, not the state's empty default:
//!
//! - opening the route directly (the client reports its path on open);
//! - navigating to it from another route;
//! - with the shared region nested inside a memory-state region.

use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use rwire::{
    el, outlet, renderer, ChangeSet, El, ElementBuilder, Router, Server, SharedServerState, State,
};
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

fn router() -> Router {
    Router::new()
        .page("/", |_| el(El::P).text("home page"))
        .page("/about", |_| el(El::P).text("about page"))
        .page("/posts/:slug", |_| el(El::Section).append([post()]))
        .page("/framed", |_| el(El::Section).append([frame()]))
}

/// The shell never reads `Blog` itself: only routed views do.
fn shell() -> ElementBuilder {
    el(El::Div).append([
        el(El::Header).text("the shell"),
        el(El::Main).append([outlet()]),
    ])
}

/// Start a server on `port` whose `Blog` is published before it runs, and
/// return its shared state once it accepts connections.
async fn start(port: u16) -> Arc<SharedServerState> {
    let mut server = Server::bind(&format!("127.0.0.1:{port}"))
        .unwrap()
        .root(shell)
        .routes(router());
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

/// Read frames until one contains `needle`. Fails on `forbidden` (a frame
/// that painted the view from the empty default) or after 5 s.
async fn expect_frame(ws: &mut Ws, needle: &str, forbidden: Option<&str>) {
    let found = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(Ok(message)) = ws.next().await {
            if let Message::Binary(data) = message {
                if let Some(bad) = forbidden {
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
    let shared = start(19091).await;
    let mut ws = connect(19091).await;
    expect_frame(&mut ws, "home page", None).await;
    ws.send(route("/posts/hello")).await.unwrap();
    expect_frame(&mut ws, POST, Some(EMPTY_POST)).await;

    // And the view is subscribed: a later publish reaches it.
    shared.update_shared_changed::<Blog>(ChangeSet::all(), |blog| {
        blog.title = "Second post".to_string();
        blog.revision = 2;
    });
    expect_frame(&mut ws, "post: Second post (rev 2)", None).await;
}

#[tokio::test]
async fn navigating_to_a_route_paints_the_shared_state() {
    let _shared = start(19092).await;
    let mut ws = connect(19092).await;
    expect_frame(&mut ws, "home page", None).await;
    ws.send(route("/about")).await.unwrap();
    expect_frame(&mut ws, "about page", None).await;
    ws.send(route("/posts/hello")).await.unwrap();
    expect_frame(&mut ws, POST, Some(EMPTY_POST)).await;
}

#[tokio::test]
async fn a_shared_region_nested_in_a_routed_region_paints_the_shared_state() {
    let shared = start(19093).await;
    let mut ws = connect(19093).await;
    expect_frame(&mut ws, "home page", None).await;
    ws.send(route("/framed")).await.unwrap();
    expect_frame(&mut ws, POST, Some(EMPTY_POST)).await;

    shared.update_shared_changed::<Blog>(ChangeSet::all(), |blog| {
        blog.revision = 3;
    });
    expect_frame(&mut ws, "post: Hello rwire (rev 3)", None).await;
}
