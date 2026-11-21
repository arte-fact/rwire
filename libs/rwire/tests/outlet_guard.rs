//! The outlet check renders regions at their default state, so an `outlet()`
//! that only appears in another state is invisible to it: `outlet_guard(false)`
//! lets such an app start. (Own test binary: the installed router is global.)

use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use rwire::{
    el, outlet, renderer, ChangeSet, El, ElementBuilder, Router, Server, SharedServerState, State,
    ROUTER_WITHOUT_OUTLET,
};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;

/// Closed until the app opens it (a login, a setting): the outlet is only in
/// the open state.
#[derive(State, Default)]
#[storage(shared)]
struct Gate {
    open: bool,
}

#[renderer]
fn gated(gate: &Gate) -> ElementBuilder {
    if gate.open {
        el(El::Main).append([outlet()])
    } else {
        el(El::P).text("locked")
    }
}

fn shell() -> ElementBuilder {
    el(El::Div).append([gated()])
}

fn router() -> Router {
    Router::new()
        .page("/", |_| el(El::P).text("home view"))
        .page("/inbox", |_| el(El::P).text("inbox view"))
}

#[tokio::test]
async fn a_conditional_outlet_is_refused_by_default() {
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        Server::bind("127.0.0.1:19211")
            .unwrap()
            .root(shell)
            .routes(router())
            .run(),
    )
    .await
    .expect("run() kept serving instead of refusing");
    assert_eq!(result.unwrap_err().to_string(), ROUTER_WITHOUT_OUTLET);
    assert!(ROUTER_WITHOUT_OUTLET.contains("outlet_guard(false)"));
    assert!(!ROUTER_WITHOUT_OUTLET.contains(".claude/"));
}

async fn start_unguarded(port: u16) -> Arc<SharedServerState> {
    let mut server = Server::bind(&format!("127.0.0.1:{port}"))
        .unwrap()
        .root(shell)
        .routes(router())
        .outlet_guard(false);
    let shared = server.shared_state();
    tokio::spawn(async move {
        let _ = server.run().await;
    });
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return shared;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the server on {port} never started");
}

type Ws = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>;

async fn expect(ws: &mut Ws, needle: &str) {
    let found = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(Ok(message)) = ws.next().await {
            if let Message::Binary(data) = message {
                if data.windows(needle.len()).any(|w| w == needle.as_bytes()) {
                    return true;
                }
            }
        }
        false
    })
    .await;
    assert_eq!(found, Ok(true), "{needle:?} never arrived");
}

/// With the check off, the app starts; once the outlet renders, it shows the
/// route, and navigation keeps driving it.
#[tokio::test]
async fn an_unguarded_conditional_outlet_routes_once_it_appears() {
    let shared = start_unguarded(19212).await;
    let (mut ws, _) = tokio_tungstenite::connect_async("ws://127.0.0.1:19212/")
        .await
        .unwrap();
    ws.send(Message::Text("R/inbox".into())).await.unwrap();
    expect(&mut ws, "locked").await;

    shared.update_shared_changed::<Gate>(ChangeSet::all(), |gate| gate.open = true);
    expect(&mut ws, "inbox view").await;

    ws.send(Message::Text("R/".into())).await.unwrap();
    expect(&mut ws, "home view").await;
}
