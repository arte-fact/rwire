# Phase 2: State + Handlers

## Goal

Enable reactive state mutations via WebSocket. After this phase, the Counter app will have working increment/decrement buttons with live UI updates.

---

## Deliverables

1. Full `DynamicState` with per-session storage
2. Handler schema types
3. Mutation execution engine
4. Dynamic handler registration with rwire
5. WebSocket integration for live updates
6. Synced region tracking

---

## Prerequisites

- Phase 1 complete (static rendering works)
- rwire's WebSocket server infrastructure

---

## Handler Schema

```rust
// src/schema.rs (additions)

#[derive(Debug, Clone, Deserialize)]
pub struct Handler {
    pub event: Event,
    #[serde(default)]
    pub id: Option<String>,  // For debugging
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Event {
    Click,
    Input,
    Change,
    Submit,
    Focus,
    Blur,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    UpdateState {
        scope: Scope,
        state_id: String,
        mutations: Vec<Mutation>,
    },
    Navigate {
        path: StringOrExpr,
    },
    PreventDefault,
    Log {
        message: StringOrExpr,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct Mutation {
    pub op: MutationOp,
    pub field: String,
    #[serde(default)]
    pub value: Option<Expr>,
    #[serde(default)]
    pub by: Option<Expr>,
    #[serde(default)]
    pub index: Option<Expr>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MutationOp {
    Set,
    Increment,
    Decrement,
    Toggle,
    Push,
    Pop,
    Clear,
    RemoveAt,
}
```

---

## Full DynamicState

```rust
// src/state.rs

use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::schema::{StateSchema, StateDefinition, Scope, Mutation, MutationOp};
use crate::expr::{Expr, EvalContext};

pub type SessionId = String;

/// Thread-safe dynamic state container
#[derive(Clone)]
pub struct DynamicState {
    schema: StateSchema,
    global: Arc<RwLock<HashMap<String, HashMap<String, Value>>>>,
    user: Arc<RwLock<HashMap<SessionId, HashMap<String, HashMap<String, Value>>>>>,
}

impl DynamicState {
    pub fn from_schema(schema: &StateSchema) -> Self {
        let mut global = HashMap::new();

        // Initialize global state with defaults
        for state_def in &schema.global {
            let fields = Self::init_fields(state_def);
            global.insert(state_def.id.clone(), fields);
        }

        Self {
            schema: schema.clone(),
            global: Arc::new(RwLock::new(global)),
            user: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn init_fields(state_def: &StateDefinition) -> HashMap<String, Value> {
        state_def.fields
            .iter()
            .map(|f| (f.name.clone(), f.default.clone()))
            .collect()
    }

    /// Get or create user session state
    fn ensure_user_session(&self, session_id: &SessionId) {
        let mut user = self.user.write().unwrap();
        if !user.contains_key(session_id) {
            let mut session_state = HashMap::new();
            for state_def in &self.schema.user {
                let fields = Self::init_fields(state_def);
                session_state.insert(state_def.id.clone(), fields);
            }
            user.insert(session_id.clone(), session_state);
        }
    }

    /// Read state value
    pub fn get(&self, scope: Scope, state_id: &str, path: &str, session_id: Option<&SessionId>) -> Option<Value> {
        match scope {
            Scope::Global => {
                let global = self.global.read().unwrap();
                global.get(state_id)?.get(path).cloned()
            }
            Scope::User => {
                let session_id = session_id?;
                self.ensure_user_session(session_id);
                let user = self.user.read().unwrap();
                user.get(session_id)?.get(state_id)?.get(path).cloned()
            }
        }
    }

    /// Write state value
    pub fn set(&self, scope: Scope, state_id: &str, path: &str, value: Value, session_id: Option<&SessionId>) {
        match scope {
            Scope::Global => {
                let mut global = self.global.write().unwrap();
                if let Some(state) = global.get_mut(state_id) {
                    state.insert(path.to_string(), value);
                }
            }
            Scope::User => {
                if let Some(session_id) = session_id {
                    self.ensure_user_session(session_id);
                    let mut user = self.user.write().unwrap();
                    if let Some(session) = user.get_mut(session_id) {
                        if let Some(state) = session.get_mut(state_id) {
                            state.insert(path.to_string(), value);
                        }
                    }
                }
            }
        }
    }

    /// Apply mutation
    pub fn mutate(
        &self,
        scope: Scope,
        state_id: &str,
        mutation: &Mutation,
        ctx: &EvalContext,
    ) {
        let current = self.get(scope, state_id, &mutation.field, ctx.session_id);

        let new_value = match mutation.op {
            MutationOp::Set => {
                mutation.value.as_ref()
                    .map(|expr| expr.eval(ctx))
                    .unwrap_or(Value::Null)
            }

            MutationOp::Increment => {
                let current_num = current
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let by = mutation.by.as_ref()
                    .map(|expr| expr.eval(ctx).as_i64().unwrap_or(1))
                    .unwrap_or(1);
                Value::Number((current_num + by).into())
            }

            MutationOp::Decrement => {
                let current_num = current
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let by = mutation.by.as_ref()
                    .map(|expr| expr.eval(ctx).as_i64().unwrap_or(1))
                    .unwrap_or(1);
                Value::Number((current_num - by).into())
            }

            MutationOp::Toggle => {
                let current_bool = current
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Value::Bool(!current_bool)
            }

            MutationOp::Push => {
                let mut arr = current
                    .and_then(|v| v.as_array().cloned())
                    .unwrap_or_default();
                if let Some(expr) = &mutation.value {
                    arr.push(expr.eval(ctx));
                }
                Value::Array(arr)
            }

            MutationOp::Pop => {
                let mut arr = current
                    .and_then(|v| v.as_array().cloned())
                    .unwrap_or_default();
                arr.pop();
                Value::Array(arr)
            }

            MutationOp::Clear => {
                // Return empty value based on field type
                Value::Array(vec![])
            }

            MutationOp::RemoveAt => {
                let mut arr = current
                    .and_then(|v| v.as_array().cloned())
                    .unwrap_or_default();
                if let Some(idx_expr) = &mutation.index {
                    if let Some(idx) = idx_expr.eval(ctx).as_u64() {
                        if (idx as usize) < arr.len() {
                            arr.remove(idx as usize);
                        }
                    }
                }
                Value::Array(arr)
            }
        };

        self.set(scope, state_id, &mutation.field, new_value, ctx.session_id);
    }
}
```

---

## Handler Registry

```rust
// src/handlers.rs

use crate::schema::{Handler, Action, Event};
use crate::state::DynamicState;
use crate::expr::EvalContext;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

pub type HandlerId = u32;

static HANDLER_COUNTER: AtomicU32 = AtomicU32::new(0);

pub struct HandlerRegistry {
    handlers: HashMap<HandlerId, Handler>,
}

impl HandlerRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register handler and return ID
    pub fn register(&mut self, handler: Handler) -> HandlerId {
        let id = HANDLER_COUNTER.fetch_add(1, Ordering::SeqCst);
        self.handlers.insert(id, handler);
        id
    }

    /// Get handler by ID
    pub fn get(&self, id: HandlerId) -> Option<&Handler> {
        self.handlers.get(&id)
    }

    /// Execute handler actions
    pub fn execute(
        &self,
        handler_id: HandlerId,
        state: &DynamicState,
        ctx: &mut EvalContext,
    ) -> ExecuteResult {
        let handler = match self.get(handler_id) {
            Some(h) => h,
            None => return ExecuteResult::default(),
        };

        let mut result = ExecuteResult::default();

        for action in &handler.actions {
            match action {
                Action::UpdateState { scope, state_id, mutations } => {
                    for mutation in mutations {
                        state.mutate(*scope, state_id, mutation, ctx);
                    }
                    result.state_changed = true;
                }

                Action::Navigate { path } => {
                    let path_value = match path {
                        StringOrExpr::String(s) => s.clone(),
                        StringOrExpr::Expr(expr) => {
                            expr.eval(ctx).as_str().unwrap_or("/").to_string()
                        }
                    };
                    result.navigate_to = Some(path_value);
                }

                Action::PreventDefault => {
                    result.prevent_default = true;
                }

                Action::Log { message } => {
                    let msg = match message {
                        StringOrExpr::String(s) => s.clone(),
                        StringOrExpr::Expr(expr) => {
                            format!("{}", expr.eval(ctx))
                        }
                    };
                    println!("[LOG] {}", msg);
                }
            }
        }

        result
    }
}

#[derive(Default)]
pub struct ExecuteResult {
    pub state_changed: bool,
    pub prevent_default: bool,
    pub navigate_to: Option<String>,
}
```

---

## Interpreter with Handler Binding

```rust
// src/interpreter.rs (updates)

impl<'a> Interpreter<'a> {
    fn render_node(&self, node: &NodeToken, ctx: &EvalContext) -> ElementBuilder {
        let el_type = string_to_el(&node.tag);
        let mut builder = el(el_type);

        // ... attributes and styles (same as Phase 1) ...

        // Bind handlers
        for handler in &node.handlers {
            let handler_id = self.registry.borrow_mut().register(handler.clone());
            builder = self.bind_handler(builder, handler_id, handler.event);
        }

        // ... children ...

        builder
    }

    fn bind_handler(
        &self,
        builder: ElementBuilder,
        handler_id: HandlerId,
        event: Event,
    ) -> ElementBuilder {
        let ev = match event {
            Event::Click => Ev::Click,
            Event::Input => Ev::Input,
            Event::Change => Ev::Change,
            Event::Submit => Ev::Submit,
            Event::Focus => Ev::Focus,
            Event::Blur => Ev::Blur,
        };

        // Create a dynamic handler that captures the handler_id
        builder.on(ev, DynamicHandler::new(handler_id))
    }
}
```

---

## Dynamic Handler Integration

Bridge between YAML handlers and rwire's handler system.

```rust
// src/runtime.rs

use rwire::{HandlerFn, EventContext, ChangeSet};
use crate::handlers::{HandlerId, HandlerRegistry};
use crate::state::DynamicState;

/// A handler that dispatches to the token runtime
pub struct DynamicHandler {
    handler_id: HandlerId,
}

impl DynamicHandler {
    pub fn new(handler_id: HandlerId) -> Self {
        Self { handler_id }
    }
}

/// Token runtime that owns state and handlers
pub struct TokenRuntime {
    pub app: AppToken,
    pub state: DynamicState,
    pub handlers: HandlerRegistry,
}

impl TokenRuntime {
    pub fn new(app: AppToken) -> Self {
        let state = DynamicState::from_schema(&app.state);
        Self {
            app,
            state,
            handlers: HandlerRegistry::new(),
        }
    }

    /// Handle incoming event from WebSocket
    pub fn handle_event(
        &self,
        handler_id: HandlerId,
        session_id: &str,
        event_value: Option<&str>,
    ) -> bool {
        let mut ctx = EvalContext {
            state: &self.state,
            session_id: Some(&session_id.to_string()),
            locals: HashMap::new(),
            params: HashMap::new(),
            event_value: event_value.map(|s| Value::String(s.to_string())),
        };

        let result = self.handlers.execute(handler_id, &self.state, &mut ctx);
        result.state_changed
    }

    /// Render page for session
    pub fn render(&self, page: &str, session_id: &str) -> Option<ElementBuilder> {
        let ctx = EvalContext {
            state: &self.state,
            session_id: Some(&session_id.to_string()),
            locals: HashMap::new(),
            params: HashMap::new(),
            event_value: None,
        };

        let interpreter = Interpreter {
            app: &self.app,
            state: &self.state,
            registry: &self.handlers,
        };

        interpreter.render_page(page, &ctx)
    }
}
```

---

## WebSocket Server Integration

```rust
// src/server.rs

use rwire::server::{Server, ServerConfig, Connection};
use crate::TokenRuntime;
use std::sync::Arc;

pub fn serve_yaml(yaml_path: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let yaml_content = std::fs::read_to_string(yaml_path)?;
    let app: AppToken = serde_yaml::from_str(&yaml_content)?;
    let runtime = Arc::new(TokenRuntime::new(app));

    let config = ServerConfig::new()
        .bind(format!("127.0.0.1:{}", port));

    // Create rwire server with token runtime
    Server::new(config)
        .with_root(move |session_id| {
            let rt = Arc::clone(&runtime);
            rt.render("home", session_id).unwrap_or_else(|| el(El::Div).text("Error"))
        })
        .with_handler(move |session_id, handler_id, event_data| {
            let rt = Arc::clone(&runtime);
            let changed = rt.handle_event(handler_id, session_id, event_data);
            if changed {
                // Return updated UI
                Some(rt.render("home", session_id))
            } else {
                None
            }
        })
        .run()?;

    Ok(())
}
```

---

## Event Value Access

Update expression evaluator to handle event values.

```rust
// src/expr.rs (update)

impl Expr {
    pub fn eval(&self, ctx: &EvalContext) -> Value {
        match self {
            // ... other cases ...

            Expr::EventValue => {
                ctx.event_value.clone().unwrap_or(Value::Null)
            }
        }
    }
}

pub struct EvalContext<'a> {
    pub state: &'a DynamicState,
    pub session_id: Option<&'a SessionId>,
    pub locals: HashMap<String, Value>,
    pub params: HashMap<String, String>,
    pub event_value: Option<Value>,  // NEW
}
```

---

## Test YAML (Counter with Working Buttons)

```yaml
version: "1.0"

state:
  user:
    - id: Counter
      fields:
        - name: count
          type: number
          default: 0

routes:
  - path: /
    page: home

ui:
  pages:
    home:
      title: "Counter"
      tag: div
      styles:
        - property: display
          value: flex
        - property: flex-direction
          value: column
        - property: align-items
          value: center
        - property: padding
          value: "2rem"
        - property: font-family
          value: "system-ui, sans-serif"
      children:
        - type: node
          tag: h1
          children:
            - type: text
              content: "Counter"
        - type: node
          tag: div
          styles:
            - property: display
              value: flex
            - property: gap
              value: "1rem"
            - property: align-items
              value: center
          children:
            - type: node
              tag: button
              styles:
                - property: font-size
                  value: "1.5rem"
                - property: padding
                  value: "0.5rem 1rem"
              handlers:
                - event: click
                  id: decrement
                  actions:
                    - action: update_state
                      scope: user
                      state_id: Counter
                      mutations:
                        - op: decrement
                          field: count
              children:
                - type: text
                  content: "-"
            - type: dynamic_text
              expr:
                type: state
                scope: user
                state_id: Counter
                path: count
            - type: node
              tag: button
              styles:
                - property: font-size
                  value: "1.5rem"
                - property: padding
                  value: "0.5rem 1rem"
              handlers:
                - event: click
                  id: increment
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

---

## Success Criteria

- [ ] DynamicState stores per-session user state
- [ ] Handler registry assigns IDs
- [ ] Mutations execute correctly (set, increment, decrement, toggle)
- [ ] Handlers bind to rwire events
- [ ] WebSocket receives click events
- [ ] Event dispatches to handler registry
- [ ] State mutation triggers re-render
- [ ] Browser updates without refresh
- [ ] Counter app works: click + → count increases

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        Browser                                   │
│  ┌──────────────┐                                               │
│  │ Button (+)   │──click──┐                                     │
│  └──────────────┘         │                                     │
│  ┌──────────────┐         │                                     │
│  │ Count: 42    │<─────── │ ─────────────────────┐              │
│  └──────────────┘         │                      │              │
└───────────────────────────│──────────────────────│──────────────┘
                            │ WebSocket            │ DOM Update
                            ▼                      │
┌───────────────────────────────────────────────────│──────────────┐
│                    TokenRuntime                   │              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │ HandlerReg   │  │ DynamicState │  │ Interpreter          │   │
│  │ ID=5 → incr  │  │ count: 42    │  │ Token → ElementBuilder│  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘   │
│         │                 ▲                      │               │
│         │    mutate()     │         render()     │               │
│         └─────────────────┘──────────────────────┘               │
└──────────────────────────────────────────────────────────────────┘
```

---

## Next Phase

Phase 3 adds:
- Loop rendering with ItemRef
- Conditional rendering
- Component slots
- Todo list example
