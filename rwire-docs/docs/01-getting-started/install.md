---
title: Installation
description: Install rwire and create your first project
order: 1
---

# Installation

## Prerequisites

- **Rust 1.75+** -- install via [rustup.rs](https://rustup.rs) if you don't have it
- **A modern browser** -- any browser with WebSocket support (Chrome, Firefox, Safari, Edge)

No Node.js, no bundler, no build tools. Just Rust.

## Create a New Project

```bash
cargo new my-app
cd my-app
```

Add rwire and the async runtime to your `Cargo.toml`:

```toml
[dependencies]
rwire = "0.1"
async-std = { version = "1.12", features = ["attributes"] }
```

Or use cargo-add:

```bash
cargo add rwire
cargo add async-std --features attributes
```

## Hello World

Replace `src/main.rs` with:

```rust
use rwire::{el, El, Server};

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Server::bind("127.0.0.1:9000")?
        .root(|| {
            el(El::Div).append([
                el(El::H1).text("Hello, rwire!"),
                el(El::P).text("Server-rendered UI over WebSocket."),
            ])
        })
        .run()
        .await
}
```

Run it:

```bash
cargo run
```

Open [http://127.0.0.1:9000](http://127.0.0.1:9000) in your browser.

## What Just Happened

When you open that URL, here is the sequence:

1. The server responds to the HTTP request with a **capsule** -- a small HTML page containing only a generated JavaScript runtime (~1.5KB) and a WebSocket connection setup. There is no application HTML in this initial page.
2. The browser executes the capsule JS, which opens a WebSocket connection back to the server.
3. The server calls your `root` function to build the element tree, encodes it as **binary opcodes** (CREATE element, SET_TEXT, APPEND child, etc.), and sends the bytes over the WebSocket.
4. The capsule JS executes those opcodes against the real DOM, and your UI appears.

All of this happens in a single round-trip. The capsule JS is tree-shaken at startup to include only the element types and event types your app actually uses -- a counter app gets roughly 1.5KB of JavaScript total.

No virtual DOM, no hydration, no client-side framework. The server owns the state and the DOM structure. The browser is a thin rendering layer.

## Optional Features

Enable markdown rendering support:

```toml
rwire = { version = "0.1", features = ["docs"] }
```

## Next Steps

Continue to the [Quick Start](./quick-start) guide to build a fully interactive counter app with state, handlers, and reactive rendering.
