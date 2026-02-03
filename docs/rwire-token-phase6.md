# Phase 6: Advanced Features

## Goal

Complete the feature set with routing, persistence, escape hatches to Rust, and async tasks.

---

## Deliverables

1. URL routing with parameters and guards
2. State persistence integration
3. Custom Rust handler escape hatch
4. Task system for async operations
5. Multi-page application support

---

## Prerequisites

- Phase 5 complete (CLI + binary format)

---

## URL Routing

### Route Definition

```yaml
routes:
  # Static routes
  - path: /
    page: home

  - path: /about
    page: about

  # Dynamic parameters
  - path: /users/:id
    page: user-profile

  - path: /posts/:category/:slug
    page: blog-post

  # Wildcards
  - path: /docs/*
    page: documentation

  # Route guards (future)
  - path: /admin
    page: admin-dashboard
    guards:
      - requireAuth
      - requireAdmin
```

### Route Matching

```rust
// src/router.rs

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Router {
    routes: Vec<CompiledRoute>,
}

#[derive(Debug, Clone)]
struct CompiledRoute {
    segments: Vec<RouteSegment>,
    page: String,
    guards: Vec<String>,
}

#[derive(Debug, Clone)]
enum RouteSegment {
    Literal(String),
    Param(String),
    Wildcard,
}

#[derive(Debug, Clone)]
pub struct RouteMatch {
    pub page: String,
    pub params: HashMap<String, String>,
    pub guards: Vec<String>,
}

impl Router {
    pub fn new(routes: &[Route]) -> Self {
        let routes = routes.iter().map(|r| {
            let segments = r.path.split('/')
                .filter(|s| !s.is_empty())
                .map(|s| {
                    if s.starts_with(':') {
                        RouteSegment::Param(s[1..].to_string())
                    } else if s == "*" {
                        RouteSegment::Wildcard
                    } else {
                        RouteSegment::Literal(s.to_string())
                    }
                })
                .collect();

            CompiledRoute {
                segments,
                page: r.page.clone(),
                guards: r.guards.clone(),
            }
        }).collect();

        Self { routes }
    }

    pub fn match_path(&self, path: &str) -> Option<RouteMatch> {
        let path_segments: Vec<&str> = path.split('/')
            .filter(|s| !s.is_empty())
            .collect();

        for route in &self.routes {
            if let Some(params) = self.match_route(route, &path_segments) {
                return Some(RouteMatch {
                    page: route.page.clone(),
                    params,
                    guards: route.guards.clone(),
                });
            }
        }

        None
    }

    fn match_route(&self, route: &CompiledRoute, path: &[&str]) -> Option<HashMap<String, String>> {
        let mut params = HashMap::new();
        let mut path_idx = 0;

        for segment in &route.segments {
            match segment {
                RouteSegment::Literal(expected) => {
                    if path_idx >= path.len() || path[path_idx] != expected {
                        return None;
                    }
                    path_idx += 1;
                }
                RouteSegment::Param(name) => {
                    if path_idx >= path.len() {
                        return None;
                    }
                    params.insert(name.clone(), path[path_idx].to_string());
                    path_idx += 1;
                }
                RouteSegment::Wildcard => {
                    // Consume rest of path
                    let rest = path[path_idx..].join("/");
                    params.insert("*".to_string(), rest);
                    return Some(params);
                }
            }
        }

        // All segments matched, check path is exhausted
        if path_idx == path.len() {
            Some(params)
        } else {
            None
        }
    }
}
```

### Navigation Action

```yaml
handlers:
  - event: click
    actions:
      - action: navigate
        path: "/users/123"

      # Or with expression
      - action: navigate
        path:
          type: format
          template: "/users/{0}"
          args:
            - type: state
              scope: user
              state_id: Selection
              path: userId
```

### Client-Side Navigation

```rust
// In capsule.js generation, add navigation support

// Handle navigation action
fn handle_navigate(&self, path: &str) {
    // Push to browser history
    // Re-render with new route params
}
```

---

## State Persistence

### Persistence Configuration

```yaml
state:
  user:
    - id: UserPrefs
      persist: true  # Persist to storage
      fields:
        - name: theme
          type: string
          default: "dark"
        - name: language
          type: string
          default: "en"

  global:
    - id: AppStats
      persist: true
      fields:
        - name: totalVisits
          type: number
          default: 0
```

### Persistence Adapter

```rust
// src/persist.rs

use serde_json::Value;
use std::collections::HashMap;

pub trait PersistenceAdapter: Send + Sync {
    /// Load state for a state definition
    fn load(&self, state_id: &str, session_id: Option<&str>) -> Option<HashMap<String, Value>>;

    /// Save state
    fn save(&self, state_id: &str, session_id: Option<&str>, data: &HashMap<String, Value>);
}

/// File-based JSON persistence
pub struct JsonFilePersist {
    base_path: PathBuf,
}

impl JsonFilePersist {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self { base_path: base_path.into() }
    }

    fn path_for(&self, state_id: &str, session_id: Option<&str>) -> PathBuf {
        match session_id {
            Some(sid) => self.base_path.join(format!("user_{}_{}.json", sid, state_id)),
            None => self.base_path.join(format!("global_{}.json", state_id)),
        }
    }
}

impl PersistenceAdapter for JsonFilePersist {
    fn load(&self, state_id: &str, session_id: Option<&str>) -> Option<HashMap<String, Value>> {
        let path = self.path_for(state_id, session_id);
        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn save(&self, state_id: &str, session_id: Option<&str>, data: &HashMap<String, Value>) {
        let path = self.path_for(state_id, session_id);
        if let Ok(content) = serde_json::to_string_pretty(data) {
            let _ = std::fs::write(&path, content);
        }
    }
}

/// SQLite persistence
pub struct SqlitePersist {
    pool: SqlitePool,
}

impl SqlitePersist {
    pub async fn new(db_path: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(db_path).await?;

        // Create tables
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS global_state (
                state_id TEXT NOT NULL,
                field TEXT NOT NULL,
                value TEXT,
                PRIMARY KEY (state_id, field)
            )
        "#).execute(&pool).await?;

        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS user_state (
                session_id TEXT NOT NULL,
                state_id TEXT NOT NULL,
                field TEXT NOT NULL,
                value TEXT,
                PRIMARY KEY (session_id, state_id, field)
            )
        "#).execute(&pool).await?;

        Ok(Self { pool })
    }
}
```

### Integration with DynamicState

```rust
// src/state.rs

impl DynamicState {
    pub fn with_persistence(mut self, adapter: Arc<dyn PersistenceAdapter>) -> Self {
        self.persistence = Some(adapter);
        self
    }

    pub fn load_persisted(&mut self, schema: &StateSchema) {
        let Some(adapter) = &self.persistence else { return };

        // Load global persisted state
        for state_def in schema.global.iter().filter(|s| s.persist) {
            if let Some(data) = adapter.load(&state_def.id, None) {
                let mut global = self.global.write().unwrap();
                global.insert(state_def.id.clone(), data);
            }
        }
    }

    pub fn save_persisted(&self, state_id: &str, scope: Scope, session_id: Option<&str>) {
        let Some(adapter) = &self.persistence else { return };

        let data = match scope {
            Scope::Global => {
                let global = self.global.read().unwrap();
                global.get(state_id).cloned()
            }
            Scope::User => {
                let session_id = session_id?;
                let user = self.user.read().unwrap();
                user.get(session_id)?.get(state_id).cloned()
            }
        };

        if let Some(data) = data {
            adapter.save(state_id, if scope == Scope::User { session_id } else { None }, &data);
        }
    }
}
```

---

## Rust Handler Escape Hatch

For complex logic that can't be expressed in YAML, allow calling Rust functions.

### Configuration

```yaml
handlers:
  - event: click
    actions:
      - action: call_rust
        function: validate_and_submit
        args:
          form_data:
            type: state
            scope: user
            state_id: ContactForm
            path: ""
```

### Rust Handler Registration

```rust
// In user's main.rs

use rwire_token::{TokenRuntime, RustHandler, HandlerContext, HandlerResult};
use serde_json::Value;

fn main() {
    let app = load_app("app.yaml");

    let mut runtime = TokenRuntime::new(app);

    // Register custom Rust handlers
    runtime.register_rust_handler("validate_and_submit", |ctx: HandlerContext| {
        let form_data = ctx.args.get("form_data")?;

        // Complex validation logic
        let email = form_data.get("email")?.as_str()?;
        if !email.contains('@') {
            return HandlerResult::error("Invalid email");
        }

        // Call external API
        let response = reqwest::blocking::post("https://api.example.com/contact")
            .json(&form_data)
            .send()
            .ok()?;

        if response.status().is_success() {
            // Update state
            HandlerResult::mutations(vec![
                Mutation::set("ContactForm", "submitted", Value::Bool(true)),
                Mutation::set("ContactForm", "error", Value::Null),
            ])
        } else {
            HandlerResult::mutations(vec![
                Mutation::set("ContactForm", "error", Value::String("Submission failed".into())),
            ])
        }
    });

    runtime.serve("127.0.0.1:9000");
}
```

### Handler Registry

```rust
// src/rust_handlers.rs

use serde_json::Value;
use std::collections::HashMap;

pub type RustHandlerFn = Box<dyn Fn(HandlerContext) -> Option<HandlerResult> + Send + Sync>;

pub struct HandlerContext {
    pub session_id: String,
    pub args: HashMap<String, Value>,
    pub event_value: Option<Value>,
}

pub struct HandlerResult {
    pub mutations: Vec<(Scope, String, String, Value)>,  // scope, state_id, field, value
    pub error: Option<String>,
    pub navigate_to: Option<String>,
}

impl HandlerResult {
    pub fn mutations(mutations: Vec<Mutation>) -> Option<Self> {
        Some(Self {
            mutations: mutations.into_iter().map(|m| m.into_tuple()).collect(),
            error: None,
            navigate_to: None,
        })
    }

    pub fn error(msg: &str) -> Option<Self> {
        Some(Self {
            mutations: vec![],
            error: Some(msg.to_string()),
            navigate_to: None,
        })
    }
}

pub struct RustHandlerRegistry {
    handlers: HashMap<String, RustHandlerFn>,
}

impl RustHandlerRegistry {
    pub fn new() -> Self {
        Self { handlers: HashMap::new() }
    }

    pub fn register<F>(&mut self, name: &str, handler: F)
    where
        F: Fn(HandlerContext) -> Option<HandlerResult> + Send + Sync + 'static,
    {
        self.handlers.insert(name.to_string(), Box::new(handler));
    }

    pub fn call(&self, name: &str, ctx: HandlerContext) -> Option<HandlerResult> {
        self.handlers.get(name).and_then(|f| f(ctx))
    }
}
```

---

## Task System

For async operations like API calls, file I/O, timers.

### Task Definition

```yaml
tasks:
  fetch_users:
    description: "Fetch users from API"
    type: http
    config:
      url: "https://api.example.com/users"
      method: GET

  save_draft:
    description: "Save draft to server"
    type: http
    config:
      url: "https://api.example.com/drafts"
      method: POST

  debounce_search:
    description: "Debounced search"
    type: debounce
    config:
      delay_ms: 300
```

### Spawning Tasks

```yaml
handlers:
  - event: click
    actions:
      - action: spawn_task
        task: fetch_users
        on_success:
          - action: update_state
            scope: user
            state_id: UserList
            mutations:
              - op: set
                field: users
                value:
                  type: task_result
        on_error:
          - action: update_state
            scope: user
            state_id: UserList
            mutations:
              - op: set
                field: error
                value:
                  type: task_error
```

### Task Executor

```rust
// src/tasks.rs

use serde_json::Value;
use tokio::sync::mpsc;

pub struct TaskExecutor {
    sender: mpsc::Sender<TaskMessage>,
}

enum TaskMessage {
    Spawn {
        task_id: String,
        task_def: TaskDefinition,
        session_id: String,
        callback: TaskCallback,
    },
    Cancel {
        task_id: String,
    },
}

pub struct TaskCallback {
    pub on_success: Vec<Action>,
    pub on_error: Vec<Action>,
}

impl TaskExecutor {
    pub fn new() -> Self {
        let (sender, mut receiver) = mpsc::channel(100);

        tokio::spawn(async move {
            while let Some(msg) = receiver.recv().await {
                match msg {
                    TaskMessage::Spawn { task_def, session_id, callback, .. } => {
                        let result = Self::execute_task(&task_def).await;
                        // Send result back through channel or callback
                    }
                    TaskMessage::Cancel { task_id } => {
                        // Cancel running task
                    }
                }
            }
        });

        Self { sender }
    }

    async fn execute_task(task_def: &TaskDefinition) -> Result<Value, String> {
        match &task_def.task_type {
            TaskType::Http { url, method, body, headers } => {
                let client = reqwest::Client::new();

                let mut request = match method.as_str() {
                    "GET" => client.get(url),
                    "POST" => client.post(url),
                    "PUT" => client.put(url),
                    "DELETE" => client.delete(url),
                    _ => return Err("Unknown method".to_string()),
                };

                if let Some(body) = body {
                    request = request.json(body);
                }

                for (key, value) in headers {
                    request = request.header(key, value);
                }

                let response = request.send().await
                    .map_err(|e| e.to_string())?;

                let json: Value = response.json().await
                    .map_err(|e| e.to_string())?;

                Ok(json)
            }

            TaskType::Debounce { delay_ms } => {
                tokio::time::sleep(std::time::Duration::from_millis(*delay_ms)).await;
                Ok(Value::Null)
            }

            TaskType::Timer { interval_ms } => {
                // Periodic timer
                tokio::time::sleep(std::time::Duration::from_millis(*interval_ms)).await;
                Ok(Value::Null)
            }
        }
    }

    pub fn spawn(&self, task_id: &str, task_def: TaskDefinition, session_id: &str, callback: TaskCallback) {
        let _ = self.sender.try_send(TaskMessage::Spawn {
            task_id: task_id.to_string(),
            task_def,
            session_id: session_id.to_string(),
            callback,
        });
    }
}
```

---

## Multi-Page Application

### Shell Layout

```yaml
ui:
  shell:
    tag: div
    styles:
      - property: min-height
        value: "100vh"
      - property: display
        value: flex
      - property: flex-direction
        value: column
    children:
      # Navigation
      - type: node
        tag: nav
        styles:
          - property: background
            value: $colors.surface
          - property: padding
            value: $spacing.md
        children:
          - type: node
            tag: a
            attrs:
              - name: href
                value: /
            children:
              - type: text
                content: "Home"
          - type: node
            tag: a
            attrs:
              - name: href
                value: /about
            children:
              - type: text
                content: "About"

      # Page content placeholder
      - type: node
        tag: main
        attrs:
          - name: id
            value: page-content
        styles:
          - property: flex
            value: "1"

      # Footer
      - type: node
        tag: footer
        styles:
          - property: background
            value: $colors.surface
          - property: padding
            value: $spacing.md
          - property: text-align
            value: center
        children:
          - type: text
            content: "© 2026 My App"
```

### Page Rendering with Shell

```rust
// src/interpreter.rs

impl<'a> Interpreter<'a> {
    pub fn render_with_shell(&self, page_name: &str, ctx: &EvalContext) -> Option<ElementBuilder> {
        let page = self.render_page(page_name, ctx)?;

        if let Some(shell) = &self.app.ui.shell {
            // Render shell and insert page at #page-content
            let shell_builder = self.render_node(shell, ctx);
            // Replace page-content placeholder with actual page
            Some(shell_builder.replace_by_id("page-content", page))
        } else {
            Some(page)
        }
    }
}
```

---

## Complete Multi-Page Example

```yaml
version: "1.0"

tokens:
  colors:
    primary: "#5E81AC"
    background: "#2E3440"
    surface: "#3B4252"
    text: "#ECEFF4"

  spacing:
    sm: "0.5rem"
    md: "1rem"
    lg: "1.5rem"

state:
  user:
    - id: Auth
      persist: true
      fields:
        - name: isLoggedIn
          type: boolean
          default: false
        - name: username
          type: string
          default: ""

    - id: Navigation
      fields:
        - name: currentPage
          type: string
          default: "home"

routes:
  - path: /
    page: home

  - path: /about
    page: about

  - path: /users/:id
    page: user-profile

  - path: /login
    page: login

  - path: /dashboard
    page: dashboard
    guards:
      - requireAuth

ui:
  shell:
    tag: div
    styles:
      - property: min-height
        value: "100vh"
      - property: background
        value: $colors.background
      - property: color
        value: $colors.text
    children:
      - type: ref
        component_id: navbar
      - type: node
        tag: main
        attrs:
          - name: id
            value: page-content
        styles:
          - property: padding
            value: $spacing.lg

  components:
    navbar:
      tag: nav
      styles:
        - property: display
          value: flex
        - property: gap
          value: $spacing.md
        - property: padding
          value: $spacing.md
        - property: background
          value: $colors.surface
      children:
        - type: ref
          component_id: nav-link
          props:
            href: /
            text: "Home"
        - type: ref
          component_id: nav-link
          props:
            href: /about
            text: "About"
        - type: conditional
          condition:
            type: state
            scope: user
            state_id: Auth
            path: isLoggedIn
          then:
            type: ref
            component_id: nav-link
            props:
              href: /dashboard
              text: "Dashboard"
          else:
            type: ref
            component_id: nav-link
            props:
              href: /login
              text: "Login"

    nav-link:
      tag: a
      attrs:
        - name: href
          value:
            type: prop
            name: href
      styles:
        - property: color
          value: $colors.text
        - property: text-decoration
          value: none
      children:
        - type: dynamic_text
          expr:
            type: prop
            name: text

  pages:
    home:
      title: "Home"
      tag: div
      children:
        - type: node
          tag: h1
          children:
            - type: text
              content: "Welcome Home"

    about:
      title: "About"
      tag: div
      children:
        - type: node
          tag: h1
          children:
            - type: text
              content: "About Us"

    user-profile:
      title: "User Profile"
      tag: div
      children:
        - type: node
          tag: h1
          children:
            - type: dynamic_text
              expr:
                type: format
                template: "User {0}"
                args:
                  - type: param
                    name: id

    login:
      title: "Login"
      tag: div
      children:
        - type: node
          tag: h1
          children:
            - type: text
              content: "Login"
        - type: node
          tag: button
          handlers:
            - event: click
              actions:
                - action: update_state
                  scope: user
                  state_id: Auth
                  mutations:
                    - op: set
                      field: isLoggedIn
                      value:
                        type: literal
                        value: true
                    - op: set
                      field: username
                      value:
                        type: literal
                        value: "demo"
                - action: navigate
                  path: "/dashboard"
          children:
            - type: text
              content: "Login as Demo User"

    dashboard:
      title: "Dashboard"
      tag: div
      children:
        - type: node
          tag: h1
          children:
            - type: dynamic_text
              expr:
                type: format
                template: "Welcome, {0}!"
                args:
                  - type: state
                    scope: user
                    state_id: Auth
                    path: username
        - type: node
          tag: button
          handlers:
            - event: click
              actions:
                - action: update_state
                  scope: user
                  state_id: Auth
                  mutations:
                    - op: set
                      field: isLoggedIn
                      value:
                        type: literal
                        value: false
                - action: navigate
                  path: "/"
          children:
            - type: text
              content: "Logout"
```

---

## Success Criteria

- [ ] URL routing matches static paths
- [ ] URL routing extracts parameters (:id)
- [ ] URL routing handles wildcards (*)
- [ ] Navigation action changes page
- [ ] Browser history updated on navigation
- [ ] Persisted state survives restart
- [ ] JsonFilePersist saves/loads correctly
- [ ] Rust handlers can be registered
- [ ] Rust handlers receive context
- [ ] Rust handlers can modify state
- [ ] Tasks spawn and execute async
- [ ] Task success/error callbacks work
- [ ] Shell wraps all pages
- [ ] Multi-page app works end-to-end

---

## Future Considerations

### Beyond Phase 6

- **Component Props** - Full prop system (not just slots)
- **Animations** - CSS transitions triggered by state
- **Forms** - Built-in validation rules
- **i18n** - Internationalization support
- **SEO** - Meta tags, SSR for crawlers
- **PWA** - Service worker, offline support
- **DevTools** - State inspector, event debugger

### Performance Optimizations

- **Incremental Updates** - Only re-render changed regions
- **Virtual Scrolling** - For large lists
- **Code Splitting** - Load pages on demand
- **Caching** - Cache rendered pages

### Ecosystem

- **VS Code Extension** - YAML intellisense, validation
- **Component Library** - Pre-built styled components
- **Templates** - Starter templates for common apps
