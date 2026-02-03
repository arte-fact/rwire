//! Expression evaluation for dynamic values.

use crate::schema::{CompareOp, Expr, LogicalOp, StringOrExpr};
use crate::state::DynamicState;
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::HashMap;

/// Context for expression evaluation.
#[derive(Clone)]
pub struct EvalContext<'a> {
    pub state: &'a DynamicState,
    pub session_id: Option<String>,
    pub locals: HashMap<String, Value>,
    pub params: HashMap<String, String>,
    pub event_value: Option<Value>,
    pub slots: HashMap<String, Vec<crate::schema::ChildToken>>,
}

impl<'a> EvalContext<'a> {
    /// Create a new context with state reference.
    pub fn new(state: &'a DynamicState) -> Self {
        Self {
            state,
            session_id: None,
            locals: HashMap::new(),
            params: HashMap::new(),
            event_value: None,
            slots: HashMap::new(),
        }
    }

    /// Set session ID.
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Add a local variable.
    pub fn with_local(mut self, name: impl Into<String>, value: Value) -> Self {
        self.locals.insert(name.into(), value);
        self
    }

    /// Add slots for component rendering.
    pub fn with_slots(
        mut self,
        slots: &HashMap<String, Vec<crate::schema::ChildToken>>,
    ) -> Self {
        self.slots = slots.clone();
        self
    }

    /// Set event value.
    pub fn with_event_value(mut self, value: Value) -> Self {
        self.event_value = Some(value);
        self
    }
}

impl Expr {
    /// Evaluate expression in context.
    pub fn eval(&self, ctx: &EvalContext) -> Value {
        match self {
            Expr::Literal { value } => value.clone(),

            Expr::State {
                scope,
                state_id,
                path,
            } => ctx
                .state
                .get(*scope, state_id, path, ctx.session_id.as_deref())
                .unwrap_or(Value::Null),

            Expr::Local { name } => ctx.locals.get(name).cloned().unwrap_or(Value::Null),

            Expr::EventValue => ctx.event_value.clone().unwrap_or(Value::Null),

            Expr::Param { name } => ctx
                .params
                .get(name)
                .map(|s| Value::String(s.clone()))
                .unwrap_or(Value::Null),

            Expr::Format { template, args } => {
                let mut result = template.clone();
                for (i, arg) in args.iter().enumerate() {
                    let value = arg.eval(ctx);
                    let placeholder = format!("{{{}}}", i);
                    result = result.replace(&placeholder, &value_to_display_string(&value));
                }
                Value::String(result)
            }

            Expr::Compare { op, left, right } => {
                let l = left.eval(ctx);
                let r = right.eval(ctx);
                let result = match op {
                    CompareOp::Eq => values_equal(&l, &r),
                    CompareOp::Ne => !values_equal(&l, &r),
                    CompareOp::Lt => compare_values(&l, &r) == Some(Ordering::Less),
                    CompareOp::Le => {
                        compare_values(&l, &r).map(|o| o != Ordering::Greater).unwrap_or(false)
                    }
                    CompareOp::Gt => compare_values(&l, &r) == Some(Ordering::Greater),
                    CompareOp::Ge => {
                        compare_values(&l, &r).map(|o| o != Ordering::Less).unwrap_or(false)
                    }
                };
                Value::Bool(result)
            }

            Expr::Length { value } => {
                let v = value.eval(ctx);
                let len = match v {
                    Value::Array(arr) => arr.len(),
                    Value::String(s) => s.len(),
                    Value::Object(obj) => obj.len(),
                    _ => 0,
                };
                Value::Number(len.into())
            }

            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                if value_is_truthy(&condition.eval(ctx)) {
                    then_expr.eval(ctx)
                } else {
                    else_expr.eval(ctx)
                }
            }

            Expr::Logical { op, operands } => {
                let result = match op {
                    LogicalOp::And => operands.iter().all(|e| value_is_truthy(&e.eval(ctx))),
                    LogicalOp::Or => operands.iter().any(|e| value_is_truthy(&e.eval(ctx))),
                    LogicalOp::Not => {
                        !operands.first().map(|e| value_is_truthy(&e.eval(ctx))).unwrap_or(true)
                    }
                };
                Value::Bool(result)
            }

            Expr::Field { base, field } => {
                let base_value = base.eval(ctx);
                match base_value {
                    Value::Object(obj) => obj.get(field).cloned().unwrap_or(Value::Null),
                    _ => Value::Null,
                }
            }
        }
    }
}

impl StringOrExpr {
    /// Resolve to string value.
    pub fn resolve(&self, ctx: &EvalContext) -> String {
        match self {
            StringOrExpr::String(s) => s.clone(),
            StringOrExpr::Expr(expr) => value_to_display_string(&expr.eval(ctx)),
        }
    }
}

/// Convert value to display string.
pub fn value_to_display_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(f) = n.as_f64() {
                if f.fract() == 0.0 {
                    format!("{:.0}", f)
                } else {
                    format!("{:.2}", f)
                }
            } else {
                n.to_string()
            }
        }
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Null => String::new(),
        Value::Array(arr) => format!("{}", arr.len()),
        Value::Object(obj) => format!("[{} fields]", obj.len()),
    }
}

/// Check if two values are equal.
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Array(a), Value::Array(b)) => a == b,
        _ => false,
    }
}

/// Compare two values for ordering.
fn compare_values(a: &Value, b: &Value) -> Option<Ordering> {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => a.as_f64().partial_cmp(&b.as_f64()),
        (Value::String(a), Value::String(b)) => Some(a.cmp(b)),
        _ => None,
    }
}

/// Check if value is truthy.
fn value_is_truthy(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        Value::Null => false,
        Value::Number(n) => n.as_i64().map(|i| i != 0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Array(arr) => !arr.is_empty(),
        Value::Object(obj) => !obj.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::StateSchema;

    fn empty_state() -> DynamicState {
        DynamicState::from_schema(&StateSchema::default())
    }

    #[test]
    fn test_literal() {
        let state = empty_state();
        let ctx = EvalContext::new(&state);
        let expr = Expr::Literal {
            value: Value::Number(42.into()),
        };
        assert_eq!(expr.eval(&ctx), Value::Number(42.into()));
    }

    #[test]
    fn test_format() {
        let state = empty_state();
        let ctx = EvalContext::new(&state);
        let expr = Expr::Format {
            template: "Hello, {0}!".to_string(),
            args: vec![Expr::Literal {
                value: Value::String("World".to_string()),
            }],
        };
        assert_eq!(
            expr.eval(&ctx),
            Value::String("Hello, World!".to_string())
        );
    }

    #[test]
    fn test_local() {
        let state = empty_state();
        let ctx = EvalContext::new(&state).with_local("item", Value::String("test".to_string()));
        let expr = Expr::Local {
            name: "item".to_string(),
        };
        assert_eq!(expr.eval(&ctx), Value::String("test".to_string()));
    }

    #[test]
    fn test_compare_eq() {
        let state = empty_state();
        let ctx = EvalContext::new(&state);
        let expr = Expr::Compare {
            op: CompareOp::Eq,
            left: Box::new(Expr::Literal {
                value: Value::Number(5.into()),
            }),
            right: Box::new(Expr::Literal {
                value: Value::Number(5.into()),
            }),
        };
        assert_eq!(expr.eval(&ctx), Value::Bool(true));
    }

    #[test]
    fn test_length() {
        let state = empty_state();
        let ctx = EvalContext::new(&state);
        let expr = Expr::Length {
            value: Box::new(Expr::Literal {
                value: Value::Array(vec![Value::Null, Value::Null, Value::Null]),
            }),
        };
        assert_eq!(expr.eval(&ctx), Value::Number(3.into()));
    }

    #[test]
    fn test_ternary() {
        let state = empty_state();
        let ctx = EvalContext::new(&state);
        let expr = Expr::Ternary {
            condition: Box::new(Expr::Literal {
                value: Value::Bool(true),
            }),
            then_expr: Box::new(Expr::Literal {
                value: Value::String("yes".to_string()),
            }),
            else_expr: Box::new(Expr::Literal {
                value: Value::String("no".to_string()),
            }),
        };
        assert_eq!(expr.eval(&ctx), Value::String("yes".to_string()));
    }
}
