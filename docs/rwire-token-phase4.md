# Phase 4: Expressions + Styling

## Goal

Complete the expression system and integrate rwire's design tokens for consistent styling.

---

## Deliverables

1. Full expression system (format, logical, ternary)
2. Design token integration
3. Component variants from YAML
4. Dynamic styles based on state
5. Styled Todo app

---

## Prerequisites

- Phase 3 complete (loops, conditionals, components work)

---

## Complete Expression System

### Format Strings

```yaml
- type: dynamic_text
  expr:
    type: format
    template: "Hello, {0}! You have {1} unread messages."
    args:
      - type: state
        scope: user
        state_id: User
        path: name
      - type: state
        scope: user
        state_id: Messages
        path: unreadCount
```

### Implementation

```rust
// src/expr.rs

Expr::Format { template, args } => {
    let mut result = template.clone();

    for (i, arg) in args.iter().enumerate() {
        let value = arg.eval(ctx);
        let placeholder = format!("{{{}}}", i);
        let replacement = value_to_display_string(&value);
        result = result.replace(&placeholder, &replacement);
    }

    Value::String(result)
}

fn value_to_display_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => {
            // Format nicely: no trailing .0 for integers
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(f) = n.as_f64() {
                format!("{:.2}", f)
            } else {
                n.to_string()
            }
        }
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Null => "".to_string(),
        Value::Array(arr) => format!("{}", arr.len()),
        Value::Object(obj) => format!("[{} fields]", obj.len()),
    }
}
```

### Logical Operators

```yaml
# AND
type: logical
op: and
operands:
  - type: state
    scope: user
    state_id: User
    path: isLoggedIn
  - type: compare
    op: gt
    left: { type: state, scope: user, state_id: Cart, path: itemCount }
    right: { type: literal, value: 0 }

# OR
type: logical
op: or
operands:
  - type: state
    scope: user
    state_id: User
    path: isAdmin
  - type: state
    scope: user
    state_id: User
    path: isModerator

# NOT
type: logical
op: not
operands:
  - type: state
    scope: user
    state_id: Form
    path: hasErrors
```

### Implementation

```rust
// src/expr.rs

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogicalOp {
    And,
    Or,
    Not,
}

Expr::Logical { op, operands } => {
    match op {
        LogicalOp::And => {
            let result = operands.iter().all(|e| {
                e.eval(ctx).as_bool().unwrap_or(false)
            });
            Value::Bool(result)
        }
        LogicalOp::Or => {
            let result = operands.iter().any(|e| {
                e.eval(ctx).as_bool().unwrap_or(false)
            });
            Value::Bool(result)
        }
        LogicalOp::Not => {
            let value = operands.first()
                .map(|e| e.eval(ctx).as_bool().unwrap_or(false))
                .unwrap_or(true);
            Value::Bool(!value)
        }
    }
}
```

### Ternary Expressions

```yaml
# Dynamic style value
- property: background
  value:
    type: ternary
    condition:
      type: state
      scope: user
      state_id: Theme
      path: isDark
    then_expr:
      type: literal
      value: "#1a1a2e"
    else_expr:
      type: literal
      value: "#ffffff"
```

### Implementation

```rust
Expr::Ternary { condition, then_expr, else_expr } => {
    let cond = condition.eval(ctx);
    let is_true = match cond {
        Value::Bool(b) => b,
        Value::Null => false,
        Value::Number(n) => n.as_i64().map(|i| i != 0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Array(arr) => !arr.is_empty(),
        _ => true,
    };

    if is_true {
        then_expr.eval(ctx)
    } else {
        else_expr.eval(ctx)
    }
}
```

### Field Access

```yaml
# Access nested field
type: field
base:
  type: local
  name: todo
field: text

# Or with path
type: state
scope: user
state_id: User
path: profile.address.city
```

### Implementation

```rust
Expr::Field { base, field } => {
    let base_value = base.eval(ctx);
    match base_value {
        Value::Object(obj) => obj.get(field).cloned().unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

// Path traversal for state access
fn get_by_path(value: &Value, path: &str) -> Option<Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value.clone();

    for part in parts {
        current = match current {
            Value::Object(obj) => obj.get(part)?.clone(),
            Value::Array(arr) => {
                let idx: usize = part.parse().ok()?;
                arr.get(idx)?.clone()
            }
            _ => return None,
        };
    }

    Some(current)
}
```

---

## Design Token Integration

### Token Schema in YAML

```yaml
version: "1.0"

tokens:
  colors:
    primary: "#5E81AC"
    secondary: "#81A1C1"
    background: "#2E3440"
    surface: "#3B4252"
    text: "#ECEFF4"
    muted: "#4C566A"
    success: "#A3BE8C"
    warning: "#EBCB8B"
    error: "#BF616A"

  spacing:
    xs: "0.25rem"
    sm: "0.5rem"
    md: "1rem"
    lg: "1.5rem"
    xl: "2rem"

  radius:
    sm: "4px"
    md: "6px"
    lg: "8px"
    full: "9999px"

  typography:
    sans: "system-ui, -apple-system, sans-serif"
    mono: "ui-monospace, monospace"
```

### Using Tokens in Styles

```yaml
# Reference tokens with $ prefix
styles:
  - property: background
    value: $colors.surface
  - property: padding
    value: $spacing.md
  - property: border-radius
    value: $radius.md
  - property: font-family
    value: $typography.sans
```

### Implementation

```rust
// src/tokens.rs

use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TokenSchema {
    #[serde(default)]
    pub colors: HashMap<String, String>,
    #[serde(default)]
    pub spacing: HashMap<String, String>,
    #[serde(default)]
    pub radius: HashMap<String, String>,
    #[serde(default)]
    pub typography: HashMap<String, String>,
}

impl TokenSchema {
    pub fn resolve(&self, reference: &str) -> Option<String> {
        // Parse "$colors.primary" -> ("colors", "primary")
        let reference = reference.strip_prefix('$')?;
        let (category, name) = reference.split_once('.')?;

        let map = match category {
            "colors" => &self.colors,
            "spacing" => &self.spacing,
            "radius" => &self.radius,
            "typography" => &self.typography,
            _ => return None,
        };

        map.get(name).cloned()
    }
}

// In interpreter
fn resolve_style_value(&self, value: &StringOrExpr, ctx: &EvalContext) -> String {
    match value {
        StringOrExpr::String(s) => {
            if s.starts_with('$') {
                self.app.tokens.resolve(s).unwrap_or_else(|| s.clone())
            } else {
                s.clone()
            }
        }
        StringOrExpr::Expr(expr) => value_to_string(&expr.eval(ctx)),
    }
}
```

---

## Component Variants

Define component variants similar to CVA (Class Variance Authority).

### Variant Schema

```yaml
ui:
  components:
    button:
      tag: button
      variants:
        intent:
          primary:
            background: $colors.primary
            color: white
          secondary:
            background: $colors.surface
            color: $colors.text
          ghost:
            background: transparent
            color: $colors.text
          destructive:
            background: $colors.error
            color: white
        size:
          sm:
            padding: "$spacing.xs $spacing.sm"
            font-size: "0.875rem"
          md:
            padding: "$spacing.sm $spacing.md"
            font-size: "1rem"
          lg:
            padding: "$spacing.md $spacing.lg"
            font-size: "1.125rem"
      defaultVariants:
        intent: primary
        size: md
      # Base styles always applied
      styles:
        - property: border
          value: none
        - property: border-radius
          value: $radius.md
        - property: cursor
          value: pointer
        - property: font-family
          value: $typography.sans
      children:
        - type: slot
          name: default
```

### Using Variants

```yaml
- type: ref
  component_id: button
  variants:
    intent: destructive
    size: sm
  slots:
    default:
      - type: text
        content: "Delete"
```

### Implementation

```rust
// src/schema.rs

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
    // NEW: Variant definitions
    #[serde(default)]
    pub variants: HashMap<String, HashMap<String, HashMap<String, String>>>,
    #[serde(default, rename = "defaultVariants")]
    pub default_variants: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChildToken {
    // ... existing variants ...
    Ref {
        component_id: String,
        #[serde(default)]
        slots: HashMap<String, Vec<ChildToken>>,
        #[serde(default)]
        variants: HashMap<String, String>,  // NEW
    },
}

// In interpreter
fn render_component(
    &self,
    component: &NodeToken,
    slots: &HashMap<String, Vec<ChildToken>>,
    variants: &HashMap<String, String>,
    ctx: &EvalContext,
) -> ElementBuilder {
    let el_type = string_to_el(&component.tag);
    let mut builder = el(el_type);

    // Collect styles: base + variant styles
    let mut all_styles: Vec<(&str, String)> = Vec::new();

    // Base styles
    for style in &component.styles {
        let value = self.resolve_style_value(&style.value, ctx);
        all_styles.push((&style.property, value));
    }

    // Variant styles
    for (variant_name, variant_options) in &component.variants {
        // Get selected variant or default
        let selected = variants.get(variant_name)
            .or_else(|| component.default_variants.get(variant_name));

        if let Some(selected_value) = selected {
            if let Some(styles) = variant_options.get(selected_value) {
                for (prop, value) in styles {
                    let resolved = if value.starts_with('$') {
                        self.app.tokens.resolve(value).unwrap_or_else(|| value.clone())
                    } else {
                        value.clone()
                    };
                    all_styles.push((prop, resolved));
                }
            }
        }
    }

    // Apply all styles
    let style_str = all_styles
        .iter()
        .map(|(p, v)| format!("{}: {}", p, v))
        .collect::<Vec<_>>()
        .join("; ");

    if !style_str.is_empty() {
        builder = builder.attr("style", &style_str);
    }

    // ... attributes, children, handlers ...

    builder
}
```

---

## Dynamic Styles Based on State

### Conditional Style Values

```yaml
- type: node
  tag: div
  styles:
    - property: background
      value:
        type: ternary
        condition:
          type: state
          scope: user
          state_id: Todo
          path: done
        then_expr:
          type: literal
          value: "#A3BE8C22"  # Green tint
        else_expr:
          type: literal
          value: transparent
    - property: text-decoration
      value:
        type: ternary
        condition:
          type: state
          scope: user
          state_id: Todo
          path: done
        then_expr:
          type: literal
          value: line-through
        else_expr:
          type: literal
          value: none
```

---

## Styled Todo Example

```yaml
version: "1.0"

tokens:
  colors:
    primary: "#5E81AC"
    background: "#2E3440"
    surface: "#3B4252"
    border: "#4C566A"
    text: "#ECEFF4"
    muted: "#4C566A"
    accent: "#88C0D0"
    success: "#A3BE8C"
    error: "#BF616A"

  spacing:
    xs: "0.25rem"
    sm: "0.5rem"
    md: "1rem"
    lg: "1.5rem"
    xl: "2rem"

  radius:
    sm: "4px"
    md: "6px"
    lg: "8px"

  typography:
    sans: "system-ui, -apple-system, sans-serif"

state:
  user:
    - id: TodoState
      fields:
        - name: items
          type: array
          default: []
        - name: input
          type: string
          default: ""

ui:
  components:
    button:
      tag: button
      variants:
        intent:
          primary:
            background: $colors.primary
            color: white
          ghost:
            background: transparent
            color: $colors.text
          destructive:
            background: $colors.error
            color: white
        size:
          sm:
            padding: "$spacing.xs $spacing.sm"
            font-size: "0.875rem"
          md:
            padding: "$spacing.sm $spacing.md"
            font-size: "1rem"
      defaultVariants:
        intent: primary
        size: md
      styles:
        - property: border
          value: none
        - property: border-radius
          value: $radius.md
        - property: cursor
          value: pointer
        - property: font-family
          value: $typography.sans
        - property: font-weight
          value: "500"
      children:
        - type: slot
          name: default

    input:
      tag: input
      styles:
        - property: flex
          value: "1"
        - property: padding
          value: $spacing.sm
        - property: background
          value: $colors.surface
        - property: border
          value: "1px solid $colors.border"
        - property: border-radius
          value: $radius.md
        - property: color
          value: $colors.text
        - property: font-family
          value: $typography.sans
        - property: outline
          value: none

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
          value: $spacing.sm
        - property: background
          value: $colors.surface
        - property: border-radius
          value: $radius.md
        - property: margin-bottom
          value: $spacing.sm
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
          value: $spacing.xl
        - property: font-family
          value: $typography.sans
        - property: background
          value: $colors.background
        - property: min-height
          value: "100vh"
        - property: color
          value: $colors.text
      children:
        # Header
        - type: node
          tag: h1
          styles:
            - property: color
              value: $colors.accent
            - property: font-weight
              value: "300"
            - property: margin-bottom
              value: $spacing.xs
          children:
            - type: text
              content: "Todo List"

        # Count
        - type: node
          tag: p
          styles:
            - property: color
              value: $colors.muted
            - property: margin-bottom
              value: $spacing.lg
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

        # Input row
        - type: node
          tag: div
          styles:
            - property: display
              value: flex
            - property: gap
              value: $spacing.sm
            - property: margin-bottom
              value: $spacing.lg
          children:
            - type: ref
              component_id: input
              attrs:
                - name: placeholder
                  value: "What needs to be done?"
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

            - type: ref
              component_id: button
              variants:
                intent: primary
                size: md
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
              slots:
                default:
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
                value: $colors.muted
              - property: padding
                value: $spacing.xl
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
                - type: ref
                  component_id: button
                  variants:
                    intent: destructive
                    size: sm
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
                  slots:
                    default:
                      - type: text
                        content: "Delete"

routes:
  - path: /
    page: home
```

---

## Success Criteria

- [ ] Format strings with {0}, {1} placeholders
- [ ] Logical operators (and, or, not)
- [ ] Ternary expressions
- [ ] Field access for nested values
- [ ] Token schema parsed from YAML
- [ ] $token.path references resolved
- [ ] Component variants defined and applied
- [ ] Default variants work
- [ ] Dynamic style values from expressions
- [ ] Styled todo app looks polished

---

## Next Phase

Phase 5 adds:
- Binary format (.ptok)
- CLI commands
- Hot reload
- Validation with errors
