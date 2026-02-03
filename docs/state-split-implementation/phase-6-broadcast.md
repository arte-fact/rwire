# Phase 6: Cross-Connection Broadcast

**Goal**: Notify other connections when persisted state changes so they can re-render.

## Overview

```
Connection A mutates state
        │
        ├──► shared_cache.update()
        ├──► mark_dirty()
        └──► broadcast(StateChanged)
                    │
        ┌───────────┴───────────┐
        ▼                       ▼
   Connection B            Connection C
   (subscribed)            (subscribed)
        │                       │
        ▼                       ▼
   Re-render              Re-render
   synced elements        synced elements
```

## Broadcast Message

```rust
/// Message sent to connections when shared state changes.
#[derive(Clone, Debug)]
pub enum BroadcastMsg {
    /// State was modified, re-render affected elements.
    StateChanged {
        /// Cache key: "{table}:{session_id}"
        key: String,
        /// TypeId of the state struct
        state_type_id: TypeId,
        /// Which fields changed (for fine-grained re-render)
        changes: ChangeSet,
    },
}
```

## Connection Event Loop with Broadcast

Using `futures::select!` to handle both WebSocket and broadcast messages:

```rust
use futures::{select, FutureExt, StreamExt, SinkExt};

async fn handle_connection(
    ws: WebSocket,
    session_id: SessionId,
    shared: Arc<SharedServerState>,
    root_fn: impl Fn() -> ElementBuilder,
) -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let connection_id = shared.next_connection_id();
    let (broadcast_tx, broadcast_rx) = async_channel::bounded(32);

    shared.register_connection(connection_id, broadcast_tx);

    let mut conn_state = ConnectionState::new(connection_id, session_id.clone(), broadcast_rx);

    // Build and send initial DOM
    // ... (existing code)

    // Subscribe to persisted state keys
    for synced in &conn_state.synced_elements {
        if synced.storage_type == StorageType::Persisted {
            let key = format!("{}:{}", synced.table_name, session_id);
            shared.subscribe(connection_id, &key);
            conn_state.subscribed_keys.insert(key);
        }
    }

    // Split WebSocket
    let (mut ws_write, mut ws_read) = ws.split();

    // Event loop
    loop {
        select! {
            // Client event
            msg = ws_read.next().fuse() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        handle_client_event(
                            ClientEvent::decode(&data)?,
                            &mut conn_state,
                            &shared,
                            &mut ws_write,
                        ).await?;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => {
                        eprintln!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            // Broadcast from another connection
            broadcast = conn_state.broadcast_rx.recv().fuse() => {
                match broadcast {
                    Ok(msg) => {
                        handle_broadcast(
                            msg,
                            &mut conn_state,
                            &shared,
                            &mut ws_write,
                        ).await?;
                    }
                    Err(_) => break, // Channel closed
                }
            }
        }
    }

    // Cleanup
    shared.unregister_connection(connection_id);
    Ok(())
}
```

## Handle Broadcast

```rust
/// Handle broadcast message from another connection.
async fn handle_broadcast(
    msg: BroadcastMsg,
    conn_state: &mut ConnectionState,
    shared: &SharedServerState,
    ws_write: &mut impl Sink<Message, Error = impl std::error::Error>,
) -> Result<(), Box<dyn std::error::Error>> {
    match msg {
        BroadcastMsg::StateChanged { key, state_type_id, changes } => {
            // Only process if subscribed
            if !conn_state.subscribed_keys.contains(&key) {
                return Ok(());
            }

            // Find synced elements that depend on this state type
            let affected: Vec<&SyncedElement> = conn_state
                .synced_elements
                .iter()
                .filter(|se| {
                    se.state_type_id == state_type_id
                        && se.deps.needs_update(changes)
                })
                .collect();

            if affected.is_empty() {
                return Ok(());
            }

            // Build update from shared cache (state already updated)
            let update = build_synced_update_for_elements(
                &affected,
                &shared.shared_cache.read().unwrap(),
                &key,
                &mut conn_state.sent_symbols,
            )?;

            // Send to client
            if !update.is_empty() {
                ws_write.send(Message::Binary(update.to_vec())).await?;
            }
        }
    }

    Ok(())
}
```

## Broadcast Implementation

```rust
impl SharedServerState {
    /// Broadcast state change to subscribed connections.
    pub fn broadcast(
        &self,
        key: &str,
        state_type_id: TypeId,
        changes: ChangeSet,
        from_connection: u64,
    ) {
        let msg = BroadcastMsg::StateChanged {
            key: key.to_string(),
            state_type_id,
            changes,
        };

        let subs = self.subscriptions.read().unwrap();
        let senders = self.broadcast_senders.read().unwrap();

        if let Some(conn_ids) = subs.get(key) {
            for &conn_id in conn_ids {
                // Skip the connection that made the change
                if conn_id == from_connection {
                    continue;
                }

                if let Some(sender) = senders.get(&conn_id) {
                    // Non-blocking send, drop if channel full
                    match sender.try_send(msg.clone()) {
                        Ok(()) => {}
                        Err(async_channel::TrySendError::Full(_)) => {
                            // Channel full, client is slow
                            // Message dropped to prevent blocking
                            eprintln!(
                                "Broadcast dropped for conn {}: channel full",
                                conn_id
                            );
                        }
                        Err(async_channel::TrySendError::Closed(_)) => {
                            // Connection closed, will be cleaned up
                        }
                    }
                }
            }
        }
    }
}
```

## Subscription Management

```rust
impl SharedServerState {
    /// Subscribe connection to state changes.
    pub fn subscribe(&self, conn_id: u64, key: &str) {
        self.subscriptions
            .write()
            .unwrap()
            .entry(key.to_string())
            .or_default()
            .push(conn_id);
    }

    /// Unsubscribe connection from a key.
    pub fn unsubscribe(&self, conn_id: u64, key: &str) {
        if let Some(conns) = self.subscriptions.write().unwrap().get_mut(key) {
            conns.retain(|&id| id != conn_id);
        }
    }

    /// Unsubscribe connection from all keys.
    pub fn unsubscribe_all(&self, conn_id: u64) {
        let mut subs = self.subscriptions.write().unwrap();
        for conns in subs.values_mut() {
            conns.retain(|&id| id != conn_id);
        }
    }
}

impl ConnectionState {
    /// Track subscribed keys for cleanup.
    pub fn subscribe(&mut self, shared: &SharedServerState, key: &str) {
        shared.subscribe(self.connection_id, key);
        self.subscribed_keys.insert(key.to_string());
    }
}
```

## Bounded Channel Configuration

```rust
/// Size of broadcast channel per connection.
/// When full, new messages are dropped (try_send fails).
/// This prevents slow clients from blocking the server.
const BROADCAST_CHANNEL_SIZE: usize = 32;

// In handle_connection:
let (broadcast_tx, broadcast_rx) = async_channel::bounded(BROADCAST_CHANNEL_SIZE);
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_broadcast_to_subscribers() {
        let shared = SharedServerState::new(None, Duration::from_millis(100));

        // Setup two connections
        let (tx1, rx1) = async_channel::bounded(10);
        let (tx2, rx2) = async_channel::bounded(10);

        shared.register_connection(1, tx1);
        shared.register_connection(2, tx2);

        // Both subscribe to same key
        shared.subscribe(1, "todos:abc");
        shared.subscribe(2, "todos:abc");

        // Broadcast from connection 1
        shared.broadcast("todos:abc", TypeId::of::<()>(), ChangeSet::all(), 1);

        // Connection 1 should NOT receive (sender excluded)
        assert!(rx1.is_empty());

        // Connection 2 SHOULD receive
        let msg = rx2.try_recv().unwrap();
        assert!(matches!(msg, BroadcastMsg::StateChanged { key, .. } if key == "todos:abc"));
    }

    #[async_std::test]
    async fn test_broadcast_ignores_unsubscribed() {
        let shared = SharedServerState::new(None, Duration::from_millis(100));

        let (tx1, rx1) = async_channel::bounded(10);
        let (tx2, rx2) = async_channel::bounded(10);

        shared.register_connection(1, tx1);
        shared.register_connection(2, tx2);

        // Only connection 1 subscribes
        shared.subscribe(1, "todos:abc");

        // Broadcast from connection 1
        shared.broadcast("todos:abc", TypeId::of::<()>(), ChangeSet::all(), 1);

        // Neither should receive (1 is sender, 2 not subscribed)
        assert!(rx1.is_empty());
        assert!(rx2.is_empty());
    }

    #[async_std::test]
    async fn test_broadcast_drops_when_channel_full() {
        let shared = SharedServerState::new(None, Duration::from_millis(100));

        // Small channel
        let (tx, rx) = async_channel::bounded(2);
        shared.register_connection(1, tx);
        shared.subscribe(1, "key");

        // Fill channel
        shared.broadcast("key", TypeId::of::<()>(), ChangeSet::all(), 0);
        shared.broadcast("key", TypeId::of::<()>(), ChangeSet::all(), 0);

        // Third should be dropped (no panic, no block)
        shared.broadcast("key", TypeId::of::<()>(), ChangeSet::all(), 0);

        assert_eq!(rx.len(), 2);
    }

    #[async_std::test]
    async fn test_unregister_cleans_subscriptions() {
        let shared = SharedServerState::new(None, Duration::from_millis(100));

        let (tx, _rx) = async_channel::bounded(10);
        shared.register_connection(1, tx);

        shared.subscribe(1, "key1");
        shared.subscribe(1, "key2");

        shared.unregister_connection(1);

        let subs = shared.subscriptions.read().unwrap();
        assert!(!subs.get("key1").map_or(false, |v| v.contains(&1)));
        assert!(!subs.get("key2").map_or(false, |v| v.contains(&1)));
    }
}
```

## Checklist

- [ ] Add `futures` crate dependency
- [ ] Implement `BroadcastMsg` enum
- [ ] Update connection event loop with `select!`
- [ ] Implement `handle_broadcast()` function
- [ ] Implement `SharedServerState::broadcast()`
- [ ] Add subscription management methods
- [ ] Configure bounded channel size
- [ ] Handle channel-full gracefully (drop message)
- [ ] Add unit tests
- [ ] Test with multiple browser tabs
- [ ] Run `cargo clippy`
