---
title: Installation
description: How to install rwire in your project
order: 1
---
# Installation

Add rwire to your `Cargo.toml`:

```toml
[dependencies]
rwire = { path = "rwire" }
async-std = { version = "1.12", features = ["attributes"] }
```

## Requirements

- Rust 1.70 or later
- A WebSocket-capable browser (all modern browsers)

## Optional Features

Enable the `docs` feature for markdown support:

```toml
rwire = { path = "rwire", features = ["docs"] }
```

## Verify Installation

Create a minimal counter app to verify everything works:

```rust
use rwire::{Server, el, El};

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Server::bind("127.0.0.1:9000")?
        .root(|| el(El::Div).text("Hello, rwire!"))
        .run()
        .await
}
```

Run with `cargo run` and open http://127.0.0.1:9000.
