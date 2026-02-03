# rwire-token: Declarative YAML Apps on rwire

## Vision

Enable building full rwire applications in YAML without writing Rust code, while maintaining full WebSocket reactivity and the efficient binary protocol.

```yaml
# counter.yaml - A complete reactive app
version: "1.0"

state:
  user:
    - id: Counter
      fields:
        - name: count
          type: number
          default: 0

ui:
  pages:
    home:
      tag: div
      children:
        - type: dynamic_text
          expr: { type: state, scope: user, state_id: Counter, path: count }
        - type: node
          tag: button
          handlers:
            - event: click
              actions:
                - action: update_state
                  scope: user
                  state_id: Counter
                  mutations:
                    - op: increment
                      field: count
          children:
            - type: text
              content: "+"
```

```bash
rwire token serve counter.yaml
# → Full reactive app at http://127.0.0.1:9000
```

---

## Goals

1. **No Rust Required** - Define entire apps in YAML
2. **Full Reactivity** - WebSocket updates, not page refreshes
3. **rwire Protocol** - Use existing binary opcodes and 1.5KB JS runtime
4. **Hot Reload** - Instant feedback during development
5. **Production Ready** - Compile to binary for fast startup

---

## Non-Goals

- Code generation (we interpret tokens at runtime)
- Replacing Rust for complex apps (escape hatch to Rust when needed)
- New wire protocol (reuse rwire's existing protocol)

---

## Architecture

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  app.yaml   │───>│  Compiler   │───>│  app.ptok   │
└─────────────┘    └─────────────┘    └─────────────┘
                                             │
                   ┌─────────────────────────┘
                   ▼
┌──────────────────────────────────────────────────────────┐
│                    TokenRuntime                           │
│  ┌────────────┐  ┌────────────┐  ┌────────────────────┐  │
│  │ Token AST  │─>│ Interpreter│─>│ ElementBuilder     │  │
│  └────────────┘  └────────────┘  └────────────────────┘  │
│                                           │              │
│  ┌────────────┐  ┌────────────┐          ▼              │
│  │ Dynamic    │<>│ Handler    │  ┌────────────────────┐  │
│  │ State      │  │ Dispatch   │  │ rwire Protocol     │  │
│  └────────────┘  └────────────┘  │ (opcodes + WS)     │  │
│                                   └────────────────────┘  │
└──────────────────────────────────────────────────────────┘
                                           │
                                           ▼
                                   ┌────────────────┐
                                   │ Browser        │
                                   │ (1.5KB runtime)│
                                   └────────────────┘
```

---

## Roadmap

### Phase 1: Foundation (Token Schema + Interpreter)
**Goal**: Render static YAML to rwire ElementBuilder

- Token schema types (NodeToken, ChildToken, Expr)
- Expression evaluator (literals, state access)
- Interpreter that produces ElementBuilder
- Basic HTTP server serving rendered HTML

**Deliverable**: Static pages render from YAML

---

### Phase 2: State + Handlers
**Goal**: Reactive state mutations via WebSocket

- DynamicState container (global/user scopes)
- Handler registration and dispatch
- Mutation execution (set, increment, toggle, etc.)
- Synced region tracking and invalidation
- WebSocket event flow integration

**Deliverable**: Counter app works with live updates

---

### Phase 3: Dynamic Content
**Goal**: Loops, conditionals, and components

- Loop rendering with index tracking
- ItemRef integration for efficient list updates
- Conditional rendering (if/else)
- Component definitions and references
- Slot system for composition

**Deliverable**: Todo list with add/delete works

---

### Phase 4: Expressions + Styling
**Goal**: Full expression system and design tokens

- Format strings with placeholders
- Comparison operators (eq, ne, lt, gt, etc.)
- Logical operators (and, or, not)
- Ternary expressions
- Integration with rwire design tokens
- Component variants from YAML

**Deliverable**: Styled apps with dynamic text

---

### Phase 5: Binary Format + CLI
**Goal**: Production-ready tooling

- Binary encoder/decoder (.ptok format)
- CLI: validate, compile, serve, run
- Hot reload in dev mode
- Validation with helpful error messages
- Watch mode

**Deliverable**: Full CLI workflow

---

### Phase 6: Advanced Features
**Goal**: Routing, persistence, and escape hatches

- URL routing with parameters
- Route guards (future)
- State persistence hooks
- Custom Rust handler escape hatch
- Task system for async work

**Deliverable**: Multi-page apps with persistence

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Counter app | Works in Phase 2 |
| Todo app | Works in Phase 3 |
| Hot reload latency | < 100ms |
| Compiled binary size | < 10KB for counter |
| JS runtime | Same 1.5KB as rwire |

---

## Crate Structure

```
rwire/
├── rwire/                    # Core library (existing)
├── rwire-macros/             # Proc macros (existing)
├── rwire-token/              # NEW: Token runtime
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── schema.rs         # Token types
│       ├── expr.rs           # Expression types + eval
│       ├── state.rs          # DynamicState
│       ├── interpreter.rs    # Token → ElementBuilder
│       ├── handlers.rs       # Handler dispatch
│       ├── binary/           # .ptok codec
│       │   ├── mod.rs
│       │   ├── encoder.rs
│       │   └── decoder.rs
│       └── validation.rs     # YAML validation
├── rwire-token-cli/          # NEW: CLI tool
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── validate.rs
│       ├── compile.rs
│       ├── serve.rs
│       └── run.rs
└── examples/
    ├── token-counter/        # NEW: YAML counter
    │   └── app.yaml
    └── token-todo/           # NEW: YAML todo
        └── app.yaml
```

---

## Open Questions

1. **State Schema Validation** - How strict? Allow arbitrary fields?
2. **Error Recovery** - Continue rendering on expression eval failure?
3. **Escape Hatch** - How to call Rust handlers from YAML?
4. **Persistence** - Reuse rwire's `#[storage(persisted)]` or custom?
5. **Component Props** - Full prop system or just slots?

---

## References

- [rwire CLAUDE.md](../CLAUDE.md) - Core framework documentation
- [ptok concept](../../rust-smart-ssr/docs/ptok-concept.md) - Original inspiration
- [ptok token reference](../../rust-smart-ssr/docs/pulsar-token-reference.md) - YAML format reference
