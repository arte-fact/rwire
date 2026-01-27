# wire-wasm

A WebSocket-based UI framework where the server owns all state and renders DOM via a compact binary protocol. The browser runs a minimal ~1.5KB JavaScript runtime that executes DOM opcodes and sends events back to the server.

## Concept

Traditional web frameworks ship large JavaScript bundles to the browser. wire-wasm inverts this:

```
┌─────────────────┐                    ┌─────────────────┐
│   Rust Server   │                    │     Browser     │
│                 │                    │                 │
│  ┌───────────┐  │   Binary Opcodes   │  ┌───────────┐  │
│  │   State   │  │ =================> │  │  Minimal  │  │
│  │  + Logic  │  │                    │  │  Runtime  │  │
│  └───────────┘  │                    │  │  (~1.5KB) │  │
│                 │   Event Messages   │  └───────────┘  │
│                 │ <================= │                 │
└─────────────────┘                    └─────────────────┘
```

**Benefits:**
- **Tiny client footprint**: ~1.5KB JavaScript runtime (tree-shaken per app)
- **No client-side state management**: All state lives on the server
- **Rust everywhere**: Write UI logic in Rust with full type safety
- **Binary protocol**: Compact opcodes minimize bandwidth
- **Reactive updates**: Only changed elements re-render

## Quick Start

```rust
use wire_wasm::{el, handler, renderer, ClientState, El, Ev, Server};

#[derive(ClientState, Default)]
struct Counter {
    count: i32,
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Server::bind("127.0.0.1:9000")?
        .root(build_counter)
        .run()
        .await
}

fn build_counter() -> ElementBuilder {
    el(El::Div).class("counter").append([
        el(El::Button).text("-").on(Ev::Click, decrement()),
        render_count(),
        el(El::Button).text("+").on(Ev::Click, increment()),
    ])
}

#[renderer]
fn render_count(state: &Counter) -> ElementBuilder {
    el(El::Span).text(&state.count.to_string())
}

#[handler]
fn increment(state: &mut Counter) {
    state.count += 1;
}

#[handler]
fn decrement(state: &mut Counter) {
    state.count -= 1;
}
```

Run with:
```bash
cargo run -p counter
# Open http://127.0.0.1:9000
```

## Architecture

### Binary Protocol

DOM operations are encoded as single-byte opcodes followed by arguments:

| Opcode | Name | Format | Description |
|--------|------|--------|-------------|
| `0x02` | CREATE | `[type]` | Create element, returns ref |
| `0x10` | SET_CLASS | `[ref, sym]` | Set CSS class |
| `0x11` | SET_TEXT | `[ref, sym]` | Set text content |
| `0x12` | SET_ATTR | `[ref, attr, val]` | Set attribute |
| `0x20` | APPEND | `[parent, child]` | Append child to parent |
| `0x30` | BIND_LOCAL | `[ref, event, handler]` | Bind event handler |
| `0xF0` | SYMBOLS | `[count, ...strings]` | Symbol table header |
| `0xFF` | END | | End of batch |

Strings are interned into a per-message symbol table, referenced by index. This keeps repeated strings (class names, text) compact.

### Tree Shaking

The JavaScript capsule is generated at startup with only the element types and events your app actually uses:

```javascript
// Full mappings (unused code)
const E={0:'div',1:'span',2:'button',3:'input',4:'p',5:'h1',6:'h2',7:'a',16:'form'};
const V={1:'click',2:'dblclick',3:'mousedown',4:'mouseup',5:'mousemove',...};

// Tree-shaken for counter app
const E={0:'div',1:'span',2:'button'};
const V={1:'click'};
```

The server analyzes your root component at startup and generates a minimal capsule containing only what's needed.

### Reactive State

Components marked with `#[renderer]` automatically re-render when state changes:

```rust
#[renderer]
fn render_count(state: &Counter) -> ElementBuilder {
    el(El::Span).text(&state.count.to_string())
}
```

When any `#[handler]` modifies state, all renderers are re-evaluated and minimal DOM updates are sent to the browser.

## Project Structure

```
wire-wasm/
├── wire-wasm/           # Core library
│   ├── src/
│   │   ├── builder.rs   # Fluent element builder API
│   │   ├── capsule.rs   # HTTP capsule serving
│   │   ├── capsule_gen.rs # Tree-shaken capsule generation
│   │   ├── protocol/    # Binary opcode encoder/decoder
│   │   ├── server.rs    # WebSocket server
│   │   └── state.rs     # Reactive state management
├── wire-wasm-macros/    # Proc macros (#[handler], #[renderer], etc.)
└── examples/
    └── counter/         # Counter example app
```

## Supported Elements

| Type | Tag |
|------|-----|
| `El::Div` | `<div>` |
| `El::Span` | `<span>` |
| `El::Button` | `<button>` |
| `El::Input` | `<input>` |
| `El::P` | `<p>` |
| `El::H1` | `<h1>` |
| `El::H2` | `<h2>` |
| `El::A` | `<a>` |
| `El::Form` | `<form>` |

## Supported Events

| Type | Event |
|------|-------|
| `Ev::Click` | click |
| `Ev::DblClick` | dblclick |
| `Ev::MouseDown` | mousedown |
| `Ev::MouseUp` | mouseup |
| `Ev::MouseMove` | mousemove |
| `Ev::Submit` | submit |
| `Ev::Input` | input |
| `Ev::Change` | change |
| `Ev::KeyDown` | keydown |
| `Ev::KeyUp` | keyup |
| `Ev::Focus` | focus |
| `Ev::Blur` | blur |

## License

MIT
