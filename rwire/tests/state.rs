//! Tests for state management.

use std::any::TypeId;
use rwire::{EventContext, EventPayload, HandlerFn, MemoryState};

#[derive(Default)]
struct TestState {
    value: i32,
}

impl MemoryState for TestState {}

fn increment(state: &mut TestState) {
    state.value += 1;
}

fn add_ten(state: &mut TestState) {
    state.value += 10;
}

#[test]
fn test_handler_fn_new() {
    let handler = HandlerFn::new(increment);
    assert_eq!(handler.state_type_id(), TypeId::of::<TestState>());
}

#[test]
fn test_handler_fn_create_state() {
    let handler = HandlerFn::new(increment);
    let state = handler.create_state();

    let test_state = state.downcast_ref::<TestState>().unwrap();
    assert_eq!(test_state.value, 0); // Default value
}

#[test]
fn test_handler_fn_call() {
    let handler = HandlerFn::new(increment);
    let mut state = TestState { value: 5 };

    handler.call(&mut state);
    assert_eq!(state.value, 6);

    handler.call(&mut state);
    assert_eq!(state.value, 7);
}

#[test]
fn test_handler_fn_call_wrong_type() {
    let handler = HandlerFn::new(increment);
    let mut wrong_state: i32 = 42;

    // Should not panic, just do nothing
    handler.call(&mut wrong_state);
    assert_eq!(wrong_state, 42);
}

#[test]
fn test_handler_fn_clone() {
    let handler = HandlerFn::new(increment);
    let cloned = handler.clone();

    let mut state = TestState { value: 0 };
    cloned.call(&mut state);
    assert_eq!(state.value, 1);
}

#[test]
fn test_handler_fn_debug() {
    let handler = HandlerFn::new(increment);
    let debug_str = format!("{:?}", handler);
    assert!(debug_str.contains("HandlerFn"));
    assert!(debug_str.contains("state_type_id"));
}

#[test]
fn test_multiple_handlers() {
    let h1 = HandlerFn::new(increment);
    let h2 = HandlerFn::new(add_ten);

    let mut state = TestState { value: 0 };

    h1.call(&mut state);
    assert_eq!(state.value, 1);

    h2.call(&mut state);
    assert_eq!(state.value, 11);

    h1.call(&mut state);
    assert_eq!(state.value, 12);
}

#[test]
fn test_handler_creates_correct_state_type() {
    #[derive(Default)]
    struct OtherState {
        name: String,
    }
    impl MemoryState for OtherState {}

    fn set_name(state: &mut OtherState) {
        state.name = "test".to_string();
    }

    let handler = HandlerFn::new(set_name);
    let state = handler.create_state();

    // Should be OtherState, not TestState
    assert!(state.downcast_ref::<OtherState>().is_some());
    assert!(state.downcast_ref::<TestState>().is_none());
}

#[test]
fn test_handler_state_isolation() {
    let handler = HandlerFn::new(increment);

    // Create two separate states
    let mut state1 = TestState { value: 0 };
    let mut state2 = TestState { value: 100 };

    handler.call(&mut state1);
    handler.call(&mut state1);

    handler.call(&mut state2);

    // States should be independent
    assert_eq!(state1.value, 2);
    assert_eq!(state2.value, 101);
}

// ============================================================================
// EventContext Tests
// ============================================================================

#[test]
fn test_event_context_empty() {
    let ctx = EventContext::empty();
    assert!(ctx.is_empty());
    assert!(ctx.text().is_none());
    assert!(ctx.data("key").is_none());
    assert!(ctx.field("name").is_none());
}

#[test]
fn test_event_context_text_payload() {
    let json = r#"{"t":"text","v":"hello world"}"#;
    let ctx = EventContext::new(json.as_bytes().to_vec());

    assert!(!ctx.is_empty());
    assert_eq!(ctx.text(), Some("hello world"));
    assert!(ctx.data("key").is_none());
    assert!(ctx.field("name").is_none());
}

#[test]
fn test_event_context_data_payload() {
    let json = r#"{"t":"data","v":{"id":"42","action":"delete"}}"#;
    let ctx = EventContext::new(json.as_bytes().to_vec());

    assert!(!ctx.is_empty());
    assert_eq!(ctx.data("id"), Some("42"));
    assert_eq!(ctx.data("action"), Some("delete"));
    assert!(ctx.data("missing").is_none());
    assert!(ctx.text().is_none());
}

#[test]
fn test_event_context_form_payload() {
    let json = r#"{"t":"form","v":{"todo":"Buy milk","priority":"high"}}"#;
    let ctx = EventContext::new(json.as_bytes().to_vec());

    assert!(!ctx.is_empty());
    assert_eq!(ctx.field("todo"), Some("Buy milk"));
    assert_eq!(ctx.field("priority"), Some("high"));
    assert!(ctx.field("missing").is_none());
    assert!(ctx.text().is_none());
}

#[test]
fn test_event_context_invalid_json() {
    let ctx = EventContext::new(b"not json".to_vec());
    assert!(ctx.is_empty());
}

#[test]
fn test_event_context_malformed_payload() {
    // Missing type field
    let json = r#"{"v":"hello"}"#;
    let ctx = EventContext::new(json.as_bytes().to_vec());
    assert!(ctx.is_empty());
}

#[test]
fn test_event_context_raw() {
    let data = b"test data";
    let ctx = EventContext::new(data.to_vec());
    assert_eq!(ctx.raw(), data);
}

#[test]
fn test_event_context_debug() {
    let ctx = EventContext::empty();
    let debug = format!("{:?}", ctx);
    assert!(debug.contains("EventContext"));
}

#[test]
fn test_event_payload_enum() {
    // Test that EventPayload variants exist and work
    let text = EventPayload::Text("hello".to_string());
    let data = EventPayload::Data(std::collections::HashMap::new());
    let form = EventPayload::Form(std::collections::HashMap::new());
    let empty = EventPayload::Empty;

    // Just verify they can be created and matched
    match text {
        EventPayload::Text(s) => assert_eq!(s, "hello"),
        _ => panic!("Expected Text variant"),
    }
    match data {
        EventPayload::Data(_) => {}
        _ => panic!("Expected Data variant"),
    }
    match form {
        EventPayload::Form(_) => {}
        _ => panic!("Expected Form variant"),
    }
    match empty {
        EventPayload::Empty => {}
        _ => panic!("Expected Empty variant"),
    }
}
