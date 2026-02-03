# CLAUDE.md - rwire

## Project Overview

rwire is a WebSocket-based UI framework where the server owns all state and renders DOM via a compact binary protocol. The browser runs a minimal ~1.5KB JavaScript runtime that executes DOM opcodes and sends events back to the server.

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
rwire/
├── rwire/               # Core library
│   ├── builder.rs       # Fluent el() API, BuildContext, tree-shaking
│   ├── capsule.rs       # HTTP serving for capsule HTML
│   ├── capsule_gen.rs   # JS runtime generation with tree-shaking
│   ├── config.rs        # Server configuration (bind address, max connections)
│   ├── form.rs          # Form builder and validation rules
│   ├── health.rs        # Health check endpoints (/health, /ready)
│   ├── item_ref.rs      # ItemRef<T> for type-safe dynamic content binding
│   ├── metrics.rs       # Prometheus-format metrics (counters, gauges, histograms)
│   ├── protocol/        # Binary opcode encoder/decoder
│   │   ├── opcodes.rs   # El, Ev enums and byte constants
│   │   ├── encoder.rs   # OpcodeBuffer for building messages
│   │   ├── decoder.rs   # ClientEvent parsing
│   │   └── varint.rs    # Variable-length integer encoding
│   ├── registry.rs      # Connection registry with admission control
│   ├── router.rs        # URL pattern matching and client-side routing
│   ├── server.rs        # WebSocket server, connection handling
│   ├── session.rs       # Session ID generation and cookie management
│   ├── state.rs         # State traits, HandlerFn, EventContext, Renderer
│   ├── store.rs         # State persistence (MemoryStore, JsonFileStore)
│   └── style.rs         # CSS-in-Rust styling utilities
├── rwire-macros/        # Proc macros
│   ├── lib.rs           # #[handler], #[renderer], #[derive(State)]
│   └── mutation_parser.rs # Local handler mutation analysis
└── examples/
    ├── counter/         # Simple counter app
    ├── todolist/        # Todo list with filtering
    └── todo-combined/   # Todo list with ItemRef dynamic binding
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
| BIND_LOCAL | 0x30 | `[ref, ev, handler]` | addEventListener (local) |
| BIND_REMOTE | 0x31 | `[ref, ev, handler]` | Server round-trip event |
| BIND_DEBOUNCED | 0x33 | `[ref, ev, handler, ms_hi, ms_lo]` | Debounced event |
| BIND_REMOTE_PARAM | 0x34 | `[ref, ev, handler, len, params...]` | Event with item params |
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
use rwire::{el, El, Ev, State, handler, renderer, ElementBuilder};

#[derive(State, Default)]
#[storage(memory)]
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

**Key benefits:**
- No data attributes or string parsing
- Type-safe: ItemRef<T> ensures you access the right collection
- Efficient: encodes as varint (1-3 bytes vs JSON strings)
- Copy: ItemRef can be used multiple times per element

### State Storage Types

rwire supports three storage types for state:

```rust
// Local state - client-side only, no server round-trip
#[derive(State, Default)]
#[storage(local)]
struct UIState { menu_open: bool }

// Memory state - server-side, lost on restart
#[derive(State, Default)]
#[storage(memory)]
struct AppState { count: i32 }

// Persisted state - survives server restart
#[derive(State, Default)]
#[storage(persisted)]
struct UserData { name: String }
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

## Code Quality

### Before Committing

Always run these checks before committing code:

```bash
# Check for warnings and lint issues
cargo clippy --workspace

# Run all tests
cargo test --workspace

# Format code (if rustfmt is configured)
cargo fmt --all
```

**Goal: Zero warnings.** All clippy warnings should be fixed, not suppressed.

### Code Style Guidelines

**Prefer:**
- `entry().or_insert_with()` over `contains_key()` + `insert()` for HashMap
- `is_some_and()` / `is_none_or()` over `map_or(false/true, ...)`
- `strip_prefix()` / `strip_suffix()` over `starts_with()` + manual slicing
- `&[T]` over `&Vec<T>` in function parameters
- Eliding lifetimes when the compiler can infer them
- `.to_string()` over `format!("{}", x)` for simple conversions

**Avoid:**
- Unused imports, fields, methods, or traits
- `#[allow(dead_code)]` - remove dead code instead
- Backwards-compatibility shims for internal code
- Over-engineering: no abstractions for one-time operations

### Dead Code Policy

- **Remove unused code immediately** - don't comment it out or mark with `_`
- **Delete deprecated APIs** once migration is complete
- **Remove feature flags** for features that are now always-on
- **Clean up plan documents** after implementation is done

### When to Refactor

Refactor when:
- Adding a third similar pattern (Rule of Three)
- A function exceeds ~50 lines
- A module exceeds ~500 lines
- Test coverage for a module is below 60%

Don't refactor:
- During unrelated feature work
- Without test coverage
- For purely aesthetic reasons

### Deprecation Process

- We are in an experimental phase; breaking changes are allowed.
- The only consumers are internal examples. Do breaking changes, then update examples using compiler errors as guidance.
- No need for formal deprecation warnings or versioning.

### Test Coverage

Each module should have:
- Unit tests in the module (`#[cfg(test)] mod tests`)
- Integration tests in `tests/` for public API
- Edge case coverage (empty inputs, large inputs, error conditions)

Test file naming: `tests/<module_name>.rs`

## E2E Debugging

When debugging browser interactions, use the Playwright MCP tools. A helper script is available:

```bash
# Restart the todo-combined server (kills existing, rebuilds, starts fresh)
./restart-server.sh

# Server logs go to /tmp/server.log
cat /tmp/server.log
```

### Debugging Workflow

1. **Start server**: `./restart-server.sh` or `cargo run -p todo-combined`
2. **Navigate**: Use `mcp__plugin_playwright_playwright__browser_navigate` to open http://127.0.0.1:9000
3. **Inspect DOM**: Use `mcp__plugin_playwright_playwright__browser_snapshot` to see element refs
4. **Interact**: Use `browser_click`, `browser_type` with element refs from snapshot
5. **Check logs**: `cat /tmp/server.log` for server-side events and errors
6. **Console**: Use `browser_console_messages` to check JS errors

### Tips

- Element refs (e.g., `ref=e33`) are used to target elements for interaction
- Always check server log after interactions to verify events are received
- For persistence debugging, restart server and check "Hydrated X entries" message
- Use `browser_snapshot` after clicks to see updated DOM state

## Common Issues

### "Address already in use"
Kill existing process: `fuser -k 9000/tcp`

### Handler not firing
- Check event type matches (Ev::Click vs Ev::Submit)
- Verify handler is registered (handler index in BIND_LOCAL opcode)

### State not updating
- Ensure state type derives `State`
- Check renderer is marked with `#[renderer]`
- Verify storage type matches expected behavior (local vs memory)

## Design Decisions

1. **Server-side state**: Simplifies client, enables server-side validation, natural fit for Rust.

2. **Binary protocol**: Compact, fast to parse, symbol interning reduces repetition.

3. **Tree shaking at startup**: Analyze root once, serve minimal capsule to all clients.

4. **ItemRef for dynamic content**: Type-safe item binding without runtime string parsing or data attributes.

5. **Varint encoding**: Compact wire format for element refs and item indices (1-3 bytes instead of fixed 4).
