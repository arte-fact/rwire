//! Todolist example - demonstrates local vs remote handlers.
//!
//! This example shows a todo list with:
//! 1. **Local handlers** - UI state mutations, no server round-trip
//! 2. **Remote handlers** - Server-side mutations with state sync
//!
//! Run with: `cargo run -p todolist`
//! Open: http://127.0.0.1:9000

use rwire::{el, handler, renderer, El, ElementBuilder, Ev, Server, State};

// ============================================================================
// Unified State - Single state type with all fields
// ============================================================================

/// Unified state containing both local UI state and todo items.
///
/// In a real app, you'd use #[storage(local)] for UI-only fields
/// and keep data fields on the server. This example demonstrates
/// the handler patterns.
#[derive(State, Default)]
#[storage(memory)]
struct TodoState {
    // UI state (would be local in production)
    show_completed: bool,
    filter_mode: i32, // 0=all, 1=active, 2=completed

    // Todo items (server state)
    items: Vec<TodoItem>,
    next_id: i32,
}

#[derive(Clone, Default)]
struct TodoItem {
    text: String,
    completed: bool,
}

// ============================================================================
// Handlers
// ============================================================================

// UI handlers (these would use #[handler(local)] in a full implementation)
#[handler]
fn toggle_show_completed(state: &mut TodoState) {
    state.show_completed = !state.show_completed;
}

#[handler]
fn set_filter_all(state: &mut TodoState) {
    state.filter_mode = 0;
}

#[handler]
fn set_filter_active(state: &mut TodoState) {
    state.filter_mode = 1;
}

#[handler]
fn set_filter_completed(state: &mut TodoState) {
    state.filter_mode = 2;
}

// Data handlers (always remote, modify server state)
#[handler]
fn add_item(state: &mut TodoState) {
    let item_num = state.next_id;
    state.next_id += 1;
    state.items.push(TodoItem {
        text: format!("Todo item #{}", item_num),
        completed: false,
    });
}

#[handler]
fn toggle_first(state: &mut TodoState) {
    if let Some(item) = state.items.first_mut() {
        item.completed = !item.completed;
    }
}

#[handler]
fn toggle_second(state: &mut TodoState) {
    if let Some(item) = state.items.get_mut(1) {
        item.completed = !item.completed;
    }
}

#[handler]
fn clear_completed(state: &mut TodoState) {
    state.items.retain(|item| !item.completed);
}

#[handler]
fn remove_first(state: &mut TodoState) {
    if !state.items.is_empty() {
        state.items.remove(0);
    }
}

// ============================================================================
// UI Components
// ============================================================================

fn build_app() -> ElementBuilder {
    el(El::Div).class("app").append([
        el(El::H1).text("rwire Todo List"),
        el(El::P).text("Demonstrating reactive state updates"),
        build_controls(),
        render_stats(),
        render_items(),
    ])
}

fn build_controls() -> ElementBuilder {
    el(El::Div).class("controls").append([
        el(El::Div).class("actions").append([
            el(El::Button).text("Add Item").on(Ev::Click, add_item()),
            el(El::Button)
                .text("Toggle First")
                .on(Ev::Click, toggle_first()),
            el(El::Button)
                .text("Toggle Second")
                .on(Ev::Click, toggle_second()),
            el(El::Button)
                .text("Remove First")
                .on(Ev::Click, remove_first()),
            el(El::Button)
                .text("Clear Completed")
                .on(Ev::Click, clear_completed()),
        ]),
        el(El::Div).class("filters").append([
            el(El::Button).text("All").on(Ev::Click, set_filter_all()),
            el(El::Button)
                .text("Active")
                .on(Ev::Click, set_filter_active()),
            el(El::Button)
                .text("Completed")
                .on(Ev::Click, set_filter_completed()),
            el(El::Button)
                .text("Toggle Show Completed")
                .on(Ev::Click, toggle_show_completed()),
        ]),
    ])
}

// Note: Renderers must return simple elements with text content for updates to work.
// The current build_synced_update only handles text content changes, not complex children.

#[renderer]
fn render_stats(state: &TodoState) -> ElementBuilder {
    let total = state.items.len();
    let completed = state.items.iter().filter(|i| i.completed).count();
    let active = total - completed;

    let filter_name = match state.filter_mode {
        0 => "all",
        1 => "active",
        2 => "completed",
        _ => "?",
    };

    let show_status = if state.show_completed { "show" } else { "hide" };

    // Simple text element for proper updates
    el(El::Span).text(&format!(
        "Items: {} total, {} active, {} done | Filter: {} | {}",
        total, active, completed, filter_name, show_status
    ))
}

#[renderer]
fn render_items(state: &TodoState) -> ElementBuilder {
    let filtered_items: Vec<_> = state
        .items
        .iter()
        .filter(|item| match state.filter_mode {
            1 => !item.completed,
            2 => item.completed,
            _ => state.show_completed || !item.completed,
        })
        .take(5)
        .collect();

    if filtered_items.is_empty() {
        return el(El::Span).text("(no items)");
    }

    // Build a simple text representation
    let items_text = filtered_items
        .iter()
        .map(|item| {
            let mark = if item.completed { "[x]" } else { "[ ]" };
            format!("{} {}", mark, item.text)
        })
        .collect::<Vec<_>>()
        .join(" | ");

    el(El::Span).text(&items_text)
}

// ============================================================================
// Main
// ============================================================================

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("rwire Server - Todo List Example");
    println!("=================================");
    println!();
    println!("Open http://127.0.0.1:9000 in your browser");
    println!();

    Server::bind("127.0.0.1:9000")?.root(build_app).run().await
}
