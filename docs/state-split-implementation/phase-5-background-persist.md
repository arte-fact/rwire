# Phase 5: Background Persist Task

**Goal**: Async task that persists dirty state to database without blocking handlers.

## Overview

```
┌────────────────────────────────────────────────────────────┐
│                    Background Persist Task                  │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  loop {                                                     │
│      sleep(persist_interval)  // e.g., 100ms               │
│                                                             │
│      dirty = shared.drain_dirty()                          │
│                                                             │
│      for key in dirty {                                    │
│          state = shared.shared_cache.get(key)              │
│          match db.save_normalized(key, state) {            │
│              Ok(()) => { /* success */ }                   │
│              Err(e) => {                                   │
│                  log_error(e)                              │
│                  shared.mark_dirty(key) // retry later     │
│              }                                             │
│          }                                                 │
│      }                                                     │
│  }                                                         │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

## Implementation: `rwire/src/server.rs`

```rust
/// Background task that persists dirty state to database.
async fn persist_task(shared: Arc<SharedServerState>) {
    loop {
        // Wait for persist interval
        async_std::task::sleep(shared.persist_interval).await;

        // Skip if no store configured
        let store = match &shared.store {
            Some(s) => s.clone(),
            None => continue,
        };

        // Drain dirty keys
        let dirty_keys = shared.drain_dirty();

        if dirty_keys.is_empty() {
            continue;
        }

        // Get database connection
        let conn = match store.connection() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Persist task: failed to get connection: {}", e);
                // Re-mark all as dirty for retry
                for key in dirty_keys {
                    shared.mark_dirty(&key);
                }
                continue;
            }
        };

        // Persist each dirty state
        for key in dirty_keys {
            if let Err(e) = persist_state(&shared, &conn, &key) {
                eprintln!("Persist task: failed to save {}: {}", key, e);
                // Re-mark for retry
                shared.mark_dirty(&key);
            }
        }
    }
}

/// Persist a single state to database.
fn persist_state(
    shared: &SharedServerState,
    conn: &rusqlite::Connection,
    key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse table name from key
    let table_name = key.split(':').next()
        .ok_or("Invalid key format")?;

    let session_id = key.split(':').nth(1)
        .ok_or("Invalid key format")?;

    // Get persistable type info
    let registry = persist_registry();
    let persistable = registry.get(table_name)
        .ok_or_else(|| format!("Unknown table: {}", table_name))?;

    // Get state from cache
    let cache = shared.shared_cache.read().unwrap();
    let state = cache.get(key)
        .ok_or_else(|| format!("State not in cache: {}", key))?;

    // Save to database
    (persistable.save_fn)(conn, session_id, &**state)?;

    Ok(())
}
```

## Batched Writes for Efficiency

For better performance, batch multiple saves in a transaction:

```rust
async fn persist_task(shared: Arc<SharedServerState>) {
    loop {
        async_std::task::sleep(shared.persist_interval).await;

        let store = match &shared.store {
            Some(s) => s.clone(),
            None => continue,
        };

        let dirty_keys = shared.drain_dirty();
        if dirty_keys.is_empty() {
            continue;
        }

        let conn = match store.connection() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Persist: connection failed: {}", e);
                for key in dirty_keys {
                    shared.mark_dirty(&key);
                }
                continue;
            }
        };

        // Batch in a transaction
        let result = conn.execute("BEGIN TRANSACTION", []);
        if result.is_err() {
            for key in dirty_keys {
                shared.mark_dirty(&key);
            }
            continue;
        }

        let mut failed_keys = Vec::new();

        for key in &dirty_keys {
            if let Err(e) = persist_state(&shared, &conn, key) {
                eprintln!("Persist: {} failed: {}", key, e);
                failed_keys.push(key.clone());
            }
        }

        // Commit or rollback
        if failed_keys.is_empty() {
            let _ = conn.execute("COMMIT", []);
        } else {
            let _ = conn.execute("ROLLBACK", []);
            // Re-mark all as dirty (transaction was rolled back)
            for key in dirty_keys {
                shared.mark_dirty(&key);
            }
        }
    }
}
```

## Debouncing Rapid Mutations

Multiple mutations to the same key within the persist interval result in a single write:

```
Time 0ms:   handler mutates todos:abc → mark_dirty("todos:abc")
Time 10ms:  handler mutates todos:abc → mark_dirty("todos:abc")  (already in set)
Time 20ms:  handler mutates todos:abc → mark_dirty("todos:abc")  (already in set)
...
Time 100ms: persist_task wakes up → drain_dirty() → ["todos:abc"]
            → single write to DB with final state
```

## Retry Logic with Exponential Backoff

For persistent failures:

```rust
/// Track retry counts per key.
struct PersistRetryTracker {
    retries: HashMap<String, u32>,
    max_retries: u32,
}

impl PersistRetryTracker {
    fn should_retry(&mut self, key: &str) -> bool {
        let count = self.retries.entry(key.to_string()).or_insert(0);
        *count += 1;
        *count <= self.max_retries
    }

    fn clear(&mut self, key: &str) {
        self.retries.remove(key);
    }
}

// In persist_task:
if let Err(e) = persist_state(&shared, &conn, key) {
    if retry_tracker.should_retry(key) {
        eprintln!("Persist: {} failed (retry {}): {}", key, retries, e);
        shared.mark_dirty(key);
    } else {
        eprintln!("Persist: {} failed permanently after {} retries", key, max_retries);
        // Consider: emit metric, alert, or callback
    }
} else {
    retry_tracker.clear(key);
}
```

## Metrics

```rust
/// Metrics for persist task (optional).
pub struct PersistMetrics {
    pub writes_total: AtomicU64,
    pub writes_failed: AtomicU64,
    pub batch_size_sum: AtomicU64,
    pub batch_count: AtomicU64,
}

impl PersistMetrics {
    pub fn record_batch(&self, size: usize, failed: usize) {
        self.writes_total.fetch_add(size as u64, Ordering::Relaxed);
        self.writes_failed.fetch_add(failed as u64, Ordering::Relaxed);
        self.batch_size_sum.fetch_add(size as u64, Ordering::Relaxed);
        self.batch_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn avg_batch_size(&self) -> f64 {
        let sum = self.batch_size_sum.load(Ordering::Relaxed);
        let count = self.batch_count.load(Ordering::Relaxed);
        if count == 0 { 0.0 } else { sum as f64 / count as f64 }
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_persist_task_saves_dirty() {
        let store = SqliteStore::memory().unwrap();
        let shared = SharedServerState::new(
            Some(Arc::new(store.clone())),
            Duration::from_millis(10),
        );

        // Pre-populate cache
        shared.shared_cache.write().unwrap().insert(
            "todos:test1".to_string(),
            Box::new(TodoState {
                session_id: "test1".into(),
                filter: Filter::All,
                items: vec![TodoItem { text: "test".into(), done: false }],
            }),
        );

        // Mark dirty
        shared.mark_dirty("todos:test1");

        // Run one iteration of persist (simulated)
        let dirty = shared.drain_dirty();
        assert_eq!(dirty.len(), 1);

        let conn = store.connection().unwrap();
        persist_state(&shared, &conn, &dirty[0]).unwrap();

        // Verify in database
        let row: i32 = conn.query_row(
            "SELECT COUNT(*) FROM todos WHERE session_id = 'test1'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(row, 1);
    }

    #[async_std::test]
    async fn test_persist_task_retries_on_failure() {
        let shared = SharedServerState::new(None, Duration::from_millis(10));

        shared.mark_dirty("test:key");

        // Drain and simulate failure
        let dirty = shared.drain_dirty();
        assert_eq!(dirty.len(), 1);

        // Re-mark (simulating error handling)
        for key in dirty {
            shared.mark_dirty(&key);
        }

        // Should still be dirty
        assert!(shared.has_dirty());
    }

    #[async_std::test]
    async fn test_debouncing() {
        let shared = SharedServerState::new(None, Duration::from_millis(100));

        // Multiple marks for same key
        shared.mark_dirty("todos:abc");
        shared.mark_dirty("todos:abc");
        shared.mark_dirty("todos:abc");

        // Should only have one entry
        assert_eq!(shared.dirty_count(), 1);

        let dirty = shared.drain_dirty();
        assert_eq!(dirty.len(), 1);
    }
}
```

## Checklist

- [ ] Implement `persist_task()` async function
- [ ] Implement `persist_state()` helper
- [ ] Add transaction batching for efficiency
- [ ] Add retry logic for failures
- [ ] Re-mark failed keys as dirty
- [ ] Spawn persist task on server start
- [ ] Add optional metrics
- [ ] Add unit tests
- [ ] Test with actual SQLite database
- [ ] Run `cargo clippy`
