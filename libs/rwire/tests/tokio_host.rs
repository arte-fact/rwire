//! A tokio application hosting rwire, now that rwire runs on tokio: no dedicated
//! thread, no `block_on`, no captured `Handle`. The server is just a task; tokio
//! tasks push state into it, handlers `tokio::spawn` work directly, and
//! `run_until` stops it cleanly.
//!
//! (The test that used to need a bridge, from the unmerged `feat/tokio-bridge`.)

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use rwire::{
    el, handler, renderer, ChangeSet, El, ElementBuilder, Ev, Server, SharedServerState, State,
};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::Message;

const ADDR: &str = "127.0.0.1:19071";

/// What the tokio side knows; one instance for every connection.
#[derive(State, Default)]
#[storage(shared)]
struct Hub {
    seq: u64,
    summary: String,
    answers: u32,
    answer: String,
}

/// One upstream snapshot, as an SSE client would decode it.
struct Snapshot {
    seq: u64,
    summary: String,
}

/// Handlers are plain functions: the shared state they write back into is
/// reached through a process-wide slot (one server per test binary).
static SHARED: OnceLock<Arc<SharedServerState>> = OnceLock::new();

/// Handlers run on the tokio runtime, so `tokio::spawn` and `tokio::time` work
/// directly in them.
#[handler]
fn ask_tokio(_hub: &mut Hub, ctx: &EventContext) {
    let Some(shared) = SHARED.get().cloned() else {
        return;
    };
    let resolver = ctx.defer();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        shared.update_shared_changed::<Hub>(
            ChangeSet::from_fields(&[Hub::FIELD_ANSWERS, Hub::FIELD_ANSWER]),
            |hub| {
                hub.answers += 1;
                hub.answer = "answered on tokio".to_string();
            },
        );
        resolver.ok();
    });
}

/// Apply snapshots, broadcasting only those whose content changed.
async fn apply_snapshots(shared: Arc<SharedServerState>, mut rx: mpsc::Receiver<Snapshot>) {
    while let Some(snapshot) = rx.recv().await {
        shared.update_shared_if::<Hub>(|hub| {
            if hub.summary == snapshot.summary {
                return None;
            }
            hub.seq = snapshot.seq;
            hub.summary = snapshot.summary;
            Some(ChangeSet::from_fields(&[
                Hub::FIELD_SEQ,
                Hub::FIELD_SUMMARY,
            ]))
        });
    }
}

#[renderer]
fn hub_view(hub: &Hub) -> ElementBuilder {
    el(El::Section).append([
        el(El::P).text(&format!("snapshot #{}: {}", hub.seq, hub.summary)),
        el(El::P).text(&format!("tokio answers: {} {}", hub.answers, hub.answer)),
    ])
}

fn app() -> ElementBuilder {
    el(El::Main).append([
        hub_view(),
        el(El::Button).text("Ask tokio").on(Ev::Click, ask_tokio()),
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

async fn wait_for(ws: &mut Ws, needle: &str) {
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
    assert_eq!(found, Ok(true), "{needle:?} never reached the client");
}

fn click(handler_id: u32) -> Message {
    let mut frame = vec![0x00];
    rwire::protocol::varint::write_varint(&mut frame, handler_id);
    frame.extend_from_slice(&[Ev::Click.as_u8(), 0, 0]);
    Message::Binary(frame)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tokio_hosts_rwire_both_ways_and_stops_it_cleanly() {
    let mut server = Server::bind(ADDR).unwrap().root(app);
    let shared = server.shared_state();
    assert!(
        SHARED.set(Arc::clone(&shared)).is_ok(),
        "one server per test binary"
    );
    let (stop, stopped) = oneshot::channel::<()>();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _ = stopped.await;
            })
            .await
            .map_err(|e| e.to_string())
    });

    // tokio → rwire: a channel fed by a tokio task, applied by another.
    let (snapshots, rx) = mpsc::channel(8);
    tokio::spawn(apply_snapshots(Arc::clone(&shared), rx));

    let mut ws = connect().await;
    wait_for(&mut ws, "snapshot #0").await;

    tokio::spawn(async move {
        snapshots
            .send(Snapshot {
                seq: 7,
                summary: "3 agents online".to_string(),
            })
            .await
            .unwrap();
    })
    .await
    .unwrap();
    wait_for(&mut ws, "snapshot #7: 3 agents online").await;

    // rwire → tokio → rwire: the handler spawns on tokio, the answer comes back.
    ws.send(click(ask_tokio().handler_id())).await.unwrap();
    wait_for(&mut ws, "tokio answers: 1 answered on tokio").await;

    // Graceful stop: the open socket is closed with 1001, run_until returns Ok,
    // and the port no longer accepts.
    stop.send(()).unwrap();
    let close = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(message) = ws.next().await {
            if let Ok(Message::Close(Some(frame))) = message {
                return Some(frame.code);
            }
        }
        None
    })
    .await
    .expect("the socket was not closed on shutdown");
    assert_eq!(close, Some(CloseCode::Away));
    let result = tokio::time::timeout(Duration::from_secs(5), running)
        .await
        .expect("run_until did not return after shutdown")
        .unwrap();
    assert_eq!(result, Ok(()));
    assert!(
        TcpStream::connect(ADDR).await.is_err(),
        "the listener should be closed after shutdown"
    );
}

/// `apply_snapshots` broadcasts a snapshot only when its content changed.
#[tokio::test]
async fn identical_snapshots_are_not_broadcast() {
    let shared = SharedServerState::new(Duration::from_secs(60));
    let (conn_tx, conn_rx) = async_channel::unbounded();
    shared.register_connection(1, conn_tx);
    shared.subscribe(1, &format!("__shared__:{}", Hub::TABLE_NAME));

    let (tx, rx) = mpsc::channel(8);
    for (seq, summary) in [(1, "1 agent"), (2, "1 agent"), (3, "2 agents")] {
        tx.send(Snapshot {
            seq,
            summary: summary.to_string(),
        })
        .await
        .unwrap();
    }
    drop(tx);
    apply_snapshots(Arc::clone(&shared), rx).await;

    assert_eq!(conn_rx.len(), 2, "the repeated snapshot must not broadcast");
}
