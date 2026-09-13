//! Tests for state management.

use rwire::{ChangeSet, EventContext, EventPayload, HandlerFn, MemoryState, RendererDeps};
use std::any::TypeId;

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

// ============================================================================
// RendererDeps and ChangeSet Tests
// ============================================================================

#[test]
fn test_renderer_deps_always() {
    let deps = RendererDeps::always();
    assert!(deps.always);

    // Always deps should trigger update for any change
    assert!(deps.needs_update(ChangeSet::new()));
    assert!(deps.needs_update(ChangeSet::all()));
    assert!(deps.needs_update(ChangeSet::from_fields(&[0])));
}

#[test]
fn test_renderer_deps_from_fields() {
    let deps = RendererDeps::from_fields(&[0, 2, 5]);

    // Should set bits 0, 2, and 5
    assert_eq!(deps.mask, 0b100101);
    assert!(!deps.always);

    // Should need update when overlapping fields change
    assert!(deps.needs_update(ChangeSet::from_fields(&[0])));
    assert!(deps.needs_update(ChangeSet::from_fields(&[2])));
    assert!(deps.needs_update(ChangeSet::from_fields(&[5])));
    assert!(deps.needs_update(ChangeSet::from_fields(&[0, 1, 2])));

    // Should not need update when non-overlapping fields change
    assert!(!deps.needs_update(ChangeSet::from_fields(&[1])));
    assert!(!deps.needs_update(ChangeSet::from_fields(&[3, 4])));
    assert!(!deps.needs_update(ChangeSet::new()));
}

#[test]
fn test_renderer_deps_all_changed() {
    let deps = RendererDeps::from_fields(&[0]);

    // all_changed should trigger any deps
    assert!(deps.needs_update(ChangeSet::all()));
}

#[test]
fn test_renderer_deps_none() {
    let deps = RendererDeps::none();
    assert_eq!(deps.mask, 0);
    assert!(!deps.always);

    // None deps should only update for all_changed
    assert!(!deps.needs_update(ChangeSet::new()));
    assert!(!deps.needs_update(ChangeSet::from_fields(&[0, 1, 2])));
    assert!(deps.needs_update(ChangeSet::all()));
}

#[test]
fn test_renderer_deps_merge() {
    let deps1 = RendererDeps::from_fields(&[0, 1]);
    let deps2 = RendererDeps::from_fields(&[2, 3]);
    let merged = deps1.merge(deps2);

    assert_eq!(merged.mask, 0b1111); // bits 0, 1, 2, 3
    assert!(!merged.always);

    // Merging with always produces always
    let always = RendererDeps::always();
    let merged_always = deps1.merge(always);
    assert!(merged_always.always);
}

#[test]
fn test_renderer_deps_high_field_ids() {
    // Field IDs >= 64 should be ignored
    let deps = RendererDeps::from_fields(&[63, 64, 100]);
    assert_eq!(deps.mask, 1u64 << 63);
}

#[test]
fn test_changeset_new() {
    let cs = ChangeSet::new();
    assert_eq!(cs.mask, 0);
    assert!(!cs.all_changed);
    assert!(cs.is_empty());
}

#[test]
fn test_changeset_all() {
    let cs = ChangeSet::all();
    assert!(cs.all_changed);
    assert!(!cs.is_empty());
}

#[test]
fn test_changeset_from_fields() {
    let cs = ChangeSet::from_fields(&[1, 3, 7]);
    assert_eq!(cs.mask, 0b10001010);
    assert!(!cs.all_changed);
    assert!(!cs.is_empty());
}

#[test]
fn test_changeset_merge() {
    let cs1 = ChangeSet::from_fields(&[0]);
    let cs2 = ChangeSet::from_fields(&[1]);
    let merged = cs1.merge(cs2);

    assert_eq!(merged.mask, 0b11);
    assert!(!merged.all_changed);

    // Merging with all produces all
    let all = ChangeSet::all();
    let merged_all = cs1.merge(all);
    assert!(merged_all.all_changed);
}

#[test]
fn test_changeset_with_field() {
    let cs = ChangeSet::new().with_field(3).with_field(5);
    assert_eq!(cs.mask, 0b101000);
}

#[test]
fn test_deps_changeset_interaction() {
    // Simulate counter app: renderer depends on field 0 (count)
    const FIELD_COUNT: u8 = 0;
    const FIELD_NAME: u8 = 1;

    let render_count_deps = RendererDeps::from_fields(&[FIELD_COUNT]);

    // Handler increments count -> field 0 changes
    let increment_changes = ChangeSet::from_fields(&[FIELD_COUNT]);
    assert!(render_count_deps.needs_update(increment_changes));

    // Handler updates name -> field 1 changes
    let update_name_changes = ChangeSet::from_fields(&[FIELD_NAME]);
    assert!(!render_count_deps.needs_update(update_name_changes));
}

#[test]
fn event_context_carries_session_identity() {
    use rwire::EventContext;
    let ctx = EventContext::empty();
    assert_eq!(ctx.session_id(), None);
    let ctx = EventContext::empty().with_session("abc123".into());
    assert_eq!(ctx.session_id(), Some("abc123"));
}
