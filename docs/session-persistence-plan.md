# Session Persistence Implementation Plan

## Problem Statement

Currently, persisted state cannot be demonstrated because:
1. Each WebSocket connection generates a new `session_id` in `ConnectionState::new()`
2. Refreshing the page creates a new WebSocket connection
3. A new connection means a new `session_id`
4. The old session's persisted data is never loaded for the new connection

**Result**: Users add items, refresh, and items are gone (even though they're in the database).

## Solution Overview

Use HTTP cookies to maintain session identity across page refreshes and reconnections.

```
First Visit:
┌──────────┐                      ┌──────────┐
│ Browser  │ ──GET /───────────→  │ Server   │
│          │ ←─HTML + Set-Cookie─ │          │
│          │   rwire_sid=abc123   │          │
│          │                      │          │
│          │ ──WS + Cookie────→   │          │
│          │   rwire_sid=abc123   │          │
│          │                      │          │
│          │   (uses session      │          │
│          │    abc123 for state) │          │
└──────────┘                      └──────────┘

Page Refresh:
┌──────────┐                      ┌──────────┐
│ Browser  │ ──GET / + Cookie──→  │ Server   │
│          │   rwire_sid=abc123   │          │
│          │ ←─HTML (no new cookie)│         │
│          │                      │          │
│          │ ──WS + Cookie────→   │          │
│          │   rwire_sid=abc123   │          │
│          │                      │          │
│          │   (same session,     │          │
│          │    loads existing    │          │
│          │    persisted state)  │          │
└──────────┘                      └──────────┘
```

## Existing Infrastructure

### session.rs (already implemented)
- `SessionId::generate()` - creates random session ID
- `SessionId::from_cookie(header)` - parses `rwire_sid=xxx` from Cookie header
- `SessionId::to_cookie(max_age)` - generates `Set-Cookie` header value

### capsule.rs
- `serve(stream, capsule)` - serves HTML capsule via HTTP
- Currently doesn't set any cookies

### server.rs
- `handle_client()` - peeks at request to determine HTTP vs WebSocket
- `handle_websocket()` - handles WebSocket connections
- `ConnectionState::new()` - generates new session_id (the problem)

## Implementation Phases

### Phase 1: Extract Cookie from HTTP Request

**File**: `rwire/src/server.rs`

Modify `handle_client()` to parse cookies from the HTTP request:

```rust
// In handle_client(), after peeking at request
let session_id = SessionId::from_cookie(&peek_str)
    .unwrap_or_else(SessionId::generate);
```

Pass `session_id` to both `capsule::serve()` and `handle_websocket()`.

**Changes**:
- Add `session_id` parameter to function signatures
- Parse Cookie header from peek buffer

### Phase 2: Set Cookie in HTTP Response

**File**: `rwire/src/capsule.rs`

Modify `serve()` to accept optional session ID and set cookie:

```rust
pub async fn serve(
    mut stream: TcpStream,
    capsule: &str,
    session_id: &SessionId,
    is_new_session: bool,
) -> Result<(), std::io::Error>
```

If `is_new_session`, include `Set-Cookie` header in response:

```
HTTP/1.1 200 OK
Content-Type: text/html
Set-Cookie: rwire_sid=abc123; Path=/; HttpOnly; SameSite=Strict; Max-Age=31536000
Content-Length: ...
```

**Changes**:
- Add session parameters to `serve()`
- Conditionally add `Set-Cookie` header
- Use 1-year max-age for persistence

### Phase 3: Use Session ID in ConnectionState

**File**: `rwire/src/server.rs`

Modify `ConnectionState::new()` to accept session ID:

```rust
impl ConnectionState {
    fn new(connection_id: u64, session_id: String) -> Self {
        Self {
            connection_id,
            session_id,  // Use provided instead of generating
            // ...
        }
    }
}
```

Modify `handle_websocket()` to pass session ID:

```rust
async fn handle_websocket<F>(
    ws_stream: ...,
    peer_addr: SocketAddr,
    root: Arc<F>,
    shared: Arc<SharedServerState>,
    session_id: SessionId,  // New parameter
) -> Result<...>
```

**Changes**:
- Update `ConnectionState::new()` signature
- Update `handle_websocket()` signature
- Pass session ID through from `handle_client()`

### Phase 4: Cookie Parsing Improvements

**File**: `rwire/src/session.rs`

The current `from_cookie()` implementation may need adjustments:

```rust
// Current: looks for "rwire_session"
// Should use: "rwire_sid" (shorter, matches convention)

pub fn from_cookie(cookie_header: &str) -> Option<Self> {
    Self::from_cookie_named(cookie_header, "rwire_sid")
}
```

Also update `to_cookie()` to use the same name.

**Changes**:
- Rename cookie from `rwire_session` to `rwire_sid`
- Ensure consistent cookie name across parse/generate

### Phase 5: WebSocket Cookie Extraction

The WebSocket upgrade request includes HTTP headers. We need to extract the Cookie header from the upgrade request before it's consumed.

**File**: `rwire/src/server.rs`

In `handle_client()`, the peek buffer contains the full HTTP request including headers:

```
GET / HTTP/1.1
Host: localhost:9000
Upgrade: websocket
Connection: Upgrade
Cookie: rwire_sid=abc123
Sec-WebSocket-Key: ...
```

Parse the Cookie header from this buffer:

```rust
fn extract_cookie_from_request(request: &str) -> Option<String> {
    for line in request.lines() {
        if let Some(value) = line.strip_prefix("Cookie: ") {
            return Some(value.to_string());
        }
        // Case-insensitive check
        if line.to_lowercase().starts_with("cookie:") {
            return Some(line[7..].trim().to_string());
        }
    }
    None
}
```

## Testing Plan

### Manual Testing

1. **Fresh visit**:
   - Clear cookies, visit http://127.0.0.1:9000
   - Check browser DevTools → Application → Cookies
   - Should see `rwire_sid` cookie set

2. **Add items**:
   - Add items to Persisted State list
   - Check server log for dirty key marking

3. **Refresh page**:
   - Refresh browser (F5)
   - Items should still be visible
   - Server log should show "Hydrated" or cache hit

4. **Server restart**:
   - Stop server (Ctrl+C)
   - Restart server
   - Refresh page
   - Items should still be visible (loaded from SQLite)

5. **New session**:
   - Open incognito/private window
   - Should have empty list (new session)

### Automated Testing

Add integration test in `rwire/tests/session.rs`:

```rust
#[test]
fn test_session_cookie_roundtrip() {
    let id = SessionId::generate();
    let cookie = id.to_cookie(Some(Duration::from_secs(3600)));

    // Simulate browser sending cookie back
    let parsed = SessionId::from_cookie(&format!("Cookie: {}", cookie));
    assert_eq!(parsed.map(|s| s.as_str()), Some(id.as_str()));
}
```

## File Changes Summary

| File | Changes |
|------|---------|
| `rwire/src/session.rs` | Rename cookie to `rwire_sid`, add helper functions |
| `rwire/src/capsule.rs` | Add `Set-Cookie` header support in `serve()` |
| `rwire/src/server.rs` | Parse cookies, pass session ID through call chain |
| `rwire/tests/session.rs` | Add roundtrip tests |

## Risks and Mitigations

### Risk: Cookie not sent with WebSocket
**Mitigation**: WebSocket connections inherit cookies from the origin. Same-origin policy ensures cookies are sent.

### Risk: Cookie parsing edge cases
**Mitigation**: Use robust parsing, handle case-insensitivity, multiple cookies.

### Risk: Session ID collision
**Mitigation**: Current implementation uses timestamp + random, collision probability is negligible.

### Risk: Cookie security
**Mitigation**: Use `HttpOnly` (no JS access), `SameSite=Strict` (CSRF protection), `Path=/`.

## Success Criteria

1. Page refresh preserves persisted state items
2. Server restart preserves persisted state items
3. New browser/incognito window gets new empty session
4. No breaking changes to existing examples
5. Zero clippy warnings
6. All tests pass
