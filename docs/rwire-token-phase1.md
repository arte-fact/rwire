# Phase 1: Foundation (Token Schema + Interpreter)

## Goal

Render static YAML applications to rwire ElementBuilder trees. No reactivity yet - just prove we can parse YAML and produce valid rwire output.

---

## Deliverables

1. `rwire-token` crate with schema types
2. YAML parsing via serde
3. Expression evaluator (literals only)
4. Interpreter that produces `ElementBuilder`
5. Basic HTTP server serving rendered HTML

---

## Schema Types

### Core Token Types

```rust
// src/schema.rs

/// Root application definition
#[derive(Debug, Clone, Deserialize)]
pub struct AppToken {
    pub version: String,
    #[serde(default)]
    pub state: StateSchema,
    pub ui: UiSchema,
    #[serde(default)]
    pub routes: Vec<Route>,
}

/// State definitions
#[derive(Debug, Clone, Default, Deserialize)]
pub struct StateSchema {
    #[serde(default)]
    pub global: Vec<StateDefinition>,
    #[serde(default)]
    pub user: Vec<StateDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StateDefinition {
    pub id: String,
    pub fields: Vec<FieldDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FieldDefinition {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    #[serde(default)]
    pub default: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

/// UI definitions
#[derive(Debug, Clone, Deserialize)]
pub struct UiSchema {
    #[serde(default)]
    pub shell: Option<NodeToken>,
    pub pages: HashMap<String, PageToken>,
    #[serde(default)]
    pub components: HashMap<String, NodeToken>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PageToken {
    pub title: String,
    #[serde(flatten)]
    pub node: NodeToken,
}

/// HTML element
#[derive(Debug, Clone, Deserialize)]
pub struct NodeToken {
    pub tag: String,
    #[serde(default)]
    pub attrs: Vec<Attribute>,
    #[serde(default)]
    pub styles: Vec<Style>,
    #[serde(default)]
    pub children: Vec<ChildToken>,
    #[serde(default)]
    pub handlers: Vec<Handler>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub value: StringOrExpr,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Style {
    pub property: String,
    pub value: StringOrExpr,
}

/// Child content types
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChildToken {
    Text { content: String },
    DynamicText { expr: Expr },
    Node(Box<NodeToken>),
    Conditional {
        condition: Expr,
        then: Box<ChildToken>,
        #[serde(rename = "else")]
        else_: Option<Box<ChildToken>>,
    },
    Loop {
        over: Expr,
        #[serde(rename = "as")]
        as_: String,
        index: Option<String>,
        body: Box<ChildToken>,
    },
    Ref {
        component_id: String,
        #[serde(default)]
        slots: HashMap<String, Vec<ChildToken>>,
    },
    Slot {
        name: String,
        #[serde(default)]
        default: Vec<ChildToken>,
    },
}

/// Route definition
#[derive(Debug, Clone, Deserialize)]
pub struct Route {
    pub path: String,
    pub page: String,
}
```

### Expression Types

```rust
// src/expr.rs

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Expr {
    /// Static value
    Literal { value: Value },

    /// State field access
    State {
        scope: Scope,
        state_id: String,
        path: String,
    },

    /// Event value (for handlers)
    EventValue,

    /// Route parameter
    Param { name: String },

    /// Local variable (loop item)
    Local { name: String },

    /// Format string: "Hello, {0}!"
    Format {
        template: String,
        args: Vec<Expr>,
    },

    /// Comparison: a == b
    Compare {
        op: CompareOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    /// Array length
    Length { value: Box<Expr> },

    /// Ternary: condition ? then : else
    Ternary {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Global,
    User,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// String or expression (for attributes/styles)
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum StringOrExpr {
    String(String),
    Expr(Expr),
}
```

---

## Expression Evaluator

Phase 1 only implements literals. State access returns placeholder values.

```rust
// src/expr.rs

use serde_json::Value;

pub struct EvalContext<'a> {
    pub state: &'a DynamicState,
    pub session_id: Option<&'a SessionId>,
    pub locals: HashMap<String, Value>,
    pub params: HashMap<String, String>,
}

impl Expr {
    pub fn eval(&self, ctx: &EvalContext) -> Value {
        match self {
            Expr::Literal { value } => value.clone(),

            // Phase 1: Return default values
            Expr::State { scope, state_id, path } => {
                ctx.state.get(*scope, state_id, path, ctx.session_id)
                    .unwrap_or(Value::Null)
            }

            Expr::Local { name } => {
                ctx.locals.get(name).cloned().unwrap_or(Value::Null)
            }

            Expr::Format { template, args } => {
                let mut result = template.clone();
                for (i, arg) in args.iter().enumerate() {
                    let value = arg.eval(ctx);
                    let placeholder = format!("{{{}}}", i);
                    result = result.replace(&placeholder, &value_to_string(&value));
                }
                Value::String(result)
            }

            Expr::Length { value } => {
                match value.eval(ctx) {
                    Value::Array(arr) => Value::Number(arr.len().into()),
                    Value::String(s) => Value::Number(s.len().into()),
                    _ => Value::Number(0.into()),
                }
            }

            // Stubs for Phase 1
            Expr::EventValue => Value::Null,
            Expr::Param { name } => {
                ctx.params.get(name)
                    .map(|s| Value::String(s.clone()))
                    .unwrap_or(Value::Null)
            }
            Expr::Compare { .. } => Value::Bool(false),
            Expr::Ternary { condition, then_expr, else_expr } => {
                if condition.eval(ctx).as_bool().unwrap_or(false) {
                    then_expr.eval(ctx)
                } else {
                    else_expr.eval(ctx)
                }
            }
        }
    }
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "".to_string(),
        Value::Array(arr) => format!("{} items", arr.len()),
        Value::Object(_) => "[object]".to_string(),
    }
}
```

---

## Interpreter

Converts tokens to rwire ElementBuilder.

```rust
// src/interpreter.rs

use rwire::{el, El, ElementBuilder};
use crate::{schema::*, expr::EvalContext};

pub struct Interpreter<'a> {
    pub app: &'a AppToken,
    pub state: &'a DynamicState,
}

impl<'a> Interpreter<'a> {
    pub fn render_page(&self, page_name: &str, ctx: &EvalContext) -> Option<ElementBuilder> {
        let page = self.app.ui.pages.get(page_name)?;
        Some(self.render_node(&page.node, ctx))
    }

    pub fn render_node(&self, node: &NodeToken, ctx: &EvalContext) -> ElementBuilder {
        let el_type = string_to_el(&node.tag);
        let mut builder = el(el_type);

        // Apply attributes
        for attr in &node.attrs {
            let value = match &attr.value {
                StringOrExpr::String(s) => s.clone(),
                StringOrExpr::Expr(expr) => value_to_string(&expr.eval(ctx)),
            };
            builder = builder.attr(&attr.name, &value);
        }

        // Apply styles
        let style_str = self.render_styles(&node.styles, ctx);
        if !style_str.is_empty() {
            builder = builder.attr("style", &style_str);
        }

        // Render children
        let children: Vec<ElementBuilder> = node.children
            .iter()
            .filter_map(|child| self.render_child(child, ctx))
            .collect();

        builder.append(children)
    }

    fn render_child(&self, child: &ChildToken, ctx: &EvalContext) -> Option<ElementBuilder> {
        match child {
            ChildToken::Text { content } => {
                Some(el(El::Span).text(content))
            }

            ChildToken::DynamicText { expr } => {
                let value = expr.eval(ctx);
                Some(el(El::Span).text(&value_to_string(&value)))
            }

            ChildToken::Node(node) => {
                Some(self.render_node(node, ctx))
            }

            ChildToken::Conditional { condition, then, else_ } => {
                let cond_value = condition.eval(ctx);
                if cond_value.as_bool().unwrap_or(false) {
                    self.render_child(then, ctx)
                } else if let Some(else_child) = else_ {
                    self.render_child(else_child, ctx)
                } else {
                    None
                }
            }

            ChildToken::Loop { over, as_, index, body } => {
                let items = match over.eval(ctx) {
                    Value::Array(arr) => arr,
                    _ => return None,
                };

                let children: Vec<ElementBuilder> = items
                    .iter()
                    .enumerate()
                    .filter_map(|(i, item)| {
                        let mut child_ctx = ctx.clone();
                        child_ctx.locals.insert(as_.clone(), item.clone());
                        if let Some(idx_name) = index {
                            child_ctx.locals.insert(idx_name.clone(), Value::Number(i.into()));
                        }
                        self.render_child(body, &child_ctx)
                    })
                    .collect();

                Some(el(El::Div).append(children))
            }

            ChildToken::Ref { component_id, slots } => {
                let component = self.app.ui.components.get(component_id)?;
                let ctx_with_slots = ctx.with_slots(slots);
                Some(self.render_node(component, &ctx_with_slots))
            }

            ChildToken::Slot { name, default } => {
                // Check if slot content provided, else use default
                if let Some(slot_content) = ctx.slots.get(name) {
                    let children: Vec<ElementBuilder> = slot_content
                        .iter()
                        .filter_map(|c| self.render_child(c, ctx))
                        .collect();
                    Some(el(El::Div).append(children))
                } else {
                    let children: Vec<ElementBuilder> = default
                        .iter()
                        .filter_map(|c| self.render_child(c, ctx))
                        .collect();
                    Some(el(El::Div).append(children))
                }
            }
        }
    }

    fn render_styles(&self, styles: &[Style], ctx: &EvalContext) -> String {
        styles.iter()
            .map(|s| {
                let value = match &s.value {
                    StringOrExpr::String(v) => v.clone(),
                    StringOrExpr::Expr(expr) => value_to_string(&expr.eval(ctx)),
                };
                format!("{}: {}", s.property, value)
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

fn string_to_el(tag: &str) -> El {
    match tag {
        "div" => El::Div,
        "span" => El::Span,
        "p" => El::P,
        "h1" => El::H1,
        "h2" => El::H2,
        "h3" => El::H3,
        "button" => El::Button,
        "input" => El::Input,
        "form" => El::Form,
        "ul" => El::Ul,
        "li" => El::Li,
        "a" => El::A,
        "img" => El::Img,
        "label" => El::Label,
        _ => El::Div, // Fallback
    }
}
```

---

## DynamicState (Minimal)

Phase 1 only needs default values from schema.

```rust
// src/state.rs

use serde_json::Value;
use std::collections::HashMap;
use crate::schema::{StateSchema, Scope};

pub struct DynamicState {
    schema: StateSchema,
}

impl DynamicState {
    pub fn from_schema(schema: &StateSchema) -> Self {
        Self {
            schema: schema.clone(),
        }
    }

    pub fn get(&self, scope: Scope, state_id: &str, path: &str, _session: Option<&SessionId>) -> Option<Value> {
        let states = match scope {
            Scope::Global => &self.schema.global,
            Scope::User => &self.schema.user,
        };

        let state_def = states.iter().find(|s| s.id == state_id)?;
        let field = state_def.fields.iter().find(|f| f.name == path)?;

        Some(field.default.clone())
    }
}
```

---

## Basic Server

Simple HTTP server that renders YAML to HTML.

```rust
// src/server.rs

use crate::{AppToken, DynamicState, Interpreter, EvalContext};
use std::fs;

pub fn serve_yaml(yaml_path: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let yaml_content = fs::read_to_string(yaml_path)?;
    let app: AppToken = serde_yaml::from_str(&yaml_content)?;
    let state = DynamicState::from_schema(&app.state);

    // Find default route
    let default_page = app.routes
        .iter()
        .find(|r| r.path == "/")
        .map(|r| r.page.as_str())
        .unwrap_or("home");

    // For Phase 1: Just serve static HTML
    let interpreter = Interpreter { app: &app, state: &state };
    let ctx = EvalContext::default();

    let builder = interpreter.render_page(default_page, &ctx)
        .ok_or("Page not found")?;

    // Use rwire's capsule serving
    rwire::serve_static(builder, port)?;

    Ok(())
}
```

---

## File Structure

```
rwire-token/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── schema.rs       # Token types (AppToken, NodeToken, etc.)
    ├── expr.rs         # Expr enum + eval()
    ├── state.rs        # DynamicState (minimal for Phase 1)
    ├── interpreter.rs  # Token → ElementBuilder
    └── server.rs       # Basic HTTP serving
```

---

## Cargo.toml

```toml
[package]
name = "rwire-token"
version = "0.1.0"
edition = "2021"

[dependencies]
rwire = { path = "../rwire" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
```

---

## Test YAML (examples/token-counter/app.yaml)

```yaml
version: "1.0"

state:
  user:
    - id: Counter
      fields:
        - name: count
          type: number
          default: 42

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
              content: "Counter (Phase 1 - Static)"
        - type: dynamic_text
          expr:
            type: state
            scope: user
            state_id: Counter
            path: count
        - type: node
          tag: p
          styles:
            - property: color
              value: "#666"
          children:
            - type: text
              content: "Buttons won't work until Phase 2"
```

---

## Success Criteria

- [ ] Parse valid YAML into AppToken
- [ ] Evaluate literal expressions
- [ ] Evaluate state expressions (returns defaults)
- [ ] Evaluate format expressions
- [ ] Render NodeToken to ElementBuilder
- [ ] Render Text, DynamicText children
- [ ] Apply styles as inline style attribute
- [ ] Serve HTML at http://127.0.0.1:9000
- [ ] Counter example displays "42"

---

## Next Phase

Phase 2 adds:
- Full DynamicState with get/set
- Handler parsing and registration
- Mutation execution
- WebSocket integration for live updates
