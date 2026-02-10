//! Tests for ItemRef and dynamic content event tracking.

use rwire::{EventContext, ItemRef, IterWithRef};

#[test]
fn test_item_ref_in_event_context() {
    // Simulate param_bytes that would be extracted from an event
    let item_ref: ItemRef<String> = ItemRef::new(42);
    let mut param_bytes = Vec::new();
    item_ref.encode(&mut param_bytes);

    // Create context with param_bytes
    let ctx = EventContext::new_with_params(vec![], param_bytes);

    // Extract ItemRef from context
    let extracted: Option<ItemRef<String>> = ctx.item_ref();
    assert!(extracted.is_some());
    assert_eq!(extracted.unwrap().index(), 42);
}

#[test]
fn test_item_ref_ctx_extraction_wrong_type() {
    // ItemRef is generic but the type is erased over the wire
    // The same bytes can be interpreted as any ItemRef<T>
    let item_ref: ItemRef<String> = ItemRef::new(10);
    let mut param_bytes = Vec::new();
    item_ref.encode(&mut param_bytes);

    let ctx = EventContext::new_with_params(vec![], param_bytes);

    // Can be extracted as any type since wire format is just an index
    let as_i32: Option<ItemRef<i32>> = ctx.item_ref();
    assert!(as_i32.is_some());
    assert_eq!(as_i32.unwrap().index(), 10);
}

#[test]
fn test_ctx_item_index_fallback() {
    // When you just need the index without the type
    let item_ref: ItemRef<()> = ItemRef::new(99);
    let mut param_bytes = Vec::new();
    item_ref.encode(&mut param_bytes);

    let ctx = EventContext::new_with_params(vec![], param_bytes);

    // item_index() is a convenience that returns just the usize
    assert_eq!(ctx.item_index(), Some(99));
}

#[test]
fn test_item_ref_large_indices() {
    // Test varint encoding for indices > 127
    for idx in [128, 255, 500, 1000, 16511, 16512, 65535, 100000] {
        let item_ref: ItemRef<()> = ItemRef::new(idx);
        let mut param_bytes = Vec::new();
        item_ref.encode(&mut param_bytes);

        let ctx = EventContext::new_with_params(vec![], param_bytes.clone());
        let extracted: Option<ItemRef<()>> = ctx.item_ref();
        assert!(extracted.is_some(), "Failed to extract index {}", idx);
        assert_eq!(
            extracted.unwrap().index(),
            idx,
            "Index mismatch for {}",
            idx
        );

        // Also test via item_index helper
        let ctx2 = EventContext::new_with_params(vec![], param_bytes);
        assert_eq!(ctx2.item_index(), Some(idx));
    }
}

#[test]
fn test_item_ref_zero_index() {
    let item_ref: ItemRef<String> = ItemRef::new(0);
    let mut param_bytes = Vec::new();
    item_ref.encode(&mut param_bytes);

    let ctx = EventContext::new_with_params(vec![], param_bytes);
    let extracted: Option<ItemRef<String>> = ctx.item_ref();
    assert!(extracted.is_some());
    assert_eq!(extracted.unwrap().index(), 0);
}

#[test]
fn test_item_ref_empty_param_bytes() {
    let ctx = EventContext::new_with_params(vec![], vec![]);
    let extracted: Option<ItemRef<String>> = ctx.item_ref();
    assert!(extracted.is_none());
    assert_eq!(ctx.item_index(), None);
}

#[test]
fn test_iter_with_ref_vec() {
    let items = vec!["a", "b", "c"];
    let collected: Vec<_> = items.iter_with_ref().collect();

    assert_eq!(collected.len(), 3);
    for (i, (item_ref, &item)) in collected.iter().enumerate() {
        assert_eq!(item_ref.index(), i);
        assert_eq!(item, items[i]);
    }
}

#[test]
fn test_iter_with_ref_empty() {
    let items: Vec<String> = vec![];
    let collected: Vec<_> = items.iter_with_ref().collect();
    assert!(collected.is_empty());
}

#[test]
fn test_item_ref_get_with_iter_with_ref() {
    let items = vec![
        "apple".to_string(),
        "banana".to_string(),
        "cherry".to_string(),
    ];

    for (item_ref, item) in items.iter_with_ref() {
        // Verify that get() returns the same item
        let fetched = item_ref.get(&items);
        assert_eq!(fetched, Some(item));
    }
}

#[test]
fn test_item_ref_get_mut_with_iter_with_ref() {
    let mut items = vec![1, 2, 3, 4, 5];

    // Collect refs first to avoid borrow issues
    let refs: Vec<ItemRef<i32>> = items.iter_with_ref().map(|(r, _)| r).collect();

    // Modify each item using the refs
    for item_ref in refs {
        if let Some(val) = item_ref.get_mut(&mut items) {
            *val *= 2;
        }
    }

    assert_eq!(items, vec![2, 4, 6, 8, 10]);
}

#[test]
fn test_item_ref_encoded_len() {
    // Single byte for 0-127
    for idx in 0..128 {
        let item_ref: ItemRef<()> = ItemRef::new(idx);
        assert_eq!(item_ref.encoded_len(), 1, "Index {} should be 1 byte", idx);
    }

    // Two bytes for 128-16511
    for idx in [128, 255, 500, 1000, 16511] {
        let item_ref: ItemRef<()> = ItemRef::new(idx);
        assert_eq!(item_ref.encoded_len(), 2, "Index {} should be 2 bytes", idx);
    }

    // Three bytes for larger
    for idx in [16512, 65535, 100000] {
        let item_ref: ItemRef<()> = ItemRef::new(idx);
        assert_eq!(item_ref.encoded_len(), 3, "Index {} should be 3 bytes", idx);
    }
}
