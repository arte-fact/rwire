//! Interpreter that converts tokens to rwire ElementBuilder.

use crate::expr::{value_to_display_string, EvalContext};
use crate::schema::{AppToken, ChildToken, NodeToken};
use crate::state::DynamicState;
use rwire::builder::ElementBuilder;
use rwire::protocol::opcodes::El;
use serde_json::Value;

/// Interpreter for rendering token trees.
pub struct Interpreter<'a> {
    pub app: &'a AppToken,
    pub state: &'a DynamicState,
}

impl<'a> Interpreter<'a> {
    /// Create interpreter with app and state.
    pub fn new(app: &'a AppToken, state: &'a DynamicState) -> Self {
        Self { app, state }
    }

    /// Render a page by name.
    pub fn render_page(&self, page_name: &str, ctx: &EvalContext) -> Option<ElementBuilder> {
        let page = self.app.ui.pages.get(page_name)?;
        Some(self.render_node(&page.node, ctx))
    }

    /// Render a node token to ElementBuilder.
    pub fn render_node(&self, node: &NodeToken, ctx: &EvalContext) -> ElementBuilder {
        let el_type = string_to_el(&node.tag);
        let mut builder = rwire::builder::el(el_type);

        // Apply attributes
        for attr in &node.attrs {
            let value = attr.value.resolve(ctx);
            builder = builder.attr(&attr.name, &value);
        }

        // Apply styles as inline style attribute
        if !node.styles.is_empty() {
            let style_str = self.render_styles(&node.styles, ctx);
            builder = builder.attr("style", &style_str);
        }

        // Render children
        let children: Vec<ElementBuilder> = node
            .children
            .iter()
            .filter_map(|child| self.render_child(child, ctx))
            .collect();

        if !children.is_empty() {
            builder = builder.append(children);
        }

        // Note: Handlers are not bound in Phase 1 (static rendering only)

        builder
    }

    /// Render a child token.
    fn render_child(&self, child: &ChildToken, ctx: &EvalContext) -> Option<ElementBuilder> {
        match child {
            ChildToken::Text { content } => Some(rwire::builder::el(El::Span).text(content)),

            ChildToken::DynamicText { expr } => {
                let value = expr.eval(ctx);
                let text = value_to_display_string(&value);
                Some(rwire::builder::el(El::Span).text(&text))
            }

            ChildToken::Node(node) => Some(self.render_node(node, ctx)),

            ChildToken::Conditional {
                condition,
                then,
                else_,
            } => {
                let cond_value = condition.eval(ctx);
                if value_is_truthy(&cond_value) {
                    self.render_child(then, ctx)
                } else if let Some(else_child) = else_ {
                    self.render_child(else_child, ctx)
                } else {
                    None
                }
            }

            ChildToken::Loop {
                over,
                as_,
                index,
                body,
            } => {
                let items = match over.eval(ctx) {
                    Value::Array(arr) => arr,
                    _ => return Some(rwire::builder::el(El::Div)), // Empty container
                };

                let children: Vec<ElementBuilder> = items
                    .iter()
                    .enumerate()
                    .filter_map(|(i, item)| {
                        let mut child_ctx = ctx.clone();
                        child_ctx.locals.insert(as_.clone(), item.clone());
                        if let Some(idx_name) = index {
                            child_ctx
                                .locals
                                .insert(idx_name.clone(), Value::Number(i.into()));
                        }
                        self.render_child(body, &child_ctx)
                    })
                    .collect();

                Some(rwire::builder::el(El::Div).append(children))
            }

            ChildToken::Ref {
                component_id,
                slots,
            } => {
                let component = self.app.ui.components.get(component_id)?;
                let ctx_with_slots = ctx.clone().with_slots(slots);
                Some(self.render_node(component, &ctx_with_slots))
            }

            ChildToken::Slot { name, default } => {
                // Check if slot content was provided
                let content = ctx
                    .slots
                    .get(name)
                    .map(|c| c.as_slice())
                    .unwrap_or(default.as_slice());

                if content.is_empty() {
                    return None;
                }

                let children: Vec<ElementBuilder> = content
                    .iter()
                    .filter_map(|c| self.render_child(c, ctx))
                    .collect();

                match children.len() {
                    0 => None,
                    1 => Some(children.into_iter().next().unwrap()),
                    _ => Some(rwire::builder::el(El::Div).append(children)),
                }
            }
        }
    }

    /// Render styles to inline style string.
    fn render_styles(&self, styles: &[crate::schema::Style], ctx: &EvalContext) -> String {
        styles
            .iter()
            .map(|s| {
                let value = s.value.resolve(ctx);
                format!("{}: {}", s.property, value)
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// Convert tag string to El enum.
fn string_to_el(tag: &str) -> El {
    match tag {
        "div" => El::Div,
        "span" => El::Span,
        "p" => El::P,
        "h1" => El::H1,
        "h2" => El::H2,
        "h3" => El::H2, // Fallback to H2 (H3 not in rwire yet)
        "button" => El::Button,
        "input" => El::Input,
        "form" => El::Form,
        "ul" => El::Ul,
        "li" => El::Li,
        "a" => El::A,
        "label" => El::Label,
        "textarea" => El::Textarea,
        "select" => El::Select,
        "option" => El::Option,
        "nav" => El::Nav,
        "header" => El::Header,
        "footer" => El::Footer,
        _ => El::Div, // Fallback
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
    use crate::schema::{Expr, PageToken, StateSchema, UiSchema};
    use std::collections::HashMap;

    fn minimal_app() -> AppToken {
        AppToken {
            version: "1.0".to_string(),
            state: StateSchema::default(),
            ui: UiSchema {
                shell: None,
                pages: {
                    let mut pages = HashMap::new();
                    pages.insert(
                        "home".to_string(),
                        PageToken {
                            title: "Test".to_string(),
                            node: NodeToken {
                                tag: "div".to_string(),
                                attrs: vec![],
                                styles: vec![],
                                children: vec![ChildToken::Text {
                                    content: "Hello".to_string(),
                                }],
                                handlers: vec![],
                            },
                        },
                    );
                    pages
                },
                components: HashMap::new(),
            },
            routes: vec![],
        }
    }

    #[test]
    fn test_render_text() {
        let app = minimal_app();
        let state = DynamicState::from_schema(&app.state);
        let interpreter = Interpreter::new(&app, &state);
        let ctx = EvalContext::new(&state);

        let result = interpreter.render_page("home", &ctx);
        assert!(result.is_some());
    }

    #[test]
    fn test_render_dynamic_text() {
        let mut app = minimal_app();
        app.ui.pages.get_mut("home").unwrap().node.children = vec![ChildToken::DynamicText {
            expr: Expr::Literal {
                value: Value::String("Dynamic!".to_string()),
            },
        }];

        let state = DynamicState::from_schema(&app.state);
        let interpreter = Interpreter::new(&app, &state);
        let ctx = EvalContext::new(&state);

        let result = interpreter.render_page("home", &ctx);
        assert!(result.is_some());
    }

    #[test]
    fn test_render_loop() {
        let mut app = minimal_app();
        app.ui.pages.get_mut("home").unwrap().node.children = vec![ChildToken::Loop {
            over: Expr::Literal {
                value: Value::Array(vec![
                    Value::String("a".to_string()),
                    Value::String("b".to_string()),
                ]),
            },
            as_: "item".to_string(),
            index: Some("i".to_string()),
            body: Box::new(ChildToken::DynamicText {
                expr: Expr::Local {
                    name: "item".to_string(),
                },
            }),
        }];

        let state = DynamicState::from_schema(&app.state);
        let interpreter = Interpreter::new(&app, &state);
        let ctx = EvalContext::new(&state);

        let result = interpreter.render_page("home", &ctx);
        assert!(result.is_some());
    }
}
