//! Tests for state management.

use std::any::TypeId;
use wire_wasm::{ClientState, HandlerFn};

#[derive(Default)]
struct TestState {
    value: i32,
}

impl ClientState for TestState {}

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
    impl ClientState for OtherState {}

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
