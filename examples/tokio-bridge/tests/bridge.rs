//! End to end: rwire started from a tokio context, a snapshot pushed by a tokio task
//! reaches a WebSocket client, a click runs work on the tokio runtime and its result
//! comes back into the shared state, and shutdown stops the server.
//!
//! One test only: the bridge is process-wide (one `start` per process).

use std::net::TcpStream;
use std::time::{Duration, Instant};

use rwire::Ev;
use tokio::runtime::Handle;
use tokio::sync::oneshot;
use tokio_bridge::{ask_tokio, start, Snapshot, ANSWER_PREFIX};
use tungstenite::{Message, WebSocket};

const ADDR: &str = "127.0.0.1:19051";

/// A blocking WebSocket client: it runs on a `spawn_blocking` thread, standing in
/// for a browser.
struct Client {
    ws: WebSocket<TcpStream>,
}

impl Client {
    fn connect() -> Self {
        let deadline = Instant::now() + Duration::from_secs(5);
        let stream = loop {
            match TcpStream::connect(ADDR) {
                Ok(stream) => break stream,
                Err(_) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(e) => panic!("the rwire server never started: {e}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .unwrap();
        let (ws, _) = tungstenite::client(format!("ws://{ADDR}/"), stream).expect("handshake");
        Self { ws }
    }

    /// Read binary frames until one contains `needle` (text travels as UTF-8 in the
    /// symbol definitions). Panics after 5 s.
    fn wait_for(&mut self, needle: &str) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            match self.ws.read() {
                Ok(Message::Binary(data)) => {
                    if data
                        .windows(needle.len())
                        .any(|window| window == needle.as_bytes())
                    {
                        return;
                    }
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(e))
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(e) => panic!("websocket error while waiting for {needle:?}: {e}"),
            }
        }
        panic!("{needle:?} never reached the client");
    }

    /// Send a click on a handler with no params and no payload.
    fn click(&mut self, handler_id: u32) {
        let mut frame = vec![0x00];
        rwire::protocol::varint::write_varint(&mut frame, handler_id);
        frame.extend_from_slice(&[Ev::Click.as_u8(), 0, 0]);
        self.ws.send(Message::Binary(frame)).unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tokio_hosts_rwire_both_ways() {
    let (rwire, snapshots) = start(ADDR, Handle::current()).expect("start");

    let (connected_tx, connected_rx) = oneshot::channel();
    let (pushed_tx, pushed_rx) = oneshot::channel::<()>();
    let browser = tokio::task::spawn_blocking(move || {
        let mut client = Client::connect();
        // Initial DOM: the shared Hub at its default.
        client.wait_for("snapshot #0");
        connected_tx.send(()).unwrap();

        // tokio → rwire: the snapshot pushed by the tokio task below.
        pushed_rx.blocking_recv().unwrap();
        client.wait_for("snapshot #7: 3 agents online");

        // rwire → tokio → rwire: the handler hands the work to tokio, whose task
        // (running on a tokio worker thread) writes the answer into the shared state.
        client.click(ask_tokio().handler_id());
        // The answer can only exist if the task ran: `tokio::time::sleep` in it
        // panics outside a tokio runtime.
        client.wait_for(&format!("tokio answers: 1 {ANSWER_PREFIX}"));
    });

    connected_rx.await.expect("the client did not connect");
    // A tokio task pushes into rwire.
    tokio::spawn(async move {
        snapshots
            .send(Snapshot {
                seq: 7,
                summary: "3 agents online".to_string(),
            })
            .await
            .unwrap();
        pushed_tx.send(()).unwrap();
        // `snapshots` dropped here: the rwire-side applier ends.
    })
    .await
    .unwrap();

    browser.await.expect("browser scenario failed");

    // Shutdown: the rwire thread ends and the port stops accepting.
    tokio::task::spawn_blocking(move || rwire.shutdown())
        .await
        .unwrap()
        .expect("clean shutdown");
    assert!(
        TcpStream::connect(ADDR).is_err(),
        "the listener should be closed after shutdown"
    );
}
