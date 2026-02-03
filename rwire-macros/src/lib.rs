//! Procedural macros for rwire client state management.
//!
//! This crate provides:
//! - `#[derive(State)]` - unified state trait with storage attribute
//! - `#[derive(ClientState)]` - marker trait for state types (deprecated)
//! - `#[handler]` - registers a handler function with its state type
//! - `#[renderer]` - transforms a render function into a synced element factory

mod field_access_parser;
mod mutation_parser;
mod schema_gen;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, FnArg, ItemFn, Type};

/// Derive macro for the `ClientState` marker trait (deprecated).
///
/// Use `#[derive(State)]` with `#[storage(memory)]` instead.
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

    // Generate both MemoryState (for new API) and backwards-compatible code
    let expanded = quote! {
        impl #impl_generics rwire::MemoryState for #name #ty_generics #where_clause {}

        impl #impl_generics rwire::State for #name #ty_generics #where_clause {
            const STORAGE_TYPE: rwire::StorageType = rwire::StorageType::Memory;
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for the unified `State` trait.
///
/// Use the `#[storage(...)]` attribute to specify storage type:
/// - `#[storage(local)]` - Client-side state, no server round-trip
/// - `#[storage(memory)]` - Server memory state (default if omitted)
/// - `#[storage(persisted, table = "...")]` - Database-backed state
///
/// # Examples
///
/// ```ignore
/// // Local state (client-side, no round-trip)
/// #[derive(State, Default)]
/// #[storage(local)]
/// struct UiState {
///     sidebar_open: bool,
/// }
///
/// // Memory state (server-side, default)
/// #[derive(State, Default)]
/// struct Counter {
///     count: i32,
/// }
///
/// // Explicit memory storage
/// #[derive(State, Default)]
/// #[storage(memory)]
/// struct Session {
///     user_id: Option<u64>,
/// }
/// ```
#[proc_macro_derive(State, attributes(storage, key))]
pub fn derive_state(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Parse #[storage(...)] attribute
    let storage_attr = input.attrs.iter().find(|a| a.path().is_ident("storage"));
    let (storage_type, table_name, key_field) = if let Some(attr) = storage_attr {
        parse_storage_attr(attr)
    } else {
        // Default to Memory storage
        (
            quote!(rwire::StorageType::Memory),
            String::new(),
            String::new(),
        )
    };

    // Extract field information for local state
    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => fields.named.iter().collect::<Vec<_>>(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };

    // Generate field index constants for local state
    let field_constants: Vec<_> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let field_name = f.ident.as_ref().unwrap();
            let const_name = syn::Ident::new(
                &format!("FIELD_{}", field_name.to_string().to_uppercase()),
                field_name.span(),
            );
            let idx = i as u8;
            quote! {
                pub const #const_name: u8 = #idx;
            }
        })
        .collect();

    // Generate the State trait implementation
    let table_name_str = if table_name.is_empty() {
        quote!("")
    } else {
        quote!(#table_name)
    };

    let key_field_str = if key_field.is_empty() {
        quote!("")
    } else {
        quote!(#key_field)
    };

    // Also generate MemoryState for backwards compatibility with memory storage
    let marker_trait_impl = if storage_type.to_string().contains("Memory") {
        quote! {
            impl #impl_generics rwire::MemoryState for #name #ty_generics #where_clause {}
        }
    } else if storage_type.to_string().contains("Local") {
        // Generate JSON serialization for local state default values
        let field_json_parts: Vec<_> = fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let field_name = f.ident.as_ref().unwrap().to_string();
                let field_type = &f.ty;
                let comma = if i > 0 { "," } else { "" };
                // Generate JSON for default value based on type
                quote! {
                    format!(
                        "{}\"{}\":{}",
                        #comma,
                        #field_name,
                        <#field_type as rwire::LocalStateJson>::default_json()
                    )
                }
            })
            .collect();

        quote! {
            impl #impl_generics rwire::LocalState for #name #ty_generics #where_clause {}

            impl #impl_generics #name #ty_generics #where_clause {
                /// Return the default state as JSON.
                ///
                /// This is used to initialize client-side local state.
                pub fn __local_state_default_json() -> String {
                    // Register this type with the local state registry on first call
                    static ONCE: std::sync::Once = std::sync::Once::new();
                    ONCE.call_once(|| {
                        rwire::register_local_state_default::<#name>(Self::__local_state_default_json);
                    });

                    let mut json = String::from("{");
                    #(json.push_str(&#field_json_parts);)*
                    json.push('}');
                    json
                }
            }
        }
    } else {
        quote! {}
    };

    // Generate SCHEMA constant for persisted types
    let schema_impl = if storage_type.to_string().contains("Persisted") && !table_name.is_empty() {
        let data_fields = match &input.data {
            syn::Data::Struct(data) => &data.fields,
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    "Only structs can derive State with persisted storage",
                )
                .to_compile_error()
                .into();
            }
        };

        let tables = schema_gen::generate_schema(&table_name, &key_field, data_fields);
        let schema_sql: Vec<String> = tables.iter().map(schema_gen::table_to_sql).collect();

        quote! {
            impl #impl_generics #name #ty_generics #where_clause {
                /// SQL statements to create tables for this state.
                pub const SCHEMA: &'static [&'static str] = &[
                    #(#schema_sql),*
                ];
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            #(#field_constants)*
        }

        impl #impl_generics rwire::State for #name #ty_generics #where_clause {
            const STORAGE_TYPE: rwire::StorageType = #storage_type;
            const TABLE_NAME: &'static str = #table_name_str;
            const KEY_FIELD: &'static str = #key_field_str;
        }

        #marker_trait_impl

        #schema_impl
    };

    TokenStream::from(expanded)
}

/// Parse the #[storage(...)] attribute.
fn parse_storage_attr(attr: &syn::Attribute) -> (proc_macro2::TokenStream, String, String) {
    use syn::Token;

    let mut storage_type = quote!(rwire::StorageType::Memory);
    let mut table_name = String::new();
    let mut key_field = String::new();

    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("local") {
            storage_type = quote!(rwire::StorageType::Local);
        } else if meta.path.is_ident("memory") {
            storage_type = quote!(rwire::StorageType::Memory);
        } else if meta.path.is_ident("persisted") {
            storage_type = quote!(rwire::StorageType::Persisted);
        } else if meta.path.is_ident("table") {
            let _: Token![=] = meta.input.parse()?;
            let lit: syn::LitStr = meta.input.parse()?;
            table_name = lit.value();
        } else if meta.path.is_ident("key") {
            let _: Token![=] = meta.input.parse()?;
            let lit: syn::LitStr = meta.input.parse()?;
            key_field = lit.value();
        }
        Ok(())
    });

    (storage_type, table_name, key_field)
}

/// Attribute macro for handler functions.
///
/// Transforms a function taking `&mut State` into a handler that can be used
/// with event bindings. The macro determines behavior based on the state's
/// storage type:
///
/// - **Memory/Persisted state**: Returns `HandlerSpec` with remote handler function
/// - **Local state**: Parses the function body into mutations, returns `HandlerSpec`
///   with local bytecode (no server round-trip)
///
/// # Example (Memory State - single parameter)
///
/// ```ignore
/// #[derive(State, Default)]
/// struct Counter { count: i32 }
///
/// #[handler]
/// fn increment(state: &mut Counter) {
///     state.count += 1;
/// }
/// // Expands to HandlerSpec::from_fn with remote handler
/// ```
///
/// # Example (Memory State - with EventContext)
///
/// ```ignore
/// #[derive(State, Default)]
/// struct TodoState { items: Vec<String> }
///
/// #[handler]
/// fn add_todo(state: &mut TodoState, ctx: &EventContext) {
///     if let Some(text) = ctx.text() {
///         state.items.push(text.to_string());
///     }
/// }
/// // Expands to HandlerSpec::from_fn_with_context
/// ```
///
/// # Example (Local State)
///
/// ```ignore
/// #[derive(State, Default)]
/// #[storage(local)]
/// struct UiState { open: bool }
///
/// #[handler]
/// fn toggle(state: &mut UiState) {
///     state.open = !state.open;
/// }
/// // Expands to HandlerSpec::local with Toggle mutation
/// ```
///
/// # Supported Local State Patterns
///
/// For local state, only these patterns are supported (compile error otherwise):
/// - `state.field = !state.field` → Toggle
/// - `state.field = true/false` → SetBool
/// - `state.field += n` → AddI8/AddI32
/// - `state.field -= n` → AddI8/AddI32 (negative)
/// - `state.field = n` → SetI32
/// - `state.field = "str"` → SetStr
#[proc_macro_attribute]
pub fn handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let inner_name = syn::Ident::new(&format!("__{}_inner", fn_name), fn_name.span());
    let vis = &input.vis;
    let block = &input.block;

    // Check for #[handler(local)] attribute
    let is_local_attr = !attr.is_empty() && {
        let attr_str = attr.to_string();
        attr_str.contains("local")
    };

    // Check if this is a 2-parameter handler (state, ctx)
    let has_context_param = input.sig.inputs.len() == 2;

    // Extract the state type from the first parameter
    let (state_type, param_pat, param_name) = match input.sig.inputs.first() {
        Some(FnArg::Typed(pat_type)) => {
            // Extract the type from &mut T
            let ty = match pat_type.ty.as_ref() {
                Type::Reference(type_ref) => type_ref.elem.as_ref().clone(),
                other => other.clone(),
            };
            let pat = pat_type.pat.as_ref().clone();
            // Get the parameter name as a string
            let name = match &pat {
                syn::Pat::Ident(ident) => ident.ident.to_string(),
                _ => "state".to_string(),
            };
            (ty, pat, name)
        }
        _ => {
            return syn::Error::new_spanned(
                &input.sig,
                "handler must take &mut State as first argument",
            )
            .to_compile_error()
            .into();
        }
    };

    // Extract the context parameter pattern if present
    let ctx_param_pat = if has_context_param {
        match input.sig.inputs.iter().nth(1) {
            Some(FnArg::Typed(pat_type)) => Some(pat_type.pat.as_ref().clone()),
            _ => None,
        }
    } else {
        None
    };

    // For #[handler(local)], parse the body into mutations
    if is_local_attr {
        match mutation_parser::parse_mutations(&block.stmts, &param_name) {
            Ok(mutations) => {
                // Generate mutation literals
                let mutation_exprs: Vec<_> = mutations.iter().map(|m| {
                    match m {
                        mutation_parser::MutationOp::Toggle { field_name } => {
                            let field_const = syn::Ident::new(
                                &format!("FIELD_{}", field_name.to_uppercase()),
                                proc_macro2::Span::call_site(),
                            );
                            quote! {
                                rwire::Mutation::Toggle { field: #state_type::#field_const }
                            }
                        }
                        mutation_parser::MutationOp::AddI8 { field_name, value } => {
                            let field_const = syn::Ident::new(
                                &format!("FIELD_{}", field_name.to_uppercase()),
                                proc_macro2::Span::call_site(),
                            );
                            quote! {
                                rwire::Mutation::AddI8 { field: #state_type::#field_const, value: #value }
                            }
                        }
                        mutation_parser::MutationOp::AddI32 { field_name, value } => {
                            let field_const = syn::Ident::new(
                                &format!("FIELD_{}", field_name.to_uppercase()),
                                proc_macro2::Span::call_site(),
                            );
                            quote! {
                                rwire::Mutation::AddI32 { field: #state_type::#field_const, value: #value }
                            }
                        }
                        mutation_parser::MutationOp::SetBool { field_name, value } => {
                            let field_const = syn::Ident::new(
                                &format!("FIELD_{}", field_name.to_uppercase()),
                                proc_macro2::Span::call_site(),
                            );
                            quote! {
                                rwire::Mutation::SetBool { field: #state_type::#field_const, value: #value }
                            }
                        }
                        mutation_parser::MutationOp::SetI32 { field_name, value } => {
                            let field_const = syn::Ident::new(
                                &format!("FIELD_{}", field_name.to_uppercase()),
                                proc_macro2::Span::call_site(),
                            );
                            quote! {
                                rwire::Mutation::SetI32 { field: #state_type::#field_const, value: #value }
                            }
                        }
                        mutation_parser::MutationOp::SetStr { field_name, value } => {
                            let field_const = syn::Ident::new(
                                &format!("FIELD_{}", field_name.to_uppercase()),
                                proc_macro2::Span::call_site(),
                            );
                            quote! {
                                rwire::Mutation::SetStr { field: #state_type::#field_const, value: #value.to_string() }
                            }
                        }
                    }
                }).collect();

                let expanded = quote! {
                    #vis fn #fn_name() -> rwire::HandlerSpec {
                        // Ensure local state type is registered with the local state registry.
                        // This triggers the Once::call_once registration.
                        let _ = #state_type::__local_state_default_json();

                        rwire::HandlerSpec::local::<#state_type>(rwire::LocalMutations::new(vec![
                            #(#mutation_exprs),*
                        ]))
                    }
                };

                return TokenStream::from(expanded);
            }
            Err(err) => {
                return syn::Error::new(err.span, err.message)
                    .to_compile_error()
                    .into();
            }
        }
    }

    // Auto-detect field writes for ChangeSet
    let field_writes = field_access_parser::extract_writes(block, &param_name);
    let changes_expr = if field_writes.is_empty() {
        // No writes detected - could be complex logic (method calls, etc.)
        // Use all() as fallback
        quote! { rwire::ChangeSet::all() }
    } else {
        // Generate field constant references
        let field_consts: Vec<_> = field_writes
            .iter()
            .map(|name| {
                let const_name = syn::Ident::new(
                    &format!("FIELD_{}", name.to_uppercase()),
                    proc_macro2::Span::call_site(),
                );
                quote! { #state_type::#const_name }
            })
            .collect();
        quote! { rwire::ChangeSet::from_fields(&[#(#field_consts),*]) }
    };

    // Check if we have a 2-parameter handler with EventContext
    if has_context_param {
        if let Some(ctx_pat) = ctx_param_pat {
            // Generate HandlerSpec with context support and ChangeSet
            let expanded = quote! {
                #vis fn #fn_name() -> rwire::HandlerSpec {
                    const CHANGES: rwire::ChangeSet = #changes_expr;
                    fn #inner_name(#param_pat: &mut #state_type, #ctx_pat: &rwire::EventContext) #block
                    rwire::HandlerSpec::from_fn_with_context_and_changes::<#state_type>(#inner_name, CHANGES)
                }
            };
            return TokenStream::from(expanded);
        }
    }

    // Default: generate HandlerSpec with auto-detected ChangeSet
    let expanded = quote! {
        #vis fn #fn_name() -> rwire::HandlerSpec {
            const CHANGES: rwire::ChangeSet = #changes_expr;
            fn #inner_name(#param_pat: &mut #state_type) #block
            rwire::HandlerSpec::from_fn_with_changes::<#state_type>(#inner_name, CHANGES)
        }
    };

    TokenStream::from(expanded)
}

/// Attribute macro for renderer functions.
///
/// Transforms a function taking `&State` and returning `ElementBuilder` into
/// a synced element factory. The returned element will automatically re-render
/// when state changes, with fine-grained dependency tracking.
///
/// # Dependency Detection
///
/// By default, the macro analyzes the function body to detect which state fields
/// are accessed. Only changes to those fields trigger re-renders.
///
/// ```ignore
/// #[renderer]
/// fn render_count(state: &Counter) -> ElementBuilder {
///     // Auto-detected: depends on `count` field
///     el(El::Span).text(&state.count.to_string())
/// }
/// ```
///
/// # Always Re-render
///
/// Use `#[renderer(always)]` when field dependencies can't be statically determined
/// (e.g., accessing fields through methods, closures, or dynamic indexes):
///
/// ```ignore
/// #[renderer(always)]
/// fn render_debug(state: &AppState) -> ElementBuilder {
///     // Complex logic that can't be analyzed
///     el(El::Pre).text(&format!("{:?}", state))
/// }
/// ```
///
/// # Expansion
///
/// The macro generates code that tracks dependencies at compile time:
///
/// ```ignore
/// fn render_count() -> ElementBuilder {
///     const DEPS: RendererDeps = RendererDeps::from_fields(&[Counter::FIELD_COUNT]);
///     fn __render_count_inner(state: &Counter) -> ElementBuilder {
///         el(El::Span).text(&state.count.to_string())
///     }
///     ElementBuilder::synced_with_deps::<Counter>(__render_count_inner, DEPS)
/// }
/// ```
#[proc_macro_attribute]
pub fn renderer(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let inner_name = syn::Ident::new(&format!("__{}_inner", fn_name), fn_name.span());
    let vis = &input.vis;
    let block = &input.block;
    let return_type = &input.sig.output;

    // Check for #[renderer(always)] attribute
    let is_always = !attr.is_empty() && attr.to_string().contains("always");

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
            return syn::Error::new_spanned(
                &input.sig,
                "renderer must take &State as first argument",
            )
            .to_compile_error()
            .into();
        }
    };

    // Get the parameter pattern (the variable name)
    let (param_pat, param_name) = match input.sig.inputs.first() {
        Some(FnArg::Typed(pat_type)) => {
            let pat = pat_type.pat.as_ref().clone();
            let name = match &pat {
                syn::Pat::Ident(ident) => ident.ident.to_string(),
                _ => "state".to_string(),
            };
            (pat, name)
        }
        _ => unreachable!(),
    };

    // Generate dependency tracking
    let deps_expr = if is_always {
        // Always re-render mode
        quote! { rwire::RendererDeps::always() }
    } else {
        // Auto-detect dependencies from function body
        let field_names = field_access_parser::extract_reads(block, &param_name);

        if field_names.is_empty() {
            // No fields detected - could be complex logic, use always
            quote! { rwire::RendererDeps::always() }
        } else {
            // Generate field constant references
            let field_consts: Vec<_> = field_names
                .iter()
                .map(|name| {
                    let const_name = syn::Ident::new(
                        &format!("FIELD_{}", name.to_uppercase()),
                        proc_macro2::Span::call_site(),
                    );
                    quote! { #state_type::#const_name }
                })
                .collect();

            quote! { rwire::RendererDeps::from_fields(&[#(#field_consts),*]) }
        }
    };

    let expanded = quote! {
        #vis fn #fn_name() -> rwire::ElementBuilder {
            const DEPS: rwire::RendererDeps = #deps_expr;
            fn #inner_name(#param_pat: &#state_type) #return_type #block
            rwire::ElementBuilder::synced_with_deps::<#state_type>(#inner_name, DEPS)
        }
    };

    TokenStream::from(expanded)
}
