//! Tests for nested renderer behavior.
//!
//! This test file explores the behavior of nested renderers (synced elements
//! containing other synced elements) and verifies they work correctly during
//! updates.

use std::any::TypeId;
use std::collections::HashMap;

use rwire::builder::{build_synced_update_multi, BuildContext, ElementBuilder};
use rwire::protocol::opcodes::{BATCH_END, CREATE, CREATE_SYNCED, GET_SYNCED, SYMBOLS};
use rwire::state::ChangeSet;
use rwire::{el, El, HandlerFn, MemoryState};

/// Check if the bytes contain a GET_SYNCED opcode (0x05) for a given synced_id.
/// The synced_id is encoded as a varint after the opcode.
fn has_get_synced_for_id(bytes: &[u8], synced_id: u32) -> bool {
    for i in 0..bytes.len() {
        if bytes[i] == GET_SYNCED {
            // Check if the next byte(s) match the synced_id as varint
            if synced_id < 0x80 && i + 1 < bytes.len() && bytes[i + 1] == synced_id as u8 {
                return true;
            }
            // For ids >= 128, would need more bytes, but test IDs are 0, 1, 2
        }
    }
    false
}

/// Check if the bytes contain a CREATE_SYNCED opcode (0x03) for a given synced_id.
/// The synced_id is encoded as a varint after the opcode.
fn has_create_synced_for_id(bytes: &[u8], synced_id: u32) -> bool {
    for i in 0..bytes.len() {
        if bytes[i] == CREATE_SYNCED {
            // Check if the next byte(s) match the synced_id as varint
            if synced_id < 0x80 && i + 1 < bytes.len() && bytes[i + 1] == synced_id as u8 {
                return true;
            }
        }
    }
    false
}

#[derive(Default)]
struct ParentState {
    count: i32,
}

impl MemoryState for ParentState {}

// Simulates a nested renderer - in real code this would use #[renderer]
fn make_child_renderer() -> ElementBuilder {
    // This creates a synced element
    ElementBuilder::synced(|state: &ParentState| {
        el(El::Span)
            .class("count")
            .text(&format!("Count: {}", state.count))
    })
}

fn make_parent_renderer_with_nested() -> ElementBuilder {
    // This creates a synced element that CONTAINS another synced element
    ElementBuilder::synced(|_state: &ParentState| {
        el(El::Div).class("parent").append([
            el(El::Span).text("Items:"),
            // This is a NESTED synced element
            make_child_renderer(),
        ])
    })
}

fn make_parent_renderer_flat() -> ElementBuilder {
    // This creates a synced element without nesting
    ElementBuilder::synced(|state: &ParentState| {
        el(El::Div).class("parent").append([
            el(El::Span).text("Items:"),
            el(El::Span)
                .class("count")
                .text(&format!("Count: {}", state.count)),
        ])
    })
}

#[test]
fn test_flat_renderer_initial_render() {
    let mut ctx = BuildContext::new();
    let state = ParentState::default();

    // Build states map for multi-state version
    let boxed: Box<dyn std::any::Any + Send + Sync> = Box::new(state);
    let state_ref: &(dyn std::any::Any + Send + Sync) = boxed.as_ref();
    let mut states: HashMap<TypeId, &(dyn std::any::Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<ParentState>(), state_ref);

    let el = make_parent_renderer_flat();

    ctx.collect_symbols_multi(&el, &states);
    ctx.emit_multi(&el);

    let synced = ctx.take_synced_elements();

    // Flat renderer should have exactly 1 synced element
    assert_eq!(
        synced.len(),
        1,
        "Flat renderer should have 1 synced element"
    );
}

#[test]
fn test_nested_renderer_initial_render() {
    let mut ctx = BuildContext::new();
    let state = ParentState::default();

    // Build states map for multi-state version
    let boxed: Box<dyn std::any::Any + Send + Sync> = Box::new(state);
    let state_ref: &(dyn std::any::Any + Send + Sync) = boxed.as_ref();
    let mut states: HashMap<TypeId, &(dyn std::any::Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<ParentState>(), state_ref);

    let el = make_parent_renderer_with_nested();

    ctx.collect_symbols_multi(&el, &states);
    ctx.emit_multi(&el);

    let synced = ctx.take_synced_elements();

    // Nested renderer should have 2 synced elements (parent + child)
    // This tests whether initial render properly discovers nested synced elements
    assert_eq!(
        synced.len(),
        2,
        "Nested renderer should discover 2 synced elements, found {}",
        synced.len()
    );
}

#[test]
fn test_nested_renderer_update_creates_wrappers() {
    let mut ctx = BuildContext::new();
    let state = ParentState { count: 5 };

    // Build states map for multi-state version
    let boxed: Box<dyn std::any::Any + Send + Sync> = Box::new(state);
    let state_ref: &(dyn std::any::Any + Send + Sync) = boxed.as_ref();
    let mut states: HashMap<TypeId, &(dyn std::any::Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<ParentState>(), state_ref);

    let el = make_parent_renderer_with_nested();

    ctx.collect_symbols_multi(&el, &states);
    ctx.emit_multi(&el);

    let synced = ctx.take_synced_elements();
    let mut handlers: std::collections::HashMap<u32, HandlerFn> = std::collections::HashMap::new();

    // This should produce update bytes
    let update_bytes = build_synced_update_multi(&synced, &states, &mut handlers, ChangeSet::all());

    // The update should not be empty
    assert!(!update_bytes.is_empty(), "Update bytes should not be empty");

    // If there are 2 synced elements, the update should reference both wrapper IDs
    // This is where the bug manifests - nested renderers may not be properly handled
    println!("Update bytes length: {}", update_bytes.len());
    println!("Synced elements: {}", synced.len());
}

#[test]
fn test_collect_symbols_finds_nested_synced() {
    let mut ctx = BuildContext::new();
    let state = ParentState::default();

    // Build states map for multi-state version
    let boxed: Box<dyn std::any::Any + Send + Sync> = Box::new(state);
    let state_ref: &(dyn std::any::Any + Send + Sync) = boxed.as_ref();
    let mut states: HashMap<TypeId, &(dyn std::any::Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<ParentState>(), state_ref);

    let el = make_parent_renderer_with_nested();

    // During collect_symbols_multi, nested synced elements should be discovered
    ctx.collect_symbols_multi(&el, &states);
    ctx.emit_multi(&el);

    let synced = ctx.take_synced_elements();

    // Print info for debugging
    println!("Found {} synced elements", synced.len());
}

/// Verifies how a nested synced element is emitted when both it and its parent
/// update. The sequence:
/// 1. Parent (id=0) gets its own GET_SYNCED + CLEAR_CHILDREN; its renderer runs.
/// 2. Encountering the nested child, the parent emits a CREATE_SYNCED *placeholder*
///    for it (no content). The client morph matches it to the live nested span by
///    id and preserves that span (it never recurses into `__synced_` regions).
/// 3. The child (id=1) ALSO gets its own GET_SYNCED + CLEAR_CHILDREN + rebuild from
///    the main loop, so its children are reconciled - including removals.
///
/// Previously the parent rebuilt the child's content inline (and the child got no
/// GET_SYNCED). The client's nested-region short-circuit then discarded that
/// content, so a nested list that shrank never dropped its removed rows.
#[test]
fn test_nested_update_preserves_structure() {
    let mut ctx = BuildContext::new();
    let state = ParentState { count: 1 };

    let boxed: Box<dyn std::any::Any + Send + Sync> = Box::new(state);
    let state_ref: &(dyn std::any::Any + Send + Sync) = boxed.as_ref();
    let mut states: HashMap<TypeId, &(dyn std::any::Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<ParentState>(), state_ref);

    let el = make_parent_renderer_with_nested();

    ctx.collect_symbols_multi(&el, &states);
    ctx.emit_multi(&el);

    let synced = ctx.take_synced_elements();
    assert_eq!(synced.len(), 2, "Should have 2 synced elements");

    // Now simulate an update
    let mut handlers: std::collections::HashMap<u32, HandlerFn> = std::collections::HashMap::new();
    let update_bytes = build_synced_update_multi(&synced, &states, &mut handlers, ChangeSet::all());

    let bytes = update_bytes.as_ref();

    // Parent (id=0) gets GET_SYNCED for its own standalone update.
    let has_synced_0 = has_get_synced_for_id(bytes, 0);
    assert!(
        has_synced_0,
        "Update should have GET_SYNCED for parent (id 0)"
    );

    // The nested child (id=1) ALSO gets its own GET_SYNCED standalone update, so its
    // children reconcile (including removals) via its own CLEAR_CHILDREN+rebuild. The
    // parent additionally emits a CREATE_SYNCED placeholder the client morph uses to
    // preserve the live nested span.
    let has_synced_1 = has_get_synced_for_id(bytes, 1);
    assert!(
        has_synced_1,
        "Nested child should get its own GET_SYNCED standalone update"
    );

    let has_create_synced_1 = has_create_synced_for_id(bytes, 1);
    assert!(
        has_create_synced_1,
        "Parent should emit a CREATE_SYNCED placeholder for the nested child (id 1)"
    );

    println!("Parent + nested child each get GET_SYNCED; parent emits a nested placeholder");
}

/// Test that nested synced elements are wrapped with correct IDs during update emission.
///
/// This test verifies the fix for the nested renderer bug: when a parent synced
/// element re-renders, nested synced elements within its content should be
/// wrapped with ID attributes so they can be targeted for future updates.
#[test]
fn test_nested_update_emits_wrapper_ids() {
    let mut ctx = BuildContext::new();
    let state = ParentState { count: 42 };

    let boxed: Box<dyn std::any::Any + Send + Sync> = Box::new(state);
    let state_ref: &(dyn std::any::Any + Send + Sync) = boxed.as_ref();
    let mut states: HashMap<TypeId, &(dyn std::any::Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<ParentState>(), state_ref);

    let el = make_parent_renderer_with_nested();

    ctx.collect_symbols_multi(&el, &states);
    ctx.emit_multi(&el);

    let synced = ctx.take_synced_elements();
    let mut handlers: std::collections::HashMap<u32, HandlerFn> = std::collections::HashMap::new();

    let update_bytes = build_synced_update_multi(&synced, &states, &mut handlers, ChangeSet::all());
    let bytes = update_bytes.as_ref();

    // Verify the update contains:
    // 1. Update should have content
    assert!(bytes.len() > 2, "Update should have content");

    // Note: The update may or may not start with SYMBOLS depending on whether
    // non-synced-ID symbols are present. Check for symbol table OR GET_SYNCED.
    let starts_with_symbols = bytes[0] == SYMBOLS;
    let has_get_synced = bytes.contains(&GET_SYNCED);
    assert!(
        starts_with_symbols || has_get_synced,
        "Should have SYMBOLS or GET_SYNCED"
    );

    // 2. GET_SYNCED opcodes - at least 2 for the top-level synced elements
    let get_synced_count = bytes.iter().filter(|&&b| b == GET_SYNCED).count();
    assert!(
        get_synced_count >= 2,
        "Should have at least 2 GET_SYNCED opcodes (one for each synced element), found {}",
        get_synced_count
    );

    // 3. CREATE opcodes for creating new elements, or CREATE_SYNCED for nested
    let create_count = bytes.iter().filter(|&&b| b == CREATE).count();
    let create_synced_count = bytes.iter().filter(|&&b| b == CREATE_SYNCED).count();
    assert!(
        create_count >= 1 || create_synced_count >= 1,
        "Should have CREATE or CREATE_SYNCED opcodes"
    );

    // 4. BATCH_END at the end
    assert_eq!(
        bytes.last(),
        Some(&BATCH_END),
        "Should end with BATCH_END opcode"
    );

    println!("Update bytes verify nested wrapper creation:");
    println!("  - GET_SYNCED count: {}", get_synced_count);
    println!("  - CREATE count: {}", create_count);
    println!("  - CREATE_SYNCED count: {}", create_synced_count);
    println!("  - Total bytes: {}", bytes.len());
}

/// A re-render reports the nested regions it produced, tagged with their parent —
/// the hook the router uses to register a swapped-in view's renderers and prune the
/// previous view's. Here the parent (id 0) re-render must surface the nested child
/// (id 1) as a discovered region whose parent is 0.
#[test]
fn test_update_collects_discovered_nested_regions_with_parent() {
    use rwire::builder::{build_synced_update_with_known_symbols, SyncedElement};

    let mut ctx = BuildContext::new();
    let state = ParentState { count: 7 };
    let boxed: Box<dyn std::any::Any + Send + Sync> = Box::new(state);
    let state_ref: &(dyn std::any::Any + Send + Sync) = boxed.as_ref();
    let mut states: HashMap<TypeId, &(dyn std::any::Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<ParentState>(), state_ref);

    let el = make_parent_renderer_with_nested();
    ctx.collect_symbols_multi(&el, &states);
    ctx.emit_multi(&el);
    let synced = ctx.take_synced_elements();
    assert_eq!(synced.len(), 2, "parent + nested child");

    let mut handlers: std::collections::HashMap<u32, HandlerFn> = std::collections::HashMap::new();
    let mut discovered: Vec<SyncedElement> = Vec::new();
    let _ = build_synced_update_with_known_symbols(
        &synced,
        &states,
        &mut handlers,
        ChangeSet::all(),
        None,
        None,
        None,
        None,
        None,
        Some(&mut discovered),
        0,
        None,
    );

    let child = discovered
        .iter()
        .find(|r| r.id == 1)
        .expect("the nested child (id 1) should be discovered during the parent's re-render");
    assert_eq!(
        child.parent(),
        Some(0),
        "the discovered child's parent should be the parent region (id 0)"
    );
}

/// Test deeply nested synced elements (3 levels).
#[test]
fn test_deeply_nested_synced_elements() {
    fn make_level3() -> ElementBuilder {
        ElementBuilder::synced(|state: &ParentState| {
            el(El::Span).text(&format!("L3: {}", state.count))
        })
    }

    fn make_level2() -> ElementBuilder {
        ElementBuilder::synced(|_state: &ParentState| {
            el(El::Div).class("level2").append([make_level3()])
        })
    }

    fn make_level1() -> ElementBuilder {
        ElementBuilder::synced(|_state: &ParentState| {
            el(El::Div).class("level1").append([make_level2()])
        })
    }

    let mut ctx = BuildContext::new();
    let state = ParentState { count: 100 };

    let boxed: Box<dyn std::any::Any + Send + Sync> = Box::new(state);
    let state_ref: &(dyn std::any::Any + Send + Sync) = boxed.as_ref();
    let mut states: HashMap<TypeId, &(dyn std::any::Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<ParentState>(), state_ref);

    let el = make_level1();

    ctx.collect_symbols_multi(&el, &states);
    ctx.emit_multi(&el);

    let synced = ctx.take_synced_elements();

    // Should discover all 3 levels of synced elements
    assert_eq!(
        synced.len(),
        3,
        "Should have 3 synced elements for 3 nesting levels"
    );

    // Verify each has a unique ID
    let ids: Vec<u32> = synced.iter().map(|s| s.id).collect();
    assert_eq!(ids, vec![0, 1, 2], "IDs should be 0, 1, 2");

    // Now test update
    let mut handlers: std::collections::HashMap<u32, HandlerFn> = std::collections::HashMap::new();
    let update_bytes = build_synced_update_multi(&synced, &states, &mut handlers, ChangeSet::all());
    let bytes = update_bytes.as_ref();

    // Every level gets its own GET_SYNCED standalone update (so each reconciles its
    // own children, removals included). Each nested level is additionally emitted as
    // a CREATE_SYNCED placeholder by its parent, which the client morph uses to
    // preserve the live span while that level's own update owns its content.
    assert!(
        has_get_synced_for_id(bytes, 0),
        "Should have GET_SYNCED for top-level (id 0)"
    );
    assert!(
        has_get_synced_for_id(bytes, 1),
        "Nested id=1 should have its own GET_SYNCED"
    );
    assert!(
        has_get_synced_for_id(bytes, 2),
        "Nested id=2 should have its own GET_SYNCED"
    );

    // Nested levels are emitted as CREATE_SYNCED placeholders by their parents.
    assert!(
        has_create_synced_for_id(bytes, 1),
        "Should have CREATE_SYNCED placeholder for nested id=1"
    );
    assert!(
        has_create_synced_for_id(bytes, 2),
        "Should have CREATE_SYNCED placeholder for nested id=2"
    );
}
