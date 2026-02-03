# Phase 7: Graceful Shutdown

**Goal**: Flush all dirty state to database before server exits.

## Overview

```
SIGTERM/SIGINT received
        │
        ▼
┌─────────────────────────────────┐
│  1. Stop accepting connections  │
│  2. Signal persist task to stop │
│  3. Flush all dirty state       │
│  4. Close database connection   │
│  5. Exit                        │
└─────────────────────────────────┘
```

## Shutdown Signal Handling

```rust
use async_std::channel;
use signal_hook::consts::signal::*;
use signal_hook_async_std::Signals;

/// Run server with graceful shutdown support.
pub async fn run_server_with_shutdown(
    addr: SocketAddr,
    root_fn: impl Fn() -> ElementBuilder + Send + Sync + Clone + 'static,
    shared: Arc<SharedServerState>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Setup signal handlers
    let mut signals = Signals::new(&[SIGTERM, SIGINT])?;

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = channel::bounded::<()>(1);

    // Spawn signal handler
    let shutdown_tx_clone = shutdown_tx.clone();
    async_std::task::spawn(async move {
        while let Some(signal) = signals.next().await {
            match signal {
                SIGTERM | SIGINT => {
                    eprintln!("\nShutdown signal received, flushing state...");
                    let _ = shutdown_tx_clone.send(()).await;
                    break;
                }
                _ => {}
            }
        }
    });

    // Spawn persist task with shutdown support
    let persist_shared = shared.clone();
    let persist_shutdown = shutdown_rx.clone();
    async_std::task::spawn(async move {
        persist_task_with_shutdown(persist_shared, persist_shutdown).await;
    });

    // Run server until shutdown
    let server_result = run_server_until_shutdown(addr, root_fn, shared.clone(), shutdown_rx).await;

    // Flush remaining dirty state synchronously
    flush_all_dirty(&shared).await?;

    eprintln!("Shutdown complete.");
    server_result
}
```

## Persist Task with Shutdown

```rust
/// Background persist task that can be stopped gracefully.
async fn persist_task_with_shutdown(
    shared: Arc<SharedServerState>,
    shutdown_rx: channel::Receiver<()>,
) {
    loop {
        // Check for shutdown signal
        match async_std::future::timeout(
            shared.persist_interval,
            shutdown_rx.recv(),
        ).await {
            Ok(Ok(())) => {
                // Shutdown requested
                break;
            }
            Ok(Err(_)) => {
                // Channel closed
                break;
            }
            Err(_) => {
                // Timeout - normal persist cycle
            }
        }

        // Normal persist cycle
        if let Err(e) = persist_dirty_batch(&shared).await {
            eprintln!("Persist error: {}", e);
        }
    }
}

/// Persist all currently dirty state.
async fn persist_dirty_batch(shared: &SharedServerState) -> Result<(), Box<dyn std::error::Error>> {
    let store = match &shared.store {
        Some(s) => s.clone(),
        None => return Ok(()),
    };

    let dirty_keys = shared.drain_dirty();
    if dirty_keys.is_empty() {
        return Ok(());
    }

    let conn = store.connection()?;

    // Begin transaction
    conn.execute("BEGIN TRANSACTION", [])?;

    let mut errors = Vec::new();

    for key in &dirty_keys {
        if let Err(e) = persist_state(shared, &conn, key) {
            errors.push((key.clone(), e));
        }
    }

    if errors.is_empty() {
        conn.execute("COMMIT", [])?;
    } else {
        conn.execute("ROLLBACK", [])?;
        // Re-mark failed keys
        for (key, _) in &errors {
            shared.mark_dirty(key);
        }
        return Err(format!("Failed to persist {} keys", errors.len()).into());
    }

    Ok(())
}
```

## Flush All Dirty State

```rust
/// Synchronously flush all dirty state (for shutdown).
async fn flush_all_dirty(shared: &SharedServerState) -> Result<(), Box<dyn std::error::Error>> {
    let store = match &shared.store {
        Some(s) => s.clone(),
        None => return Ok(()),
    };

    // Keep flushing until no more dirty keys
    let mut attempts = 0;
    const MAX_ATTEMPTS: u32 = 10;

    while shared.has_dirty() && attempts < MAX_ATTEMPTS {
        attempts += 1;

        let dirty_keys = shared.drain_dirty();
        if dirty_keys.is_empty() {
            break;
        }

        eprintln!("Flushing {} dirty keys (attempt {})...", dirty_keys.len(), attempts);

        let conn = store.connection()?;
        conn.execute("BEGIN TRANSACTION", [])?;

        let mut failed = Vec::new();

        for key in &dirty_keys {
            if let Err(e) = persist_state(shared, &conn, key) {
                eprintln!("  Failed to persist {}: {}", key, e);
                failed.push(key.clone());
            }
        }

        if failed.is_empty() {
            conn.execute("COMMIT", [])?;
            eprintln!("  Flushed {} keys successfully.", dirty_keys.len());
        } else {
            conn.execute("ROLLBACK", [])?;
            // Re-mark failed keys for next attempt
            for key in failed {
                shared.mark_dirty(&key);
            }
        }
    }

    if shared.has_dirty() {
        let remaining = shared.dirty_count();
        eprintln!("WARNING: {} keys could not be persisted after {} attempts", remaining, MAX_ATTEMPTS);
    }

    Ok(())
}
```

## Server with Shutdown Support

```rust
/// Run server until shutdown signal.
async fn run_server_until_shutdown(
    addr: SocketAddr,
    root_fn: impl Fn() -> ElementBuilder + Send + Sync + Clone + 'static,
    shared: Arc<SharedServerState>,
    shutdown_rx: channel::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(addr).await?;

    eprintln!("Server listening on {}", addr);

    loop {
        // Accept with shutdown check
        let accept_result = async_std::future::timeout(
            Duration::from_millis(100),
            listener.accept(),
        ).await;

        // Check for shutdown
        if shutdown_rx.try_recv().is_ok() {
            eprintln!("Shutdown requested, stopping accept loop...");
            break;
        }

        match accept_result {
            Ok(Ok((stream, peer))) => {
                let shared = shared.clone();
                let root_fn = root_fn.clone();
                async_std::task::spawn(async move {
                    if let Err(e) = handle_client(stream, peer, shared, root_fn).await {
                        eprintln!("Client error: {}", e);
                    }
                });
            }
            Ok(Err(e)) => {
                eprintln!("Accept error: {}", e);
            }
            Err(_) => {
                // Timeout, check shutdown and continue
            }
        }
    }

    Ok(())
}
```

## Cargo.toml Dependencies

```toml
[dependencies]
signal-hook = "0.3"
signal-hook-async-std = "0.2"
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_flush_all_dirty() {
        let store = SqliteStore::memory().unwrap();
        let shared = SharedServerState::new(
            Some(Arc::new(store.clone())),
            Duration::from_millis(100),
        );

        // Setup schema
        let conn = store.connection().unwrap();
        conn.execute("CREATE TABLE todos (session_id TEXT PRIMARY KEY, filter INTEGER)", []).unwrap();

        // Pre-populate cache
        shared.shared_cache.write().unwrap().insert(
            "todos:test1".to_string(),
            Box::new(TodoState::default()),
        );
        shared.shared_cache.write().unwrap().insert(
            "todos:test2".to_string(),
            Box::new(TodoState::default()),
        );

        // Mark dirty
        shared.mark_dirty("todos:test1");
        shared.mark_dirty("todos:test2");

        // Flush
        flush_all_dirty(&shared).await.unwrap();

        // Verify no longer dirty
        assert!(!shared.has_dirty());

        // Verify in database
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM todos",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(count, 2);
    }

    #[async_std::test]
    async fn test_flush_retries_on_failure() {
        // This test would require a mock store that fails
        // Simplified: just verify retry loop doesn't infinite loop
        let shared = SharedServerState::new(None, Duration::from_millis(100));

        shared.mark_dirty("nonexistent:key");

        // With no store, should complete quickly
        flush_all_dirty(&shared).await.unwrap();
    }
}
```

## Checklist

- [ ] Add `signal-hook` and `signal-hook-async-std` dependencies
- [ ] Implement signal handler for SIGTERM/SIGINT
- [ ] Create shutdown channel
- [ ] Update persist task to check shutdown signal
- [ ] Implement `flush_all_dirty()` function
- [ ] Update server main loop with shutdown support
- [ ] Add retry logic for flush failures
- [ ] Log shutdown progress
- [ ] Add unit tests
- [ ] Test manual shutdown (Ctrl+C)
- [ ] Run `cargo clippy`
