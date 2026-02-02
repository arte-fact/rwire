//! Item references for dynamic content event tracking.
//!
//! This module provides `ItemRef<T>`, a typed reference to an item in a collection
//! by its index. This enables type-safe event handling for dynamically-generated
//! content without runtime string parsing.
//!
//! # Example
//!
//! ```ignore
//! use rwire::{el, El, Ev, handler, renderer, State, ItemRef, IterWithRef};
//!
//! #[derive(State, Default)]
//! struct TodoState {
//!     items: Vec<TodoItem>,
//! }
//!
//! #[derive(Default, Clone)]
//! struct TodoItem {
//!     text: String,
//!     done: bool,
//! }
//!
//! #[renderer]
//! fn render_items(state: &TodoState) -> ElementBuilder {
//!     el(El::Ul).append(
//!         state.items.iter_with_ref().map(|(item_ref, item)| {
//!             el(El::Li)
//!                 .text(&item.text)
//!                 .on_ref(Ev::Click, toggle_item(), item_ref)
//!         })
//!     )
//! }
//!
//! #[handler]
//! fn toggle_item(state: &mut TodoState, item_ref: ItemRef<TodoItem>) {
//!     if let Some(item) = item_ref.get_mut(&mut state.items) {
//!         item.done = !item.done;
//!     }
//! }
//! ```

use std::marker::PhantomData;

use crate::protocol::varint::{read_varint, write_varint};

/// A typed reference to an item in a collection by its index.
///
/// `ItemRef<T>` is a zero-cost abstraction that wraps an index into a collection.
/// It provides type safety by tracking the item type `T`, preventing accidental
/// use with the wrong collection.
///
/// # Wire Encoding
///
/// When sent over the wire, `ItemRef<T>` is encoded as a varint, providing
/// compact representation:
/// - Indices 0-127: 1 byte
/// - Indices 128-16,511: 2 bytes
/// - Larger indices: 3 bytes
///
/// # Example
///
/// ```ignore
/// let item_ref: ItemRef<TodoItem> = ItemRef::new(5);
/// assert_eq!(item_ref.index(), 5);
///
/// // Access the item through the reference
/// if let Some(item) = item_ref.get_mut(&mut state.items) {
///     item.done = true;
/// }
/// ```
/// Use `fn() -> T` instead of `T` to make PhantomData always Copy
/// regardless of whether T is Copy. We manually implement Clone and Copy
/// to avoid trait bounds on T.
pub struct ItemRef<T> {
    index: usize,
    _marker: PhantomData<fn() -> T>,
}

// Manual implementations to avoid T: Copy/Clone bounds
impl<T> Clone for ItemRef<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ItemRef<T> {}

impl<T> std::fmt::Debug for ItemRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ItemRef")
            .field("index", &self.index)
            .finish()
    }
}

impl<T> PartialEq for ItemRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<T> Eq for ItemRef<T> {}

impl<T> std::hash::Hash for ItemRef<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl<T> ItemRef<T> {
    /// Create a new item reference with the given index.
    pub fn new(index: usize) -> Self {
        Self {
            index,
            _marker: PhantomData,
        }
    }

    /// Get the index of the referenced item.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Get a reference to the item in the given slice.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get<'a>(&self, collection: &'a [T]) -> Option<&'a T> {
        collection.get(self.index)
    }

    /// Get a mutable reference to the item in the given slice.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get_mut<'a>(&self, collection: &'a mut [T]) -> Option<&'a mut T> {
        collection.get_mut(self.index)
    }

    /// Get a reference to the item in the given Vec.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get_from_vec<'a>(&self, collection: &'a [T]) -> Option<&'a T> {
        collection.get(self.index)
    }

    /// Get a mutable reference to the item in the given Vec.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get_mut_from_vec<'a>(&self, collection: &'a mut [T]) -> Option<&'a mut T> {
        collection.get_mut(self.index)
    }

    /// Encode this reference to bytes for wire transmission.
    ///
    /// Uses varint encoding for compact representation.
    pub fn encode(&self, buf: &mut Vec<u8>) {
        let mut temp = bytes::BytesMut::new();
        write_varint(&mut temp, self.index as u32);
        buf.extend_from_slice(&temp);
    }

    /// Decode an item reference from bytes.
    ///
    /// Returns the decoded reference and the number of bytes consumed,
    /// or `None` if the buffer is too short.
    pub fn decode(data: &[u8]) -> Option<(Self, usize)> {
        let (value, consumed) = read_varint(data)?;
        Some((Self::new(value as usize), consumed))
    }

    /// Get the encoded byte length of this reference.
    pub fn encoded_len(&self) -> usize {
        if self.index < 0x80 {
            1
        } else if self.index < 0x4080 {
            2
        } else {
            3
        }
    }
}

impl<T> Default for ItemRef<T> {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Extension trait for iterating over collections with item references.
///
/// This trait adds the `iter_with_ref()` method to collections, which yields
/// `(ItemRef<T>, &T)` pairs. This is useful for building UI elements that need
/// to bind events to specific items.
///
/// # Example
///
/// ```ignore
/// use rwire::IterWithRef;
///
/// let items = vec!["apple", "banana", "cherry"];
/// for (item_ref, item) in items.iter_with_ref() {
///     println!("Item {} at index {}", item, item_ref.index());
/// }
/// ```
pub trait IterWithRef<T> {
    /// Returns an iterator that yields `(ItemRef<T>, &T)` pairs.
    fn iter_with_ref<'a>(&'a self) -> impl Iterator<Item = (ItemRef<T>, &'a T)>
    where
        T: 'a;
}

impl<T> IterWithRef<T> for Vec<T> {
    fn iter_with_ref<'a>(&'a self) -> impl Iterator<Item = (ItemRef<T>, &'a T)>
    where
        T: 'a,
    {
        self.iter().enumerate().map(|(idx, item)| {
            (ItemRef::new(idx), item)
        })
    }
}

impl<T> IterWithRef<T> for [T] {
    fn iter_with_ref<'a>(&'a self) -> impl Iterator<Item = (ItemRef<T>, &'a T)>
    where
        T: 'a,
    {
        self.iter().enumerate().map(|(idx, item)| {
            (ItemRef::new(idx), item)
        })
    }
}

impl<T, const N: usize> IterWithRef<T> for [T; N] {
    fn iter_with_ref<'a>(&'a self) -> impl Iterator<Item = (ItemRef<T>, &'a T)>
    where
        T: 'a,
    {
        self.iter().enumerate().map(|(idx, item)| {
            (ItemRef::new(idx), item)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_ref_new() {
        let item_ref: ItemRef<String> = ItemRef::new(42);
        assert_eq!(item_ref.index(), 42);
    }

    #[test]
    fn test_item_ref_get() {
        let items = vec!["a", "b", "c"];
        let item_ref: ItemRef<&str> = ItemRef::new(1);
        assert_eq!(item_ref.get(&items), Some(&"b"));

        let out_of_bounds: ItemRef<&str> = ItemRef::new(10);
        assert_eq!(out_of_bounds.get(&items), None);
    }

    #[test]
    fn test_item_ref_get_mut() {
        let mut items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let item_ref: ItemRef<String> = ItemRef::new(1);
        if let Some(item) = item_ref.get_mut(&mut items) {
            *item = "modified".to_string();
        }
        assert_eq!(items[1], "modified");
    }

    #[test]
    fn test_item_ref_encode_decode() {
        for idx in [0, 1, 127, 128, 255, 1000, 16511, 16512, 100000] {
            let item_ref: ItemRef<()> = ItemRef::new(idx);
            let mut buf = Vec::new();
            item_ref.encode(&mut buf);

            let (decoded, consumed) = ItemRef::<()>::decode(&buf).unwrap();
            assert_eq!(decoded.index(), idx);
            assert_eq!(consumed, buf.len());
        }
    }

    #[test]
    fn test_iter_with_ref() {
        let items = vec!["apple", "banana", "cherry"];
        let collected: Vec<_> = items.iter_with_ref().collect();

        assert_eq!(collected.len(), 3);
        assert_eq!(collected[0].0.index(), 0);
        assert_eq!(collected[0].1, &"apple");
        assert_eq!(collected[1].0.index(), 1);
        assert_eq!(collected[1].1, &"banana");
        assert_eq!(collected[2].0.index(), 2);
        assert_eq!(collected[2].1, &"cherry");
    }

    #[test]
    fn test_iter_with_ref_slice() {
        let items: &[i32] = &[10, 20, 30];
        let collected: Vec<_> = items.iter_with_ref().collect();

        assert_eq!(collected.len(), 3);
        assert_eq!(collected[0].0.index(), 0);
        assert_eq!(*collected[0].1, 10);
    }
}
