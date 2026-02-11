//! Procedural macros for rwire client state management.
//!
//! This crate provides:
//! - `#[derive(State)]` - unified state trait with storage attribute
//! - `#[derive(ClientState)]` - marker trait for state types (deprecated)
//! - `#[handler]` - registers a handler function with its state type
//! - `#[renderer]` - transforms a render function into a synced element factory

mod schema_gen;
mod token_scanner;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, FnArg, ItemFn, ItemImpl, Type};

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
/// - `#[storage(memory)]` - Server memory state (default if omitted)
/// - `#[storage(persisted, table = "...")]` - Database-backed state
///
/// # Examples
///
/// ```ignore
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

    // Generate MemoryState marker for memory storage
    let marker_trait_impl = if storage_type.to_string().contains("Memory") {
        quote! {
            impl #impl_generics rwire::MemoryState for #name #ty_generics #where_clause {}
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
        if meta.path.is_ident("memory") {
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
/// storage type. Returns a `HandlerSpec` with the remote handler function.
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
#[proc_macro_attribute]
pub fn handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let inner_name = syn::Ident::new(&format!("__{}_inner", fn_name), fn_name.span());
    let vis = &input.vis;
    let block = &input.block;

    // Check if this is a 2-parameter handler (state, ctx)
    let has_context_param = input.sig.inputs.len() == 2;

    // Extract the state type from the first parameter
    let (state_type, param_pat, _param_name) = match input.sig.inputs.first() {
        Some(FnArg::Typed(pat_type)) => {
            // Extract the type from &mut T
            let ty = match pat_type.ty.as_ref() {
                Type::Reference(type_ref) => type_ref.elem.as_ref().clone(),
                other => other.clone(),
            };
            let pat = pat_type.pat.as_ref().clone();
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

    // Always re-render: every handler triggers all synced elements.
    // This avoids fragile static analysis of field access patterns.
    let changes_expr = quote! { rwire::ChangeSet::all() };

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

/// Attribute macro for theme provider functions.
///
/// Transforms a function returning `Theme` into a `ThemeProvider` that can be
/// passed to `Server::bind(...).root(app).theme(my_theme)`.
///
/// The function must take no arguments and return `Theme`.
///
/// # Example
///
/// ```ignore
/// #[theme]
/// fn app_theme() -> Theme {
///     Theme::dark().accent("#5E81AC")
/// }
///
/// Server::bind("0.0.0.0:9000")?
///     .root(app)
///     .theme(app_theme)
///     .run().await
/// ```
#[proc_macro_attribute]
pub fn theme(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let vis = &input.vis;
    let block = &input.block;

    // Validate: no parameters
    if !input.sig.inputs.is_empty() {
        return syn::Error::new_spanned(
            &input.sig.inputs,
            "#[theme] function must take no arguments",
        )
        .to_compile_error()
        .into();
    }

    let expanded = quote! {
        #vis fn #fn_name() -> rwire::theme::ThemeProvider {
            fn __theme_init() -> rwire::theme::Theme #block
            rwire::theme::ThemeProvider::new(__theme_init)
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
            return syn::Error::new_spanned(
                &input.sig,
                "renderer must take &State as first argument",
            )
            .to_compile_error()
            .into();
        }
    };

    // Get the parameter pattern
    let param_pat = match input.sig.inputs.first() {
        Some(FnArg::Typed(pat_type)) => pat_type.pat.as_ref().clone(),
        _ => unreachable!(),
    };

    // Always re-render: no dependency tracking needed.
    // Every handler triggers all synced elements to re-render.
    let deps_expr = quote! { rwire::RendererDeps::always() };

    // Scan function body for St::*, .hover(), .sm() etc. tokens
    // to build a compile-time token inventory for tree-shaking.
    let body_tokens: proc_macro2::TokenStream = block.stmts.iter().map(|s| quote! { #s }).collect();
    let scan = token_scanner::scan_tokens(&body_tokens);
    let inventory_expr = token_scanner::generate_inventory(&scan);

    let expanded = quote! {
        #vis fn #fn_name() -> rwire::ElementBuilder {
            const DEPS: rwire::RendererDeps = #deps_expr;
            const TOKENS: rwire::TokenInventory = #inventory_expr;
            fn #inner_name(#param_pat: &#state_type) #return_type #block
            rwire::ElementBuilder::synced_with_tokens::<#state_type>(#inner_name, DEPS, &TOKENS)
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for the `Target` marker trait (bool toggle).
///
/// Must be applied to a unit struct:
///
/// ```ignore
/// #[derive(Target)]
/// pub struct ModalOpen;
/// ```
///
/// Generates: `impl rwire::Target for ModalOpen {}`
#[proc_macro_derive(Target)]
pub fn derive_target(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Verify it's a unit struct
    match &input.data {
        syn::Data::Struct(data) => {
            if !matches!(data.fields, syn::Fields::Unit) {
                return syn::Error::new_spanned(
                    &input,
                    "Target can only be derived for unit structs (e.g., `struct ModalOpen;`)",
                )
                .to_compile_error()
                .into();
            }
        }
        _ => {
            return syn::Error::new_spanned(
                &input,
                "Target can only be derived for unit structs",
            )
            .to_compile_error()
            .into();
        }
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics rwire::Target for #name #ty_generics #where_clause {}
    };

    TokenStream::from(expanded)
}

/// Derive macro for the `Selector` trait (exclusive enum choice).
///
/// Must be applied to an enum with only unit variants.
/// Use `#[default]` to mark the default variant.
///
/// ```ignore
/// #[derive(Selector)]
/// pub enum ActiveTab {
///     #[default]
///     Home,
///     Settings,
///     Profile,
/// }
/// ```
///
/// Generates variant u8 values (0, 1, 2, ...) and `impl rwire::Selector`.
#[proc_macro_derive(Selector, attributes(default))]
pub fn derive_selector(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let variants = match &input.data {
        syn::Data::Enum(data) => &data.variants,
        _ => {
            return syn::Error::new_spanned(
                &input,
                "Selector can only be derived for enums",
            )
            .to_compile_error()
            .into();
        }
    };

    // Verify all variants are unit variants and find default
    let mut default_idx: Option<u8> = None;
    let mut match_arms = Vec::new();

    for (i, variant) in variants.iter().enumerate() {
        if !matches!(variant.fields, syn::Fields::Unit) {
            return syn::Error::new_spanned(
                variant,
                "Selector variants must be unit variants (no fields)",
            )
            .to_compile_error()
            .into();
        }

        let idx = i as u8;
        let vname = &variant.ident;

        // Check for #[default] attribute
        if variant.attrs.iter().any(|a| a.path().is_ident("default")) {
            default_idx = Some(idx);
        }

        match_arms.push(quote! {
            #name::#vname => #idx
        });
    }

    let default_val = default_idx.unwrap_or(0);
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics rwire::Selector for #name #ty_generics #where_clause {
            fn default_value() -> u8 {
                #default_val
            }

            fn variant_value(&self) -> u8 {
                match self {
                    #(#match_arms),*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// Attribute macro for component impl blocks.
///
/// Scans all method bodies in the impl block for style tokens (`St::*`),
/// pseudo-class methods (`.hover()`, `.focus()`, etc.), and breakpoint
/// methods (`.sm()`, `.md()`, etc.). Generates a compile-time
/// `TokenInventory` that captures all tokens across every code branch.
///
/// The `build()` method's return value is automatically wrapped with
/// `.with_token_inventory()` to attach the inventory to the output
/// `ElementBuilder`, enabling complete tree-shaking without relying
/// on default-state rendering alone.
///
/// # Example
///
/// ```ignore
/// use rwire_macros::component;
///
/// #[component]
/// impl Button {
///     pub fn compute_tokens(&self) -> Vec<St> {
///         match self.intent {
///             ButtonIntent::Primary => vec![St::BgPrimary],
///             ButtonIntent::Destructive => vec![St::BgDestructive],
///         }
///     }
///
///     pub fn build(self) -> ElementBuilder {
///         el(El::Button).st(self.compute_tokens())
///             .hover([St::BgPrimaryHover])
///         // ↑ All St tokens, .hover(), .sm() etc. are discovered at compile time
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemImpl);

    // Scan ALL method bodies in the impl block for tokens
    let all_methods_tokens: proc_macro2::TokenStream = input
        .items
        .iter()
        .filter_map(|item| {
            if let syn::ImplItem::Fn(method) = item {
                Some(quote! { #method })
            } else {
                None
            }
        })
        .collect();

    let scan = token_scanner::scan_tokens(&all_methods_tokens);
    let inventory_expr = token_scanner::generate_inventory(&scan);

    // Find build() method and wrap its return value
    for item in &mut input.items {
        if let syn::ImplItem::Fn(method) = item {
            if method.sig.ident == "build" {
                let original_block = &method.block;
                method.block = syn::parse_quote! {{
                    let __component_result: rwire::ElementBuilder = #original_block;
                    __component_result.with_token_inventory(&Self::__COMPONENT_TOKENS)
                }};
                break;
            }
        }
    }

    // Add the const inventory to the impl block
    let const_item: syn::ImplItem = syn::parse_quote! {
        /// Compile-time token inventory for tree-shaking (generated by `#[component]`).
        #[doc(hidden)]
        pub const __COMPONENT_TOKENS: rwire::TokenInventory = #inventory_expr;
    };
    input.items.push(const_item);

    TokenStream::from(quote! { #input })
}
