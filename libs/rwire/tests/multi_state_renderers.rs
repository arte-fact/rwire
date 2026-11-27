//! Multi-state renderers: a synced region that reads several states (and chosen
//! fields of each), so a view can combine e.g. the per-connection route with a
//! shared fleet without nesting regions. See `docs/multi-state-renderers.md`.
//!
//! Covered here (the study's test plan 4.1/4.2):
//! - a two-state region re-renders when EITHER state changes, and an unrelated
//!   type does not touch it;
//! - a change to an untouched field of one state does NOT re-render (per-read
//!   masks), while a touched field does;
//! - a missing second state renders nothing (None, as a missing single state);
//! - both shared keys of a two-shared-state region are subscribed: a broadcast
//!   of either reaches the region;
//! - discovery seeds an unseen second type, so the first frame paints both;
//! - SSR paints both states (CurrentRoute + a shared state);
//! - `region::<S>` / `region_with_deps` alias a storage-aware region.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use rwire::builder::{build_synced_update_with_known_symbols, BuildContext, SyncedElement};
use rwire::protocol::opcodes::GET_SYNCED;
use rwire::{
    el, outlet, renderer, ChangeSet, CurrentRoute, El, ElementBuilder, Router, Server,
    SharedServerState, State,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;

// ============================================================================
// Update filter (unit)
// ============================================================================

#[derive(State, Default, Debug)]
#[storage(memory)]
struct Alpha {
    x: u32,
    z: u32,
}

#[derive(State, Default)]
#[storage(memory)]
struct Beta {
    y: u32,
    w: u32,
}

#[derive(State, Default)]
#[storage(memory)]
struct Unrelated {
    v: u32,
}

/// Reads `a.x` and `b.y` only: per-parameter masks are FIELD_X and FIELD_Y.
#[renderer]
fn panel(a: &Alpha, b: &Beta) -> ElementBuilder {
    let x = a.x;
    let y = b.y;
    el(El::P).text(&format!("{}|{}", x, y))
}

fn has_get_synced_for_id(bytes: &[u8], synced_id: u32) -> bool {
    for i in 0..bytes.len() {
        if bytes[i] == GET_SYNCED
            && synced_id < 0x80
            && i + 1 < bytes.len()
            && bytes[i + 1] == synced_id as u8
        {
            return true;
        }
    }
    false
}

fn collect_regions(
    root: &ElementBuilder,
    states: &HashMap<TypeId, &(dyn Any + Send + Sync)>,
) -> Vec<SyncedElement> {
    let mut ctx = BuildContext::new();
    ctx.collect_symbols_multi(root, states);
    ctx.emit_multi(root);
    ctx.take_synced_elements()
}

fn update_for(
    synced: &[SyncedElement],
    states: &HashMap<TypeId, &(dyn Any + Send + Sync)>,
    changed: Option<TypeId>,
    changes: ChangeSet,
) -> Vec<u8> {
    let mut handlers: HashMap<u32, rwire::HandlerFn> = HashMap::new();
    let bytes = build_synced_update_with_known_symbols(
        synced,
        states,
        &mut handlers,
        changes,
        None,
        changed,
        None,
        None,
        None,
        None,
        None,
        0,
        None,
    );
    bytes.as_ref().to_vec()
}

fn alpha_beta_states<'a>(
    alpha: &'a Alpha,
    beta: &'a Beta,
) -> HashMap<TypeId, &'a (dyn Any + Send + Sync)> {
    let mut states: HashMap<TypeId, &(dyn Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<Alpha>(), alpha);
    states.insert(TypeId::of::<Beta>(), beta);
    states
}

#[test]
fn a_two_state_region_rerenders_when_either_state_changes() {
    let alpha = Alpha { x: 1, z: 0 };
    let beta = Beta { y: 2, w: 0 };
    let states = alpha_beta_states(&alpha, &beta);
    let root = el(El::Div).append([panel()]);
    let synced = collect_regions(&root, &states);
    assert_eq!(synced.len(), 1, "one region for the two-state renderer");

    // The region carries two reads; the first (Alpha) is the primary type used
    // for nested matching and the single-state accessors.
    let reads = synced[0].reads();
    assert_eq!(reads.len(), 2);
    assert_eq!(reads[0].type_id, TypeId::of::<Alpha>());
    assert_eq!(reads[1].type_id, TypeId::of::<Beta>());
    assert_eq!(synced[0].state_type_id(), TypeId::of::<Alpha>());

    // A change of either state re-renders the region once.
    let bytes = update_for(
        &synced,
        &states,
        Some(TypeId::of::<Alpha>()),
        ChangeSet::all(),
    );
    assert!(
        has_get_synced_for_id(&bytes, 0),
        "an Alpha change must re-render the region"
    );
    let bytes = update_for(
        &synced,
        &states,
        Some(TypeId::of::<Beta>()),
        ChangeSet::all(),
    );
    assert!(
        has_get_synced_for_id(&bytes, 0),
        "a Beta change must re-render the region"
    );

    // An unrelated type does not.
    let unrelated = Unrelated { v: 9 };
    assert_eq!(unrelated.v, 9);
    let bytes = update_for(
        &synced,
        &states,
        Some(TypeId::of::<Unrelated>()),
        ChangeSet::all(),
    );
    assert!(bytes.is_empty(), "an unrelated type must not re-render it");
}

#[test]
fn an_untouched_field_of_one_state_does_not_rerender() {
    let alpha = Alpha { x: 1, z: 0 };
    let beta = Beta { y: 2, w: 0 };
    assert_eq!((alpha.z, beta.w), (0, 0), "the untouched fields");
    let states = alpha_beta_states(&alpha, &beta);
    let root = el(El::Div).append([panel()]);
    let synced = collect_regions(&root, &states);

    // Per-read masks: the renderer reads only Alpha.x and Beta.y.
    let reads = synced[0].reads();
    assert_eq!(
        reads[0].deps.mask,
        1u64 << Alpha::FIELD_X,
        "Alpha read mask"
    );
    assert_eq!(reads[1].deps.mask, 1u64 << Beta::FIELD_Y, "Beta read mask");

    // Alpha.z changed (Alpha is the changed type, but its read mask excludes z).
    let bytes = update_for(
        &synced,
        &states,
        Some(TypeId::of::<Alpha>()),
        ChangeSet::from_fields(&[Alpha::FIELD_Z]),
    );
    assert!(
        bytes.is_empty(),
        "an untouched field of Alpha must not re-render the region"
    );
    // Beta.w changed: same for Beta.
    let bytes = update_for(
        &synced,
        &states,
        Some(TypeId::of::<Beta>()),
        ChangeSet::from_fields(&[Beta::FIELD_W]),
    );
    assert!(
        bytes.is_empty(),
        "an untouched field of Beta must not re-render the region"
    );

    // The touched fields do.
    let bytes = update_for(
        &synced,
        &states,
        Some(TypeId::of::<Alpha>()),
        ChangeSet::from_fields(&[Alpha::FIELD_X]),
    );
    assert!(has_get_synced_for_id(&bytes, 0), "Alpha.x must re-render");
    let bytes = update_for(
        &synced,
        &states,
        Some(TypeId::of::<Beta>()),
        ChangeSet::from_fields(&[Beta::FIELD_Y]),
    );
    assert!(has_get_synced_for_id(&bytes, 0), "Beta.y must re-render");
}

#[test]
fn a_read_missing_from_the_pass_map_falls_back_to_its_default() {
    let alpha = Alpha { x: 1, z: 0 };
    let mut states: HashMap<TypeId, &(dyn Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<Alpha>(), &alpha);
    // Beta deliberately absent from the pass's map: like a missing single state
    // (which renders from a fresh default), the missing read resolves to
    // Beta::default() and the region renders.
    let root = el(El::Div).append([panel()]);
    let synced = collect_regions(&root, &states);
    assert_eq!(synced.len(), 1, "the region is registered even unrendered");

    let bytes = update_for(
        &synced,
        &states,
        Some(TypeId::of::<Alpha>()),
        ChangeSet::all(),
    );
    assert!(
        has_get_synced_for_id(&bytes, 0),
        "a missing read renders from its default, like a missing single state"
    );

    // The renderer itself returns None when a read cannot be resolved at all:
    // a lookup that answers None for every type leaves the render unresolved.
    let renderers = rwire::builder::extract_renderers(&root);
    let lookup = |_type_id: TypeId| -> Option<&dyn Any> { None };
    assert!(
        renderers[0].render_with_states(&lookup).is_none(),
        "an unresolvable read yields None from render_with_states"
    );
}

#[test]
fn a_single_state_renderer_keeps_its_one_read() {
    #[renderer]
    fn solo(a: &Alpha) -> ElementBuilder {
        el(El::P).text(&a.x.to_string())
    }
    let alpha = Alpha { x: 5, z: 0 };
    let mut states: HashMap<TypeId, &(dyn Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<Alpha>(), &alpha);
    let root = el(El::Div).append([solo()]);
    let synced = collect_regions(&root, &states);
    let reads = synced[0].reads();
    assert_eq!(
        reads.len(),
        1,
        "the single-state case is a one-element list"
    );
    assert_eq!(reads[0].type_id, TypeId::of::<Alpha>());
    assert_eq!(reads[0].deps.mask, 1u64 << Alpha::FIELD_X);
}

// ============================================================================
// region::<S> alias
// ============================================================================

#[derive(State, Default)]
#[storage(shared)]
struct Aliased {
    note: String,
}

#[test]
fn region_alias_carries_storage_and_table_like_the_long_form() {
    let root = el(El::Div).append([ElementBuilder::region::<Aliased>(|a| {
        el(El::P).text(&a.note)
    })]);
    let mut states: HashMap<TypeId, &(dyn Any + Send + Sync)> = HashMap::new();
    let aliased = Aliased { note: "hi".into() };
    states.insert(TypeId::of::<Aliased>(), &aliased);
    let synced = collect_regions(&root, &states);
    let reads = synced[0].reads();
    assert_eq!(reads.len(), 1);
    assert_eq!(reads[0].storage, rwire::StorageType::Shared);
    assert_eq!(reads[0].table, Some("Aliased"));
    assert!(reads[0].deps.always, "region() uses RendererDeps::always()");

    let root = el(El::Div).append([ElementBuilder::region_with_deps::<Aliased>(
        |a| el(El::P).text(&a.note),
        rwire::RendererDeps::from_fields(&[Aliased::FIELD_NOTE]),
    )]);
    let synced = collect_regions(&root, &states);
    assert_eq!(synced[0].reads()[0].deps.mask, 1u64 << Aliased::FIELD_NOTE);
}

// ============================================================================
// Live server: discovery, subscriptions, either-state broadcasts, SSR
// ============================================================================

/// What the hub publishes: one status line per session, plus a clock field the
/// panel does not read.
#[derive(State, Default)]
#[storage(shared)]
struct Fleet {
    sessions: Vec<(String, String)>,
    clock: u64,
}

/// A second shared state, to prove both keys of a two-shared-read region are
/// subscribed.
#[derive(State, Default)]
#[storage(shared)]
struct Presence {
    online: u32,
}

/// The trial's case: the selected session is in the tab's route (per
/// connection), the fleet is shared — one region reading both.
#[renderer]
fn session_panel(route: &CurrentRoute, fleet: &Fleet) -> ElementBuilder {
    let selected = route.param("id").unwrap_or_default();
    let status = fleet
        .sessions
        .iter()
        .find(|(id, _)| *id == selected)
        .map(|(_, status)| status.clone())
        .unwrap_or_else(|| "unknown".to_string());
    el(El::P).text(&format!("panel {selected}: {status}"))
}

/// A region over two shared states.
#[renderer]
fn duo(fleet: &Fleet, presence: &Presence) -> ElementBuilder {
    el(El::P).text(&format!(
        "duo: {} sessions, {} online",
        fleet.sessions.len(),
        presence.online
    ))
}

fn shell() -> ElementBuilder {
    el(El::Main).append([outlet()])
}

fn router() -> Router {
    Router::new()
        .page("/", |_| el(El::P).text("home"))
        .page("/s/:id", |_| session_panel())
        .page("/duo", |_| duo())
        .page("/alias", |_| {
            ElementBuilder::region::<Fleet>(|fleet| {
                el(El::P).text(&format!("alias: {} sessions", fleet.sessions.len()))
            })
        })
}

async fn start(port: u16, ssr: bool) -> Arc<SharedServerState> {
    let mut server = Server::bind(&format!("127.0.0.1:{port}"))
        .unwrap()
        .root(shell)
        .routes(router());
    if ssr {
        server = server.capsule_config(rwire::CapsuleConfig::new().ssr(true));
    }
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

async fn tab(port: u16, path: &str) -> Ws {
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}/"))
        .await
        .unwrap();
    ws.send(Message::Text(format!("R{path}"))).await.unwrap();
    ws
}

fn contains(data: &[u8], needle: &str) -> bool {
    data.windows(needle.len()).any(|w| w == needle.as_bytes())
}

/// Read frames until one contains `needle`; fail on `forbidden` or after 5 s.
async fn expect(ws: &mut Ws, needle: &str, forbidden: &[&str]) {
    let found = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(Ok(message)) = ws.next().await {
            if let Message::Binary(data) = message {
                for bad in forbidden {
                    assert!(
                        !contains(&data, bad),
                        "a frame painted {bad:?} while waiting for {needle:?}"
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
    assert_eq!(found, Ok(true), "{needle:?} never reached the tab");
}

/// `GET path` and the response up to the runtime `<script>`, so a failure
/// prints the page, not 40 KB of JS.
async fn get(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    stream
        .write_all(format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes())
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

/// Discovery seeds the unseen second type (the shared `Fleet` has no handler
/// anywhere), so the first frame paints BOTH states; a publish of the second
/// state reaches the region; a route change (the primary read) repaints it.
#[tokio::test]
async fn a_two_state_region_reads_its_route_and_the_shared_fleet() {
    let shared = start(19301, false).await;
    let mut tab_a = tab(19301, "/s/a").await;
    let mut tab_b = tab(19301, "/s/b").await;
    // Fleet is unpublished: the region still renders (discovery seeded its
    // default), painting the route selection.
    expect(&mut tab_a, "panel a: unknown", &[]).await;
    expect(&mut tab_b, "panel b: unknown", &[]).await;

    // A publish touching only the field the panel does NOT read (the clock)
    // leaves the region alone: the per-read mask excludes it.
    shared.update_shared_changed::<Fleet>(ChangeSet::from_fields(&[Fleet::FIELD_CLOCK]), |fleet| {
        fleet.clock += 1;
    });

    // A publish of the SECOND read repaints the region (its key is subscribed).
    shared.update_shared_changed::<Fleet>(ChangeSet::all(), |fleet| {
        fleet.sessions = vec![("a".into(), "idle".into()), ("b".into(), "idle".into())];
    });
    expect(&mut tab_a, "panel a: idle", &["panel b"]).await;
    expect(&mut tab_b, "panel b: idle", &["panel a"]).await;

    // A change of the PRIMARY read (route) repaints it too.
    tab_a.send(Message::Text("R/s/b".into())).await.unwrap();
    expect(&mut tab_a, "panel b: idle", &[]).await;

    // And the next publish of the second read follows the new selection.
    shared.update_shared_changed::<Fleet>(ChangeSet::all(), |fleet| {
        fleet.sessions[1].1 = "done".into();
    });
    expect(&mut tab_a, "panel b: done", &["panel a"]).await;
    expect(&mut tab_b, "panel b: done", &[]).await;
}

/// A region reading two SHARED states subscribes to both keys: a broadcast of
/// either reaches it.
#[tokio::test]
async fn both_shared_keys_of_a_two_state_region_are_subscribed() {
    let shared = start(19302, false).await;
    shared.update_shared_changed::<Fleet>(ChangeSet::all(), |fleet| {
        fleet.sessions = vec![("a".into(), "idle".into())];
    });
    shared.update_shared_changed::<Presence>(ChangeSet::all(), |p| p.online = 3);

    let mut ws = tab(19302, "/duo").await;
    expect(&mut ws, "duo: 1 sessions, 3 online", &[]).await;

    // Broadcast of the first shared read.
    shared.update_shared_changed::<Fleet>(ChangeSet::all(), |fleet| {
        fleet.sessions.push(("b".into(), "idle".into()));
    });
    expect(&mut ws, "duo: 2 sessions, 3 online", &[]).await;

    // Broadcast of the second shared read.
    shared.update_shared_changed::<Presence>(ChangeSet::all(), |p| p.online = 7);
    expect(&mut ws, "duo: 2 sessions, 7 online", &[]).await;
}

/// The `region::<S>` alias's region is subscribed like the long form.
#[tokio::test]
async fn the_region_alias_is_subscribed_like_synced_with_storage() {
    let shared = start(19303, false).await;
    let mut ws = tab(19303, "/alias").await;
    expect(&mut ws, "alias: 0 sessions", &[]).await;
    shared.update_shared_changed::<Fleet>(ChangeSet::all(), |fleet| {
        fleet.sessions = vec![("a".into(), "idle".into())];
    });
    expect(&mut ws, "alias: 1 sessions", &[]).await;
}

/// SSR paints both states of a multi-state region: `CurrentRoute` resolves to
/// the requested route, the shared read to its cache copy.
#[tokio::test]
async fn ssr_paints_the_route_and_the_shared_state_of_a_two_state_region() {
    let shared = start(19304, true).await;
    shared.update_shared_if::<Fleet>(|fleet| {
        fleet.sessions = vec![("a".into(), "live".into()), ("b".into(), "live".into())];
        Some(ChangeSet::all())
    });

    let page = get(19304, "/s/a").await;
    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert!(
        page.contains("panel a: live"),
        "the SSR paint is missing the two-state region:\n{page}"
    );
    assert!(!page.contains("panel b"), "served the wrong route:\n{page}");

    let page = get(19304, "/s/b").await;
    assert!(
        page.contains("panel b: live"),
        "the SSR paint is missing the two-state region:\n{page}"
    );
}

// ============================================================================
// The front's worst case (perf study limit 1): ONE region reading FIVE states
// ============================================================================

/// The web front's agent window reads five states: the fleet, the hub's
/// answers, the action blocks, the thread, and the per-connection `Ui`. As one
/// region, with a per-state field mask each.
#[derive(State, Default)]
#[storage(shared)]
struct WinFleet {
    tick: u64,
    machines: u32,
}

#[derive(State, Default)]
#[storage(shared)]
struct WinAnswers {
    pending: u32,
    other: u32,
}

#[derive(State, Default)]
#[storage(shared)]
struct WinBlocks {
    count: u32,
    other: u32,
}

#[derive(State, Default)]
#[storage(shared)]
struct WinThread {
    fragments: u32,
    other: u32,
}

#[derive(State, Default)]
#[storage(memory)]
struct WinUi {
    overlay: bool,
    draft: String,
}

/// Reads one field of each of the five states.
#[renderer]
fn agent_window(
    fleet: &WinFleet,
    answers: &WinAnswers,
    blocks: &WinBlocks,
    thread: &WinThread,
    ui: &WinUi,
) -> ElementBuilder {
    let tick = fleet.tick;
    let pending = answers.pending;
    let count = blocks.count;
    let fragments = thread.fragments;
    let overlay = ui.overlay;
    el(El::Section).text(&format!("{tick}|{pending}|{count}|{fragments}|{overlay}"))
}

#[test]
fn one_region_can_read_five_states_with_per_state_masks() {
    let fleet = WinFleet {
        tick: 1,
        machines: 2,
    };
    let answers = WinAnswers {
        pending: 3,
        other: 0,
    };
    let blocks = WinBlocks { count: 4, other: 0 };
    let thread = WinThread {
        fragments: 5,
        other: 0,
    };
    let ui = WinUi {
        overlay: true,
        draft: String::new(),
    };
    let mut states: HashMap<TypeId, &(dyn Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<WinFleet>(), &fleet);
    states.insert(TypeId::of::<WinAnswers>(), &answers);
    states.insert(TypeId::of::<WinBlocks>(), &blocks);
    states.insert(TypeId::of::<WinThread>(), &thread);
    states.insert(TypeId::of::<WinUi>(), &ui);

    // The untouched fields exist to prove the per-read masks exclude them.
    assert_eq!(
        (
            fleet.machines,
            answers.other,
            blocks.other,
            thread.other,
            ui.draft.len()
        ),
        (2, 0, 0, 0, 0)
    );

    let root = el(El::Div).append([agent_window()]);
    let synced = collect_regions(&root, &states);
    assert_eq!(synced.len(), 1, "five states, ONE region");

    // Five reads, in parameter order; the first (fleet) is the primary type.
    let reads = synced[0].reads();
    assert_eq!(reads.len(), 5);
    assert_eq!(
        reads.iter().map(|r| r.type_id).collect::<Vec<_>>(),
        vec![
            TypeId::of::<WinFleet>(),
            TypeId::of::<WinAnswers>(),
            TypeId::of::<WinBlocks>(),
            TypeId::of::<WinThread>(),
            TypeId::of::<WinUi>(),
        ]
    );
    assert_eq!(synced[0].state_type_id(), TypeId::of::<WinFleet>());

    // Per-state field masks: only the read field of each state.
    assert_eq!(reads[0].deps.mask, 1u64 << WinFleet::FIELD_TICK);
    assert_eq!(reads[1].deps.mask, 1u64 << WinAnswers::FIELD_PENDING);
    assert_eq!(reads[2].deps.mask, 1u64 << WinBlocks::FIELD_COUNT);
    assert_eq!(reads[3].deps.mask, 1u64 << WinThread::FIELD_FRAGMENTS);
    assert_eq!(reads[4].deps.mask, 1u64 << WinUi::FIELD_OVERLAY);

    // A change to a READ field of ANY of the five states re-renders the region.
    for (tid, field) in [
        (TypeId::of::<WinFleet>(), WinFleet::FIELD_TICK),
        (TypeId::of::<WinAnswers>(), WinAnswers::FIELD_PENDING),
        (TypeId::of::<WinBlocks>(), WinBlocks::FIELD_COUNT),
        (TypeId::of::<WinThread>(), WinThread::FIELD_FRAGMENTS),
        (TypeId::of::<WinUi>(), WinUi::FIELD_OVERLAY),
    ] {
        let bytes = update_for(
            &synced,
            &states,
            Some(tid),
            ChangeSet::from_fields(&[field]),
        );
        assert!(
            has_get_synced_for_id(&bytes, 0),
            "a change to a read field of {tid:?} must re-render the five-state region"
        );
    }

    // A change to an UNTOUCHED field of any state does not.
    for (tid, field) in [
        (TypeId::of::<WinFleet>(), WinFleet::FIELD_MACHINES),
        (TypeId::of::<WinAnswers>(), WinAnswers::FIELD_OTHER),
        (TypeId::of::<WinBlocks>(), WinBlocks::FIELD_OTHER),
        (TypeId::of::<WinThread>(), WinThread::FIELD_OTHER),
        (TypeId::of::<WinUi>(), WinUi::FIELD_DRAFT),
    ] {
        let bytes = update_for(
            &synced,
            &states,
            Some(tid),
            ChangeSet::from_fields(&[field]),
        );
        assert!(
            bytes.is_empty(),
            "an untouched field of {tid:?} must not re-render the five-state region"
        );
    }
}

// ============================================================================
// Limit 5: an UNREAD renderer parameter contributes an empty dependency set
// ============================================================================

/// `b` is never read: its read carries an empty mask, so Beta's changes do not
/// re-render the region.
#[renderer]
fn half_read(a: &Alpha, _b: &Beta) -> ElementBuilder {
    let x = a.x;
    el(El::P).text(&x.to_string())
}

/// All parameters unread: the region renders once and only re-renders on
/// whole-change passes (ChangeSet::all(), e.g. a route swap).
#[renderer]
fn fully_unread(_a: &Alpha, _b: &Beta) -> ElementBuilder {
    el(El::P).text("static content")
}

/// Used only through a macro the analysis cannot see into: still always().
#[renderer]
fn macro_hidden(a: &Alpha) -> ElementBuilder {
    el(El::Pre).text(&format!("{a:?}"))
}

#[test]
fn an_unread_parameter_has_an_empty_mask_for_its_state() {
    let alpha = Alpha { x: 1, z: 0 };
    let beta = Beta { y: 2, w: 0 };
    let states = alpha_beta_states(&alpha, &beta);
    let root = el(El::Div).append([half_read()]);
    let synced = collect_regions(&root, &states);

    let reads = synced[0].reads();
    assert_eq!(reads.len(), 2);
    assert_eq!(
        reads[0].deps.mask,
        1u64 << Alpha::FIELD_X,
        "read param: mask"
    );
    assert_eq!(reads[1].deps.mask, 0, "unread param: empty mask");
    assert!(!reads[1].deps.always, "unread param is not always()");

    // Beta changed (any field): no re-render.
    for field in [Beta::FIELD_Y, Beta::FIELD_W] {
        let bytes = update_for(
            &synced,
            &states,
            Some(TypeId::of::<Beta>()),
            ChangeSet::from_fields(&[field]),
        );
        assert!(
            bytes.is_empty(),
            "an unread parameter's state must not re-render the region"
        );
    }
    // Alpha.x changed: re-render.
    let bytes = update_for(
        &synced,
        &states,
        Some(TypeId::of::<Alpha>()),
        ChangeSet::from_fields(&[Alpha::FIELD_X]),
    );
    assert!(has_get_synced_for_id(&bytes, 0));
}

#[test]
fn an_all_unread_renderer_renders_once_and_only_on_all_changed() {
    let alpha = Alpha { x: 1, z: 0 };
    let beta = Beta { y: 2, w: 0 };
    let states = alpha_beta_states(&alpha, &beta);
    let root = el(El::Div).append([fully_unread()]);
    let synced = collect_regions(&root, &states);
    assert_eq!(synced.len(), 1, "it still renders (once) at registration");
    for read in synced[0].reads() {
        assert_eq!(read.deps.mask, 0);
        assert!(!read.deps.always);
    }

    // Field-level changes of either state: no re-render.
    for (tid, field) in [
        (TypeId::of::<Alpha>(), Alpha::FIELD_X),
        (TypeId::of::<Beta>(), Beta::FIELD_Y),
    ] {
        let bytes = update_for(
            &synced,
            &states,
            Some(tid),
            ChangeSet::from_fields(&[field]),
        );
        assert!(
            bytes.is_empty(),
            "all-unread renderer ignores field changes"
        );
    }
    // A whole-change pass (route swap) still re-renders it.
    let bytes = update_for(
        &synced,
        &states,
        Some(TypeId::of::<Alpha>()),
        ChangeSet::all(),
    );
    assert!(
        has_get_synced_for_id(&bytes, 0),
        "ChangeSet::all() still reaches an all-unread region"
    );
}

#[test]
fn a_parameter_used_only_inside_a_macro_stays_always() {
    let alpha = Alpha { x: 1, z: 0 };
    let mut states: HashMap<TypeId, &(dyn Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<Alpha>(), &alpha);
    let root = el(El::Div).append([macro_hidden()]);
    let synced = collect_regions(&root, &states);
    assert!(
        synced[0].reads()[0].deps.always,
        "a use hidden inside format! stays always()"
    );
}

// ============================================================================
// Limit 9: an optional human-readable name on regions, surfaced in RWIRE_TRACE
// ============================================================================

#[renderer]
fn named_solo(a: &Alpha) -> ElementBuilder {
    el(El::P).text(&a.x.to_string())
}

#[test]
fn the_macro_names_the_region_after_the_function() {
    let alpha = Alpha { x: 1, z: 0 };
    let mut states: HashMap<TypeId, &(dyn Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<Alpha>(), &alpha);

    let root = el(El::Div).append([named_solo(), panel().with_name("custom.panel")]);
    let synced = collect_regions(&root, &states);
    assert_eq!(synced.len(), 2);
    // The single-state renderer is named after its fn.
    let names: Vec<Option<&str>> = synced.iter().map(|se| se.name()).collect();
    assert!(
        names.contains(&Some("named_solo")),
        "the #[renderer] fn name lands on the region: {names:?}"
    );
    assert!(
        names.contains(&Some("custom.panel")),
        "with_name overrides it: {names:?}"
    );

    // A closure region gets a name only via with_name.
    let root = el(El::Div).append([ElementBuilder::region::<Alpha>(|a| {
        el(El::P).text(&a.x.to_string())
    })]);
    let synced = collect_regions(&root, &states);
    assert_eq!(synced[0].name(), None, "no name unless asked");
    let root =
        el(El::Div).append([
            ElementBuilder::region::<Alpha>(|a| el(El::P).text(&a.x.to_string()))
                .with_name("closure region"),
        ]);
    let synced = collect_regions(&root, &states);
    assert_eq!(synced[0].name(), Some("closure region"));
}
