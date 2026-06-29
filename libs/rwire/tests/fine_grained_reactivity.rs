//! Tests for fine-grained reactivity system.
//!
//! These tests verify that:
//! 1. RendererDeps correctly tracks field dependencies
//! 2. ChangeSet correctly tracks field mutations
//! 3. The needs_update logic works correctly
//! 4. build_synced_update_multi filters correctly based on ChangeSet

use std::any::{Any, TypeId};
use std::collections::HashMap;

use rwire::builder::{build_synced_update_multi, ElementBuilder, SyncedElement, SyncedRenderer};
use rwire::protocol::opcodes::GET_SYNCED;
use rwire::state::{ChangeSet, RendererDeps};
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

// ============================================================================
// RendererDeps Tests
// ============================================================================

#[test]
fn test_renderer_deps_from_single_field() {
    let deps = RendererDeps::from_fields(&[0]);
    assert_eq!(deps.mask, 1);
    assert!(!deps.always);
}

#[test]
fn test_renderer_deps_from_multiple_fields() {
    let deps = RendererDeps::from_fields(&[0, 2, 5]);
    // Bits 0, 2, 5 should be set: 0b100101 = 37
    assert_eq!(deps.mask, 0b100101);
    assert!(!deps.always);
}

#[test]
fn test_renderer_deps_always() {
    let deps = RendererDeps::always();
    assert!(deps.always);
}

#[test]
fn test_renderer_deps_none() {
    let deps = RendererDeps::none();
    assert_eq!(deps.mask, 0);
    assert!(!deps.always);
}

#[test]
fn test_renderer_deps_high_field_ids() {
    // Field IDs >= 64 should be ignored
    let deps = RendererDeps::from_fields(&[0, 64, 100, 255]);
    assert_eq!(deps.mask, 1); // Only field 0 should be set
}

#[test]
fn test_renderer_deps_merge() {
    let deps1 = RendererDeps::from_fields(&[0, 1]);
    let deps2 = RendererDeps::from_fields(&[2, 3]);
    let merged = deps1.merge(deps2);
    assert_eq!(merged.mask, 0b1111); // Fields 0,1,2,3
}

#[test]
fn test_renderer_deps_merge_with_always() {
    let deps1 = RendererDeps::from_fields(&[0]);
    let deps2 = RendererDeps::always();
    let merged = deps1.merge(deps2);
    assert!(merged.always);
}

// ============================================================================
// ChangeSet Tests
// ============================================================================

#[test]
fn test_changeset_new_is_empty() {
    let changes = ChangeSet::new();
    assert!(changes.is_empty());
    assert_eq!(changes.mask, 0);
    assert!(!changes.all_changed);
}

#[test]
fn test_changeset_all() {
    let changes = ChangeSet::all();
    assert!(!changes.is_empty());
    assert!(changes.all_changed);
}

#[test]
fn test_changeset_from_fields() {
    let changes = ChangeSet::from_fields(&[1, 3]);
    assert_eq!(changes.mask, 0b1010);
    assert!(!changes.all_changed);
}

#[test]
fn test_changeset_with_field() {
    let changes = ChangeSet::new().with_field(2).with_field(4);
    assert_eq!(changes.mask, 0b10100);
}

#[test]
fn test_changeset_merge() {
    let c1 = ChangeSet::from_fields(&[0]);
    let c2 = ChangeSet::from_fields(&[1]);
    let merged = c1.merge(c2);
    assert_eq!(merged.mask, 0b11);
}

#[test]
fn test_changeset_merge_with_all() {
    let c1 = ChangeSet::from_fields(&[0]);
    let c2 = ChangeSet::all();
    let merged = c1.merge(c2);
    assert!(merged.all_changed);
}

// ============================================================================
// needs_update Tests
// ============================================================================

#[test]
fn test_needs_update_matching_fields() {
    let deps = RendererDeps::from_fields(&[1, 2]);
    let changes = ChangeSet::from_fields(&[2, 3]);
    // Field 2 overlaps
    assert!(deps.needs_update(changes));
}

#[test]
fn test_needs_update_no_overlap() {
    let deps = RendererDeps::from_fields(&[0, 1]);
    let changes = ChangeSet::from_fields(&[2, 3]);
    // No overlap
    assert!(!deps.needs_update(changes));
}

#[test]
fn test_needs_update_always_deps() {
    let deps = RendererDeps::always();
    let changes = ChangeSet::from_fields(&[5]);
    // Always deps should always update
    assert!(deps.needs_update(changes));
}

#[test]
fn test_needs_update_all_changed() {
    let deps = RendererDeps::from_fields(&[0]);
    let changes = ChangeSet::all();
    // All changed should trigger update
    assert!(deps.needs_update(changes));
}

#[test]
fn test_needs_update_empty_changes() {
    let deps = RendererDeps::from_fields(&[0, 1, 2]);
    let changes = ChangeSet::new();
    // No changes, no update needed
    assert!(!deps.needs_update(changes));
}

#[test]
fn test_needs_update_none_deps() {
    let deps = RendererDeps::none();
    let changes = ChangeSet::from_fields(&[0, 1, 2]);
    // No deps, no update needed (unless changes is all)
    assert!(!deps.needs_update(changes));
}

#[test]
fn test_needs_update_none_deps_with_all_changed() {
    let deps = RendererDeps::none();
    let changes = ChangeSet::all();
    // all_changed overrides none deps
    assert!(deps.needs_update(changes));
}

// ============================================================================
// Integration Tests - build_synced_update_multi filtering
// ============================================================================

// Field-less marker: these tests exercise RendererDeps bitmasks keyed by the
// FIELD_* ids below, not real field storage, so the struct needs no fields.
#[derive(Default)]
struct MultiFieldState;

impl MemoryState for MultiFieldState {}

// Field IDs as generated by #[derive(State)] — used to build RendererDeps masks.
impl MultiFieldState {
    const FIELD_A: u8 = 0;
    const FIELD_B: u8 = 1;
    const FIELD_C: u8 = 2;
}

fn make_synced_element_with_deps(id: u32, deps: RendererDeps) -> SyncedElement {
    SyncedElement::new_with_deps(
        id,
        Box::new(TestRenderer { deps }),
        TypeId::of::<MultiFieldState>(),
        deps,
    )
}

struct TestRenderer {
    deps: RendererDeps,
}

impl SyncedRenderer for TestRenderer {
    fn render_with_state(&self, _state: &dyn Any) -> Option<ElementBuilder> {
        Some(el(El::Span).text("test"))
    }

    fn clone_box(&self) -> Box<dyn SyncedRenderer> {
        Box::new(TestRenderer { deps: self.deps })
    }

    fn state_type_id(&self) -> TypeId {
        TypeId::of::<MultiFieldState>()
    }

    fn create_default_state(&self) -> Box<dyn Any + Send + Sync> {
        Box::new(MultiFieldState)
    }

    fn deps(&self) -> RendererDeps {
        self.deps
    }
}

#[test]
fn test_build_synced_update_filters_by_changeset() {
    let state = MultiFieldState;
    let mut states: HashMap<TypeId, &(dyn std::any::Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<MultiFieldState>(), &state);

    // Create synced elements with different deps
    let synced = vec![
        make_synced_element_with_deps(0, RendererDeps::from_fields(&[MultiFieldState::FIELD_A])),
        make_synced_element_with_deps(1, RendererDeps::from_fields(&[MultiFieldState::FIELD_B])),
        make_synced_element_with_deps(2, RendererDeps::from_fields(&[MultiFieldState::FIELD_C])),
    ];

    let mut handlers: std::collections::HashMap<u32, HandlerFn> = std::collections::HashMap::new();

    // Only field_a changed
    let changes = ChangeSet::from_fields(&[MultiFieldState::FIELD_A]);
    let update = build_synced_update_multi(&synced, &states, &mut handlers, changes);

    // Should only update element 0 (depends on field_a)
    let bytes = update.as_ref();
    assert!(
        has_get_synced_for_id(bytes, 0),
        "Should have GET_SYNCED for id 0 (depends on field_a)"
    );
    assert!(
        !has_get_synced_for_id(bytes, 1),
        "Should NOT have GET_SYNCED for id 1 (depends on field_b)"
    );
    assert!(
        !has_get_synced_for_id(bytes, 2),
        "Should NOT have GET_SYNCED for id 2 (depends on field_c)"
    );
}

#[test]
fn test_build_synced_update_all_changes() {
    let state = MultiFieldState;
    let mut states: HashMap<TypeId, &(dyn std::any::Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<MultiFieldState>(), &state);

    let synced = vec![
        make_synced_element_with_deps(0, RendererDeps::from_fields(&[MultiFieldState::FIELD_A])),
        make_synced_element_with_deps(1, RendererDeps::from_fields(&[MultiFieldState::FIELD_B])),
    ];

    let mut handlers: std::collections::HashMap<u32, HandlerFn> = std::collections::HashMap::new();

    // All fields changed
    let changes = ChangeSet::all();
    let update = build_synced_update_multi(&synced, &states, &mut handlers, changes);

    // Should update both elements
    let bytes = update.as_ref();
    assert!(
        has_get_synced_for_id(bytes, 0),
        "Should have GET_SYNCED for id 0"
    );
    assert!(
        has_get_synced_for_id(bytes, 1),
        "Should have GET_SYNCED for id 1"
    );
}

#[test]
fn test_build_synced_update_no_changes() {
    let state = MultiFieldState;
    let mut states: HashMap<TypeId, &(dyn std::any::Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<MultiFieldState>(), &state);

    let synced = vec![make_synced_element_with_deps(
        0,
        RendererDeps::from_fields(&[MultiFieldState::FIELD_A]),
    )];

    let mut handlers: std::collections::HashMap<u32, HandlerFn> = std::collections::HashMap::new();

    // No changes
    let changes = ChangeSet::new();
    let update = build_synced_update_multi(&synced, &states, &mut handlers, changes);

    // Should be empty - nothing to update
    assert!(update.is_empty(), "Should be empty when no changes");
}

#[test]
fn test_build_synced_update_always_deps() {
    let state = MultiFieldState;
    let mut states: HashMap<TypeId, &(dyn std::any::Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<MultiFieldState>(), &state);

    let synced = vec![
        make_synced_element_with_deps(0, RendererDeps::from_fields(&[MultiFieldState::FIELD_A])),
        make_synced_element_with_deps(1, RendererDeps::always()), // Always update
    ];

    let mut handlers: std::collections::HashMap<u32, HandlerFn> = std::collections::HashMap::new();

    // Only field_b changed (element 0 doesn't depend on it)
    let changes = ChangeSet::from_fields(&[MultiFieldState::FIELD_B]);
    let update = build_synced_update_multi(&synced, &states, &mut handlers, changes);

    let bytes = update.as_ref();
    // Element 0 should NOT update (depends on field_a, not field_b)
    assert!(
        !has_get_synced_for_id(bytes, 0),
        "Should NOT have GET_SYNCED for id 0"
    );
    // Element 1 should update (always deps)
    assert!(
        has_get_synced_for_id(bytes, 1),
        "Should have GET_SYNCED for id 1 (always deps)"
    );
}

#[test]
fn test_build_synced_update_multiple_fields_overlap() {
    let state = MultiFieldState;
    let mut states: HashMap<TypeId, &(dyn std::any::Any + Send + Sync)> = HashMap::new();
    states.insert(TypeId::of::<MultiFieldState>(), &state);

    let synced = vec![
        // Depends on both A and B
        make_synced_element_with_deps(
            0,
            RendererDeps::from_fields(&[MultiFieldState::FIELD_A, MultiFieldState::FIELD_B]),
        ),
        // Depends only on C
        make_synced_element_with_deps(1, RendererDeps::from_fields(&[MultiFieldState::FIELD_C])),
    ];

    let mut handlers: std::collections::HashMap<u32, HandlerFn> = std::collections::HashMap::new();

    // Only field_a changed
    let changes = ChangeSet::from_fields(&[MultiFieldState::FIELD_A]);
    let update = build_synced_update_multi(&synced, &states, &mut handlers, changes);

    let bytes = update.as_ref();
    // Element 0 should update (depends on A, which changed)
    assert!(
        has_get_synced_for_id(bytes, 0),
        "Should have GET_SYNCED for id 0 (depends on A)"
    );
    // Element 1 should NOT update (depends only on C)
    assert!(
        !has_get_synced_for_id(bytes, 1),
        "Should NOT have GET_SYNCED for id 1 (depends on C)"
    );
}
