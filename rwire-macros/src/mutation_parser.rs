//! Parse handler function bodies to extract mutation operations.
//!
//! This module analyzes Rust statements and expressions to determine what
//! field mutations a handler performs. Only supported patterns are allowed;
//! unsupported patterns result in compile-time errors.

use proc_macro2::Span;
use syn::{spanned::Spanned, BinOp, Expr, ExprAssign, ExprBinary, ExprUnary, Stmt, UnOp};

/// Represents a mutation operation extracted from handler code.
#[derive(Clone, Debug)]
pub enum MutationOp {
    /// Toggle boolean field: `state.field = !state.field`
    Toggle { field_name: String },
    /// Add i8 value: `state.field += n` where n fits in i8
    AddI8 { field_name: String, value: i8 },
    /// Add i32 value: `state.field += n`
    AddI32 { field_name: String, value: i32 },
    /// Set boolean: `state.field = true/false`
    SetBool { field_name: String, value: bool },
    /// Set i32: `state.field = n`
    SetI32 { field_name: String, value: i32 },
    /// Set string: `state.field = "..."`
    SetStr { field_name: String, value: String },
}

/// Error during mutation parsing.
#[derive(Debug)]
pub struct MutationError {
    pub span: Span,
    pub message: String,
}

impl MutationError {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

/// Parse a list of statements into mutation operations.
pub fn parse_mutations(
    stmts: &[Stmt],
    state_param: &str,
) -> Result<Vec<MutationOp>, MutationError> {
    let mut mutations = Vec::new();

    for stmt in stmts {
        match stmt {
            Stmt::Expr(expr, _semi) => {
                if let Some(m) = parse_expr_mutation(expr, state_param)? {
                    mutations.push(m);
                }
            }
            Stmt::Local(local) => {
                return Err(MutationError::new(
                    local.span(),
                    "let bindings not supported in local handlers; use direct field assignments",
                ));
            }
            Stmt::Item(_) => {
                return Err(MutationError::new(
                    stmt.span(),
                    "item definitions not supported in local handlers",
                ));
            }
            Stmt::Macro(m) => {
                return Err(MutationError::new(
                    m.span(),
                    "macro invocations not supported in local handlers",
                ));
            }
        }
    }

    Ok(mutations)
}

/// Parse a single expression into a mutation operation.
fn parse_expr_mutation(expr: &Expr, state_param: &str) -> Result<Option<MutationOp>, MutationError> {
    match expr {
        // Assignment: state.field = value
        Expr::Assign(assign) => parse_assignment(assign, state_param),

        // Compound assignment: state.field += value, state.field -= value
        Expr::Binary(binary) => parse_compound_assignment(binary, state_param),

        // Other expressions are not allowed
        _ => Err(MutationError::new(
            expr.span(),
            "unsupported expression in local handler; only field assignments \
                 (state.field = value) and compound assignments (state.field += n) are supported".to_string(),
        )),
    }
}

/// Parse an assignment expression: state.field = value
fn parse_assignment(assign: &ExprAssign, state_param: &str) -> Result<Option<MutationOp>, MutationError> {
    // Left side must be state.field
    let field_name = extract_field_access(&assign.left, state_param)?;

    // Right side determines the mutation type
    let right = &*assign.right;

    // Check for toggle: state.field = !state.field
    if let Expr::Unary(ExprUnary {
        op: UnOp::Not(_),
        expr: inner,
        ..
    }) = right
    {
        if let Some(inner_field) = try_extract_field_access(inner, state_param) {
            if inner_field == field_name {
                return Ok(Some(MutationOp::Toggle { field_name }));
            }
        }
    }

    // Check for boolean literal
    if let Expr::Lit(lit) = right {
        if let syn::Lit::Bool(b) = &lit.lit {
            return Ok(Some(MutationOp::SetBool {
                field_name,
                value: b.value,
            }));
        }
        if let syn::Lit::Int(i) = &lit.lit {
            let value: i32 = i.base10_parse().map_err(|e| {
                MutationError::new(i.span(), format!("invalid integer literal: {}", e))
            })?;
            return Ok(Some(MutationOp::SetI32 { field_name, value }));
        }
        if let syn::Lit::Str(s) = &lit.lit {
            return Ok(Some(MutationOp::SetStr {
                field_name,
                value: s.value(),
            }));
        }
    }

    // Check for negative literal: state.field = -n
    if let Expr::Unary(ExprUnary {
        op: UnOp::Neg(_),
        expr: inner,
        ..
    }) = right
    {
        if let Expr::Lit(lit) = &**inner {
            if let syn::Lit::Int(i) = &lit.lit {
                let value: i32 = i.base10_parse().map_err(|e| {
                    MutationError::new(i.span(), format!("invalid integer literal: {}", e))
                })?;
                return Ok(Some(MutationOp::SetI32 {
                    field_name,
                    value: -value,
                }));
            }
        }
    }

    Err(MutationError::new(
        assign.span(),
        format!(
            "unsupported assignment value; supported patterns: \
             `state.{} = !state.{}` (toggle), \
             `state.{} = true/false` (set bool), \
             `state.{} = 42` (set i32), \
             `state.{} = \"str\"` (set string)",
            field_name, field_name, field_name, field_name, field_name
        ),
    ))
}

/// Parse a compound assignment: state.field += n or state.field -= n
fn parse_compound_assignment(
    binary: &ExprBinary,
    state_param: &str,
) -> Result<Option<MutationOp>, MutationError> {
    // Check if this is an assignment operator
    let is_add = matches!(binary.op, BinOp::AddAssign(_));
    let is_sub = matches!(binary.op, BinOp::SubAssign(_));

    if !is_add && !is_sub {
        return Err(MutationError::new(
            binary.span(),
            "only += and -= compound assignments are supported in local handlers",
        ));
    }

    // Left side must be state.field
    let field_name = extract_field_access(&binary.left, state_param)?;

    // Right side must be a literal integer
    let value = extract_i32_literal(&binary.right)?;
    let value = if is_sub { -value } else { value };

    // Use AddI8 if value fits, otherwise AddI32
    if value >= i8::MIN as i32 && value <= i8::MAX as i32 {
        Ok(Some(MutationOp::AddI8 {
            field_name,
            value: value as i8,
        }))
    } else {
        Ok(Some(MutationOp::AddI32 { field_name, value }))
    }
}

/// Extract the field name from a state.field expression.
fn extract_field_access(expr: &Expr, state_param: &str) -> Result<String, MutationError> {
    try_extract_field_access(expr, state_param).ok_or_else(|| {
        MutationError::new(
            expr.span(),
            format!(
                "expected `{}.field` access; got something else",
                state_param
            ),
        )
    })
}

/// Try to extract the field name from a state.field expression.
fn try_extract_field_access(expr: &Expr, state_param: &str) -> Option<String> {
    if let Expr::Field(field_expr) = expr {
        // Check that base is the state parameter
        if let Expr::Path(path) = &*field_expr.base {
            if path.path.is_ident(state_param) {
                // Get the field name
                if let syn::Member::Named(ident) = &field_expr.member {
                    return Some(ident.to_string());
                }
            }
        }
    }
    None
}

/// Extract an i32 literal from an expression.
fn extract_i32_literal(expr: &Expr) -> Result<i32, MutationError> {
    // Handle positive literal
    if let Expr::Lit(lit) = expr {
        if let syn::Lit::Int(i) = &lit.lit {
            return i.base10_parse().map_err(|e| {
                MutationError::new(i.span(), format!("invalid integer literal: {}", e))
            });
        }
    }

    // Handle negative literal
    if let Expr::Unary(ExprUnary {
        op: UnOp::Neg(_),
        expr: inner,
        ..
    }) = expr
    {
        if let Expr::Lit(lit) = &**inner {
            if let syn::Lit::Int(i) = &lit.lit {
                let value: i32 = i.base10_parse().map_err(|e| {
                    MutationError::new(i.span(), format!("invalid integer literal: {}", e))
                })?;
                return Ok(-value);
            }
        }
    }

    Err(MutationError::new(
        expr.span(),
        "expected integer literal; only constant values are supported in local handlers",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    fn parse_test_stmts(code: proc_macro2::TokenStream) -> Vec<Stmt> {
        let block: syn::Block = syn::parse2(quote! { { #code } }).unwrap();
        block.stmts
    }

    #[test]
    fn test_parse_toggle() {
        let stmts = parse_test_stmts(quote! {
            state.open = !state.open;
        });
        let mutations = parse_mutations(&stmts, "state").unwrap();
        assert_eq!(mutations.len(), 1);
        assert!(matches!(&mutations[0], MutationOp::Toggle { field_name } if field_name == "open"));
    }

    #[test]
    fn test_parse_add() {
        let stmts = parse_test_stmts(quote! {
            state.count += 1;
        });
        let mutations = parse_mutations(&stmts, "state").unwrap();
        assert_eq!(mutations.len(), 1);
        assert!(matches!(&mutations[0], MutationOp::AddI8 { field_name, value } if field_name == "count" && *value == 1));
    }

    #[test]
    fn test_parse_subtract() {
        let stmts = parse_test_stmts(quote! {
            state.count -= 5;
        });
        let mutations = parse_mutations(&stmts, "state").unwrap();
        assert_eq!(mutations.len(), 1);
        assert!(matches!(&mutations[0], MutationOp::AddI8 { field_name, value } if field_name == "count" && *value == -5));
    }

    #[test]
    fn test_parse_set_bool() {
        let stmts = parse_test_stmts(quote! {
            state.flag = true;
        });
        let mutations = parse_mutations(&stmts, "state").unwrap();
        assert_eq!(mutations.len(), 1);
        assert!(matches!(&mutations[0], MutationOp::SetBool { field_name, value } if field_name == "flag" && *value));
    }

    #[test]
    fn test_parse_set_i32() {
        let stmts = parse_test_stmts(quote! {
            state.value = 42;
        });
        let mutations = parse_mutations(&stmts, "state").unwrap();
        assert_eq!(mutations.len(), 1);
        assert!(matches!(&mutations[0], MutationOp::SetI32 { field_name, value } if field_name == "value" && *value == 42));
    }

    #[test]
    fn test_parse_set_negative_i32() {
        let stmts = parse_test_stmts(quote! {
            state.value = -10;
        });
        let mutations = parse_mutations(&stmts, "state").unwrap();
        assert_eq!(mutations.len(), 1);
        assert!(matches!(&mutations[0], MutationOp::SetI32 { field_name, value } if field_name == "value" && *value == -10));
    }

    #[test]
    fn test_parse_set_str() {
        let stmts = parse_test_stmts(quote! {
            state.name = "hello";
        });
        let mutations = parse_mutations(&stmts, "state").unwrap();
        assert_eq!(mutations.len(), 1);
        assert!(matches!(&mutations[0], MutationOp::SetStr { field_name, value } if field_name == "name" && value == "hello"));
    }

    #[test]
    fn test_parse_multiple() {
        let stmts = parse_test_stmts(quote! {
            state.open = !state.open;
            state.count += 1;
        });
        let mutations = parse_mutations(&stmts, "state").unwrap();
        assert_eq!(mutations.len(), 2);
    }

    #[test]
    fn test_reject_let_binding() {
        let stmts = parse_test_stmts(quote! {
            let x = 5;
        });
        let result = parse_mutations(&stmts, "state");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_method_call() {
        let stmts = parse_test_stmts(quote! {
            state.items.push(1);
        });
        let result = parse_mutations(&stmts, "state");
        assert!(result.is_err());
    }
}
