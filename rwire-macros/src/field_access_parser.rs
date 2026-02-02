//! Parse function bodies to extract field accesses from state parameters.
//!
//! This module provides AST analysis to determine which state fields a renderer
//! depends on (reads) or a handler modifies (writes). The results are used to
//! generate compile-time bitmasks for fine-grained reactivity.
//!
//! # Limitations
//!
//! - Only direct `state.field` accesses are detected
//! - Method calls like `state.items.len()` detect `items` but not nested fields
//! - Conditionals like `if cond { state.a } else { state.b }` detect both fields
//! - Complex patterns (closures, method call chains) may miss some accesses

use syn::visit::Visit;
use syn::{Block, Expr, ExprField, ExprMethodCall, Member};

/// Result of parsing a function body for field accesses.
#[derive(Debug, Default)]
pub struct FieldAccesses {
    /// Field names that are read from the state parameter
    pub reads: Vec<String>,
    /// Field names that are written to the state parameter
    pub writes: Vec<String>,
}

impl FieldAccesses {
    /// Deduplicate and sort the field lists.
    pub fn normalize(&mut self) {
        self.reads.sort();
        self.reads.dedup();
        self.writes.sort();
        self.writes.dedup();
    }
}

/// Visitor that extracts field accesses from an AST.
struct FieldAccessVisitor<'a> {
    /// The name of the state parameter (e.g., "state")
    state_param: &'a str,
    /// Collected field accesses
    accesses: FieldAccesses,
    /// Whether we're currently in an assignment LHS (write context)
    in_write_context: bool,
}

impl<'a> FieldAccessVisitor<'a> {
    fn new(state_param: &'a str) -> Self {
        Self {
            state_param,
            accesses: FieldAccesses::default(),
            in_write_context: false,
        }
    }

    /// Check if an expression is a reference to the state parameter.
    fn is_state_param(&self, expr: &Expr) -> bool {
        matches!(expr, Expr::Path(path) if path.path.is_ident(self.state_param))
    }

    /// Extract field name from a member access.
    fn get_field_name(&self, member: &Member) -> Option<String> {
        match member {
            Member::Named(ident) => Some(ident.to_string()),
            Member::Unnamed(_) => None, // Tuple field access, not supported
        }
    }

    /// Record a field access.
    fn record_access(&mut self, field_name: String, is_write: bool) {
        if is_write {
            self.accesses.writes.push(field_name);
        } else {
            self.accesses.reads.push(field_name);
        }
    }
}

impl<'ast> Visit<'ast> for FieldAccessVisitor<'_> {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            // Handle assignments: state.field = value
            Expr::Assign(assign) => {
                // Visit LHS in write context
                self.in_write_context = true;
                self.visit_expr(&assign.left);
                self.in_write_context = false;

                // Visit RHS in read context
                self.visit_expr(&assign.right);
            }

            // Handle compound assignments: state.field += value
            Expr::Binary(binary) if is_assign_op(&binary.op) => {
                // Visit LHS in write context (compound assignments are read+write)
                self.in_write_context = true;
                self.visit_expr(&binary.left);
                self.in_write_context = false;

                // Also record as read since += reads before writing
                self.visit_expr(&binary.left);

                // Visit RHS in read context
                self.visit_expr(&binary.right);
            }

            // Handle field access: state.field
            Expr::Field(field) => {
                self.visit_field_expr(field);
            }

            // Handle method calls on state fields: state.items.push(x)
            Expr::MethodCall(method_call) => {
                self.visit_method_call(method_call);
            }

            // Default: visit children
            _ => {
                syn::visit::visit_expr(self, expr);
            }
        }
    }

    fn visit_expr_field(&mut self, field: &'ast ExprField) {
        // Check if this is state.field
        if self.is_state_param(&field.base) {
            if let Some(field_name) = self.get_field_name(&field.member) {
                self.record_access(field_name, self.in_write_context);
            }
        } else {
            // Could be nested: state.items.len() - the base might be a method call
            // Continue visiting to find state accesses in the base
            syn::visit::visit_expr_field(self, field);
        }
    }
}

impl FieldAccessVisitor<'_> {
    fn visit_field_expr(&mut self, field: &ExprField) {
        // Check if this is state.field
        if self.is_state_param(&field.base) {
            if let Some(field_name) = self.get_field_name(&field.member) {
                self.record_access(field_name, self.in_write_context);
            }
        } else if let Expr::Field(inner_field) = &*field.base {
            // Nested field access like state.nested.field
            // Check if the innermost is state
            self.visit_field_expr(inner_field);
        } else {
            // Visit the base expression
            syn::visit::visit_expr(self, &field.base);
        }
    }

    fn visit_method_call(&mut self, method_call: &ExprMethodCall) {
        // Check if this is a method call on a state field: state.items.push(x)
        // The receiver could be state.field or state.field.method()...

        // Extract the base - could be state.field or deeper
        if let Expr::Field(field) = &*method_call.receiver {
            if self.is_state_param(&field.base) {
                if let Some(field_name) = self.get_field_name(&field.member) {
                    // Method calls on state fields are treated as writes
                    // because methods like push(), clear(), retain() mutate
                    let is_mutating = is_mutating_method(&method_call.method.to_string());
                    self.record_access(field_name.clone(), is_mutating);
                    // Also record as read since we're accessing the field
                    if !is_mutating {
                        self.record_access(field_name, false);
                    }
                }
            } else {
                // Nested method call, visit recursively
                self.visit_field_expr(field);
            }
        } else {
            // Visit receiver in case it contains state accesses
            syn::visit::visit_expr(self, &method_call.receiver);
        }

        // Visit method arguments
        for arg in &method_call.args {
            syn::visit::visit_expr(self, arg);
        }
    }
}

/// Check if a binary operator is an assignment operator.
fn is_assign_op(op: &syn::BinOp) -> bool {
    matches!(
        op,
        syn::BinOp::AddAssign(_)
            | syn::BinOp::SubAssign(_)
            | syn::BinOp::MulAssign(_)
            | syn::BinOp::DivAssign(_)
            | syn::BinOp::RemAssign(_)
            | syn::BinOp::BitXorAssign(_)
            | syn::BinOp::BitAndAssign(_)
            | syn::BinOp::BitOrAssign(_)
            | syn::BinOp::ShlAssign(_)
            | syn::BinOp::ShrAssign(_)
    )
}

/// Check if a method name is known to mutate its receiver.
fn is_mutating_method(method: &str) -> bool {
    matches!(
        method,
        "push"
            | "pop"
            | "insert"
            | "remove"
            | "clear"
            | "retain"
            | "sort"
            | "sort_by"
            | "reverse"
            | "extend"
            | "append"
            | "drain"
            | "truncate"
            | "swap"
            | "swap_remove"
    )
}

/// Extract field accesses from a function block.
///
/// Returns a list of field names that the function accesses on the state parameter.
pub fn extract_field_accesses(block: &Block, state_param: &str) -> FieldAccesses {
    let mut visitor = FieldAccessVisitor::new(state_param);
    syn::visit::visit_block(&mut visitor, block);
    visitor.accesses.normalize();
    visitor.accesses
}

/// Extract read-only field accesses (for renderers).
///
/// Renderers only read state, so we just need the reads.
pub fn extract_reads(block: &Block, state_param: &str) -> Vec<String> {
    let accesses = extract_field_accesses(block, state_param);
    // For renderers, all accesses are reads (they take &State)
    let mut all: Vec<_> = accesses.reads.into_iter().chain(accesses.writes).collect();
    all.sort();
    all.dedup();
    all
}

/// Extract write-only field accesses (for handlers).
///
/// Handlers mutate state, so we need the writes.
pub fn extract_writes(block: &Block, state_param: &str) -> Vec<String> {
    let accesses = extract_field_accesses(block, state_param);
    accesses.writes
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    fn parse_block(code: proc_macro2::TokenStream) -> Block {
        syn::parse2(quote! { { #code } }).unwrap()
    }

    #[test]
    fn test_simple_read() {
        let block = parse_block(quote! {
            let x = state.count;
        });
        let accesses = extract_field_accesses(&block, "state");
        assert!(accesses.reads.contains(&"count".to_string()));
        assert!(accesses.writes.is_empty());
    }

    #[test]
    fn test_simple_write() {
        let block = parse_block(quote! {
            state.count = 5;
        });
        let accesses = extract_field_accesses(&block, "state");
        assert!(accesses.writes.contains(&"count".to_string()));
    }

    #[test]
    fn test_compound_assignment() {
        let block = parse_block(quote! {
            state.count += 1;
        });
        let accesses = extract_field_accesses(&block, "state");
        // Compound assignment is both read and write
        assert!(accesses.writes.contains(&"count".to_string()));
        assert!(accesses.reads.contains(&"count".to_string()));
    }

    #[test]
    fn test_method_call_mutating() {
        let block = parse_block(quote! {
            state.items.push(item);
        });
        let accesses = extract_field_accesses(&block, "state");
        assert!(accesses.writes.contains(&"items".to_string()));
    }

    #[test]
    fn test_method_call_non_mutating() {
        let block = parse_block(quote! {
            let len = state.items.len();
        });
        let accesses = extract_field_accesses(&block, "state");
        assert!(accesses.reads.contains(&"items".to_string()));
    }

    #[test]
    fn test_multiple_fields() {
        let block = parse_block(quote! {
            state.count += 1;
            state.name = "hello".to_string();
        });
        let accesses = extract_field_accesses(&block, "state");
        assert!(accesses.writes.contains(&"count".to_string()));
        assert!(accesses.writes.contains(&"name".to_string()));
    }

    #[test]
    fn test_conditional_access() {
        let block = parse_block(quote! {
            if state.enabled {
                return state.value;
            }
        });
        let accesses = extract_field_accesses(&block, "state");
        assert!(accesses.reads.contains(&"enabled".to_string()));
        assert!(accesses.reads.contains(&"value".to_string()));
    }

    #[test]
    fn test_format_string() {
        // Note: format! macros are not fully expanded, so we test with direct access
        let block = parse_block(quote! {
            let count = state.count;
            format!("Count: {}", count)
        });
        let accesses = extract_field_accesses(&block, "state");
        assert!(accesses.reads.contains(&"count".to_string()));
    }

    #[test]
    fn test_different_param_name() {
        let block = parse_block(quote! {
            s.count += 1;
        });
        let accesses = extract_field_accesses(&block, "s");
        assert!(accesses.writes.contains(&"count".to_string()));
    }

    #[test]
    fn test_extract_reads() {
        let block = parse_block(quote! {
            el.text(&state.count.to_string())
        });
        let reads = extract_reads(&block, "state");
        assert!(reads.contains(&"count".to_string()));
    }

    #[test]
    fn test_extract_writes() {
        let block = parse_block(quote! {
            state.items.retain(|x| x.done);
        });
        let writes = extract_writes(&block, "state");
        assert!(writes.contains(&"items".to_string()));
    }
}
