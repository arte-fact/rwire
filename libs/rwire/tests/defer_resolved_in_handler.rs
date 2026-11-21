//! A deferred action resolved before its handler returns — inline, or by work
//! the handler waited for — settles at once instead of spinning until the
//! timeout backstop (30 s). Resolving after the handler returns still shows
//! the loading phase first.

use std::time::Duration;

use futures::{SinkExt, StreamExt};
use rwire::{el, handler, reflect_action, El, ElementBuilder, Ev, Server, State};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;

const ADDR: &str = "127.0.0.1:19221";

#[derive(State, Default)]
#[storage(memory)]
struct Page {
    pings: u32,
}

#[handler]
fn ok_inline(_page: &mut Page, ctx: &EventContext) {
    ctx.defer().ok();
}

#[handler]
fn err_inline(_page: &mut Page, ctx: &EventContext) {
    ctx.defer().err("refused inline");
}

/// Work the handler hands off but waits for: it resolves before the handler
/// returns, from another thread.
#[handler]
fn err_joined(_page: &mut Page, ctx: &EventContext) {
    let resolver = ctx.defer();
    std::thread::spawn(move || resolver.err("refused by the worker"))
        .join()
        .unwrap();
}

/// The usual case: resolved after the handler returned.
#[handler]
fn err_later(_page: &mut Page, ctx: &EventContext) {
    let resolver = ctx.defer();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        resolver.err("refused later");
    });
}

#[handler]
fn ping(page: &mut Page) {
    page.pings += 1;
}

#[rwire::renderer]
fn pings(page: &Page) -> ElementBuilder {
    el(El::P).text(&format!("pings: {}", page.pings))
}

fn reflected(name: &'static str, handler_id: u32) -> ElementBuilder {
    reflect_action(
        handler_id,
        Vec::new(),
        move || el(El::Span).text(&format!("{name}-idle")),
        move || el(El::Span).text(&format!("{name}-loading")),
        move |m| el(El::Span).text(&format!("{name}-error: {m}")),
    )
}

fn app() -> ElementBuilder {
    el(El::Main).append([
        pings(),
        el(El::Button).text("ping").on(Ev::Click, ping()),
        el(El::Button).text("a").on(Ev::Click, ok_inline()),
        el(El::Button).text("b").on(Ev::Click, err_inline()),
        el(El::Button).text("c").on(Ev::Click, err_joined()),
        el(El::Button).text("d").on(Ev::Click, err_later()),
        reflected("ok-inline", ok_inline().handler_id()),
        reflected("err-inline", err_inline().handler_id()),
        reflected("err-joined", err_joined().handler_id()),
        reflected("err-later", err_later().handler_id()),
    ])
}

type Ws = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>;

async fn connect() -> Ws {
    for _ in 0..100 {
        if let Ok((ws, _)) = tokio_tungstenite::connect_async(format!("ws://{ADDR}/")).await {
            return ws;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the server never started");
}

/// Every frame until `needle` shows up (5 s at most, far below the 30 s
/// backstop), as one lossy string.
async fn until(ws: &mut Ws, needle: &str) -> String {
    let mut seen = String::new();
    let found = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(Ok(message)) = ws.next().await {
            if let Message::Binary(data) = message {
                seen.push_str(&String::from_utf8_lossy(&data));
                if seen.contains(needle) {
                    return true;
                }
            }
        }
        false
    })
    .await;
    assert_eq!(found, Ok(true), "{needle:?} never reached the client");
    seen
}

fn click(handler_id: u32) -> Message {
    let mut frame = vec![0x00];
    rwire::protocol::varint::write_varint(&mut frame, handler_id);
    frame.extend_from_slice(&[Ev::Click.as_u8(), 0, 0]);
    Message::Binary(frame)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_defer_resolved_in_the_handler_settles_at_once() {
    tokio::spawn(async {
        let _ = Server::bind(ADDR).unwrap().root(app).run().await;
    });
    let mut ws = connect().await;
    until(&mut ws, "pings: 0").await;

    // Errors resolved before the handler returned reach the client.
    ws.send(click(err_inline().handler_id())).await.unwrap();
    let seen = until(&mut ws, "err-inline-error: refused inline").await;
    assert!(!seen.contains("err-inline-loading"), "{seen}");
    ws.send(click(err_joined().handler_id())).await.unwrap();
    let seen = until(&mut ws, "err-joined-error: refused by the worker").await;
    assert!(!seen.contains("err-joined-loading"), "{seen}");

    // A success resolved in the handler never spins: by the time a later
    // event's update arrives (and a little after), no loading was shown.
    ws.send(click(ok_inline().handler_id())).await.unwrap();
    ws.send(click(ping().handler_id())).await.unwrap();
    let mut seen = until(&mut ws, "pings: 1").await;
    let _ = tokio::time::timeout(Duration::from_millis(300), async {
        while let Some(Ok(Message::Binary(data))) = ws.next().await {
            seen.push_str(&String::from_utf8_lossy(&data));
        }
    })
    .await;
    assert!(!seen.contains("ok-inline-loading"), "{seen}");

    // Resolved after the handler returned: loading first, then the error.
    ws.send(click(err_later().handler_id())).await.unwrap();
    until(&mut ws, "err-later-loading").await;
    until(&mut ws, "err-later-error: refused later").await;
}
