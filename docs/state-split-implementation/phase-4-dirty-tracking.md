# Phase 4: Dirty Tracking

**Goal**: Track which state keys have been modified and need background persistence.

## Overview

```
Handler mutates state
        │
        ▼
┌───────────────────────┐
│ shared_cache.update() │
│ dirty_keys.insert()   │ ◄── Mark for persistence
│ broadcast()           │
└───────────────────────┘
        │
        ▼
   Return to client (instant)


Background task (later)
        │
        ▼
┌───────────────────────┐
│ dirty_keys.drain()    │
│ for key in dirty:     │
│   save_to_db(key)     │
└───────────────────────┘
```

## Update Handler Execution: `rwire/src/server.rs`

```rust
async fn handle_client_event(
    event: ClientEvent,
    conn_state: &mut ConnectionState,
    shared: &SharedServerState,
    write: &mut impl Sink<Message, Error = impl std::error::Error>,
) -> Result<(), Box<dyn std::error::Error>> {
    let handler_idx = event.handler_idx as usize;

    if handler_idx >= conn_state.handlers.len() {
        return Ok(());
    }

    let handler = &conn_state.handlers[handler_idx];
    let state_type_id = handler.state_type_id();
    let storage_type = handler.storage_type();
    let changes = handler.changes();

    // Build event context
    let ctx = EventContext::new_with_params(event.payload, event.param_bytes);

    match storage_type {
        StorageType::Local => {
            // Should not reach server
            return Ok(());
        }

        StorageType::Memory => {
            // Per-connection state
            if let Some(state) = conn_state.memory_states.get_mut(&state_type_id) {
                handler.call_with_context(state, &ctx);
            }
        }

        StorageType::Persisted => {
            // Shared state with dirty tracking
            let table_name = handler.table_name();
            let key = format!("{}:{}", table_name, conn_state.session_id);

            // Get mutable access to shared cache
            {
                let mut cache = shared.shared_cache.write().unwrap();
                if let Some(state) = cache.get_mut(&key) {
                    handler.call_with_context(state, &ctx);
                }
            }

            // Mark dirty for background persistence
            shared.mark_dirty(&key);

            // Broadcast to other connections
            shared.broadcast(&key, state_type_id, changes, conn_state.connection_id);
        }
    }

    // Re-render affected synced elements
    let update = build_synced_update(conn_state, shared, changes)?;

    if !update.is_empty() {
        write.send(Message::Binary(update.to_vec())).await?;
    }

    Ok(())
}
```

## SharedServerState Dirty Tracking

```rust
impl SharedServerState {
    /// Mark a key as dirty (needs persistence).
    pub fn mark_dirty(&self, key: &str) {
        self.dirty_keys.write().unwrap().insert(key.to_string());
    }

    /// Drain all dirty keys (for background task).
    pub fn drain_dirty(&self) -> Vec<String> {
        self.dirty_keys.write().unwrap().drain().collect()
    }

    /// Check if any keys are dirty.
    pub fn has_dirty(&self) -> bool {
        !self.dirty_keys.read().unwrap().is_empty()
    }

    /// Get count of dirty keys.
    pub fn dirty_count(&self) -> usize {
        self.dirty_keys.read().unwrap().len()
    }
}
```

## Efficient State Access Pattern

For persisted state, we need a pattern that allows mutation without cloning:

```rust
/// Guard that provides mutable access to shared state.
pub struct SharedStateGuard<'a, S> {
    key: String,
    cache: RwLockWriteGuard<'a, HashMap<String, Box<dyn Any + Send + Sync>>>,
    _marker: std::marker::PhantomData<S>,
}

impl<'a, S: 'static> SharedStateGuard<'a, S> {
    pub fn get_mut(&mut self) -> Option<&mut S> {
        self.cache
            .get_mut(&self.key)?
            .downcast_mut::<S>()
    }
}

impl SharedServerState {
    /// Get mutable access to shared state.
    pub fn state_mut<'a, S: 'static>(
        &'a self,
        key: &str,
    ) -> SharedStateGuard<'a, S> {
        SharedStateGuard {
            key: key.to_string(),
            cache: self.shared_cache.write().unwrap(),
            _marker: std::marker::PhantomData,
        }
    }
}

// Usage in handler:
{
    let mut guard = shared.state_mut::<TodoState>(&key);
    if let Some(state) = guard.get_mut() {
        handler.call_with_context(state as &mut dyn Any, &ctx);
    }
}
// Lock released here
shared.mark_dirty(&key);
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mark_dirty() {
        let shared = SharedServerState::new(None, Duration::from_millis(100));

        assert!(!shared.has_dirty());

        shared.mark_dirty("test:abc");

        assert!(shared.has_dirty());
        assert_eq!(shared.dirty_count(), 1);
    }

    #[test]
    fn test_drain_dirty() {
        let shared = SharedServerState::new(None, Duration::from_millis(100));

        shared.mark_dirty("key1");
        shared.mark_dirty("key2");
        shared.mark_dirty("key1"); // Duplicate, should not increase count

        assert_eq!(shared.dirty_count(), 2);

        let keys = shared.drain_dirty();

        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));

        // Should be empty after drain
        assert!(!shared.has_dirty());
    }

    #[test]
    fn test_handler_marks_dirty() {
        let shared = SharedServerState::new(None, Duration::from_millis(100));

        // Pre-populate cache
        shared.shared_cache.write().unwrap().insert(
            "todos:session1".to_string(),
            Box::new(TodoState::default()),
        );

        // Simulate handler execution for persisted state
        let key = "todos:session1";
        {
            let mut cache = shared.shared_cache.write().unwrap();
            if let Some(state) = cache.get_mut(key) {
                if let Some(todo) = state.downcast_mut::<TodoState>() {
                    todo.items.push(TodoItem::default());
                }
            }
        }
        shared.mark_dirty(key);

        // Verify dirty
        assert!(shared.dirty_keys.read().unwrap().contains(key));
    }
}
```

## Performance Considerations

1. **Lock contention**: Keep write lock duration minimal
   - Acquire lock → mutate → release → mark dirty
   - Don't hold lock during serialization

2. **Dirty set size**: HashSet is O(1) insert
   - Multiple mutations to same key = single entry
   - Deduplication is automatic

3. **Memory pressure**: dirty_keys is just strings
   - State itself is already in shared_cache
   - No additional copies

## Checklist

- [ ] Add `mark_dirty()` method to SharedServerState
- [ ] Add `drain_dirty()` for background task
- [ ] Update handler execution to mark dirty after mutation
- [ ] Ensure lock is released before marking dirty
- [ ] Add helper methods for dirty status
- [ ] Add unit tests
- [ ] Run `cargo clippy`
