# CLAUDE.md - wire-wasm

## Project Overview

wire-wasm is a WebSocket-based UI framework where the server owns all state and renders DOM via a compact binary protocol. The browser runs a minimal ~1.5KB JavaScript runtime that executes DOM opcodes and sends events back to the server.

## Quick Start

```bash
# Run the counter example
cargo run -p counter
# Open http://127.0.0.1:9000
```

## Architecture

```
Server (Rust)                    Browser (JS ~1.5KB)
┌─────────────────┐              ┌─────────────────┐
│ State + Logic   │  ──binary──> │ Opcode Executor │
│ ElementBuilder  │  <──events── │ Event Sender    │
└─────────────────┘              └─────────────────┘
```

**Key insight**: All application logic lives on the server. The browser is a thin rendering layer.

## Crate Structure

```
wire-wasm/
├── wire-wasm/           # Core library
│   ├── builder.rs       # Fluent el() API, BuildContext, tree-shaking
│   ├── capsule.rs       # HTTP serving for capsule HTML
│   ├── capsule_gen.rs   # JS runtime generation with tree-shaking
│   ├── protocol/        # Binary opcode encoder/decoder
│   │   ├── opcodes.rs   # El, Ev enums and byte constants
│   │   ├── encoder.rs   # OpcodeBuffer for building messages
│   │   └── decoder.rs   # ClientEvent parsing
│   ├── server.rs        # WebSocket server, connection handling
│   └── state.rs         # ClientState trait, HandlerFn, Renderer
├── wire-wasm-macros/    # Proc macros
│   └── lib.rs           # #[handler], #[renderer], #[derive(ClientState)]
└── examples/counter/    # Example app
```

## Binary Protocol

Single-byte opcodes followed by arguments. Strings are interned in a symbol table.

| Opcode | Hex | Format | Description |
|--------|-----|--------|-------------|
| SYMBOLS | 0xF0 | `[count, len, bytes...]` | Symbol table |
| GET_BY_ID | 0x01 | `[sym]` | Get element by ID |
| CREATE | 0x02 | `[type]` | Create element |
| SET_CLASS | 0x10 | `[ref, sym]` | Set className |
| SET_TEXT | 0x11 | `[ref, sym]` | Set textContent |
| SET_ATTR | 0x12 | `[ref, attr, val]` | setAttribute |
| SET_DATA | 0x14 | `[ref, key, val]` | dataset[key]=val |
| APPEND | 0x20 | `[parent, child]` | appendChild |
| BIND_LOCAL | 0x30 | `[ref, ev, handler]` | addEventListener |
| BIND_DEBOUNCED | 0x33 | `[ref, ev, handler, ms_hi, ms_lo]` | Debounced event |
| BATCH_END | 0xFF | | End of message |

Symbol indices: 0x00-0x7F reserved (e.g., 0x04="id"), 0x80-0xFF session-specific.

## Key Patterns

### Adding a New Element Type

1. Add constant in `protocol/opcodes.rs`:
   ```rust
   pub const EL_TEXTAREA: u8 = 0x08;
   ```
2. Add variant to `El` enum and `as_u8()` match
3. Add to `ELEMENT_MAPPINGS` in `capsule_gen.rs`:
   ```rust
   (8, "textarea"),
   ```

### Adding a New Event Type

1. Add constant in `protocol/opcodes.rs`:
   ```rust
   pub const EV_SCROLL: u8 = 0x0D;
   ```
2. Add variant to `Ev` enum and `as_u8()` match
3. Add to `EVENT_MAPPINGS` in `capsule_gen.rs`:
   ```rust
   (13, "scroll"),
   ```

### Creating Components

```rust
use wire_wasm::{el, El, Ev, ClientState, handler, renderer, ElementBuilder};

#[derive(ClientState, Default)]
struct MyState { value: i32 }

fn my_component() -> ElementBuilder {
    el(El::Div).class("container").append([
        el(El::Button).text("Click").on(Ev::Click, handle_click()),
        render_value(),  // Synced region
    ])
}

#[renderer]
fn render_value(state: &MyState) -> ElementBuilder {
    el(El::Span).text(&state.value.to_string())
}

#[handler]
fn handle_click(state: &mut MyState) {
    state.value += 1;
}
```

## Tree Shaking

The capsule JS is generated with only used element/event types:
- `BuildContext::collect_symbols()` tracks `used_elements` and `used_events`
- `capsule_gen::generate_capsule()` filters `ELEMENT_MAPPINGS` and `EVENT_MAPPINGS`
- Result: Counter app capsule is ~1.5KB instead of ~2KB

## Testing

```bash
cargo test --workspace
```

## Common Issues

### "Address already in use"
Kill existing process: `fuser -k 9000/tcp`

### Handler not firing
- Check event type matches (Ev::Click vs Ev::Submit)
- Verify handler is registered (handler index in BIND_LOCAL opcode)

### State not updating
- Ensure state type derives `ClientState`
- Check renderer is marked with `#[renderer]`

## Design Decisions

1. **Server-side state**: Simplifies client, enables server-side validation, natural fit for Rust.

2. **Binary protocol**: Compact, fast to parse, symbol interning reduces repetition.

3. **Tree shaking at startup**: Analyze root once, serve minimal capsule to all clients.
