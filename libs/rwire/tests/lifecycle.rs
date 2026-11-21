//! `on_connect` / `on_disconnect` and `EventContext::spawn`, end to end: a real
//! server, a delegated-auth gate whose cookie names the user, and WebSocket
//! clients that connect, leave, and watch each other.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use rwire::{
    el, handler, renderer, AuthRequest, DelegatedAuth, El, ElementBuilder, Identity, Server, State,
};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;

const COOKIE: &str = "user";

/// The cookie is the user name: enough to give each connection an identity.
fn auth() -> DelegatedAuth {
    DelegatedAuth::new(COOKIE, |req: AuthRequest| {
        let user = req.cookie(COOKIE).map(str::to_string);
        async move { user.map(Identity::new) }
    })
}

type Ws = tokio_tungstenite::WebSocketStream<TcpStream>;

async fn connect(port: u16, user: &str) -> Ws {
    let mut request = format!("ws://127.0.0.1:{port}/")
        .into_client_request()
        .unwrap();
    request.headers_mut().insert(
        "Cookie",
        HeaderValue::from_str(&format!("{COOKIE}={user}")).unwrap(),
    );
    let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    tokio_tungstenite::client_async(request, stream)
        .await
        .expect("handshake")
        .0
}

/// Read until a binary frame contains `needle`.
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

async fn until(what: &str, cond: impl Fn() -> bool) {
    for _ in 0..250 {
        if cond() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("timed out waiting for {what}");
}

type App = rwire::ServerWithRoot<fn() -> ElementBuilder>;

/// Serve `root` behind the test gate, configured by `f`.
async fn serve(
    port: u16,
    root: fn() -> ElementBuilder,
    f: impl FnOnce(App) -> App + Send + 'static,
) {
    tokio::spawn(async move {
        let server = Server::bind(&format!("127.0.0.1:{port}"))
            .unwrap()
            .root(root)
            .auth_with(auth());
        let _ = f(server).run().await;
    });
    tokio::time::sleep(Duration::from_millis(150)).await;
}

async fn close(mut ws: Ws) {
    ws.send(Message::Close(None)).await.unwrap();
    // Drain until the server's close (or the end of the stream).
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(Ok(m)) = ws.next().await {
            if matches!(m, Message::Close(_)) {
                break;
            }
        }
    })
    .await;
}

// ── Once per connection, with the identity, before the first render ────────

static CONNECTS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static DISCONNECTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

#[derive(State, Default)]
#[storage(memory)]
struct Me {
    line: String,
}

#[handler]
fn greet(me: &mut Me, ctx: &EventContext) {
    let user = ctx
        .identity()
        .map(|i| i.id().to_string())
        .unwrap_or_default();
    me.line = format!("hello {user} session:{}", ctx.session_id().is_some());
    CONNECTS.lock().unwrap().push(user);
}

#[handler]
fn farewell(me: &mut Me, ctx: &EventContext) {
    let user = ctx
        .identity()
        .map(|i| i.id().to_string())
        .unwrap_or_default();
    DISCONNECTS
        .lock()
        .unwrap()
        .push(format!("{user} (was: {})", me.line));
}

#[renderer]
fn me_view(me: &Me) -> ElementBuilder {
    el(El::P).text(if me.line.is_empty() {
        "nobody"
    } else {
        &me.line
    })
}

fn me_app() -> ElementBuilder {
    el(El::Main).append([me_view()])
}

#[tokio::test]
async fn on_connect_runs_once_per_connection_with_the_identity_before_the_first_paint() {
    serve(19071, me_app, |s| {
        s.on_connect(greet()).on_disconnect(farewell())
    })
    .await;

    // The very first frame already shows what on_connect wrote: no event,
    // no route message was sent.
    let mut ada = connect(19071, "ada").await;
    wait_for(&mut ada, "hello ada session:true").await;
    let mut bob = connect(19071, "bob").await;
    wait_for(&mut bob, "hello bob session:true").await;

    let connects = |u: &str| CONNECTS.lock().unwrap().iter().filter(|c| *c == u).count();
    assert_eq!((connects("ada"), connects("bob")), (1, 1), "once each");

    // Closing runs on_disconnect, with the identity and the connection's state.
    close(ada).await;
    until("ada's on_disconnect", || {
        DISCONNECTS
            .lock()
            .unwrap()
            .contains(&"ada (was: hello ada session:true)".to_string())
    })
    .await;
    assert!(
        !DISCONNECTS
            .lock()
            .unwrap()
            .iter()
            .any(|d| d.starts_with("bob")),
        "bob is still connected"
    );

    // A dropped TCP connection (no close frame) is a disconnect too.
    drop(bob);
    until("bob's on_disconnect", || {
        DISCONNECTS
            .lock()
            .unwrap()
            .iter()
            .any(|d| d.starts_with("bob"))
    })
    .await;

    // A reconnect is a new connection: on_connect runs again.
    let mut again = connect(19071, "ada").await;
    wait_for(&mut again, "hello ada").await;
    assert_eq!(connects("ada"), 2);
}

// ── A task spawned at connect is aborted at disconnect ─────────────────────

static TICKS: AtomicUsize = AtomicUsize::new(0);
static TASK_DROPPED: AtomicBool = AtomicBool::new(false);
static SPAWN_AFTER_CLOSE: Mutex<Option<bool>> = Mutex::new(None);

/// Flags its own drop: an aborted future is dropped.
struct DropFlag;
impl Drop for DropFlag {
    fn drop(&mut self) {
        TASK_DROPPED.store(true, Ordering::SeqCst);
    }
}

#[derive(State, Default)]
#[storage(memory)]
struct Worker {
    started: bool,
}

#[handler]
fn start_worker(w: &mut Worker, ctx: &EventContext) {
    w.started = ctx.spawn(async {
        let _flag = DropFlag;
        loop {
            TICKS.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });
}

#[handler]
fn stop_worker(_w: &mut Worker, ctx: &EventContext) {
    // Too late to start anything: the connection's tasks are gone.
    *SPAWN_AFTER_CLOSE.lock().unwrap() = Some(ctx.spawn(async {}));
}

#[renderer]
fn worker_view(w: &Worker) -> ElementBuilder {
    el(El::P).text(if w.started {
        "worker started"
    } else {
        "no worker"
    })
}

fn worker_app() -> ElementBuilder {
    el(El::Main).append([worker_view()])
}

#[tokio::test]
async fn a_task_spawned_on_connect_is_aborted_on_disconnect() {
    serve(19072, worker_app, |s| {
        s.on_connect(start_worker()).on_disconnect(stop_worker())
    })
    .await;

    let mut ws = connect(19072, "ada").await;
    wait_for(&mut ws, "worker started").await;
    until("the task to tick", || TICKS.load(Ordering::SeqCst) >= 3).await;
    assert!(
        !TASK_DROPPED.load(Ordering::SeqCst),
        "alive while connected"
    );

    close(ws).await;
    until("the task to be aborted", || {
        TASK_DROPPED.load(Ordering::SeqCst)
    })
    .await;
    let after = TICKS.load(Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        TICKS.load(Ordering::SeqCst),
        after,
        "no tick after the abort"
    );
    until("on_disconnect", || {
        SPAWN_AFTER_CLOSE.lock().unwrap().is_some()
    })
    .await;
    assert_eq!(
        *SPAWN_AFTER_CLOSE.lock().unwrap(),
        Some(false),
        "spawn is refused in on_disconnect"
    );
}

// ── Shared state: presence broadcast to the other connections ──────────────

#[derive(State, Default)]
#[storage(shared)]
struct Room {
    present: Vec<String>,
    left: Vec<String>,
}

#[handler]
fn join(room: &mut Room, ctx: &EventContext) {
    if let Some(i) = ctx.identity() {
        room.present.push(i.id().to_string());
    }
}

#[handler]
fn leave(room: &mut Room, ctx: &EventContext) {
    if let Some(i) = ctx.identity() {
        room.present.retain(|p| p != i.id());
        room.left.push(i.id().to_string());
    }
}

#[renderer]
fn room_view(room: &Room) -> ElementBuilder {
    // A text already sent is referenced by symbol index afterwards, so each
    // state the test waits for renders a text not seen before.
    el(El::P).text(&format!(
        "present: [{}] left: [{}]",
        room.present.join(","),
        room.left.join(",")
    ))
}

fn room_app() -> ElementBuilder {
    el(El::Main).append([room_view()])
}

#[tokio::test]
async fn shared_state_lifecycle_handlers_broadcast_presence() {
    serve(19073, room_app, |s| {
        s.on_connect(join()).on_disconnect(leave())
    })
    .await;

    let mut ada = connect(19073, "ada").await;
    wait_for(&mut ada, "present: [ada] left: []").await;
    let mut bob = connect(19073, "bob").await;
    wait_for(&mut bob, "present: [ada,bob] left: []").await;
    // Ada sees Bob join, then leave.
    wait_for(&mut ada, "present: [ada,bob] left: []").await;
    close(bob).await;
    wait_for(&mut ada, "present: [ada] left: [bob]").await;
}

// ── Server shutdown is a disconnect too ────────────────────────────────────

static SHUTDOWN_DISCONNECTS: AtomicUsize = AtomicUsize::new(0);
static SHUTDOWN_TASK_DROPPED: AtomicBool = AtomicBool::new(false);

struct ShutdownDropFlag;
impl Drop for ShutdownDropFlag {
    fn drop(&mut self) {
        SHUTDOWN_TASK_DROPPED.store(true, Ordering::SeqCst);
    }
}

#[handler]
fn start_pending(w: &mut Worker, ctx: &EventContext) {
    w.started = ctx.spawn(async {
        let _flag = ShutdownDropFlag;
        std::future::pending::<()>().await;
    });
}

#[handler]
fn count_disconnect(_w: &mut Worker) {
    SHUTDOWN_DISCONNECTS.fetch_add(1, Ordering::SeqCst);
}

#[tokio::test]
async fn run_until_shutdown_runs_on_disconnect_and_aborts_tasks() {
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        Server::bind("127.0.0.1:19074")
            .unwrap()
            .root(worker_app)
            .auth_with(auth())
            .on_connect(start_pending())
            .on_disconnect(count_disconnect())
            .run_until(async {
                let _ = stop_rx.await;
            })
            .await
            .map_err(|e| e.to_string())
    });
    tokio::time::sleep(Duration::from_millis(150)).await;

    let mut ws = connect(19074, "ada").await;
    wait_for(&mut ws, "worker started").await;
    stop_tx.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(10), server)
        .await
        .expect("server stopped")
        .unwrap()
        .unwrap();
    assert_eq!(SHUTDOWN_DISCONNECTS.load(Ordering::SeqCst), 1);
    assert!(SHUTDOWN_TASK_DROPPED.load(Ordering::SeqCst));
}

// ── A panicking handler still aborts the connection's tasks ────────────────

static PANIC_TASK_DROPPED: AtomicBool = AtomicBool::new(false);

struct PanicDropFlag;
impl Drop for PanicDropFlag {
    fn drop(&mut self) {
        PANIC_TASK_DROPPED.store(true, Ordering::SeqCst);
    }
}

#[handler]
fn start_pending_for_panic(w: &mut Worker, ctx: &EventContext) {
    w.started = ctx.spawn(async {
        let _flag = PanicDropFlag;
        std::future::pending::<()>().await;
    });
}

#[handler]
fn explode(_w: &mut Worker) {
    panic!("handler bug (expected by the test)");
}

fn panic_app() -> ElementBuilder {
    el(El::Main).append([
        worker_view(),
        el(El::Button).text("boom").on(rwire::Ev::Click, explode()),
    ])
}

#[tokio::test]
async fn a_panicking_handler_still_aborts_the_tasks() {
    serve(19075, panic_app, |s| {
        s.on_connect(start_pending_for_panic())
    })
    .await;
    let mut ws = connect(19075, "ada").await;
    wait_for(&mut ws, "worker started").await;
    let mut frame = vec![0x00];
    rwire::protocol::varint::write_varint(&mut frame, explode().handler_id());
    frame.extend_from_slice(&[rwire::Ev::Click.as_u8(), 0, 0]);
    ws.send(Message::Binary(frame)).await.unwrap();
    until("the task to be aborted by the unwinding connection", || {
        PANIC_TASK_DROPPED.load(Ordering::SeqCst)
    })
    .await;
}
