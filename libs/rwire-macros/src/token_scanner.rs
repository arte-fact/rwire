//! Scans a renderer function body for style tokens at compile time.
//!
//! Extracts `St::Variant` references, `.hover([...])` pseudo-class calls,
//! and `.sm([...])` breakpoint calls from the token stream. This enables
//! tree-shaking to discover all tokens across every branch, not just the
//! default-state path.
//!
//! # How it works
//!
//! The scanner walks the proc_macro2 token stream and:
//! 1. Finds all `St :: Variant` patterns → collects as style tokens
//! 2. Finds `.hover([...])` / `.focus([...])` / `.active([...])` etc.
//!    → extracts `St::Variant` inside → collects as `(Pc, St)` pairs
//! 3. Finds `.sm([...])` / `.md([...])` / `.lg([...])` / `.xl([...])`
//!    → extracts `St::Variant` inside → collects as `(Bp, St)` pairs

use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::quote;
use std::collections::BTreeSet;

/// Pseudo-class method names and their Pc enum variants.
const PSEUDO_METHODS: &[(&str, &str)] = &[
    ("hover", "Hover"),
    ("focus", "Focus"),
    ("focus_visible", "FocusVisible"),
    ("active", "Active"),
    ("checked", "Checked"),
    ("disabled", "Disabled"),
    ("focus_within", "FocusWithin"),
    ("placeholder", "Placeholder"),
];

/// Breakpoint method names and their Bp enum variants.
const BREAKPOINT_METHODS: &[(&str, &str)] = &[
    ("sm", "Sm"),
    ("md", "Md"),
    ("lg", "Lg"),
    ("xl", "Xl"),
];

/// Result of scanning a renderer function body.
pub struct ScanResult {
    /// All `St::Variant` names found (deduplicated, sorted).
    pub styles: BTreeSet<String>,
    /// Pseudo-class pairs: (Pc variant name, St variant name).
    pub pseudo_pairs: BTreeSet<(String, String)>,
    /// Breakpoint pairs: (Bp variant name, St variant name).
    pub breakpoint_pairs: BTreeSet<(String, String)>,
}

/// Scan a renderer function body for style tokens.
pub fn scan_tokens(body: &TokenStream) -> ScanResult {
    let mut result = ScanResult {
        styles: BTreeSet::new(),
        pseudo_pairs: BTreeSet::new(),
        breakpoint_pairs: BTreeSet::new(),
    };

    let tokens: Vec<TokenTree> = body.clone().into_iter().collect();
    scan_recursive(&tokens, &mut result);

    result
}

/// Recursively scan tokens, tracking method call context.
fn scan_recursive(tokens: &[TokenTree], result: &mut ScanResult) {
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            // Check for St::Variant pattern
            TokenTree::Ident(ident) if ident == "St" => {
                if let Some(variant) = extract_path_variant(tokens, i) {
                    result.styles.insert(variant);
                }
                i += 1;
            }

            // Check for .method_name([...]) pattern (pseudo or breakpoint)
            TokenTree::Punct(p) if p.as_char() == '.' => {
                if let Some((method_name, args_group)) = extract_method_call(tokens, i) {
                    // Check if this is a pseudo-class method
                    if let Some((_, pc_variant)) =
                        PSEUDO_METHODS.iter().find(|(name, _)| *name == method_name)
                    {
                        let st_variants = extract_st_variants_from_group(args_group);
                        for st in st_variants {
                            result
                                .pseudo_pairs
                                .insert((pc_variant.to_string(), st));
                        }
                    }

                    // Check if this is a breakpoint method
                    if let Some((_, bp_variant)) =
                        BREAKPOINT_METHODS.iter().find(|(name, _)| *name == method_name)
                    {
                        let st_variants = extract_st_variants_from_group(args_group);
                        for st in st_variants {
                            result
                                .breakpoint_pairs
                                .insert((bp_variant.to_string(), st));
                        }
                    }
                }
                i += 1;
            }

            // Recurse into groups (parentheses, brackets, braces)
            TokenTree::Group(group) => {
                let inner: Vec<TokenTree> = group.stream().into_iter().collect();
                scan_recursive(&inner, result);
                i += 1;
            }

            _ => {
                i += 1;
            }
        }
    }
}

/// Extract variant name from `Ident :: Ident` pattern starting at position `i`.
/// Returns the variant name if pattern matches `St :: VariantName`.
fn extract_path_variant(tokens: &[TokenTree], i: usize) -> Option<String> {
    // Need at least 3 more tokens: :: and Ident
    if i + 3 > tokens.len() {
        return None;
    }

    // Check for :: (two consecutive Punct(':'))
    let is_colon1 = matches!(&tokens[i + 1], TokenTree::Punct(p) if p.as_char() == ':');
    let is_colon2 = matches!(&tokens[i + 2], TokenTree::Punct(p) if p.as_char() == ':');

    if !is_colon1 || !is_colon2 {
        return None;
    }

    // Check for variant ident
    if i + 3 < tokens.len() {
        if let TokenTree::Ident(variant) = &tokens[i + 3] {
            return Some(variant.to_string());
        }
    }

    None
}

/// Extract method name and args group from `.method_name(...)` pattern.
/// Returns (method_name, args_token_stream) if pattern matches.
fn extract_method_call(
    tokens: &[TokenTree],
    dot_pos: usize,
) -> Option<(String, &proc_macro2::Group)> {
    // Need at least: . ident group
    if dot_pos + 2 >= tokens.len() {
        return None;
    }

    let method_name = match &tokens[dot_pos + 1] {
        TokenTree::Ident(ident) => ident.to_string(),
        _ => return None,
    };

    let group = match &tokens[dot_pos + 2] {
        TokenTree::Group(g)
            if g.delimiter() == proc_macro2::Delimiter::Parenthesis =>
        {
            g
        }
        _ => return None,
    };

    Some((method_name, group))
}

/// Extract all `St::Variant` names from a group (the args of a method call).
fn extract_st_variants_from_group(group: &proc_macro2::Group) -> Vec<String> {
    let mut variants = Vec::new();
    let tokens: Vec<TokenTree> = group.stream().into_iter().collect();
    collect_st_variants_recursive(&tokens, &mut variants);
    variants
}

/// Recursively collect St::Variant names from a token slice.
fn collect_st_variants_recursive(tokens: &[TokenTree], variants: &mut Vec<String>) {
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            TokenTree::Ident(ident) if ident == "St" => {
                if let Some(variant) = extract_path_variant(tokens, i) {
                    variants.push(variant);
                    i += 4; // Skip St :: Variant
                    continue;
                }
                i += 1;
            }
            TokenTree::Group(group) => {
                let inner: Vec<TokenTree> = group.stream().into_iter().collect();
                collect_st_variants_recursive(&inner, variants);
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
}

/// Generate the const `TokenInventory` expression from scan results.
///
/// Produces a `rwire::TokenInventory { styles, pseudo_pairs, breakpoint_pairs }`
/// expression using fully-qualified paths for `St`, `Pc`, and `Bp` variants.
pub fn generate_inventory(result: &ScanResult) -> TokenStream {
    let style_entries: Vec<TokenStream> = result
        .styles
        .iter()
        .map(|variant| {
            let ident = syn::Ident::new(variant, proc_macro2::Span::call_site());
            quote! { rwire::style_tokens::St::#ident as u16 }
        })
        .collect();

    let pseudo_entries: Vec<TokenStream> = result
        .pseudo_pairs
        .iter()
        .map(|(pc, st)| {
            let pc_ident = syn::Ident::new(pc, proc_macro2::Span::call_site());
            let st_ident = syn::Ident::new(st, proc_macro2::Span::call_site());
            quote! { (rwire::style_tokens::Pc::#pc_ident as u8, rwire::style_tokens::St::#st_ident as u16) }
        })
        .collect();

    let bp_entries: Vec<TokenStream> = result
        .breakpoint_pairs
        .iter()
        .map(|(bp, st)| {
            let bp_ident = syn::Ident::new(bp, proc_macro2::Span::call_site());
            let st_ident = syn::Ident::new(st, proc_macro2::Span::call_site());
            quote! { (rwire::style_tokens::Bp::#bp_ident as u8, rwire::style_tokens::St::#st_ident as u16) }
        })
        .collect();

    quote! {
        rwire::TokenInventory {
            styles: &[#(#style_entries),*],
            pseudo_pairs: &[#(#pseudo_entries),*],
            breakpoint_pairs: &[#(#bp_entries),*],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(code: &str) -> ScanResult {
        let tokens: TokenStream = code.parse().unwrap();
        scan_tokens(&tokens)
    }

    #[test]
    fn finds_direct_st_tokens() {
        let result = scan("el(El::Div).st([St::BgApp, St::Px4])");
        assert!(result.styles.contains("BgApp"));
        assert!(result.styles.contains("Px4"));
    }

    #[test]
    fn finds_st_tokens_in_branches() {
        let result = scan(
            r#"
            if state.danger {
                el(El::Button).st([St::BgDestructive])
            } else {
                el(El::Button).st([St::BgPrimary])
            }
            "#,
        );
        assert!(result.styles.contains("BgDestructive"));
        assert!(result.styles.contains("BgPrimary"));
    }

    #[test]
    fn finds_hover_pseudo_pairs() {
        let result = scan("el(El::Div).hover([St::BgHover, St::TextDefault])");
        assert!(result.pseudo_pairs.contains(&("Hover".to_string(), "BgHover".to_string())));
        assert!(result.pseudo_pairs.contains(&("Hover".to_string(), "TextDefault".to_string())));
        // Hover tokens also appear in styles (global scan)
        assert!(result.styles.contains("BgHover"));
    }

    #[test]
    fn finds_focus_pseudo_pairs() {
        let result = scan("el(El::Input).focus([St::RingPrimary])");
        assert!(result.pseudo_pairs.contains(&("Focus".to_string(), "RingPrimary".to_string())));
    }

    #[test]
    fn finds_active_pseudo_pairs() {
        let result = scan("el(El::Button).active([St::BgPrimaryActive])");
        assert!(result.pseudo_pairs.contains(&("Active".to_string(), "BgPrimaryActive".to_string())));
    }

    #[test]
    fn finds_breakpoint_pairs() {
        let result = scan("el(El::Div).sm([St::DisplayFlex]).md([St::DisplayBlock])");
        assert!(result.breakpoint_pairs.contains(&("Sm".to_string(), "DisplayFlex".to_string())));
        assert!(result.breakpoint_pairs.contains(&("Md".to_string(), "DisplayBlock".to_string())));
    }

    #[test]
    fn finds_xl_breakpoint() {
        let result = scan("el(El::Div).xl([St::Px8])");
        assert!(result.breakpoint_pairs.contains(&("Xl".to_string(), "Px8".to_string())));
    }

    #[test]
    fn finds_tokens_in_match_arms() {
        let result = scan(
            r#"
            match state.intent {
                Intent::Primary => el(El::Button).st([St::BgPrimary]).hover([St::BgPrimaryHover]),
                Intent::Destructive => el(El::Button).st([St::BgDestructive]).hover([St::BgDestructiveHover]),
            }
            "#,
        );
        assert!(result.styles.contains("BgPrimary"));
        assert!(result.styles.contains("BgDestructive"));
        assert!(result.pseudo_pairs.contains(&("Hover".to_string(), "BgPrimaryHover".to_string())));
        assert!(result.pseudo_pairs.contains(&("Hover".to_string(), "BgDestructiveHover".to_string())));
    }

    #[test]
    fn deduplicates_tokens() {
        let result = scan(
            r#"
            el(El::Div).st([St::BgApp]);
            el(El::Span).st([St::BgApp]);
            "#,
        );
        // BTreeSet automatically deduplicates
        assert_eq!(result.styles.len(), 1);
        assert!(result.styles.contains("BgApp"));
    }

    #[test]
    fn empty_body_returns_empty_inventory() {
        let result = scan("");
        assert!(result.styles.is_empty());
        assert!(result.pseudo_pairs.is_empty());
        assert!(result.breakpoint_pairs.is_empty());
    }

    #[test]
    fn ignores_non_st_paths() {
        let result = scan("El::Div; Ev::Click; ButtonIntent::Primary");
        assert!(result.styles.is_empty());
    }
}
