# rwire

A WebSocket-based UI framework where the server owns all state and renders DOM via a compact binary protocol. The browser runs a small (~13KB) JavaScript runtime that executes DOM opcodes and sends events back to the server.

## Concept

Traditional web frameworks ship large JavaScript bundles to the browser. rwire inverts this:

```
┌─────────────────┐                    ┌─────────────────┐
│   Rust Server   │                    │     Browser     │
│                 │                    │                 │
│  ┌───────────┐  │   Binary Opcodes   │  ┌───────────┐  │
│  │   State   │  │ =================> │  │  Minimal  │  │
│  │  + Logic  │  │                    │  │  Runtime  │  │
│  └───────────┘  │                    │  │  (~13KB)  │  │
│                 │   Event Messages   │  └───────────┘  │
│                 │ <================= │                 │
└─────────────────┘                    └─────────────────┘
```

**Benefits:**
- **Tiny client footprint**: a ~13KB runtime; element/event names and CSS stream lazily over the wire
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

### Capsule Size

The capsule ships only the runtime. The element/event/attribute **name maps** start empty, and the utility **CSS** is not embedded — both are delivered lazily over the WebSocket the first time a connection references them, deduped per connection:

```javascript
// Capsule ships empty maps
const E={},V={},P={},Y={},AT={},AV={},SE={};

// The server streams names on first use (MAP_DEF), e.g. for a counter:
//   E[0]="div", E[1]="span", E[2]="button"; V[1]="click"
```

So a minimal app's capsule is ~17KB (~13KB runtime + ~4KB global CSS), and it stays small as the app grows — a connection only ever receives the names and CSS its pages actually render. See `apps/rwire-docs/docs/05-advanced/tree-shaking.md`.

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

## Styling & Theming

### Style Tokens

Styles are applied with the `St` token enum (700+ utility tokens) rather than CSS files. Tokens compile to compact CSS classes whose rules are delivered lazily over the wire on first use:

```rust
el(El::Div)
    .st([St::BgApp, St::Px4, St::Py2])
    .hover([St::BgSubtle])   // pseudo-class styles
    .sm([St::Px6]);          // responsive breakpoint
```

### Theme as State

The theme is a framework-provided state type. Handlers mutate `&mut Theme`, and a built-in renderer converts it to CSS variables — so mode/accent/palette changes are reactive, with no page reload:

```rust
Server::bind("127.0.0.1:9000")?
    .root(app)
    .theme(Theme::dark().palette(palettes::nord()))
    .run()
    .await

#[handler]
fn toggle_mode(theme: &mut Theme) {
    theme.mode = theme.mode.toggle();
}
```

`rwire-themes` ships ready-made palettes: `nord`, `indigo`, `catppuccin`, `dracula`, `solarized`, `gruvbox`, `tokyo_night`, `rose_pine`, `one_dark`.

### Component Library

`rwire-components` provides 55 prebuilt components (buttons, cards, modals, navigation, forms, chat — `Composer`/`ChatScroll` — status — `Chip`/`Badge`/`StatusDot` — …), all built from `St` tokens.

## Project Structure

```
rwire/
├── libs/
│   ├── rwire/               # Core framework: builder, protocol, state, router,
│   │                        #   store, theme, style_tokens, attr_tokens, tokens/
│   ├── rwire-macros/        # Proc macros: #[handler], #[renderer], #[derive(State)], #[theme]
│   ├── rwire-components/    # UI component library (55 components)
│   ├── rwire-themes/        # Predefined palettes + style presets
│   └── rwire-markdown/      # Markdown rendering for docs
├── apps/
│   ├── rwire-website/       # Marketing landing page
│   ├── rwire-docs/          # Documentation site
│   ├── rwire-design-system/ # Component catalog / showcase
│   └── rwire-examples/      # Examples gallery
└── examples/
    ├── counter/             # Simple counter
    ├── todolist/            # Todo list with filtering
    ├── todo-combined/       # Todo list with ItemRef + JSON file persistence
    └── fine-grained/        # Fine-grained reactivity demo
```

## Supported Elements

A common subset (see the `El` enum for the full list):

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

A common subset (see the `Ev` enum for the full list):

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
- [x] Multi-state support (memory, persisted)
- [x] ItemRef for dynamic list binding
- [x] Style token system + reactive theming (palettes, dark/light, style presets)
- [x] Component library (55 components)
- [x] Client actions (Target/Selector) and CSS transitions
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
