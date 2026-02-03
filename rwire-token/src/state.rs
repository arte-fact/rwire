//! Dynamic state management for token apps.

use crate::expr::EvalContext;
use crate::schema::{Mutation, MutationOp, Scope, StateDefinition, StateSchema};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Thread-safe dynamic state container.
#[derive(Clone)]
pub struct DynamicState {
    schema: StateSchema,
    global: Arc<RwLock<HashMap<String, HashMap<String, Value>>>>,
    user: Arc<RwLock<HashMap<String, HashMap<String, HashMap<String, Value>>>>>,
}

impl DynamicState {
    /// Create state from schema with default values.
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

    /// Initialize fields with default values from definition.
    fn init_fields(state_def: &StateDefinition) -> HashMap<String, Value> {
        state_def
            .fields
            .iter()
            .map(|f| (f.name.clone(), f.default.clone()))
            .collect()
    }

    /// Ensure user session state exists.
    fn ensure_user_session(&self, session_id: &str) {
        let mut user = self.user.write().unwrap();
        if !user.contains_key(session_id) {
            let mut session_state = HashMap::new();
            for state_def in &self.schema.user {
                let fields = Self::init_fields(state_def);
                session_state.insert(state_def.id.clone(), fields);
            }
            user.insert(session_id.to_string(), session_state);
        }
    }

    /// Get state value by path.
    pub fn get(
        &self,
        scope: Scope,
        state_id: &str,
        path: &str,
        session_id: Option<&str>,
    ) -> Option<Value> {
        match scope {
            Scope::Global => {
                let global = self.global.read().unwrap();
                let state = global.get(state_id)?;
                get_by_path(state, path)
            }
            Scope::User => {
                let session_id = session_id?;
                self.ensure_user_session(session_id);
                let user = self.user.read().unwrap();
                let session = user.get(session_id)?;
                let state = session.get(state_id)?;
                get_by_path(state, path)
            }
        }
    }

    /// Set state value.
    pub fn set(
        &self,
        scope: Scope,
        state_id: &str,
        field: &str,
        value: Value,
        session_id: Option<&str>,
    ) {
        match scope {
            Scope::Global => {
                let mut global = self.global.write().unwrap();
                if let Some(state) = global.get_mut(state_id) {
                    state.insert(field.to_string(), value);
                }
            }
            Scope::User => {
                if let Some(session_id) = session_id {
                    self.ensure_user_session(session_id);
                    let mut user = self.user.write().unwrap();
                    if let Some(session) = user.get_mut(session_id) {
                        if let Some(state) = session.get_mut(state_id) {
                            state.insert(field.to_string(), value);
                        }
                    }
                }
            }
        }
    }

    /// Apply mutation to state.
    pub fn mutate(
        &self,
        scope: Scope,
        state_id: &str,
        mutation: &Mutation,
        ctx: &EvalContext,
    ) {
        let current = self.get(scope, state_id, &mutation.field, ctx.session_id.as_deref());

        let new_value = match mutation.op {
            MutationOp::Set => mutation
                .value
                .as_ref()
                .map(|expr| expr.eval(ctx))
                .unwrap_or(Value::Null),

            MutationOp::Increment => {
                let current_num = current.and_then(|v| v.as_i64()).unwrap_or(0);
                let by = mutation
                    .by
                    .as_ref()
                    .map(|expr| expr.eval(ctx).as_i64().unwrap_or(1))
                    .unwrap_or(1);
                Value::Number((current_num + by).into())
            }

            MutationOp::Decrement => {
                let current_num = current.and_then(|v| v.as_i64()).unwrap_or(0);
                let by = mutation
                    .by
                    .as_ref()
                    .map(|expr| expr.eval(ctx).as_i64().unwrap_or(1))
                    .unwrap_or(1);
                Value::Number((current_num - by).into())
            }

            MutationOp::Toggle => {
                let current_bool = current.and_then(|v| v.as_bool()).unwrap_or(false);
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

            MutationOp::Clear => Value::Array(vec![]),

            MutationOp::RemoveAt => {
                let mut arr = current
                    .and_then(|v| v.as_array().cloned())
                    .unwrap_or_default();
                if let Some(idx_expr) = &mutation.index {
                    if let Some(idx) = idx_expr.eval(ctx).as_u64() {
                        let idx = idx as usize;
                        if idx < arr.len() {
                            arr.remove(idx);
                        }
                    }
                }
                Value::Array(arr)
            }
        };

        self.set(
            scope,
            state_id,
            &mutation.field,
            new_value,
            ctx.session_id.as_deref(),
        );
    }
}

/// Get value by dot-separated path from field map.
fn get_by_path(fields: &HashMap<String, Value>, path: &str) -> Option<Value> {
    let parts: Vec<&str> = path.split('.').collect();

    if parts.is_empty() {
        return None;
    }

    // Get the root field
    let mut current = fields.get(parts[0])?.clone();

    // Navigate nested paths
    for part in parts.iter().skip(1) {
        current = match current {
            Value::Object(obj) => obj.get(*part)?.clone(),
            Value::Array(arr) => {
                let idx: usize = part.parse().ok()?;
                arr.get(idx)?.clone()
            }
            _ => return None,
        };
    }

    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{FieldDefinition, FieldType};

    fn test_schema() -> StateSchema {
        StateSchema {
            global: vec![],
            user: vec![StateDefinition {
                id: "Counter".to_string(),
                fields: vec![FieldDefinition {
                    name: "count".to_string(),
                    field_type: FieldType::Number,
                    default: Value::Number(0.into()),
                }],
            }],
        }
    }

    #[test]
    fn test_get_default() {
        let schema = test_schema();
        let state = DynamicState::from_schema(&schema);

        let value = state.get(Scope::User, "Counter", "count", Some("session1"));
        assert_eq!(value, Some(Value::Number(0.into())));
    }

    #[test]
    fn test_set_and_get() {
        let schema = test_schema();
        let state = DynamicState::from_schema(&schema);

        state.set(
            Scope::User,
            "Counter",
            "count",
            Value::Number(42.into()),
            Some("session1"),
        );

        let value = state.get(Scope::User, "Counter", "count", Some("session1"));
        assert_eq!(value, Some(Value::Number(42.into())));
    }

    #[test]
    fn test_session_isolation() {
        let schema = test_schema();
        let state = DynamicState::from_schema(&schema);

        state.set(
            Scope::User,
            "Counter",
            "count",
            Value::Number(10.into()),
            Some("session1"),
        );
        state.set(
            Scope::User,
            "Counter",
            "count",
            Value::Number(20.into()),
            Some("session2"),
        );

        assert_eq!(
            state.get(Scope::User, "Counter", "count", Some("session1")),
            Some(Value::Number(10.into()))
        );
        assert_eq!(
            state.get(Scope::User, "Counter", "count", Some("session2")),
            Some(Value::Number(20.into()))
        );
    }

    #[test]
    fn test_mutate_increment() {
        let schema = test_schema();
        let state = DynamicState::from_schema(&schema);
        let ctx = EvalContext::new(&state).with_session("session1");

        let mutation = Mutation {
            op: MutationOp::Increment,
            field: "count".to_string(),
            value: None,
            by: None,
            index: None,
        };

        state.mutate(Scope::User, "Counter", &mutation, &ctx);

        assert_eq!(
            state.get(Scope::User, "Counter", "count", Some("session1")),
            Some(Value::Number(1.into()))
        );
    }
}
