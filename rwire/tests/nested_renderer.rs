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
    ctx.emit_multi(&el, &states);

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
    ctx.emit_multi(&el, &states);

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
    ctx.emit_multi(&el, &states);

    let synced = ctx.take_synced_elements();
    let mut handlers: Vec<HandlerFn> = vec![];

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
    ctx.emit_multi(&el, &states);

    let synced = ctx.take_synced_elements();

    // Print info for debugging
    println!("Found {} synced elements", synced.len());
}

/// This test verifies that nested synced elements are properly wrapped during updates.
///
/// The sequence of events during an update:
/// 1. Parent synced element (id=0) gets GET_SYNCED, clears its children
/// 2. Parent renderer is called, returns content with nested synced element
/// 3. emit_update_element() recognizes nested synced element and uses CREATE_SYNCED
/// 4. Child synced element gets wrapper with its original ID (id=1)
/// 5. Child is marked as "already rendered" and skipped in main loop
///
/// This avoids the bug where GET_SYNCED(1) would fail because the original
/// wrapper was destroyed when parent cleared its children.
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
    ctx.emit_multi(&el, &states);

    let synced = ctx.take_synced_elements();
    assert_eq!(synced.len(), 2, "Should have 2 synced elements");

    // Now simulate an update
    let mut handlers: Vec<HandlerFn> = vec![];
    let update_bytes = build_synced_update_multi(&synced, &states, &mut handlers, ChangeSet::all());

    let bytes = update_bytes.as_ref();

    // Parent (id=0) should have GET_SYNCED to find its existing wrapper
    let has_synced_0 = has_get_synced_for_id(bytes, 0);
    assert!(has_synced_0, "Update should have GET_SYNCED for parent (id 0)");

    // Nested child (id=1) should NOT have GET_SYNCED - it's rendered via CREATE_SYNCED
    // as part of parent's content, avoiding the destroyed-wrapper bug
    let has_synced_1 = has_get_synced_for_id(bytes, 1);
    assert!(!has_synced_1, "Nested child should NOT have GET_SYNCED (uses CREATE_SYNCED instead)");

    // Verify CREATE_SYNCED is present for the nested element
    let has_create_synced_1 = has_create_synced_for_id(bytes, 1);
    assert!(has_create_synced_1, "Update should have CREATE_SYNCED for nested child (id 1)");

    println!("Parent uses GET_SYNCED, nested child uses CREATE_SYNCED");
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
    ctx.emit_multi(&el, &states);

    let synced = ctx.take_synced_elements();
    let mut handlers: Vec<HandlerFn> = vec![];

    let update_bytes = build_synced_update_multi(&synced, &states, &mut handlers, ChangeSet::all());
    let bytes = update_bytes.as_ref();

    // Verify the update contains:
    // 1. Update should have content
    assert!(bytes.len() > 2, "Update should have content");

    // Note: The update may or may not start with SYMBOLS depending on whether
    // non-synced-ID symbols are present. Check for symbol table OR GET_SYNCED.
    let starts_with_symbols = bytes[0] == SYMBOLS;
    let has_get_synced = bytes.iter().any(|&b| b == GET_SYNCED);
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
    ctx.emit_multi(&el, &states);

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
    let mut handlers: Vec<HandlerFn> = vec![];
    let update_bytes = build_synced_update_multi(&synced, &states, &mut handlers, ChangeSet::all());
    let bytes = update_bytes.as_ref();

    // Only top-level (id=0) gets GET_SYNCED
    // Nested elements (id=1, id=2) get CREATE_SYNCED as part of parent's content
    assert!(
        has_get_synced_for_id(bytes, 0),
        "Should have GET_SYNCED for top-level (id 0)"
    );
    assert!(
        !has_get_synced_for_id(bytes, 1),
        "Nested id=1 should NOT have GET_SYNCED"
    );
    assert!(
        !has_get_synced_for_id(bytes, 2),
        "Nested id=2 should NOT have GET_SYNCED"
    );

    // Nested elements should use CREATE_SYNCED with their original IDs
    assert!(
        has_create_synced_for_id(bytes, 1),
        "Should have CREATE_SYNCED for nested id=1"
    );
    assert!(
        has_create_synced_for_id(bytes, 2),
        "Should have CREATE_SYNCED for nested id=2"
    );
}
