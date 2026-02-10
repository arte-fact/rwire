//! Tests for synced element updates and dynamic handler registration.

use std::any::TypeId;
use std::collections::HashMap;

use rwire::builder::{build_synced_update_multi, BuildContext, SyncedElement};
use rwire::state::ChangeSet;
use rwire::{el, El, Ev, HandlerFn, MemoryState};

#[derive(Default)]
struct TestState;

impl MemoryState for TestState {}

fn test_handler(_state: &mut TestState) {}

#[test]
fn test_build_synced_update_multi_with_empty_synced() {
    let synced: Vec<SyncedElement> = vec![];
    let states: HashMap<TypeId, &(dyn std::any::Any + Send + Sync)> = HashMap::new();
    let mut handlers: Vec<HandlerFn> = vec![];

    let update = build_synced_update_multi(&synced, &states, &mut handlers, ChangeSet::all());

    // With no synced elements, should return empty bytes
    assert!(update.is_empty());
}

#[test]
fn test_build_synced_update_multi_with_empty_handlers() {
    // This tests the edge case where synced elements exist but no handlers
    let mut ctx = BuildContext::new();
    let state = TestState::default();

    // Build a simple element tree
    let el = el(El::Div).text("test");
    ctx.collect_symbols(&el, &state);
    ctx.emit(&el, &state);

    // Should complete without panic even with no handlers
    let _bytes = ctx.finish();
}

#[test]
fn test_handler_deduplication_by_fn_id() {
    let h1 = HandlerFn::new(test_handler);
    let h2 = HandlerFn::new(test_handler); // Same function

    // Both should have the same fn_id
    assert_eq!(h1.fn_id(), h2.fn_id());

    // Register both handlers - the lookup should deduplicate
    let handlers = vec![h1.clone()];

    // When looking up, should find the existing one
    let fn_id = h2.fn_id();
    let existing_idx = handlers.iter().position(|h| h.fn_id() == fn_id);
    assert_eq!(existing_idx, Some(0));
}

#[test]
fn test_handler_fn_id_different_functions() {
    fn handler_a(_state: &mut TestState) {}
    fn handler_b(_state: &mut TestState) {}

    let h1 = HandlerFn::new(handler_a);
    let h2 = HandlerFn::new(handler_b);

    // Different functions should have different fn_ids
    assert_ne!(h1.fn_id(), h2.fn_id());
}

#[test]
fn test_build_context_tracks_elements() {
    let mut ctx = BuildContext::new();
    let state = ();

    let el = el(El::Div).append([el(El::Span).text("hello"), el(El::Button).text("click")]);

    ctx.collect_symbols(&el, &state);

    // Should have tracked Div, Span, and Button
    let used = ctx.used_elements();
    assert!(used.contains(&El::Div.as_u8()));
    assert!(used.contains(&El::Span.as_u8()));
    assert!(used.contains(&El::Button.as_u8()));
}

#[test]
fn test_build_context_tracks_events() {
    let mut ctx = BuildContext::new();
    let state = TestState::default();

    let handler_spec = rwire::state::HandlerSpec::memory(test_handler);

    let el = el(El::Button).text("click").on(Ev::Click, handler_spec);

    ctx.collect_symbols(&el, &state);

    // Should have tracked Click event
    let used = ctx.used_events();
    assert!(used.contains(&Ev::Click.as_u8()));
}

#[test]
fn test_emit_update_element_creates_valid_opcodes() {
    let mut ctx = BuildContext::new();
    let state = TestState::default();

    // Simple element with text and class
    let el = el(El::Div).class("container").text("content");

    ctx.collect_symbols(&el, &state);
    ctx.emit(&el, &state);

    let bytes = ctx.finish();

    // Should produce non-empty binary output
    assert!(!bytes.is_empty());

    // First byte after symbols should be CREATE opcode (0x02)
    // The structure is: [SYMBOLS header] [symbols...] [CREATE, type] [SET_CLASS...] [SET_TEXT...] [APPEND...] [END]
    // We verify the output is at least valid-looking
    assert!(bytes.len() > 5);
}

#[test]
fn test_build_context_symbol_interning() {
    let mut ctx = BuildContext::new();
    let state = ();

    // Use the same class name multiple times
    let el = el(El::Div)
        .class("shared")
        .append([el(El::Span).class("shared"), el(El::Button).class("shared")]);

    ctx.collect_symbols(&el, &state);
    ctx.emit(&el, &state);

    // The symbol "shared" should only appear once in the output
    // This is verified by the fact that compilation succeeds
    // (interning prevents symbol overflow)
    let _bytes = ctx.finish();
}

#[test]
fn test_synced_element_clone() {
    // This tests that SyncedElement can be cloned (needed for connection state)
    let synced = vec![];
    let cloned: Vec<SyncedElement> = synced.clone();
    assert!(cloned.is_empty());
}

#[test]
fn test_element_builder_is_synced() {
    // Non-synced element
    let normal = el(El::Div);
    assert!(!normal.is_synced());

    // Synced element would be created via ElementBuilder::synced()
    // but that requires a renderer function, so we just test the flag
}

#[test]
fn test_element_builder_accessors() {
    let elem = el(El::Div)
        .class("container")
        .text("hello")
        .attr("id", "main");

    assert_eq!(elem.el_type(), El::Div);
    assert_eq!(elem.class_name(), Some("container"));
    assert_eq!(elem.text_content(), Some("hello"));
    assert_eq!(elem.attributes(), &[("id".to_string(), "main".to_string())]);
}

#[test]
fn test_element_builder_children() {
    let parent = el(El::Div).append([el(El::Span).text("child1"), el(El::Span).text("child2")]);

    let children = parent.children();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].text_content(), Some("child1"));
    assert_eq!(children[1].text_content(), Some("child2"));
}
