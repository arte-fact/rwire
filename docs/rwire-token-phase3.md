# Phase 3: Dynamic Content

## Goal

Support loops, conditionals, and components to enable list-based apps like Todo. After this phase, a full Todo app with add/delete will work.

---

## Deliverables

1. Loop rendering with index tracking
2. ItemRef integration for efficient list updates
3. Conditional rendering (if/else)
4. Component definitions and references
5. Slot system for composition
6. Working Todo example

---

## Prerequisites

- Phase 2 complete (handlers + state work)

---

## Loop Rendering

### Basic Loop

```yaml
- type: loop
  over:
    type: state
    scope: user
    state_id: Todos
    path: items
  as: todo
  index: i
  body:
    type: node
    tag: li
    children:
      - type: dynamic_text
        expr:
          type: local
          name: todo
```

### Implementation

```rust
// src/interpreter.rs

impl<'a> Interpreter<'a> {
    fn render_child(&self, child: &ChildToken, ctx: &EvalContext) -> Option<ElementBuilder> {
        match child {
            ChildToken::Loop { over, as_, index, body } => {
                let items = match over.eval(ctx) {
                    Value::Array(arr) => arr,
                    _ => return Some(el(El::Div)), // Empty container for non-arrays
                };

                // Use rwire's iter_with_ref for efficient updates
                let children: Vec<ElementBuilder> = items
                    .iter()
                    .enumerate()
                    .filter_map(|(i, item)| {
                        let mut child_ctx = ctx.clone();
                        child_ctx.locals.insert(as_.clone(), item.clone());
                        child_ctx.loop_index = Some(i);

                        if let Some(idx_name) = index {
                            child_ctx.locals.insert(
                                idx_name.clone(),
                                Value::Number(i.into())
                            );
                        }

                        self.render_child(body, &child_ctx)
                    })
                    .collect();

                // Wrap in container with data-loop-id for targeted updates
                Some(el(El::Div)
                    .data("loop-id", &self.next_loop_id())
                    .append(children))
            }
            // ... other cases
        }
    }
}
```

### Loop Index in Handlers

For delete buttons, we need to pass the loop index to handlers.

```yaml
- type: loop
  over: { type: state, scope: user, state_id: Todos, path: items }
  as: todo
  index: i
  body:
    type: node
    tag: li
    children:
      - type: dynamic_text
        expr: { type: local, name: todo }
      - type: node
        tag: button
        handlers:
          - event: click
            actions:
              - action: update_state
                scope: user
                state_id: Todos
                mutations:
                  - op: remove_at
                    field: items
                    index:
                      type: local
                      name: i
        children:
          - type: text
            content: "Delete"
```

### EvalContext with Loop Context

```rust
// src/expr.rs

pub struct EvalContext<'a> {
    pub state: &'a DynamicState,
    pub session_id: Option<&'a SessionId>,
    pub locals: HashMap<String, Value>,
    pub params: HashMap<String, String>,
    pub event_value: Option<Value>,
    pub loop_index: Option<usize>,  // NEW: current loop index
    pub slots: HashMap<String, Vec<ChildToken>>,  // NEW: slot content
}

impl EvalContext<'_> {
    pub fn with_local(&self, name: &str, value: Value) -> Self {
        let mut ctx = self.clone();
        ctx.locals.insert(name.to_string(), value);
        ctx
    }

    pub fn with_slots(&self, slots: &HashMap<String, Vec<ChildToken>>) -> Self {
        let mut ctx = self.clone();
        ctx.slots = slots.clone();
        ctx
    }
}
```

---

## Conditional Rendering

### Basic Conditional

```yaml
- type: conditional
  condition:
    type: compare
    op: eq
    left:
      type: length
      value: { type: state, scope: user, state_id: Todos, path: items }
    right:
      type: literal
      value: 0
  then:
    type: text
    content: "No items yet"
  else:
    type: node
    tag: ul
    children:
      - type: loop
        # ...
```

### Implementation

```rust
// src/interpreter.rs

ChildToken::Conditional { condition, then, else_ } => {
    let cond_value = condition.eval(ctx);
    let is_true = match cond_value {
        Value::Bool(b) => b,
        Value::Number(n) => n.as_i64().map(|i| i != 0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Array(arr) => !arr.is_empty(),
        Value::Null => false,
        _ => true,
    };

    if is_true {
        self.render_child(then, ctx)
    } else if let Some(else_child) = else_ {
        self.render_child(else_child, ctx)
    } else {
        None // No output when condition false and no else
    }
}
```

### Comparison Operators

```rust
// src/expr.rs

Expr::Compare { op, left, right } => {
    let l = left.eval(ctx);
    let r = right.eval(ctx);

    let result = match op {
        CompareOp::Eq => values_equal(&l, &r),
        CompareOp::Ne => !values_equal(&l, &r),
        CompareOp::Lt => compare_values(&l, &r) == Some(Ordering::Less),
        CompareOp::Le => compare_values(&l, &r).map(|o| o != Ordering::Greater).unwrap_or(false),
        CompareOp::Gt => compare_values(&l, &r) == Some(Ordering::Greater),
        CompareOp::Ge => compare_values(&l, &r).map(|o| o != Ordering::Less).unwrap_or(false),
    };

    Value::Bool(result)
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        (Value::String(a), Value::String(b)) => a == b,
        _ => false,
    }
}

fn compare_values(a: &Value, b: &Value) -> Option<Ordering> {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            a.as_f64().partial_cmp(&b.as_f64())
        }
        (Value::String(a), Value::String(b)) => Some(a.cmp(b)),
        _ => None,
    }
}
```

---

## Component System

### Defining Components

```yaml
ui:
  components:
    card:
      tag: div
      styles:
        - property: background
          value: "#3B4252"
        - property: padding
          value: "1rem"
        - property: border-radius
          value: "8px"
      children:
        - type: slot
          name: header
          default:
            - type: text
              content: "Card"
        - type: slot
          name: default
```

### Using Components

```yaml
- type: ref
  component_id: card
  slots:
    header:
      - type: text
        content: "My Custom Header"
    default:
      - type: text
        content: "Card body content"
```

### Implementation

```rust
// src/interpreter.rs

ChildToken::Ref { component_id, slots } => {
    let component = self.app.ui.components.get(component_id)?;

    // Create context with slot content
    let ctx_with_slots = ctx.with_slots(slots);

    Some(self.render_node(component, &ctx_with_slots))
}

ChildToken::Slot { name, default } => {
    // Check if slot content was provided
    let content = ctx.slots.get(name)
        .map(|c| c.as_slice())
        .unwrap_or(default.as_slice());

    if content.is_empty() {
        return None;
    }

    let children: Vec<ElementBuilder> = content
        .iter()
        .filter_map(|c| self.render_child(c, ctx))
        .collect();

    // Slots render as fragments (no wrapper)
    if children.len() == 1 {
        Some(children.into_iter().next().unwrap())
    } else {
        Some(el(El::Div).append(children))
    }
}
```

---

## Todo Example

### Full YAML

```yaml
version: "1.0"

state:
  user:
    - id: TodoState
      fields:
        - name: items
          type: array
          default: ["Buy groceries", "Walk the dog"]
        - name: input
          type: string
          default: ""

routes:
  - path: /
    page: home

ui:
  components:
    todo-item:
      tag: div
      styles:
        - property: display
          value: flex
        - property: justify-content
          value: space-between
        - property: align-items
          value: center
        - property: padding
          value: "0.75rem"
        - property: background
          value: "#3B4252"
        - property: border-radius
          value: "6px"
        - property: margin-bottom
          value: "0.5rem"
      children:
        - type: slot
          name: content
        - type: slot
          name: actions

  pages:
    home:
      title: "Todo List"
      tag: div
      styles:
        - property: max-width
          value: "500px"
        - property: margin
          value: "0 auto"
        - property: padding
          value: "2rem"
        - property: font-family
          value: "system-ui, sans-serif"
        - property: background
          value: "#2E3440"
        - property: min-height
          value: "100vh"
        - property: color
          value: "#ECEFF4"
      children:
        - type: node
          tag: h1
          styles:
            - property: color
              value: "#88C0D0"
          children:
            - type: text
              content: "Todo List"

        # Item count
        - type: node
          tag: p
          styles:
            - property: color
              value: "#4C566A"
            - property: margin-bottom
              value: "1rem"
          children:
            - type: dynamic_text
              expr:
                type: format
                template: "{0} items"
                args:
                  - type: length
                    value:
                      type: state
                      scope: user
                      state_id: TodoState
                      path: items

        # Input form
        - type: node
          tag: div
          styles:
            - property: display
              value: flex
            - property: gap
              value: "0.5rem"
            - property: margin-bottom
              value: "1.5rem"
          children:
            - type: node
              tag: input
              attrs:
                - name: type
                  value: text
                - name: placeholder
                  value: "What needs to be done?"
              styles:
                - property: flex
                  value: "1"
                - property: padding
                  value: "0.75rem"
                - property: background
                  value: "#3B4252"
                - property: border
                  value: "1px solid #4C566A"
                - property: border-radius
                  value: "6px"
                - property: color
                  value: "#ECEFF4"
              handlers:
                - event: input
                  actions:
                    - action: update_state
                      scope: user
                      state_id: TodoState
                      mutations:
                        - op: set
                          field: input
                          value:
                            type: event_value

            - type: node
              tag: button
              styles:
                - property: padding
                  value: "0.75rem 1.5rem"
                - property: background
                  value: "#5E81AC"
                - property: color
                  value: "#ECEFF4"
                - property: border
                  value: none
                - property: border-radius
                  value: "6px"
                - property: cursor
                  value: pointer
              handlers:
                - event: click
                  actions:
                    - action: update_state
                      scope: user
                      state_id: TodoState
                      mutations:
                        - op: push
                          field: items
                          value:
                            type: state
                            scope: user
                            state_id: TodoState
                            path: input
                        - op: set
                          field: input
                          value:
                            type: literal
                            value: ""
              children:
                - type: text
                  content: "Add"

        # Empty state
        - type: conditional
          condition:
            type: compare
            op: eq
            left:
              type: length
              value:
                type: state
                scope: user
                state_id: TodoState
                path: items
            right:
              type: literal
              value: 0
          then:
            type: node
            tag: p
            styles:
              - property: text-align
                value: center
              - property: color
                value: "#4C566A"
              - property: padding
                value: "2rem"
            children:
              - type: text
                content: "No todos yet. Add one above!"

        # Todo list
        - type: loop
          over:
            type: state
            scope: user
            state_id: TodoState
            path: items
          as: todo
          index: i
          body:
            type: ref
            component_id: todo-item
            slots:
              content:
                - type: dynamic_text
                  expr:
                    type: local
                    name: todo
              actions:
                - type: node
                  tag: button
                  styles:
                    - property: background
                      value: "#BF616A"
                    - property: color
                      value: white
                    - property: border
                      value: none
                    - property: padding
                      value: "0.25rem 0.75rem"
                    - property: border-radius
                      value: "4px"
                    - property: cursor
                      value: pointer
                  handlers:
                    - event: click
                      actions:
                        - action: update_state
                          scope: user
                          state_id: TodoState
                          mutations:
                            - op: remove_at
                              field: items
                              index:
                                type: local
                                name: i
                  children:
                    - type: text
                      content: "Delete"
```

---

## Handler Context with Loop Index

For delete to work, handlers need access to the loop index at execution time.

```rust
// src/handlers.rs

pub struct HandlerContext {
    pub handler_id: HandlerId,
    pub loop_indices: Vec<usize>,  // Stack of loop indices
}

impl TokenRuntime {
    pub fn handle_event(
        &self,
        ctx: HandlerContext,
        session_id: &str,
        event_value: Option<&str>,
    ) -> bool {
        // Reconstruct EvalContext with loop indices
        let mut eval_ctx = EvalContext {
            state: &self.state,
            session_id: Some(&session_id.to_string()),
            locals: HashMap::new(),
            params: HashMap::new(),
            event_value: event_value.map(|s| Value::String(s.to_string())),
            loop_index: ctx.loop_indices.last().copied(),
            slots: HashMap::new(),
        };

        // Populate locals from loop indices
        // This requires storing loop variable names with handlers
        // ...

        let result = self.handlers.execute(ctx.handler_id, &self.state, &mut eval_ctx);
        result.state_changed
    }
}
```

---

## Wire Protocol Enhancement

When binding handlers inside loops, we need to encode the loop index.

```rust
// Handler binding in interpreter
fn bind_handler(
    &self,
    builder: ElementBuilder,
    handler_id: HandlerId,
    event: Event,
    ctx: &EvalContext,
) -> ElementBuilder {
    let ev = event_to_ev(event);

    // If inside a loop, encode the index with the handler
    if let Some(loop_idx) = ctx.loop_index {
        // Use on_ref with ItemRef-like encoding
        builder.on_with_data(ev, handler_id, &[loop_idx as u8])
    } else {
        builder.on(ev, handler_id)
    }
}
```

---

## Success Criteria

- [ ] Loop renders array items
- [ ] Loop index accessible via `type: local`
- [ ] Conditionals render based on boolean
- [ ] Comparison operators work (eq, ne, lt, gt, le, ge)
- [ ] Components defined and referenced
- [ ] Slots receive and render content
- [ ] Default slot content works
- [ ] Delete button removes correct item
- [ ] Add button appends to list
- [ ] Empty state shows when list empty
- [ ] Todo app fully functional

---

## Next Phase

Phase 4 adds:
- Full expression system
- Format strings
- Logical operators
- Design token integration
- Styled components
