//! rwire-token: Declarative YAML apps on rwire.
//!
//! This crate enables building reactive applications in YAML without writing Rust code,
//! using rwire's WebSocket infrastructure for live updates.
//!
//! # Example
//!
//! ```yaml
//! version: "1.0"
//!
//! state:
//!   user:
//!     - id: Counter
//!       fields:
//!         - name: count
//!           type: number
//!           default: 0
//!
//! ui:
//!   pages:
//!     home:
//!       tag: div
//!       children:
//!         - type: dynamic_text
//!           expr: { type: state, scope: user, state_id: Counter, path: count }
//! ```

pub mod expr;
pub mod interpreter;
pub mod schema;
pub mod state;

pub use expr::EvalContext;
pub use interpreter::Interpreter;
pub use schema::AppToken;
pub use state::DynamicState;

use std::path::Path;

/// Load app from YAML file.
pub fn load_yaml(path: impl AsRef<Path>) -> Result<AppToken, LoadError> {
    let content = std::fs::read_to_string(path.as_ref()).map_err(|e| LoadError::Io(e.to_string()))?;
    parse_yaml(&content)
}

/// Parse YAML string into AppToken.
pub fn parse_yaml(content: &str) -> Result<AppToken, LoadError> {
    serde_yaml::from_str(content).map_err(|e| LoadError::Parse(e.to_string()))
}

/// Error loading app.
#[derive(Debug)]
pub enum LoadError {
    Io(String),
    Parse(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(msg) => write!(f, "IO error: {}", msg),
            LoadError::Parse(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for LoadError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal() {
        let yaml = r#"
version: "1.0"

ui:
  pages:
    home:
      title: "Test"
      tag: div
      children:
        - type: text
          content: "Hello"
"#;
        let app = parse_yaml(yaml).unwrap();
        assert_eq!(app.version, "1.0");
        assert!(app.ui.pages.contains_key("home"));
    }

    #[test]
    fn test_parse_with_state() {
        let yaml = r#"
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
      title: "Counter"
      tag: div
      children:
        - type: dynamic_text
          expr:
            type: state
            scope: user
            state_id: Counter
            path: count
"#;
        let app = parse_yaml(yaml).unwrap();
        assert_eq!(app.state.user.len(), 1);
        assert_eq!(app.state.user[0].id, "Counter");
    }

    #[test]
    fn test_parse_with_handlers() {
        let yaml = r#"
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
      title: "Counter"
      tag: div
      children:
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
"#;
        let app = parse_yaml(yaml).unwrap();
        let page = app.ui.pages.get("home").unwrap();
        if let schema::ChildToken::Node(node) = &page.node.children[0] {
            assert_eq!(node.handlers.len(), 1);
        } else {
            panic!("Expected node");
        }
    }

    #[test]
    fn test_render_from_yaml() {
        let yaml = r#"
version: "1.0"

state:
  user:
    - id: Counter
      fields:
        - name: count
          type: number
          default: 42

ui:
  pages:
    home:
      title: "Counter"
      tag: div
      children:
        - type: dynamic_text
          expr:
            type: state
            scope: user
            state_id: Counter
            path: count
"#;
        let app = parse_yaml(yaml).unwrap();
        let state = DynamicState::from_schema(&app.state);
        let interpreter = Interpreter::new(&app, &state);
        let ctx = EvalContext::new(&state).with_session("test-session");

        let builder = interpreter.render_page("home", &ctx);
        assert!(builder.is_some());
    }
}
