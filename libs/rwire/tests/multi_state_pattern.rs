//! A view that combines per-connection state with shared state, the way the
//! mcp-agents trial needed it: the selected session is in each tab's route,
//! the fleet is shared and published by a background task.
//!
//! Pattern (no global lock, no `notify_all`): a renderer over the
//! per-connection state (`CurrentRoute`) nests a *closure region* over the
//! shared state (`Fleet`), capturing what it needs from the outer state.
//! - a fleet publish re-renders the nested region of every connection, each
//!   with its own captured selection;
//! - a route change re-renders the outer region, which hands the nested one a
//!   fresh closure (new selection) and re-renders it.
//!
//! See docs/multi-state-renderers.md.

use std::time::Duration;

use futures::{SinkExt, StreamExt};
use rwire::{
    el, outlet, renderer, ChangeSet, CurrentRoute, El, ElementBuilder, RendererDeps, Router,
    Server, SharedServerState, State,
};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;

/// What the hub publishes: one status line per session.
#[derive(State, Default)]
#[storage(shared)]
struct Fleet {
    sessions: Vec<(String, String)>,
}

/// The session panel: the route says which session (per tab), the fleet says
/// how it is doing (shared).
#[renderer]
fn session_panel(route: &CurrentRoute) -> ElementBuilder {
    let selected = route.param("id").unwrap_or_default();
    el(El::Section).append([ElementBuilder::synced_with_storage::<Fleet, _>(
        move |fleet: &Fleet| {
            let line = fleet
                .sessions
                .iter()
                .find(|(id, _)| *id == selected)
                .map(|(id, status)| format!("panel {id}: {status}"))
                .unwrap_or_else(|| format!("panel {selected}: unknown"));
            el(El::P).text(&line)
        },
        RendererDeps::always(),
    )])
}

fn shell() -> ElementBuilder {
    el(El::Main).append([outlet()])
}

async fn start(port: u16) -> Arc<SharedServerState> {
    let mut server = Server::bind(&format!("127.0.0.1:{port}"))
        .unwrap()
        .root(shell)
        .routes(
            Router::new()
                .page("/", |_| el(El::P).text("home"))
                .page("/s/:id", |_| session_panel()),
        );
    let shared = server.shared_state();
    shared.update_shared_if::<Fleet>(|fleet| {
        fleet.sessions = vec![("a".into(), "idle".into()), ("b".into(), "idle".into())];
        Some(ChangeSet::all())
    });
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

/// Open a tab on `path` (the runtime reports it on open).
async fn tab(port: u16, path: &str) -> Ws {
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}/"))
        .await
        .unwrap();
    ws.send(Message::Text(format!("R{path}"))).await.unwrap();
    ws
}

/// Read frames until one contains `needle`; fail on `forbidden` or after 5 s.
async fn expect(ws: &mut Ws, needle: &str, forbidden: &[&str]) {
    let found = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(Ok(message)) = ws.next().await {
            if let Message::Binary(data) = message {
                for bad in forbidden {
                    assert!(
                        !data.windows(bad.len()).any(|w| w == bad.as_bytes()),
                        "a frame painted {bad:?} while waiting for {needle:?}"
                    );
                }
                if data.windows(needle.len()).any(|w| w == needle.as_bytes()) {
                    return true;
                }
            }
        }
        false
    })
    .await;
    assert_eq!(found, Ok(true), "{needle:?} never reached the tab");
}

#[tokio::test]
async fn each_tab_combines_its_route_with_the_shared_fleet() {
    let shared = start(19201).await;
    let mut tab_a = tab(19201, "/s/a").await;
    let mut tab_b = tab(19201, "/s/b").await;
    expect(&mut tab_a, "panel a: idle", &[]).await;
    expect(&mut tab_b, "panel b: idle", &[]).await;

    // A background publish: no global lock, no notify_all on CurrentRoute.
    shared.update_shared_changed::<Fleet>(ChangeSet::all(), |fleet| {
        fleet.sessions[0].1 = "running".into();
        fleet.sessions[1].1 = "waiting".into();
    });
    expect(&mut tab_a, "panel a: running", &["panel b"]).await;
    expect(&mut tab_b, "panel b: waiting", &["panel a"]).await;

    // A route change hands the nested region the new selection.
    tab_a.send(Message::Text("R/s/b".into())).await.unwrap();
    expect(&mut tab_a, "panel b: waiting", &[]).await;

    // And the next publish follows it.
    shared.update_shared_changed::<Fleet>(ChangeSet::all(), |fleet| {
        fleet.sessions[1].1 = "done".into();
    });
    expect(&mut tab_a, "panel b: done", &["panel a"]).await;
    expect(&mut tab_b, "panel b: done", &[]).await;
}
