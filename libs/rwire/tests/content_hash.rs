//! Render-dedup tests for event params in the content hash.
//!
//! A re-render that changes only a binding's params (e.g. a sentinel's cursor)
//! must be re-emitted — the client re-keys the binding (`__hk`) and installs a
//! fresh observer. An identical re-render must stay deduplicated (empty update).

use std::any::TypeId;
use std::collections::HashMap;

use rwire::builder::{
    build_synced_update_with_known_symbols, BuildContext, ElementBuilder, SyncedElement,
};
use rwire::protocol::opcodes::GET_SYNCED;
use rwire::state::{ChangeSet, RendererDeps};
use rwire::{el, El, Ev, HandlerFn, HandlerSpec, State, stable_handler_id};

#[derive(State, Default)]
#[storage(memory)]
struct SentinelState {
    cursor: u8,
}

fn noop(_s: &mut SentinelState) {}

fn handler() -> HandlerSpec {
    HandlerSpec::from_fn_with_changes::<SentinelState>(noop, ChangeSet::all())
        .with_handler_id(stable_handler_id("content_hash_tests", "noop"))
}

/// A region whose only varying content is the `Visible` binding's param.
fn sentinel_region() -> ElementBuilder {
    let h = handler();
    ElementBuilder::synced_with_deps::<SentinelState, _>(
        move |s| el(El::Div).on(Ev::Visible, h.clone().with_param_bytes(vec![s.cursor])),
        RendererDeps::always(),
    )
}

/// Initial render with `cursor`, then an update pass rendering `update_cursor`.
/// Returns the update bytes (empty when deduplicated).
fn render_then_update(cursor: u8, update_cursor: u8) -> Vec<u8> {
    let b1: Box<dyn std::any::Any + Send + Sync> = Box::new(SentinelState { cursor });
    let b2: Box<dyn std::any::Any + Send + Sync> =
        Box::new(SentinelState { cursor: update_cursor });

    let mut ctx = BuildContext::new();
    let mut states: HashMap<TypeId, &(dyn std::any::Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<SentinelState>(), b1.as_ref());

    let el = sentinel_region();
    ctx.collect_symbols_multi(&el, &states);
    ctx.emit_multi(&el);
    let synced = ctx.take_synced_elements();
    let mut hashes = ctx.take_synced_hashes();
    let mut known = ctx.take_symbol_map();
    assert_eq!(synced.len(), 1);

    let mut states2: HashMap<TypeId, &(dyn std::any::Any + Send + Sync)> = HashMap::new();
    states2.insert(TypeId::of::<SentinelState>(), b2.as_ref());
    let mut handlers: HashMap<u32, HandlerFn> = HashMap::new();
    let mut discovered: Vec<SyncedElement> = Vec::new();
    let bytes = build_synced_update_with_known_symbols(
        &synced,
        &states2,
        &mut handlers,
        ChangeSet::all(),
        Some(&mut known),
        None,
        Some(&mut hashes),
        None,
        None,
        Some(&mut discovered),
        None,
        0,
        None,
    );
    bytes.to_vec()
}

/// (a) Two identical renders emit a single update: the second pass is empty.
#[test]
fn identical_rerender_is_deduplicated() {
    let bytes = render_then_update(1, 1);
    assert!(
        bytes.is_empty(),
        "identical re-render must emit nothing, got {} bytes",
        bytes.len()
    );
}

/// (b) Two renders differing only in the binding param emit two updates.
#[test]
fn param_only_change_is_reemitted() {
    let bytes = render_then_update(1, 2);
    assert!(
        !bytes.is_empty(),
        "a re-render changing only the binding param must be re-emitted"
    );
    assert!(
        bytes.contains(&GET_SYNCED),
        "the region's own update (GET_SYNCED + rebuild) must be present"
    );
}
