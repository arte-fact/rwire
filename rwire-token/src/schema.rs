//! Token schema types for declarative YAML apps.

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// Root application definition.
#[derive(Debug, Clone, Deserialize)]
pub struct AppToken {
    pub version: String,
    #[serde(default)]
    pub state: StateSchema,
    pub ui: UiSchema,
    #[serde(default)]
    pub routes: Vec<Route>,
}

/// State definitions with global and user scopes.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct StateSchema {
    #[serde(default)]
    pub global: Vec<StateDefinition>,
    #[serde(default)]
    pub user: Vec<StateDefinition>,
}

/// A single state definition with fields.
#[derive(Debug, Clone, Deserialize)]
pub struct StateDefinition {
    pub id: String,
    pub fields: Vec<FieldDefinition>,
}

/// Field definition within a state.
#[derive(Debug, Clone, Deserialize)]
pub struct FieldDefinition {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    #[serde(default)]
    pub default: Value,
}

/// Supported field types.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

/// UI definitions including pages and components.
#[derive(Debug, Clone, Deserialize)]
pub struct UiSchema {
    #[serde(default)]
    pub shell: Option<NodeToken>,
    pub pages: HashMap<String, PageToken>,
    #[serde(default)]
    pub components: HashMap<String, NodeToken>,
}

/// Page definition with title and content.
#[derive(Debug, Clone, Deserialize)]
pub struct PageToken {
    pub title: String,
    #[serde(flatten)]
    pub node: NodeToken,
}

/// HTML element token.
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

/// Element attribute.
#[derive(Debug, Clone, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub value: StringOrExpr,
}

/// CSS style property.
#[derive(Debug, Clone, Deserialize)]
pub struct Style {
    pub property: String,
    pub value: StringOrExpr,
}

/// Child content types.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChildToken {
    /// Static text content.
    Text { content: String },

    /// Dynamic text from expression.
    DynamicText { expr: Expr },

    /// Nested HTML element.
    Node(Box<NodeToken>),

    /// Conditional rendering.
    Conditional {
        condition: Expr,
        then: Box<ChildToken>,
        #[serde(rename = "else")]
        else_: Option<Box<ChildToken>>,
    },

    /// Loop over array.
    Loop {
        over: Expr,
        #[serde(rename = "as")]
        as_: String,
        index: Option<String>,
        body: Box<ChildToken>,
    },

    /// Reference to component.
    Ref {
        component_id: String,
        #[serde(default)]
        slots: HashMap<String, Vec<ChildToken>>,
    },

    /// Slot placeholder in component.
    Slot {
        name: String,
        #[serde(default)]
        default: Vec<ChildToken>,
    },
}

/// Route definition.
#[derive(Debug, Clone, Deserialize)]
pub struct Route {
    pub path: String,
    pub page: String,
}

/// Event handler definition.
#[derive(Debug, Clone, Deserialize)]
pub struct Handler {
    pub event: Event,
    #[serde(default)]
    pub id: Option<String>,
    pub actions: Vec<Action>,
}

/// Supported event types.
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

/// Actions to execute on events.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    /// Update state with mutations.
    UpdateState {
        scope: Scope,
        state_id: String,
        mutations: Vec<Mutation>,
    },

    /// Navigate to path.
    Navigate { path: StringOrExpr },

    /// Prevent default browser behavior.
    PreventDefault,

    /// Log message (debugging).
    Log { message: StringOrExpr },
}

/// State mutation.
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

/// Mutation operations.
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

/// State scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Global,
    User,
}

/// Expression for dynamic values.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Expr {
    /// Static value.
    Literal { value: Value },

    /// State field access.
    State {
        scope: Scope,
        state_id: String,
        path: String,
    },

    /// Event value (input value, etc).
    EventValue,

    /// Route parameter.
    Param { name: String },

    /// Local variable (loop item).
    Local { name: String },

    /// Format string with placeholders.
    Format { template: String, args: Vec<Expr> },

    /// Comparison expression.
    Compare {
        op: CompareOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    /// Array/string length.
    Length { value: Box<Expr> },

    /// Ternary conditional.
    Ternary {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },

    /// Logical operations.
    Logical { op: LogicalOp, operands: Vec<Expr> },

    /// Field access on object.
    Field { base: Box<Expr>, field: String },
}

/// Comparison operators.
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

/// Logical operators.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogicalOp {
    And,
    Or,
    Not,
}

/// String or expression value.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum StringOrExpr {
    String(String),
    Expr(Expr),
}

impl Default for StringOrExpr {
    fn default() -> Self {
        Self::String(String::new())
    }
}
