---
title: Authentication & Identity
description: The built-in login gate, session identity in handlers, and where authorization lives
order: 8
---
# Authentication & Identity

rwire ships a deliberately small auth story: a **single-credential login
gate** guarding the whole app, and **session identity in handlers** for
building your own per-user logic. Multi-user credential stores, roles, and
OAuth are application concerns (or a future crate) — the framework gives you
the primitives and stays out of the way.

## The login gate

```rust
Server::bind("0.0.0.0:9000")?
    .root(app)
    .auth("admin", std::env::var("APP_PASSWORD")?)  // gate page + WebSocket
    .auth_brand("My Dashboard")                     // login page title
    .run()
    .await
```

With `auth` set, unauthenticated requests get a login form (a static
pre-capsule page); a correct login sets an `HttpOnly; SameSite=Strict` token
cookie (7-day TTL, CSPRNG, constant-time compared; `Secure` auto-added behind
an `X-Forwarded-Proto: https` proxy) and every request — including the
WebSocket upgrade — requires it. `GET /logout` clears it. For local tooling,
`.dev_session("token")` pre-authorizes a fixed token.

Run behind TLS: the gate is only as good as the transport.

## Who is this connection? `ctx.session_id()`

Handlers on `#[storage(shared)]` state serve *every* connection, and the
event itself carries no identity. The connection's session id does:

```rust
#[handler]
fn join_room(room: &mut Room, ctx: &EventContext) {
    let Some(sid) = ctx.session_id() else { return };
    room.members.insert(sid.to_string(), display_name);
}
```

The id is stable across reconnects for the cookie's lifetime, so it works as
a per-user key for presence, membership, rate limiting, and authorization
maps. It is an opaque random id — map it to your own user model; don't parse
it.

## Where authorization lives

In your handlers. A handler is plain Rust running on the server with the full
state in hand — check `room.members`, an admin set, or anything else before
mutating, and return early otherwise. There is no framework-level permission
layer to configure or misconfigure.
