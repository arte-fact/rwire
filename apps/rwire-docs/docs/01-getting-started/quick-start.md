---
title: Quick Start
description: Build your first rwire app in 5 minutes
order: 2
---

# Quick Start

This guide walks you through building a counter app with reactive state. You will define a state struct, write event handlers, create a reactive renderer, and wire it all together.

## 1. Create the Project

```bash
cargo new counter-app
cd counter-app
```

Add dependencies to `Cargo.toml`:

```toml
[dependencies]
rwire = "0.1"
async-std = { version = "1.12", features = ["attributes"] }
```

## 2. Define State

State is a Rust struct annotated with `#[derive(State)]`. The `#[storage(memory)]` attribute tells rwire to keep it in server memory (per-session, lost on disconnect).

```rust
use rwire::{State};

#[derive(State, Default)]
#[storage(memory)]
struct Counter {
    count: i32,
}
```

The `Default` derive provides the initial state for each new connection.

## 3. Build the UI

Use the `el()` function to construct a DOM tree. Each call creates an element builder with a fluent API for setting attributes, text, children, and event bindings.

```rust
use rwire::{el, El, Ev, ElementBuilder};

fn app() -> ElementBuilder {
    el(El::Div).class("counter").append([
        el(El::H1).text("Counter"),
        el(El::Button).text("-").on(Ev::Click, decrement()),
        render_count(),
        el(El::Button).text("+").on(Ev::Click, increment()),
    ])
}
```

Notice that `render_count()` is called inline as a child element. This is a **renderer** -- a reactive region that automatically re-renders when its state changes.

## 4. Add Handlers

Handlers are functions that mutate state in response to events. The `#[handler]` macro registers them with the framework and generates the wiring code.

```rust
use rwire::handler;

#[handler]
fn increment(state: &mut Counter) {
    state.count += 1;
}

#[handler]
fn decrement(state: &mut Counter) {
    state.count -= 1;
}
```

When the browser sends a click event, the server looks up the handler by index, calls it with a mutable reference to the session's state, and then re-renders any affected regions.

## 5. Add a Renderer

Renderers are functions that produce UI from the current state. The `#[renderer]` macro marks them as reactive -- the framework tracks which state type they depend on and re-renders them whenever that state changes.

```rust
use rwire::renderer;

#[renderer]
fn render_count(state: &Counter) -> ElementBuilder {
    el(El::Span)
        .class("count")
        .text(&state.count.to_string())
}
```

Only the `<span>` inside this renderer is updated when the count changes. The surrounding `<div>`, `<h1>`, and `<button>` elements are never re-sent.

## 6. Wire It Up

The `Server::bind` function starts the server. Pass your root UI function to `.root()` and call `.run().await` to start listening.

```rust
#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Server::bind("127.0.0.1:9000")?
        .root(app)
        .run()
        .await
}
```

## Complete Code

Here is the full `src/main.rs`:

```rust
use rwire::{el, handler, renderer, El, ElementBuilder, Ev, Server, State};

#[derive(State, Default)]
#[storage(memory)]
struct Counter {
    count: i32,
}

fn app() -> ElementBuilder {
    el(El::Div).class("counter").append([
        el(El::H1).text("Counter"),
        el(El::Button).text("-").on(Ev::Click, decrement()),
        render_count(),
        el(El::Button).text("+").on(Ev::Click, increment()),
    ])
}

#[renderer]
fn render_count(state: &Counter) -> ElementBuilder {
    el(El::Span)
        .class("count")
        .text(&state.count.to_string())
}

#[handler]
fn increment(state: &mut Counter) {
    state.count += 1;
}

#[handler]
fn decrement(state: &mut Counter) {
    state.count -= 1;
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Server::bind("127.0.0.1:9000")?
        .root(app)
        .run()
        .await
}
```

Run it:

```bash
cargo run
# Open http://127.0.0.1:9000
```

## What Happens Under the Hood

When you click the "+" button:

1. The capsule JS captures the click event and sends a binary message over WebSocket: the handler index (1 byte) plus the event type.
2. The server deserializes the event, looks up the handler (`increment`), and calls it with `&mut Counter`.
3. The framework detects that `Counter` changed and re-runs `render_count`.
4. The new element tree is diffed and encoded as binary opcodes. For a count change, this is typically just a `SET_TEXT` opcode with the new value -- about 5-10 bytes.
5. The browser receives the binary message and executes the opcode, updating the DOM in place.

Strings are interned in a **symbol table** that is sent once per connection. The first time "Counter" appears, it costs the full string length. Every subsequent use is a 1-byte index lookup.

The entire interaction -- click to screen update -- is a single WebSocket round-trip. No JSON parsing, no virtual DOM diffing on the client, no hydration.

## Next Steps

Read the [Project Structure](./project-structure) guide to understand how rwire applications are organized, or jump to the [Core Concepts](/docs/02-core-concepts/concepts) section for a deeper look at state, handlers, and renderers.
