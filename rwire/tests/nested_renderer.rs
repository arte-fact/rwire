//! Tests for nested renderer behavior.
//!
//! This test file explores the behavior of nested renderers (synced elements
//! containing other synced elements) and verifies they work correctly during
//! updates.

use std::any::TypeId;
use std::collections::HashMap;

use rwire::builder::{build_synced_update_multi, BuildContext, ElementBuilder};
use rwire::protocol::opcodes::{BATCH_END, CREATE, GET_BY_ID, SYMBOLS};
use rwire::state::ChangeSet;
use rwire::{el, El, HandlerFn, MemoryState};

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
/// 1. Parent synced element (id=0) clears its children
/// 2. Parent renderer is called, returns content with nested synced element
/// 3. emit_update_element() now recognizes synced elements and creates wrapper spans
/// 4. Child synced element gets wrapped with __synced_N ID
/// 5. When child synced element updates independently, it can find its wrapper
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

    // Check that update bytes contain both synced IDs in the symbol table
    let bytes = update_bytes.as_ref();

    // Find "__synced_0" and "__synced_1" strings in the bytes
    let synced_0 = b"__synced_0";
    let synced_1 = b"__synced_1";

    let has_synced_0 = bytes.windows(synced_0.len()).any(|w| w == synced_0);
    let has_synced_1 = bytes.windows(synced_1.len()).any(|w| w == synced_1);

    assert!(has_synced_0, "Update should reference __synced_0");
    assert!(has_synced_1, "Update should reference __synced_1");

    println!("Both synced element IDs found in update bytes");
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
    // 1. SYMBOLS opcode at the start
    assert!(bytes.len() > 2, "Update should have content");
    assert_eq!(bytes[0], SYMBOLS, "Should start with SYMBOLS opcode");

    // 2. GET_BY_ID opcodes - at least 2 for the top-level synced elements
    // Note: there may be more due to nested structures
    let get_by_id_count = bytes.iter().filter(|&&b| b == GET_BY_ID).count();
    assert!(
        get_by_id_count >= 2,
        "Should have at least 2 GET_BY_ID opcodes (one for each synced element), found {}",
        get_by_id_count
    );

    // 3. CREATE opcodes for creating new elements
    let create_count = bytes.iter().filter(|&&b| b == CREATE).count();
    assert!(
        create_count >= 2,
        "Should have at least 2 CREATE opcodes (parent content + nested wrapper)"
    );

    // 4. BATCH_END at the end
    assert_eq!(
        bytes.last(),
        Some(&BATCH_END),
        "Should end with BATCH_END opcode"
    );

    println!("Update bytes verify nested wrapper creation:");
    println!("  - SYMBOLS count: 1");
    println!("  - GET_BY_ID count: {}", get_by_id_count);
    println!("  - CREATE count: {}", create_count);
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

    // All three wrapper IDs should be in the update
    assert!(
        bytes
            .windows(b"__synced_0".len())
            .any(|w| w == b"__synced_0"),
        "Should have __synced_0"
    );
    assert!(
        bytes
            .windows(b"__synced_1".len())
            .any(|w| w == b"__synced_1"),
        "Should have __synced_1"
    );
    assert!(
        bytes
            .windows(b"__synced_2".len())
            .any(|w| w == b"__synced_2"),
        "Should have __synced_2"
    );
}
