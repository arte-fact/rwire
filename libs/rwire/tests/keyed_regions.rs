//! Tests for keyed synced-region identity (`ElementBuilder::with_identity`).
//!
//! A parent region renders a windowed list of child regions. When a child is
//! prepended, children carrying an identity must keep their wire ids (matched
//! by identity, never positionally), so their live spans survive and their
//! content is not re-emitted. Identity-less children match positionally and
//! shift ids — the counter-test.

use std::any::TypeId;
use std::collections::HashMap;

use rwire::builder::{
    build_synced_update_with_known_symbols, BuildContext, ElementBuilder, SyncedElement,
};
use rwire::state::{ChangeSet, RendererDeps};
use rwire::{el, El, HandlerFn, MemoryState};

#[derive(Default)]
struct ListState {
    keys: Vec<u64>,
}

impl MemoryState for ListState {}

#[derive(Default)]
struct BlockState;

/// One child region of the list, optionally carrying `key` as its identity.
fn block_region(key: u64, keyed: bool) -> ElementBuilder {
    let region = ElementBuilder::synced_with_deps::<BlockState, _>(
        move |_| el(El::Div).class("block").text(&format!("block {key}")),
        RendererDeps::always(),
    );
    if keyed { region.with_identity(key) } else { region }
}

/// The parent region: a div holding one placeholder per key, oldest first.
fn list_region(keyed: bool) -> ElementBuilder {
    ElementBuilder::synced_with_deps::<ListState, _>(
        move |s| {
            el(El::Div)
                .class("list")
                .append(s.keys.iter().map(|&k| block_region(k, keyed)))
        },
        RendererDeps::always(),
    )
}

type States<'a> = HashMap<TypeId, &'a (dyn std::any::Any + Send + Sync)>;

struct Update {
    /// Wire ids of the parent's nested regions, in document order (the
    /// placeholder assignment of this re-render).
    discovered_child_ids: Vec<u32>,
    /// Regions actually re-emitted (GET_SYNCED + CLEAR_CHILDREN + rebuild).
    rerendered: Vec<u32>,
}

/// Initial render of `initial` keys, then an update pass with `updated` keys.
fn render_then_update(keyed: bool, initial: Vec<u64>, updated: Vec<u64>) -> Update {
    let n_initial = initial.len();
    let list1: Box<dyn std::any::Any + Send + Sync> = Box::new(ListState { keys: initial });
    let block1: Box<dyn std::any::Any + Send + Sync> = Box::new(BlockState);
    let list2: Box<dyn std::any::Any + Send + Sync> = Box::new(ListState { keys: updated });
    let block2: Box<dyn std::any::Any + Send + Sync> = Box::new(BlockState);

    let mut states: States = HashMap::new();
    states.insert(TypeId::of::<ListState>(), list1.as_ref());
    states.insert(TypeId::of::<BlockState>(), block1.as_ref());

    let mut ctx = BuildContext::new();
    let el = list_region(keyed);
    ctx.collect_symbols_multi(&el, &states);
    ctx.emit_multi(&el);
    let synced = ctx.take_synced_elements();
    let mut hashes = ctx.take_synced_hashes();
    let mut known = ctx.take_symbol_map();

    // Parent is id 0, children follow in document order.
    let parent = synced.iter().find(|s| s.parent().is_none()).unwrap();
    assert_eq!(parent.id, 0);
    let child_ids: Vec<u32> = synced
        .iter()
        .filter(|s| s.parent() == Some(0))
        .map(|s| s.id)
        .collect();
    assert_eq!(
        child_ids,
        (1..=n_initial as u32).collect::<Vec<_>>(),
        "initial children get ids 1..=n in document order"
    );

    let mut states2: States = HashMap::new();
    states2.insert(TypeId::of::<ListState>(), list2.as_ref());
    states2.insert(TypeId::of::<BlockState>(), block2.as_ref());
    let mut handlers: HashMap<u32, HandlerFn> = HashMap::new();
    let mut discovered: Vec<SyncedElement> = Vec::new();
    let mut rerendered: Vec<u32> = Vec::new();
    let _bytes = build_synced_update_with_known_symbols(
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
        Some(&mut rerendered),
        0,
        None,
    );

    // `discovered` = emit-pass regions in document order, then the `refreshed`
    // list (matched regions re-adopted by the render pass) — same ids again.
    // Keep the first occurrence of each id: that is the placeholder assignment.
    let mut seen = std::collections::HashSet::new();
    let discovered_child_ids: Vec<u32> = discovered
        .iter()
        .filter(|s| s.parent() == Some(0))
        .map(|s| s.id)
        .filter(|id| seen.insert(*id))
        .collect();

    Update {
        discovered_child_ids,
        rerendered,
    }
}

/// (c) Keyed: prepend a third region — the two existing keep their wire ids
/// (document order 5,10,20 → placeholders [new, 1, 2]) and their content is
/// NOT re-emitted (only the parent is in `rerendered`: zero CLEAR_CHILDREN on
/// live blocks).
#[test]
fn prepend_keeps_wire_ids_with_identity() {
    let upd = render_then_update(true, vec![10, 20], vec![5, 10, 20]);
    assert_eq!(
        upd.discovered_child_ids,
        vec![3, 1, 2],
        "prepended block gets a fresh id; keyed blocks keep theirs"
    );
    assert_eq!(
        upd.rerendered,
        vec![0],
        "only the parent re-renders; unchanged keyed blocks are not re-emitted"
    );
}

/// (c, counter-test) Identity-less: the same prepend shifts every id
/// (positional matching): placeholders [1, 2, 3], and the blocks landing in
/// shifted slots are re-emitted (rerendered contains 1 and 2).
#[test]
fn prepend_shifts_wire_ids_without_identity() {
    let upd = render_then_update(false, vec![10, 20], vec![5, 10, 20]);
    assert_eq!(
        upd.discovered_child_ids,
        vec![1, 2, 3],
        "positional matching: every child takes the next free slot, ids shift"
    );
    assert!(
        upd.rerendered.contains(&1) && upd.rerendered.contains(&2),
        "blocks re-emitted under shifted ids: {:?}",
        upd.rerendered
    );
}

/// (d) Collision: two children with the SAME identity under one parent —
/// documented behavior: the first unconsumed match wins (keeps id 1), the
/// colliding second gets a fresh id (2).
#[test]
fn identity_collision_first_match_wins() {
    let upd = render_then_update(true, vec![7], vec![7, 7]);
    assert_eq!(
        upd.discovered_child_ids,
        vec![1, 2],
        "first child with identity 7 keeps id 1; the colliding second gets a fresh id"
    );
}
