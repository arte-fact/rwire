//! Procedural macros for wire-wasm client state management.
//!
//! This crate provides:
//! - `#[derive(ClientState)]` - marker trait for state types
//! - `#[handler]` - registers a handler function with its state type
//! - `#[renderer]` - transforms a render function into a synced element factory

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, FnArg, ItemFn, Type};

/// Derive macro for the `ClientState` marker trait.
///
/// # Example
///
/// ```ignore
/// #[derive(ClientState, Default)]
/// struct Counter {
///     count: i32,
/// }
/// ```
#[proc_macro_derive(ClientState)]
pub fn derive_client_state(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics wire_wasm::ClientState for #name #ty_generics #where_clause {}
    };

    TokenStream::from(expanded)
}

/// Attribute macro for handler functions.
///
/// Transforms a function taking `&mut State` into a handler that can be used
/// with event bindings. The macro registers the handler with its state type.
///
/// # Example
///
/// ```ignore
/// #[handler]
/// fn increment(state: &mut Counter) {
///     state.count += 1;
/// }
/// ```
///
/// Expands to:
///
/// ```ignore
/// fn increment() -> wire_wasm::HandlerFn {
///     fn __increment_inner(state: &mut Counter) {
///         state.count += 1;
///     }
///     wire_wasm::HandlerFn::new::<Counter>(__increment_inner)
/// }
/// ```
#[proc_macro_attribute]
pub fn handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let inner_name = syn::Ident::new(&format!("__{}_inner", fn_name), fn_name.span());
    let vis = &input.vis;
    let block = &input.block;

    // Extract the state type from the first parameter
    let state_type = match input.sig.inputs.first() {
        Some(FnArg::Typed(pat_type)) => {
            // Extract the type from &mut T
            match pat_type.ty.as_ref() {
                Type::Reference(type_ref) => type_ref.elem.as_ref().clone(),
                other => other.clone(),
            }
        }
        _ => {
            return syn::Error::new_spanned(&input.sig, "handler must take &mut State as first argument")
                .to_compile_error()
                .into();
        }
    };

    // Get the parameter pattern (the variable name)
    let param_pat = match input.sig.inputs.first() {
        Some(FnArg::Typed(pat_type)) => pat_type.pat.as_ref().clone(),
        _ => unreachable!(),
    };

    let expanded = quote! {
        #vis fn #fn_name() -> wire_wasm::HandlerFn {
            fn #inner_name(#param_pat: &mut #state_type) #block
            wire_wasm::HandlerFn::new::<#state_type>(#inner_name)
        }
    };

    TokenStream::from(expanded)
}

/// Attribute macro for renderer functions.
///
/// Transforms a function taking `&State` and returning `ElementBuilder` into
/// a synced element factory. The returned element will automatically re-render
/// when the state changes.
///
/// # Example
///
/// ```ignore
/// #[renderer]
/// fn render_count(state: &Counter) -> ElementBuilder {
///     el(El::Span).text(&state.count.to_string())
/// }
/// ```
///
/// Expands to:
///
/// ```ignore
/// fn render_count() -> ElementBuilder {
///     fn __render_count_inner(state: &Counter) -> ElementBuilder {
///         el(El::Span).text(&state.count.to_string())
///     }
///     ElementBuilder::synced::<Counter>(__render_count_inner)
/// }
/// ```
#[proc_macro_attribute]
pub fn renderer(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let inner_name = syn::Ident::new(&format!("__{}_inner", fn_name), fn_name.span());
    let vis = &input.vis;
    let block = &input.block;
    let return_type = &input.sig.output;

    // Extract the state type from the first parameter
    let state_type = match input.sig.inputs.first() {
        Some(FnArg::Typed(pat_type)) => {
            // Extract the type from &T
            match pat_type.ty.as_ref() {
                Type::Reference(type_ref) => type_ref.elem.as_ref().clone(),
                other => other.clone(),
            }
        }
        _ => {
            return syn::Error::new_spanned(&input.sig, "renderer must take &State as first argument")
                .to_compile_error()
                .into();
        }
    };

    // Get the parameter pattern (the variable name)
    let param_pat = match input.sig.inputs.first() {
        Some(FnArg::Typed(pat_type)) => pat_type.pat.as_ref().clone(),
        _ => unreachable!(),
    };

    let expanded = quote! {
        #vis fn #fn_name() -> wire_wasm::ElementBuilder {
            fn #inner_name(#param_pat: &#state_type) #return_type #block
            wire_wasm::ElementBuilder::synced::<#state_type>(#inner_name)
        }
    };

    TokenStream::from(expanded)
}
