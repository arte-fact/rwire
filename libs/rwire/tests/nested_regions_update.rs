//! Nested synced regions during update passes.
//!
//! Three bugs found 2026-11-27 by a downstream front's e2e suite, reproduced
//! here at the server level over a real WebSocket connection:
//!
//! (a) a grand-child region is re-rendered only when its parent's output
//!     changes, so a change of an outer state never reaches level 3;
//! (b) a region first created during an update pass renders EMPTY when it
//!     reads a memory state not yet in the connection's state map (memory
//!     states are seeded only at first render and on view swap);
//! (c) a region whose position shifts (a sibling inserted before it) is
//!     minted anew and renders empty.
//!
//! The app nests three memory-state regions: `shell` (Outer) contains a `Mid`
//! region whose renderer captures Outer's `n`, which contains an `Inner`
//! region whose renderer captures that `n` too — so the level-3 region's
//! content changes with the outer state even though its own state and its
//! parent's output do not. `Late` regions exist only once an update pass
//! flips `show_late` (no `Late` handler exists, so the connection never seeds
//! a `Late` state at first render), and the `prepend_*` handlers insert a
//! same-typed sibling region before an existing one, shifting its position.

use std::time::Duration;

use futures::{SinkExt, StreamExt};
use rwire::protocol::varint::write_varint;
use rwire::{el, handler, renderer, El, ElementBuilder, Ev, RendererDeps, Server, State};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;

/// Level 1: the outer memory state.
#[derive(State, Default)]
#[storage(memory)]
struct Outer {
    n: u32,
    show_late: bool,
    prepend_late: bool,
    prepend_mid: bool,
}

/// Level 2: a memory state read by the regions nested in `shell`.
#[derive(State, Default)]
#[storage(memory)]
struct Mid {
    m: u32,
}

/// Level 3: a memory state read by the region nested in a `Mid` region.
#[derive(State, Default)]
#[storage(memory)]
struct Inner {
    i: u32,
}

/// Read by regions that only exist once `show_late` is flipped: at first
/// render no `Late` region exists and no handler references `Late`, so the
/// connection never seeds a `Late` state.
#[derive(State, Default)]
#[storage(memory)]
struct Late {
    v: u32,
}

#[handler]
fn bump_outer(o: &mut Outer) {
    o.n += 1;
}

#[handler]
fn reveal_late(o: &mut Outer) {
    o.show_late = true;
}

#[handler]
fn prepend_late(o: &mut Outer) {
    o.prepend_late = true;
    o.n += 1;
}

#[handler]
fn prepend_mid(o: &mut Outer) {
    o.prepend_mid = true;
    o.n += 1;
}

/// A `Mid` region tagged `tag`: its own text reads `Mid`, and it nests an
/// `Inner` region whose renderer captures the outer `n` — so the level-3
/// content changes with `Outer` while this region's own output does not.
fn mid_region(tag: &'static str, n: u32) -> ElementBuilder {
    ElementBuilder::synced_with_storage::<Mid, _>(
        move |mid| {
            let m = mid.m;
            el(El::Section).append([
                el(El::P).text(&format!("mid:{tag}:{m}")),
                ElementBuilder::synced_with_storage::<Inner, _>(
                    move |inner| el(El::B).text(&format!("inner:{tag}:{n}:{m}:{}", inner.i)),
                    RendererDeps::always(),
                ),
            ])
        },
        RendererDeps::always(),
    )
}

/// A `Late` region tagged `tag`: renders its own memory state plus the outer
/// `n` it captures, so each update pass emits fresh text (a symbol the client
/// has never seen) and a region left empty is detectable.
fn late_region(tag: &'static str, n: u32) -> ElementBuilder {
    ElementBuilder::synced_with_storage::<Late, _>(
        move |late| el(El::Span).text(&format!("late:{tag}:{}:{n}", late.v)),
        RendererDeps::always(),
    )
}

#[renderer]
fn shell(o: &Outer) -> ElementBuilder {
    let n = o.n;
    let mut kids: Vec<ElementBuilder> = Vec::new();
    if o.prepend_late {
        kids.push(late_region("c", n));
    }
    if o.show_late {
        kids.push(late_region("a", n));
        kids.push(late_region("b", n));
    }
    if o.prepend_mid {
        kids.push(mid_region("extra", n));
    }
    kids.push(mid_region("main", n));
    el(El::Div).append(kids).append([
        el(El::Button).text("bump").on(Ev::Click, bump_outer()),
        el(El::Button).text("reveal").on(Ev::Click, reveal_late()),
        el(El::Button)
            .text("prepend-late")
            .on(Ev::Click, prepend_late()),
        el(El::Button)
            .text("prepend-mid")
            .on(Ev::Click, prepend_mid()),
    ])
}

/// Start the app on `port`; returns once it accepts connections.
async fn start(port: u16) {
    tokio::spawn(async move {
        let _ = Server::bind(&format!("127.0.0.1:{port}"))
            .unwrap()
            .root(shell)
            .run()
            .await;
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

type Ws = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>;

/// A test client: the socket plus every byte received so far. A text reaches
/// the client exactly when its symbol is first interned, and the symbol's
/// bytes appear verbatim in that batch — so `saw` over all received bytes is
/// "the client has painted this text".
struct Client {
    ws: Ws,
    seen: Vec<u8>,
}

async fn connect(port: u16) -> Client {
    let (ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}/"))
        .await
        .unwrap();
    Client {
        ws,
        seen: Vec::new(),
    }
}

fn contains(data: &[u8], needle: &str) -> bool {
    data.windows(needle.len()).any(|w| w == needle.as_bytes())
}

impl Client {
    /// Read the initial DOM: frames until the one closing the batch
    /// (BATCH_END).
    async fn read_initial(&mut self) -> Vec<u8> {
        let mut batch = Vec::new();
        let done = tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(Ok(message)) = self.ws.next().await {
                if let Message::Binary(data) = message {
                    let closes = data.last() == Some(&0xFF);
                    batch.extend_from_slice(&data);
                    if closes {
                        return true;
                    }
                }
            }
            false
        })
        .await;
        assert_eq!(done, Ok(true), "no initial DOM reached the client");
        self.seen.extend_from_slice(&batch);
        batch
    }

    /// Read the whole answer to one event: the DOM update frame(s), then the
    /// two-byte EVENT_ACK that closes the answer.
    async fn read_answer(&mut self) -> Vec<u8> {
        let mut batch = Vec::new();
        let done = tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(Ok(message)) = self.ws.next().await {
                if let Message::Binary(data) = message {
                    let ack = data == [0xF3, 0xFF];
                    batch.extend_from_slice(&data);
                    if ack {
                        return true;
                    }
                }
            }
            false
        })
        .await;
        assert_eq!(done, Ok(true), "no answer reached the client");
        self.seen.extend_from_slice(&batch);
        batch
    }

    /// Send a click event for `spec` to the server, the way the runtime does:
    /// [flags, handler varint, event, target_ref, payload_len].
    async fn click(&mut self, spec: rwire::HandlerSpec) {
        let mut msg = vec![0x00];
        write_varint(&mut msg, spec.handler_id());
        msg.extend_from_slice(&[Ev::Click.as_u8(), 0, 0]);
        self.ws.send(Message::Binary(msg)).await.unwrap();
    }

    /// Has the client painted `needle` in any batch so far?
    fn saw(&self, needle: &str) -> bool {
        contains(&self.seen, needle)
    }
}

fn assert_saw(client: &Client, needle: &str) {
    assert!(
        client.saw(needle),
        "{needle:?} never reached the client ({} bytes seen)",
        client.seen.len()
    );
}

fn assert_lacks(batch: &[u8], needle: &str) {
    assert!(
        !contains(batch, needle),
        "{needle:?} unexpectedly present in the batch"
    );
}

/// (a) Bump the outer state: the level-3 (`Inner`) region's content captures
/// the outer value, so the update pass must reach it even though neither its
/// own state nor its parent's (`Mid`) output changed.
#[tokio::test]
async fn an_outer_state_change_reaches_the_grandchild_region() {
    start(19201).await;
    let mut client = connect(19201).await;
    let initial = client.read_initial().await;
    assert_lacks(&initial, "inner:main:1");
    assert_saw(&client, "mid:main:0");
    assert_saw(&client, "inner:main:0:0:0");

    client.click(bump_outer()).await;
    client.read_answer().await;
    assert_saw(&client, "inner:main:1:0:0");

    client.click(bump_outer()).await;
    client.read_answer().await;
    assert_saw(&client, "inner:main:2:0:0");
}

/// (b) Flip `show_late`: `Late` regions are first created during this update
/// pass. Their memory state was never seeded (no `Late` region or handler
/// existed at first render), so the new regions must still render their
/// state's default, not empty spans.
#[tokio::test]
async fn a_region_created_in_an_update_pass_renders_its_memory_default() {
    start(19202).await;
    let mut client = connect(19202).await;
    let initial = client.read_initial().await;
    assert_lacks(&initial, "late:");
    assert_saw(&client, "inner:main:0:0:0");

    client.click(reveal_late()).await;
    client.read_answer().await;
    assert_saw(&client, "late:a:0:0");
    assert_saw(&client, "late:b:0:0");

    // And the page stays live: a later outer change still reaches level 3.
    client.click(bump_outer()).await;
    client.read_answer().await;
    assert_saw(&client, "inner:main:1:0:0");
}

/// (c) Insert a same-typed sibling region before existing ones: the regions
/// after the insertion point shift position and one is minted anew — they
/// must still render their content, not empty spans.
#[tokio::test]
async fn a_position_shifted_region_is_minted_anew_but_still_renders() {
    start(19203).await;
    let mut client = connect(19203).await;
    client.read_initial().await;
    assert_saw(&client, "inner:main:0:0:0");

    // Two Late regions appear during this update pass.
    client.click(reveal_late()).await;
    client.read_answer().await;
    assert_saw(&client, "late:a:0:0");
    assert_saw(&client, "late:b:0:0");

    // A third Late region is inserted before them: a and b shift position
    // and the last one is minted anew — all three must render, each with the
    // bumped outer n (fresh symbols, so an empty span is detectable).
    client.click(prepend_late()).await;
    client.read_answer().await;
    assert_saw(&client, "late:c:0:1");
    assert_saw(&client, "late:a:0:1");
    assert_saw(&client, "late:b:0:1");
    // The level-3 region still receives the bumped outer value.
    assert_saw(&client, "inner:main:1:0:0");
}

/// Guard: a same-typed `Mid` sibling prepended before the existing `Mid`
/// region (whose state IS seeded) — the shifted region is minted anew and
/// renders inline, with its own nested level-3 region.
#[tokio::test]
async fn a_minted_anew_region_renders_inline_with_its_nested_region() {
    start(19204).await;
    let mut client = connect(19204).await;
    client.read_initial().await;
    assert_saw(&client, "mid:main:0");
    assert_saw(&client, "inner:main:0:0:0");

    client.click(prepend_mid()).await;
    client.read_answer().await;
    assert_saw(&client, "mid:extra:0");
    assert_saw(&client, "inner:main:1:0:0");
    assert_saw(&client, "inner:extra:1:0:0");
}
