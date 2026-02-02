//! Fine-Grained Reactivity Demo
//!
//! This example demonstrates rwire's compile-time fine-grained reactivity system.
//! Each renderer only re-renders when the specific fields it depends on change.
//!
//! The state has 4 independent fields:
//! - `counter` - A simple counter
//! - `name` - A text string
//! - `enabled` - A boolean toggle
//! - `items` - A list of items
//!
//! Each field has its own renderer and handlers. When you click a button,
//! ONLY the renderer that depends on the changed field will re-render.
//!
//! Watch the server console to see which renderers are called!

use rwire::{el, handler, renderer, El, ElementBuilder, Ev, Server, State};

#[derive(State, Default)]
#[storage(memory)]
struct AppState {
    counter: i32,
    name: String,
    enabled: bool,
    items: Vec<String>,
}

// ============================================================================
// Handlers - Each modifies a specific field
// ============================================================================

#[handler]
fn increment(state: &mut AppState) {
    state.counter += 1;
    println!("  [handler] increment: counter = {}", state.counter);
}

#[handler]
fn decrement(state: &mut AppState) {
    state.counter -= 1;
    println!("  [handler] decrement: counter = {}", state.counter);
}

#[handler]
fn set_name_alice(state: &mut AppState) {
    state.name = "Alice".to_string();
    println!("  [handler] set_name_alice: name = {}", state.name);
}

#[handler]
fn set_name_bob(state: &mut AppState) {
    state.name = "Bob".to_string();
    println!("  [handler] set_name_bob: name = {}", state.name);
}

#[handler]
fn toggle_enabled(state: &mut AppState) {
    state.enabled = !state.enabled;
    println!("  [handler] toggle_enabled: enabled = {}", state.enabled);
}

#[handler]
fn add_item(state: &mut AppState) {
    let n = state.items.len() + 1;
    state.items.push(format!("Item {}", n));
    println!("  [handler] add_item: items = {:?}", state.items);
}

#[handler]
fn remove_item(state: &mut AppState) {
    state.items.pop();
    println!("  [handler] remove_item: items = {:?}", state.items);
}

// Handler that modifies multiple fields
#[handler]
fn reset_all(state: &mut AppState) {
    state.counter = 0;
    state.name = String::new();
    state.enabled = false;
    state.items.clear();
    println!("  [handler] reset_all: all fields reset");
}

// ============================================================================
// Renderers - Each depends on specific field(s)
// ============================================================================

/// Renders the counter - only re-renders when `counter` changes
#[renderer]
fn render_counter(state: &AppState) -> ElementBuilder {
    println!("  [render] render_counter called (depends on: counter)");
    el(El::Div).class("field").append([
        el(El::Span).class("label").text("Counter:"),
        el(El::Span).class("value").text(&state.counter.to_string()),
    ])
}

/// Renders the name - only re-renders when `name` changes
#[renderer]
fn render_name(state: &AppState) -> ElementBuilder {
    println!("  [render] render_name called (depends on: name)");
    let display = if state.name.is_empty() {
        "(not set)".to_string()
    } else {
        state.name.clone()
    };
    el(El::Div).class("field").append([
        el(El::Span).class("label").text("Name:"),
        el(El::Span).class("value").text(&display),
    ])
}

/// Renders the enabled status - only re-renders when `enabled` changes
#[renderer]
fn render_enabled(state: &AppState) -> ElementBuilder {
    println!("  [render] render_enabled called (depends on: enabled)");
    let status = if state.enabled { "ON" } else { "OFF" };
    el(El::Div).class("field").append([
        el(El::Span).class("label").text("Enabled:"),
        el(El::Span).class("value").text(status),
    ])
}

/// Renders the items list - only re-renders when `items` changes
#[renderer]
fn render_items(state: &AppState) -> ElementBuilder {
    println!("  [render] render_items called (depends on: items)");
    let items_text = if state.items.is_empty() {
        "(empty)".to_string()
    } else {
        state.items.join(", ")
    };
    el(El::Div).class("field").append([
        el(El::Span).class("label").text("Items:"),
        el(El::Span).class("value").text(&items_text),
    ])
}

/// Renders a summary using multiple fields - re-renders when ANY of them change
#[renderer]
fn render_summary(state: &AppState) -> ElementBuilder {
    println!("  [render] render_summary called (depends on: counter, name, enabled, items)");
    let summary = format!(
        "Summary: counter={}, name={}, enabled={}, items={}",
        state.counter,
        if state.name.is_empty() {
            "?"
        } else {
            &state.name
        },
        state.enabled,
        state.items.len()
    );
    el(El::Div).class("summary").text(&summary)
}

// ============================================================================
// UI Layout
// ============================================================================

fn build_app() -> ElementBuilder {
    el(El::Div).class("app").append([
        el(El::H1).text("Fine-Grained Reactivity Demo"),
        el(El::P).class("intro").text(
            "Each renderer only updates when its specific fields change. \
             Watch the server console to see which renderers are called!",
        ),
        // Counter section
        el(El::Div).class("section").append([
            el(El::H2).text("Counter (field: counter)"),
            el(El::Div).class("controls").append([
                el(El::Button).text("-").on(Ev::Click, decrement()),
                el(El::Button).text("+").on(Ev::Click, increment()),
            ]),
            render_counter(),
        ]),
        // Name section
        el(El::Div).class("section").append([
            el(El::H2).text("Name (field: name)"),
            el(El::Div).class("controls").append([
                el(El::Button)
                    .text("Set Alice")
                    .on(Ev::Click, set_name_alice()),
                el(El::Button).text("Set Bob").on(Ev::Click, set_name_bob()),
            ]),
            render_name(),
        ]),
        // Enabled section
        el(El::Div).class("section").append([
            el(El::H2).text("Enabled (field: enabled)"),
            el(El::Div).class("controls").append([el(El::Button)
                .text("Toggle")
                .on(Ev::Click, toggle_enabled())]),
            render_enabled(),
        ]),
        // Items section
        el(El::Div).class("section").append([
            el(El::H2).text("Items (field: items)"),
            el(El::Div).class("controls").append([
                el(El::Button).text("Add").on(Ev::Click, add_item()),
                el(El::Button).text("Remove").on(Ev::Click, remove_item()),
            ]),
            render_items(),
        ]),
        // Summary section (depends on all fields)
        el(El::Div).class("section").append([
            el(El::H2).text("Summary (all fields)"),
            el(El::P)
                .class("note")
                .text("This renderer depends on ALL fields, so it updates on any change."),
            render_summary(),
        ]),
        // Reset button
        el(El::Div).class("section").append([
            el(El::H2).text("Reset"),
            el(El::Button)
                .class("reset")
                .text("Reset All")
                .on(Ev::Click, reset_all()),
        ]),
    ])
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Fine-Grained Reactivity Demo");
    println!("============================");
    println!();
    println!("This demo shows how rwire's compile-time dependency tracking works.");
    println!("Each renderer only re-renders when its dependent fields change.");
    println!();
    println!("Open http://127.0.0.1:9000 and watch this console!");
    println!();
    println!("Expected behavior:");
    println!("  - Click '+'/'-'      -> only render_counter + render_summary");
    println!("  - Click 'Set Alice'  -> only render_name + render_summary");
    println!("  - Click 'Toggle'     -> only render_enabled + render_summary");
    println!("  - Click 'Add'        -> only render_items + render_summary");
    println!("  - Click 'Reset All'  -> ALL renderers (modifies all fields)");
    println!();

    Server::bind("127.0.0.1:9000")?.root(build_app).run().await
}
