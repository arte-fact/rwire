# rwire

A WebSocket-based UI framework where the server owns all state and renders DOM via a compact binary protocol. The browser runs a minimal ~1.5KB JavaScript runtime that executes DOM opcodes and sends events back to the server.

## Concept

Traditional web frameworks ship large JavaScript bundles to the browser. rwire inverts this:

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
use rwire::{el, handler, renderer, El, Ev, Server, State};

#[derive(State, Default)]
#[storage(memory)]
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
| `0x30` | BIND_LOCAL | `[ref, event, handler]` | Bind local event handler |
| `0x31` | BIND_REMOTE | `[ref, event, handler]` | Bind remote event handler |
| `0x34` | BIND_REMOTE_PARAM | `[ref, event, handler, len, params...]` | Bind handler with item params |
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

### Dynamic Content with ItemRef

For lists and dynamic content, use `ItemRef<T>` and `iter_with_ref()` for type-safe item binding:

```rust
use rwire::{el, El, Ev, handler, renderer, State, ItemRef, IterWithRef, EventContext};

#[derive(State, Default)]
#[storage(memory)]
struct TodoState {
    items: Vec<TodoItem>,
}

#[derive(Default, Clone)]
struct TodoItem {
    text: String,
    done: bool,
}

#[renderer]
fn render_items(state: &TodoState) -> ElementBuilder {
    el(El::Ul).append(
        // iter_with_ref() yields (ItemRef<T>, &T) tuples
        state.items.iter_with_ref().map(|(item_ref, item)| {
            el(El::Li)
                .text(&item.text)
                // on_ref() binds handler with item reference
                .on_ref(Ev::Click, toggle_item(), item_ref)
        })
    )
}

#[handler]
fn toggle_item(state: &mut TodoState, ctx: &EventContext) {
    // Extract ItemRef from context and use it to access the item
    if let Some(item_ref) = ctx.item_ref::<TodoItem>() {
        if let Some(item) = item_ref.get_mut(&mut state.items) {
            item.done = !item.done;
        }
    }
}
```

## Project Structure

```
rwire/
├── rwire/               # Core library
│   ├── src/
│   │   ├── builder.rs   # Fluent element builder API
│   │   ├── capsule.rs   # HTTP capsule serving
│   │   ├── capsule_gen.rs # Tree-shaken capsule generation
│   │   ├── config.rs    # Server configuration
│   │   ├── form.rs      # Form handling utilities
│   │   ├── health.rs    # Health check endpoints
│   │   ├── item_ref.rs  # ItemRef for dynamic content
│   │   ├── metrics.rs   # Prometheus metrics
│   │   ├── protocol/    # Binary opcode encoder/decoder
│   │   ├── registry.rs  # Connection registry
│   │   ├── router.rs    # URL-based routing
│   │   ├── server.rs    # WebSocket server
│   │   ├── session.rs   # Session management
│   │   ├── state.rs     # Reactive state management
│   │   ├── store.rs     # State persistence
│   │   └── style.rs     # Styling utilities
├── rwire-macros/        # Proc macros (#[handler], #[renderer], #[derive(State)])
└── examples/
    ├── counter/         # Counter example app
    ├── todolist/        # Todo list with filtering
    └── todo-combined/   # Todo list with ItemRef
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
| `El::Ul` | `<ul>` |
| `El::Li` | `<li>` |
| `El::Label` | `<label>` |
| `El::Textarea` | `<textarea>` |
| `El::Select` | `<select>` |
| `El::Option` | `<option>` |

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

## Roadmap

### Completed
- [x] Multi-state support (local, memory, persisted)
- [x] ItemRef for dynamic list binding
- [x] Local handler mutations (client-side)
- [x] Router, form, and style helpers
- [x] Health checks and metrics

### Planned
- [ ] Keyed children (virtual DOM-like diffing)
- [ ] Event delegation for large lists
- [ ] Database persistence adapters
- [ ] Authentication middleware
- [ ] SSR support

## License

MIT
