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
//!
//! Assertions on update answers are OPCODE-level (`wire::parse` +
//! `assert_painted`): a text counts as painted only when a `SET_TEXT` on a
//! ref appended under a `CREATE_SYNCED`/`GET_SYNCED` wrapper of the same batch
//! carries its symbol. Raw byte presence (`saw`) only proves the symbol table
//! interned the text — `collect_symbols_recursive_with_known` interns even
//! when the span itself stays empty — so it cannot prove the emit fix.
//!
//! Two more server-side behaviors are covered here (review round 2):
//! reconcile_nested_regions seeding the connection's state map for a
//! mid-pass-discovered region (`Session` mints a distinct token per default,
//! so rendering from a throwaway default instead of the connection's copy is
//! visible on the wire), and build_type_update lazily creating the
//! shared-cache entry + subscription for a `#[storage(shared)]` state first
//! read by a region discovered during an update pass (`Board`).

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use rwire::protocol::opcodes as ops;
use rwire::protocol::read_varint;
use rwire::protocol::varint::write_varint;
use rwire::{
    el, handler, renderer, ChangeSet, El, ElementBuilder, Ev, HandlerFn, MemoryState, RendererDeps,
    Server, SharedServerState, State,
};
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

/// Start an app on `port`; returns its shared state once it accepts connections.
async fn start_with<F>(port: u16, root: F) -> Arc<SharedServerState>
where
    F: Fn() -> ElementBuilder + Clone + Send + Sync + 'static,
{
    let mut server = Server::bind(&format!("127.0.0.1:{port}"))
        .unwrap()
        .root(root);
    let shared = server.shared_state();
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

/// Start the nested-regions app on `port`; returns once it accepts connections.
async fn start(port: u16) {
    let _ = start_with(port, shell).await;
}

/// Poll `cond` until it holds (the server processes a pass before the answer
/// reaches the client, but the assertion below is on the server-side maps).
async fn wait_until(mut cond: impl FnMut() -> bool) {
    for _ in 0..100 {
        if cond() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(cond(), "condition never became true");
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

/// A minimal parser for one server→client batch (an answer may span several
/// frames; they are concatenated). Refs and the symbol table are per-batch
/// (each message gets a fresh `OpcodeBuffer`), symbol indices absolute.
///
/// Only the opcodes this fixture can produce are decoded; anything else panics
/// with the opcode byte, so a new opcode in a batch fails loudly here rather
/// than silently desyncing the assertions.
mod wire {
    use super::*;

    pub struct Batch {
        /// symbol index -> string
        symbols: HashMap<u32, String>,
        /// ref -> text set by SET_TEXT
        texts: HashMap<u32, String>,
        /// child ref -> parent ref (APPEND)
        parent_of: HashMap<u32, u32>,
        /// ref from CREATE_SYNCED / GET_SYNCED -> synced id
        synced_of_ref: HashMap<u32, u32>,
    }

    impl Batch {
        /// Was `needle` painted by a SET_TEXT whose ref sits inside a synced
        /// wrapper span (CREATE_SYNCED or GET_SYNCED) of this same batch?
        /// This is what "the span is not empty" means on the wire.
        pub fn painted_in_synced_span(&self, needle: &str) -> bool {
            self.texts
                .iter()
                .any(|(r, t)| t == needle && self.inside_synced(*r))
        }

        /// Every text painted by a SET_TEXT in this batch.
        pub fn painted_texts(&self) -> Vec<&str> {
            self.texts.values().map(String::as_str).collect()
        }

        fn inside_synced(&self, mut r: u32) -> bool {
            loop {
                if self.synced_of_ref.contains_key(&r) {
                    return true;
                }
                match self.parent_of.get(&r) {
                    Some(&p) => r = p,
                    None => return false,
                }
            }
        }
    }

    struct Cur<'a> {
        d: &'a [u8],
        p: usize,
    }

    impl<'a> Cur<'a> {
        fn u8(&mut self) -> u8 {
            let b = self.d[self.p];
            self.p += 1;
            b
        }
        fn varint(&mut self) -> u32 {
            let (v, n) = read_varint(&self.d[self.p..]).expect("truncated varint");
            self.p += n;
            v
        }
        fn take(&mut self, n: usize) -> &'a [u8] {
            let s = &self.d[self.p..self.p + n];
            self.p += n;
            s
        }
    }

    pub fn parse(data: &[u8]) -> Batch {
        let mut b = Batch {
            symbols: HashMap::new(),
            texts: HashMap::new(),
            parent_of: HashMap::new(),
            synced_of_ref: HashMap::new(),
        };
        let mut cur = Cur { d: data, p: 0 };
        let mut next_ref: u32 = 0;
        let mut next_sym: u32 = ops::SYMBOL_SESSION_START as u32;
        while cur.p < cur.d.len() {
            let op = cur.u8();
            match op {
                ops::BATCH_END => break,
                ops::EVENT_ACK => {}
                ops::SYMBOLS => {
                    let count = cur.varint();
                    for _ in 0..count {
                        let len = cur.varint() as usize;
                        let s = String::from_utf8_lossy(cur.take(len)).into_owned();
                        b.symbols.insert(next_sym, s);
                        next_sym += 1;
                    }
                }
                ops::SYMBOLS_EXTEND => {
                    let count = cur.varint();
                    next_sym = cur.varint();
                    for _ in 0..count {
                        let len = cur.varint() as usize;
                        let s = String::from_utf8_lossy(cur.take(len)).into_owned();
                        b.symbols.insert(next_sym, s);
                        next_sym += 1;
                    }
                }
                ops::WORD_TABLE => {
                    let count = cur.u8() as usize;
                    for _ in 0..count {
                        let len = cur.varint() as usize;
                        cur.take(len);
                    }
                }
                ops::MOD_DEF => {
                    let count = cur.varint();
                    for _ in 0..count {
                        let len = cur.varint() as usize;
                        cur.take(len);
                    }
                }
                ops::MAP_DEF => {
                    let count = cur.varint();
                    for _ in 0..count {
                        cur.u8(); // kind
                        cur.u8(); // code
                        let len = cur.varint() as usize;
                        cur.take(len);
                    }
                }
                ops::STYLE_DEF => {
                    let count = cur.varint();
                    for _ in 0..count {
                        cur.varint(); // order
                        let len = cur.varint() as usize;
                        cur.take(len);
                    }
                }
                ops::CREATE => {
                    cur.u8(); // element type
                    next_ref += 1;
                }
                ops::CREATE_SYNCED => {
                    let id = cur.varint();
                    b.synced_of_ref.insert(next_ref, id);
                    next_ref += 1;
                }
                ops::GET_BY_ID => {
                    cur.varint();
                    next_ref += 1;
                }
                ops::GET_SYNCED => {
                    let id = cur.varint();
                    b.synced_of_ref.insert(next_ref, id);
                    next_ref += 1;
                }
                ops::SET_CLASS => {
                    cur.varint();
                    cur.varint();
                }
                ops::SET_TEXT => {
                    let r = cur.varint();
                    let sym = cur.varint();
                    if let Some(s) = b.symbols.get(&sym) {
                        b.texts.insert(r, s.clone());
                    }
                }
                ops::SET_ATTR | ops::SET_DATA => {
                    cur.varint();
                    cur.varint();
                    cur.varint();
                }
                ops::SET_TEXT_WORDS => {
                    cur.varint();
                    let count = cur.u8() as usize;
                    for _ in 0..count {
                        cur.u8();
                    }
                }
                ops::SET_TEXT_INT => {
                    cur.varint();
                    cur.varint();
                }
                ops::SET_KEY => {
                    cur.varint();
                    cur.varint();
                }
                ops::APPEND => {
                    let parent = cur.varint();
                    let child = cur.varint();
                    b.parent_of.insert(child, parent);
                }
                ops::CLEAR_CHILDREN => {
                    cur.varint();
                }
                ops::SET_ATTR_ENUM => {
                    cur.varint();
                    cur.u8();
                    cur.u8();
                }
                ops::SET_ATTR_BOOL => {
                    cur.varint();
                    cur.u8();
                }
                ops::SET_ATTR_KEY_SYM => {
                    cur.varint();
                    cur.u8();
                    cur.varint();
                }
                ops::BIND_LOCAL | ops::BIND_REMOTE | ops::BIND_OPTIMISTIC => {
                    cur.varint();
                    cur.u8();
                    cur.varint();
                }
                ops::BIND_DEBOUNCED => {
                    cur.varint();
                    cur.u8();
                    cur.varint();
                    cur.u8();
                    cur.u8();
                }
                ops::BIND_REMOTE_PARAM | ops::BIND_SENTINEL => {
                    cur.varint();
                    if op == ops::BIND_REMOTE_PARAM {
                        cur.u8(); // event type
                    }
                    cur.varint(); // handler
                    let len = cur.varint() as usize;
                    cur.take(len);
                }
                ops::INIT_TARGET => {
                    cur.varint();
                    cur.u8();
                }
                ops::BIND_TARGET => {
                    cur.varint();
                    cur.varint();
                    cur.varint();
                    cur.u8();
                }
                ops::BIND_TOGGLE => {
                    cur.varint();
                    cur.u8();
                    cur.varint();
                }
                ops::INIT_SELECTOR => {
                    cur.varint();
                    cur.u8();
                }
                ops::BIND_SELECTOR => {
                    cur.varint();
                    cur.varint();
                    cur.u8();
                    cur.varint();
                }
                ops::BIND_SELECT => {
                    cur.varint();
                    cur.u8();
                    cur.varint();
                    cur.u8();
                }
                ops::BIND_TIMED_TOGGLE => {
                    cur.varint();
                    cur.u8();
                    cur.varint();
                    cur.u8();
                    cur.u8();
                }
                ops::AUTO_TOGGLE => {
                    cur.varint();
                    cur.u8();
                    cur.u8();
                }
                ops::BIND_RESIZE => {
                    cur.varint();
                }
                ops::LIVE_SOURCE => {
                    cur.varint();
                    cur.varint();
                }
                ops::FORM_SET_VALIDATION | ops::FORM_SHOW_ERROR => {
                    cur.varint();
                    cur.varint();
                }
                ops::FORM_CLEAR_ERROR => {
                    cur.varint();
                }
                ops::FORM_SET_REQUIRED => {
                    cur.varint();
                    cur.u8();
                }
                ops::ROUTE_PUSH | ops::ROUTE_REPLACE => {
                    cur.varint();
                }
                ops::ROUTE_PUSH_INLINE | ops::ROUTE_REPLACE_INLINE => {
                    let len = cur.varint() as usize;
                    cur.take(len);
                }
                ops::STYLE_SET => {
                    cur.varint();
                    cur.varint();
                }
                ops::STYLE_UTIL => {
                    cur.varint();
                    cur.varint();
                }
                ops::STYLE_PROP => {
                    cur.varint();
                    cur.u8();
                    cur.u8();
                }
                ops::STYLE_MULTI => {
                    cur.varint();
                    let count = cur.u8() as usize;
                    for _ in 0..count {
                        cur.varint();
                    }
                }
                ops::STYLE_COMPOSITE => {
                    cur.varint();
                    cur.varint();
                }
                ops::STYLE_PSEUDO | ops::STYLE_BREAKPOINT => {
                    cur.varint();
                    cur.u8();
                    let count = cur.u8() as usize;
                    for _ in 0..count {
                        cur.varint();
                    }
                }
                other => {
                    panic!("wire::parse: unhandled opcode {other:#04x} (extend the test parser)")
                }
            }
        }
        b
    }
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

    /// Read one server-initiated frame (a broadcast), closed by BATCH_END.
    async fn read_broadcast(&mut self) -> Vec<u8> {
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
        assert_eq!(done, Ok(true), "no broadcast frame reached the client");
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

/// Opcode-level assertion for an update batch: `needle`'s text must be carried
/// by a SET_TEXT opcode on a ref appended under a synced wrapper span
/// (CREATE_SYNCED/GET_SYNCED) of this same batch — i.e. the region's span is
/// really filled, not just its symbol interned.
fn assert_painted(batch: &[u8], needle: &str) {
    let parsed = wire::parse(batch);
    assert!(
        parsed.painted_in_synced_span(needle),
        "{needle:?} was not painted by a SET_TEXT inside a synced span in this batch"
    );
}

/// The painted texts of a batch that start with `prefix`, in wire order.
fn painted_with_prefix(batch: &[u8], prefix: &str) -> Vec<String> {
    wire::parse(batch)
        .painted_texts()
        .into_iter()
        .filter(|t| t.starts_with(prefix))
        .map(str::to_string)
        .collect()
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
    assert_painted(&initial, "mid:main:0");
    assert_painted(&initial, "inner:main:0:0:0");

    client.click(bump_outer()).await;
    let answer = client.read_answer().await;
    assert_painted(&answer, "inner:main:1:0:0");

    client.click(bump_outer()).await;
    let answer = client.read_answer().await;
    assert_painted(&answer, "inner:main:2:0:0");
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
    assert_painted(&initial, "inner:main:0:0:0");

    client.click(reveal_late()).await;
    let answer = client.read_answer().await;
    assert_painted(&answer, "late:a:0:0");
    assert_painted(&answer, "late:b:0:0");

    // And the page stays live: a later outer change still reaches level 3.
    client.click(bump_outer()).await;
    let answer = client.read_answer().await;
    assert_painted(&answer, "inner:main:1:0:0");
}

/// (c) Insert a same-typed sibling region before existing ones: the regions
/// after the insertion point shift position and one is minted anew — they
/// must still render their content, not empty spans.
#[tokio::test]
async fn a_position_shifted_region_is_minted_anew_but_still_renders() {
    start(19203).await;
    let mut client = connect(19203).await;
    let initial = client.read_initial().await;
    assert_painted(&initial, "inner:main:0:0:0");

    // Two Late regions appear during this update pass.
    client.click(reveal_late()).await;
    let answer = client.read_answer().await;
    assert_painted(&answer, "late:a:0:0");
    assert_painted(&answer, "late:b:0:0");

    // A third Late region is inserted before them: a and b shift position
    // and the last one is minted anew — all three must render, each with the
    // bumped outer n (fresh symbols, so an empty span is detectable).
    client.click(prepend_late()).await;
    let answer = client.read_answer().await;
    assert_painted(&answer, "late:c:0:1");
    assert_painted(&answer, "late:a:0:1");
    assert_painted(&answer, "late:b:0:1");
    // The level-3 region still receives the bumped outer value.
    assert_painted(&answer, "inner:main:1:0:0");
}

/// Guard: a same-typed `Mid` sibling prepended before the existing `Mid`
/// region (whose state IS seeded) — the shifted region is minted anew and
/// renders inline, with its own nested level-3 region.
#[tokio::test]
async fn a_minted_anew_region_renders_inline_with_its_nested_region() {
    start(19204).await;
    let mut client = connect(19204).await;
    let initial = client.read_initial().await;
    assert_painted(&initial, "mid:main:0");
    assert_painted(&initial, "inner:main:0:0:0");

    client.click(prepend_mid()).await;
    let answer = client.read_answer().await;
    assert_painted(&answer, "mid:extra:0");
    assert_painted(&answer, "inner:main:1:0:0");
    assert_painted(&answer, "inner:extra:1:0:0");
}

/// Limit 3 of the perf study: a nested region the cascade re-rendered with the
/// parent's fresh renderer — but whose output is unchanged — must NOT be
/// rendered a second time by the layer-1 pass with its stored (older)
/// renderer when its deps match the ChangeSet. Counted directly: the child's
/// renderer runs exactly once during the update pass.
#[test]
fn a_cascade_touched_child_is_not_rendered_twice() {
    #[derive(Default)]
    struct Level {
        a: u32,
    }
    impl MemoryState for Level {}

    let renders = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&renders);
    // The parent (type Level) nests a child region of the SAME type, whose
    // renderer counts its invocations and renders constant content (so the
    // cascade's fresh render is hash-unchanged, yet the layer-1 pass would
    // still pick it up: same type as the changed state, deps always).
    let parent = ElementBuilder::synced_with_deps::<Level, _>(
        move |level| {
            let _ = level.a; // read, but kept out of the output: parent hash unchanged
            let cnt = Arc::clone(&counter);
            el(El::Div)
                .text("p")
                .append([ElementBuilder::synced_with_deps::<Level, _>(
                    move |_child| {
                        cnt.fetch_add(1, Ordering::SeqCst);
                        el(El::Span).text("child")
                    },
                    RendererDeps::always(),
                )])
        },
        RendererDeps::always(),
    );

    type States<'a> = HashMap<TypeId, &'a (dyn Any + Send + Sync)>;

    let initial: Box<dyn Any + Send + Sync> = Box::new(Level { a: 0 });
    let mut states: States = HashMap::new();
    states.insert(TypeId::of::<Level>(), initial.as_ref());
    let mut ctx = rwire::builder::BuildContext::new();
    ctx.collect_symbols_multi(&parent, &states);
    ctx.emit_multi(&parent);
    let synced = ctx.take_synced_elements();
    let mut hashes = ctx.take_synced_hashes();
    let mut known = ctx.take_symbol_map();
    assert_eq!(synced.len(), 2, "parent + nested child region");

    // Update pass: Level changed (a: 0 -> 1), changed type = Level.
    renders.store(0, Ordering::SeqCst);
    let updated: Box<dyn Any + Send + Sync> = Box::new(Level { a: 1 });
    let mut states2: States = HashMap::new();
    states2.insert(TypeId::of::<Level>(), updated.as_ref());
    let mut handlers: HashMap<u32, HandlerFn> = HashMap::new();
    let mut discovered: Vec<rwire::builder::SyncedElement> = Vec::new();
    let mut rerendered: Vec<u32> = Vec::new();
    let _bytes = rwire::builder::build_synced_update_with_known_symbols(
        &synced,
        &states2,
        &mut handlers,
        rwire::ChangeSet::all(),
        Some(&mut known),
        Some(TypeId::of::<Level>()),
        Some(&mut hashes),
        None,
        None,
        Some(&mut discovered),
        Some(&mut rerendered),
        0,
        None,
    );
    assert_eq!(
        renders.load(Ordering::SeqCst),
        1,
        "the nested child rendered once (by the cascade), not again by the layer-1 pass"
    );
}

// ── Review round 2, item 2a: reconcile_nested_regions seeds the connection's
// ── state map for a region first discovered during an update pass.

static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);

/// A per-connection identity minted in `Default` (the multiplayer pattern):
/// every fresh default is a DISTINCT token, so a pass that renders the region
/// from a throwaway default instead of the connection's seeded copy is
/// visible on the wire — the token changes between passes.
#[derive(State)]
#[storage(memory)]
struct Session {
    token: u64,
    v: u32,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            token: NEXT_TOKEN.fetch_add(1, Ordering::SeqCst),
            v: 0,
        }
    }
}

/// The gate state: `show` flips a `Session` region into existence mid-pass
/// (no `Session` region or handler exists at first render, so the connection
/// never seeds a `Session` state then).
#[derive(State, Default)]
#[storage(memory)]
struct Gate {
    n: u32,
    show: bool,
}

#[handler]
fn reveal_gate(g: &mut Gate) {
    g.show = true;
}

#[handler]
fn bump_gate(g: &mut Gate) {
    g.n += 1;
}

/// A `Session` region whose text also captures the outer `n`, so every outer
/// bump re-renders (and re-paints) it.
fn sess_region(n: u32) -> ElementBuilder {
    ElementBuilder::synced_with_storage::<Session, _>(
        move |s| el(El::Span).text(&format!("sess:{}:{n}:{}", s.token, s.v)),
        RendererDeps::always(),
    )
}

#[renderer]
fn gate_shell(g: &Gate) -> ElementBuilder {
    let n = g.n;
    let mut kids = vec![
        el(El::Button).text("bump").on(Ev::Click, bump_gate()),
        el(El::Button).text("reveal").on(Ev::Click, reveal_gate()),
    ];
    if g.show {
        kids.push(sess_region(n));
    }
    el(El::Div).append(kids)
}

/// `reconcile_nested_regions` seeds `states` with every discovered region's
/// type, so later update passes render the mid-pass-discovered region from
/// the CONNECTION'S copy — not from a fresh default per pass. With a
/// per-default-distinct token that is observable: the token painted after
/// each bump must be the same. (Without the seeding, each pass mints a new
/// throwaway default and the token changes every pass — this test fails.)
#[tokio::test]
async fn a_mid_pass_discovered_region_renders_from_the_connections_seeded_state() {
    let _ = start_with(19205, gate_shell).await;
    let mut client = connect(19205).await;
    let initial = client.read_initial().await;
    assert_lacks(&initial, "sess:");

    // The Session region appears only during this update pass.
    client.click(reveal_gate()).await;
    client.read_answer().await;
    assert_saw(&client, "sess:"); // the inline fill rendered a default and interned it

    // Two outer bumps: the region re-renders (n is captured) each time.
    client.click(bump_gate()).await;
    let answer1 = client.read_answer().await;
    client.click(bump_gate()).await;
    let answer2 = client.read_answer().await;

    let t1 = painted_with_prefix(&answer1, "sess:");
    let t2 = painted_with_prefix(&answer2, "sess:");
    assert_eq!(t1.len(), 1, "one sess text painted in answer 1: {t1:?}");
    assert_eq!(t2.len(), 1, "one sess text painted in answer 2: {t2:?}");
    let token1 = t1[0].split(':').nth(1).unwrap().to_string();
    let token2 = t2[0].split(':').nth(1).unwrap().to_string();
    let n1 = t1[0].split(':').nth(2).unwrap();
    let n2 = t2[0].split(':').nth(2).unwrap();
    assert_ne!(
        n1, n2,
        "the region re-rendered with the bumped n: {t1:?} vs {t2:?}"
    );
    assert_eq!(
        token1, token2,
        "the region must render from the connection's seeded copy (reconcile_nested_regions \
         seeding), not from a fresh default per pass: {t1:?} vs {t2:?}"
    );
}

// ── Review round 2, item 2b: build_type_update lazily creates the shared-cache
// ── entry and the subscription for a #[storage(shared)] state first read by a
// ── region discovered during an update pass.

/// Shared state nobody renders at first connect (the `Door` shell shows no
/// board region until `open_door` runs), so no cache entry and no
/// subscription exist for it at connect time.
#[derive(State)]
#[storage(shared)]
struct Board {
    label: String,
    n: u32,
}

impl Default for Board {
    fn default() -> Self {
        Self {
            label: "default".to_string(),
            n: 0,
        }
    }
}

#[derive(State, Default)]
#[storage(memory)]
struct Door {
    show: bool,
}

#[handler]
fn open_door(d: &mut Door) {
    d.show = true;
}

fn board_region() -> ElementBuilder {
    ElementBuilder::synced_with_storage::<Board, _>(
        move |b| el(El::Span).text(&format!("board:{}:{}", b.label, b.n)),
        RendererDeps::always(),
    )
}

#[renderer]
fn board_shell(d: &Door) -> ElementBuilder {
    let mut kids = vec![el(El::Button).text("open").on(Ev::Click, open_door())];
    if d.show {
        kids.push(board_region());
    }
    el(El::Div).append(kids)
}

fn board_key() -> String {
    format!("__shared__:{}", Board::TABLE_NAME)
}

fn board_subscribed(shared: &SharedServerState) -> bool {
    shared
        .subscriptions
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .get(&board_key())
        .is_some_and(|v| !v.is_empty())
}

/// A `#[storage(shared)]` state first read by a region discovered mid-pass:
/// the discovering pass renders the state's default, and the pass must then
/// create the shared-cache entry and subscribe the connection — so a later
/// background publish reaches this connection with the REAL shared copy (not
/// a default). Without the mid-pass entry + subscription, the broadcast never
/// reaches the client and this test times out waiting for the frame.
#[tokio::test]
async fn a_shared_state_discovered_mid_pass_gets_cache_entry_and_subscription() {
    let shared = start_with(19206, board_shell).await;
    let mut client = connect(19206).await;
    let initial = client.read_initial().await;
    assert_lacks(&initial, "board:");
    assert!(
        !shared.has_state(&board_key()),
        "no region reads Board at first render, so no cache entry exists yet"
    );

    // The Board region appears only during this update pass.
    client.click(open_door()).await;
    let answer = client.read_answer().await;
    // The discovering pass itself renders the state's default (no entry yet)…
    assert_painted(&answer, "board:default:0");
    // …and the pass creates the shared-cache entry and subscribes the connection.
    wait_until(|| shared.has_state(&board_key()) && board_subscribed(&shared)).await;

    // A background publish (the `update_shared_if` pattern) now reaches this
    // connection through the mid-pass subscription, injecting the real copy.
    shared.update_shared_if::<Board>(|b| {
        b.label = "real".to_string();
        b.n = 42;
        Some(ChangeSet::all())
    });
    let frame = client.read_broadcast().await;
    assert_painted(&frame, "board:real:42");
}

// ── Review round 2, item 3: measured cost of the cascade walking the whole
// ── subtree of a re-rendered region regardless of the children's dep masks.

#[derive(State, Default)]
#[storage(memory)]
struct Deep {
    top: u32,
    leaf: u32,
}

/// A chain of `depth` nested regions, all over `Deep`. The outermost region's
/// mask covers only `FIELD_TOP`; every nested one only `FIELD_LEAF` — a change
/// of `top` alone matches none of the nested masks, so the layer-1 pass would
/// skip all of them. Each renderer invocation is counted.
fn deep_chain(depth: usize, counter: &Arc<AtomicUsize>, root: bool) -> ElementBuilder {
    if depth == 0 {
        return el(El::Span).text("bottom");
    }
    let counter = Arc::clone(counter);
    let deps = if root {
        RendererDeps::from_fields(&[Deep::FIELD_TOP])
    } else {
        RendererDeps::from_fields(&[Deep::FIELD_LEAF])
    };
    ElementBuilder::synced_with_deps::<Deep, _>(
        move |d| {
            // Read the state (kept out of the output, so every level's hash is
            // constant and the pass emits nothing — pure cascade CPU cost).
            let _ = (d.top, d.leaf);
            counter.fetch_add(1, Ordering::SeqCst);
            el(El::Div)
                .text("lvl")
                .append([deep_chain(depth - 1, &counter, false)])
        },
        deps,
    )
}

/// The cascade now descends into a re-rendered region's whole subtree even
/// when each child's own output is unchanged AND its dep mask excludes the
/// change: matching is by (parent, state type, identity), not by deps. The
/// cost, measured here: a depth-12 chain whose root's mask matches a
/// `FIELD_TOP`-only change re-renders all 12 levels — 12 renderer invocations
/// per pass (the 11 nested levels have `FIELD_LEAF`-only masks, which alone
/// would have skipped every one of them).
#[test]
fn the_cascade_walks_the_whole_subtree_regardless_of_child_masks() {
    const DEPTH: usize = 12;

    let renders = Arc::new(AtomicUsize::new(0));
    let tree = deep_chain(DEPTH, &renders, true);

    type States<'a> = HashMap<TypeId, &'a (dyn Any + Send + Sync)>;

    let initial: Box<dyn Any + Send + Sync> = Box::new(Deep { top: 0, leaf: 0 });
    let mut states: States = HashMap::new();
    states.insert(TypeId::of::<Deep>(), initial.as_ref());
    let mut ctx = rwire::builder::BuildContext::new();
    ctx.collect_symbols_multi(&tree, &states);
    ctx.emit_multi(&tree);
    let synced = ctx.take_synced_elements();
    let mut hashes = ctx.take_synced_hashes();
    let mut known = ctx.take_symbol_map();
    assert_eq!(synced.len(), DEPTH, "a chain of DEPTH nested regions");

    // Update pass: only `top` changed; the changed type is `Deep`.
    renders.store(0, Ordering::SeqCst);
    let updated: Box<dyn Any + Send + Sync> = Box::new(Deep { top: 1, leaf: 0 });
    let mut states2: States = HashMap::new();
    states2.insert(TypeId::of::<Deep>(), updated.as_ref());
    let mut handlers: HashMap<u32, HandlerFn> = HashMap::new();
    let mut discovered: Vec<rwire::builder::SyncedElement> = Vec::new();
    let mut rerendered: Vec<u32> = Vec::new();
    let _bytes = rwire::builder::build_synced_update_with_known_symbols(
        &synced,
        &states2,
        &mut handlers,
        ChangeSet::from_fields(&[Deep::FIELD_TOP]),
        Some(&mut known),
        Some(TypeId::of::<Deep>()),
        Some(&mut hashes),
        None,
        None,
        Some(&mut discovered),
        Some(&mut rerendered),
        0,
        None,
    );
    let count = renders.load(Ordering::SeqCst);
    assert_eq!(
        count, DEPTH,
        "measured: the cascade re-renders all {DEPTH} levels of the subtree (one renderer \
         invocation per level) even though the 11 nested levels' masks (FIELD_LEAF) exclude \
         the FIELD_TOP change — layer-1 alone would have rendered only the root"
    );
}

// ── Review round 3 (MR !2), item 2: a MULTI-STATE region discovered mid-pass
// ── must seed/cache EVERY read (region.reads()), not just its primary read.

static NEXT_COUNT: AtomicU64 = AtomicU64::new(7);

/// The SECONDARY memory state of `sec_chip`, with a non-default Default: a
/// counter starting at 7, minted per default. Without reconcile_nested_regions
/// seeding the read, each update pass renders the region from a fresh
/// throwaway default and the counter climbs between passes; with seeding it
/// renders the connection's copy (stable).
#[derive(State)]
#[storage(memory)]
struct SecCounter {
    count: u64,
}

impl Default for SecCounter {
    fn default() -> Self {
        Self {
            count: NEXT_COUNT.fetch_add(1, Ordering::SeqCst),
        }
    }
}

#[derive(State, Default)]
#[storage(memory)]
struct SecGate {
    n: u32,
    show: bool,
}

#[handler]
fn reveal_sec_gate(g: &mut SecGate) {
    g.show = true;
}

#[handler]
fn bump_sec_gate(g: &mut SecGate) {
    g.n += 1;
}

/// A multi-state region first discovered mid-pass: SecGate (primary) plus the
/// SECONDARY memory state SecCounter (no handler, no region at first render).
#[renderer]
fn sec_chip(g: &SecGate, s: &SecCounter) -> ElementBuilder {
    let n = g.n;
    let count = s.count;
    el(El::Span).text(&format!("chip:{n}:{count}"))
}

#[renderer]
fn sec_gate_shell(g: &SecGate) -> ElementBuilder {
    let mut kids = vec![
        el(El::Button).text("bump").on(Ev::Click, bump_sec_gate()),
        el(El::Button)
            .text("reveal")
            .on(Ev::Click, reveal_sec_gate()),
    ];
    if g.show {
        kids.push(sec_chip());
    }
    el(El::Div).append(kids)
}

/// The secondary MEMORY read of a mid-pass-discovered multi-state region is
/// seeded into the connection's state map: after the discovering pass, later
/// passes render it from the connection's copy, not from a fresh default per
/// pass (observable via the per-default-distinct counter starting at 7).
#[tokio::test]
async fn a_mid_pass_discovered_multi_state_region_seeds_its_secondary_memory_read() {
    let _ = start_with(19207, sec_gate_shell).await;
    let mut client = connect(19207).await;
    let initial = client.read_initial().await;
    assert_lacks(&initial, "chip:");

    // The chip region appears only during this update pass; its secondary
    // memory state was never seeded at connect.
    // The chip region appears only during this update pass; its secondary
    // memory state was never seeded at connect, so the discovering pass itself
    // renders a throwaway default (the span fill lands on the next pass).
    client.click(reveal_sec_gate()).await;
    let _answer = client.read_answer().await;
    assert_saw(&client, "chip:0:"); // the region was discovered and rendered

    // Two outer bumps re-render the region (SecGate is one of its reads). With
    // the secondary read SEEDED into the connection's state map, both passes
    // render the SAME string (interned in the render pass, emitted in the emit
    // pass) — the span is really painted, with the seeded default's counter.
    // Without the seeding every pass mints a fresh throwaway default, the
    // emitted text was never interned, and the span stays EMPTY.
    client.click(bump_sec_gate()).await;
    let a1 = client.read_answer().await;
    client.click(bump_sec_gate()).await;
    let a2 = client.read_answer().await;

    let c1 = painted_with_prefix(&a1, "chip:");
    let c2 = painted_with_prefix(&a2, "chip:");
    assert_eq!(
        c1.len(),
        1,
        "answer 1 must paint the chip from the connection's seeded copy (empty span without the fix): {c1:?}"
    );
    assert_eq!(c2.len(), 1, "one chip text in answer 2: {c2:?}");
    let v1: u64 = c1[0].split(':').nth(2).unwrap().parse().unwrap();
    assert!(
        v1 >= 7,
        "the secondary read paints its non-default default (counter starts at 7): {v1}"
    );
    let n1 = c1[0].split(':').nth(1).unwrap();
    let n2 = c2[0].split(':').nth(1).unwrap();
    let v1 = c1[0].split(':').nth(2).unwrap();
    let v2 = c2[0].split(':').nth(2).unwrap();
    assert_ne!(
        n1, n2,
        "the region re-rendered with the bumped n: {c1:?} vs {c2:?}"
    );
    assert_eq!(
        v1, v2,
        "the secondary memory read must render from the connection's seeded copy, not a \
         fresh default per pass: {c1:?} vs {c2:?}"
    );
}

// ── The SECONDARY SHARED read of a mid-pass-discovered multi-state region gets
// ── a shared-cache entry + subscription (the region's primary is the door).

#[derive(State, Default)]
#[storage(memory)]
struct SharedDoor {
    show: bool,
}

#[handler]
fn open_shared_door(d: &mut SharedDoor) {
    d.show = true;
}

/// Multi-state region discovered mid-pass: SharedDoor (primary) + Board
/// (SECONDARY shared read — no region has Board as its primary type).
#[renderer]
fn shared_board_chip(d: &SharedDoor, b: &Board) -> ElementBuilder {
    let show = d.show;
    el(El::Span).text(&format!("boardchip:{show}:{}:{}", b.label, b.n))
}

#[renderer]
fn shared_door_shell(d: &SharedDoor) -> ElementBuilder {
    let mut kids = vec![el(El::Button)
        .text("open")
        .on(Ev::Click, open_shared_door())];
    if d.show {
        kids.push(shared_board_chip());
    }
    el(El::Div).append(kids)
}

/// The discovering pass renders the secondary shared read's default, then must
/// create the shared-cache entry (via the read's default_state_for, not the
/// primary-only lookup) and subscribe the connection, so a later background
/// publish reaches it with the real copy.
#[tokio::test]
async fn a_mid_pass_discovered_multi_state_region_caches_and_subscribes_its_secondary_shared_read()
{
    let shared = start_with(19208, shared_door_shell).await;
    let mut client = connect(19208).await;
    let initial = client.read_initial().await;
    assert_lacks(&initial, "boardchip:");
    assert!(
        !shared.has_state(&board_key()),
        "no region reads Board at first render, so no cache entry exists yet"
    );

    client.click(open_shared_door()).await;
    let answer = client.read_answer().await;
    assert_painted(&answer, "boardchip:true:default:0");
    // The pass must create the cache entry for the SECONDARY shared read and
    // subscribe the connection.
    wait_until(|| shared.has_state(&board_key()) && board_subscribed(&shared)).await;

    // A background publish now reaches this connection with the real copy.
    shared.update_shared_if::<Board>(|b| {
        b.label = "real".to_string();
        b.n = 42;
        Some(ChangeSet::all())
    });
    let frame = client.read_broadcast().await;
    assert_painted(&frame, "boardchip:true:real:42");
}

// ── The SECONDARY PERSISTED read of a mid-pass-discovered multi-state region
// ── gets a cache entry too (the mid-pass entry creation used to cover only
// ── "__shared__:" keys, not "{table}:{session}" persisted keys).

/// Persisted state read only as a SECONDARY read of the mid-pass region.
#[derive(State, Default)]
#[storage(persisted, table = "note")]
struct Note {
    text: String,
}

#[derive(State, Default)]
#[storage(memory)]
struct PersistDoor {
    show: bool,
}

#[handler]
fn open_persist_door(d: &mut PersistDoor) {
    d.show = true;
}

/// Multi-state region discovered mid-pass: PersistDoor (primary) + Note
/// (SECONDARY persisted read).
#[renderer]
fn persist_note_chip(d: &PersistDoor, note: &Note) -> ElementBuilder {
    let show = d.show;
    el(El::Span).text(&format!("notechip:{show}:{}", note.text))
}

#[renderer]
fn persist_door_shell(d: &PersistDoor) -> ElementBuilder {
    let mut kids = vec![el(El::Button)
        .text("open")
        .on(Ev::Click, open_persist_door())];
    if d.show {
        kids.push(persist_note_chip());
    }
    el(El::Div).append(kids)
}

/// The persisted cache key ("{table}:{session}") of this server, if any.
fn note_cache_key(shared: &SharedServerState) -> Option<String> {
    shared
        .shared_cache
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .keys()
        .find(|k| k.starts_with(&format!("{}:", Note::TABLE_NAME)))
        .cloned()
}

fn note_subscribed(shared: &SharedServerState) -> bool {
    shared
        .subscriptions
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .any(|(k, v)| k.starts_with(&format!("{}:", Note::TABLE_NAME)) && !v.is_empty())
}

/// The discovering pass must create the persisted cache entry for the
/// secondary read (covering "{table}:{session}" keys, not only __shared__)
/// and subscribe, so a cross-tab persisted write injects the real copy.
#[tokio::test]
async fn a_mid_pass_discovered_multi_state_region_caches_its_secondary_persisted_read() {
    let shared = start_with(19209, persist_door_shell).await;
    let mut client = connect(19209).await;
    let initial = client.read_initial().await;
    assert_lacks(&initial, "notechip:");
    assert!(
        note_cache_key(&shared).is_none(),
        "no persisted cache entry at first render"
    );

    client.click(open_persist_door()).await;
    let answer = client.read_answer().await;
    assert_painted(&answer, "notechip:true:");
    wait_until(|| note_cache_key(&shared).is_some() && note_subscribed(&shared)).await;
    let key = note_cache_key(&shared).unwrap();

    // A cross-tab persisted write (same session key) reaches the region.
    shared.insert_state(
        key.clone(),
        Box::new(Note {
            text: "real".into(),
        }),
    );
    shared.broadcast(&key, TypeId::of::<Note>(), ChangeSet::all(), 0);
    let frame = client.read_broadcast().await;
    assert_painted(&frame, "notechip:true:real");
}
