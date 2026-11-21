---
title: Async Actions
description: Controls that reflect background work automatically, with no key to author
order: 7
---

# Async Actions

```rust
use rwire::{handler, EventContext, State};

#[handler]
fn deliver(state: &mut App, ctx: &EventContext) {
    let resolver = ctx.defer();          // hand this action to a background thread
    std::thread::spawn(move || {
        let result = do_the_work();      // slow: a merge, an upload, an LLM call
        match result {
            Ok(()) => resolver.ok(),     // control returns to idle
            Err(e) => resolver.err(e),   // control shows the error
        }
    });
}
```

```rust
// The button is plain. Because its handler defers, it reflects the whole lifecycle.
Button::primary("Deliver").on_click(deliver())
```

A handler that hands work to a background thread calls `ctx.defer()`. That marks the action **loading** and pushes the fact to the browser, so the control that triggered it spins in place — no extra client event. When the work finishes, the worker calls `resolver.ok()` (the control returns to idle) or `resolver.err(message)` (the control turns destructive and carries the message). The control you already wrote reflects all of this: no `ActionButton`, no key, no loading markup.

## The contract

> Any control that triggers deferred work shows loading and surfaces its error automatically. A handler that returns without deferring shows nothing.

Coverage is complete because deferral is observed at runtime, not guessed from a signature. An immediate handler — one that mutates and returns without `defer()` — resolves in the same round-trip, so its control never spins:

```rust
#[handler]
fn bump(state: &mut App) {
    state.count += 1;   // no defer(): instant, the button never spins
}
```

## Resolver

`ctx.defer()` returns a `Resolver`. It is `Send`, so it moves into the background thread, and it resolves the action exactly once:

| Call | Effect |
|------|--------|
| `resolver.ok()` | Success — the action entry is cleared, the control returns to idle. |
| `resolver.err(message)` | Failure — the action becomes `Errored`; the message surfaces on the control and any page-level error slot. |

Success is *not* stored — only in-flight loading and unacknowledged errors live in the registry, so it stays bounded. If a `Resolver` is dropped without resolving, a 30-second timeout flips the stuck action to errored so nothing hangs forever.

A `Resolver` may also be resolved before the handler returns (the work turned out to be instant, or failed up front): the action then settles as soon as the handler returns, with no loading phase — success leaves the control idle, an error shows at once.

## Per-instance identity

An action is keyed by `(session, handler_id, param)` — all already on the event binding, so you author no correlation key. Give each control in a list its own `param` and the same handler drives them independently; only the control you click reflects:

```rust
requests.iter().map(|r| {
    let spec = deliver().with_param_bytes(r.id_bytes());
    Button::primary("Deliver").on_click(spec)
})
```

## Which controls reflect

Reflection is **on by default** for the controls that trigger handlers:

| Control | Binding |
|---------|---------|
| `Button` | `.on_click(handler())` |
| `Composer` | `.on_submit(handler())` |
| `Form` | `.action(handler())` — the submit button reflects |

Opt a control out with `.reflect(false)` for the rare case that shouldn't show feedback.

## Surfacing errors on the page

By default an error recolours the control and shows the message on hover. For a non-colocated surface — a failure that should be visible wherever the user is looking — drop one of these near the page root. Both read the same live error list and clear as failed actions are retried:

```rust
use rwire_components::{ActionErrors, ActionToasts};

ActionErrors::new().build()   // inline stack of alerts
ActionToasts::new().build()   // fixed, bottom-right toast notifications
```

## When to reach for it

Use `ctx.defer()` whenever a handler starts work that outlives the event: a git operation, a file write, an upload, a model call. Handlers that only mutate state return normally and stay instant — you never opt those in, and you never have to remember which is which.
