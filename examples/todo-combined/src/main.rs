//! Todo Combined Example - Full TodoMVC with EventContext and ItemRef support.
//!
//! This example demonstrates all three storage types with full TodoMVC functionality:
//! 1. **Local Todo** - Instant toggles without server round-trip (UI state)
//! 2. **Memory Todo** - Server-side state with text input (session state)
//! 3. **Persisted Todo** - File-backed, survives refresh (data state)
//!
//! Key features demonstrated:
//! - Text input capture via EventContext
//! - Item-specific actions using ItemRef and iter_with_ref() (new API)
//! - Type-safe event binding with on_ref()
//!
//! Run with: `cargo run -p todo-combined`
//! Open: http://127.0.0.1:9000

// EventContext is used in handler signatures (macro-transformed, so rustc doesn't see it)
#[allow(unused_imports)]
use rwire::{
    el, handler, renderer, El, ElementBuilder, Ev, EventContext, IterWithRef, Server, State,
};
use serde::{Deserialize, Serialize};

// ============================================================================
// Local State - Client-side, instant response
// ============================================================================

/// Local state for UI interactions - toggle without server round-trip.
///
/// NOTE: Local state handlers require #[handler(local)] attribute and
/// can only use simple mutations (toggle, add, set).
#[derive(State, Default)]
#[storage(local)]
struct LocalUiState {
    item1_done: bool,
    item2_done: bool,
    item3_done: bool,
    show_completed: bool,
}

// Local handlers use #[handler(local)] and compile to client-side mutations.
// These handlers execute entirely in the browser without network round-trips.

#[handler(local)]
fn toggle_local_1(state: &mut LocalUiState) {
    state.item1_done = !state.item1_done;
}

#[handler(local)]
fn toggle_local_2(state: &mut LocalUiState) {
    state.item2_done = !state.item2_done;
}

#[handler(local)]
fn toggle_local_3(state: &mut LocalUiState) {
    state.item3_done = !state.item3_done;
}

#[handler(local)]
fn toggle_show_completed(state: &mut LocalUiState) {
    state.show_completed = !state.show_completed;
}

// ============================================================================
// Memory State - Server-side, per-session with text input support
// ============================================================================

/// Memory state for session-scoped todo items.
///
/// This state lives on the server and is lost when the connection closes.
/// Demonstrates text input capture and item-specific actions using ItemRef.
#[derive(State, Default)]
#[storage(memory)]
struct MemoryTodoState {
    items: Vec<MemoryTodoItem>,
    input_value: String,
}

/// A todo item - no ID needed since ItemRef tracks items by index.
#[derive(Clone, Default)]
struct MemoryTodoItem {
    text: String,
    done: bool,
}

/// Handler with EventContext - captures text input value
#[handler]
fn update_memory_input(state: &mut MemoryTodoState, ctx: &EventContext) {
    if let Some(text) = ctx.text() {
        state.input_value = text.to_string();
    }
}

/// Handler with EventContext - adds todo from input
#[handler]
fn add_memory_item(state: &mut MemoryTodoState, ctx: &EventContext) {
    // Try to get text from form field or direct input
    let text = ctx
        .field("todo")
        .or_else(|| ctx.text())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            // Fallback to stored input value
            if !state.input_value.is_empty() {
                Some(std::mem::take(&mut state.input_value))
            } else {
                None
            }
        });

    if let Some(text) = text {
        // No ID needed - ItemRef tracks items by their index in the Vec
        state.items.push(MemoryTodoItem { text, done: false });
        state.input_value.clear();
    }
}

/// Handler with ItemRef - toggles specific item by index
/// This is the new cleaner API using on_ref() and ItemRef
#[handler]
fn toggle_memory_item(state: &mut MemoryTodoState, ctx: &EventContext) {
    if let Some(item_ref) = ctx.item_ref::<MemoryTodoItem>() {
        if let Some(item) = item_ref.get_mut(&mut state.items) {
            item.done = !item.done;
        }
    }
}

/// Handler with ItemRef - deletes specific item by index
#[handler]
fn delete_memory_item(state: &mut MemoryTodoState, ctx: &EventContext) {
    if let Some(idx) = ctx.item_index() {
        if idx < state.items.len() {
            state.items.remove(idx);
        }
    }
}

#[handler]
fn clear_memory_completed(state: &mut MemoryTodoState) {
    state.items.retain(|item| !item.done);
}

// ============================================================================
// Persisted State - SQLite-backed, survives restart
// ============================================================================

/// Persisted state for durable todo items.
///
/// This state demonstrates the persistence architecture:
/// - Memory-first: State lives in SharedServerState.shared_cache for instant access
/// - Background persistence: Dirty keys are persisted asynchronously via persist_task
/// - Normalized schema: Vec<T> fields become child tables
///
/// Full persistence requires registering a PersistableType with SqliteStore.
/// See rwire/src/persist.rs for the registration API.
///
/// Currently using memory storage for demonstration. To enable full persistence:
/// 1. Change #[storage(memory)] to #[storage(persisted, table = "todos")]
/// 2. Add #[key] attribute to a session_id field
/// 3. Register the type with SqliteStore in main()
#[derive(State, Default, Clone, Serialize, Deserialize)]
#[storage(memory)] // TODO: Change to #[storage(persisted, table = "todos")]
struct PersistedTodoState {
    // TODO: Add #[key] session_id: String,
    items: Vec<PersistedTodoItem>,
}

/// A persisted todo item - no ID needed since ItemRef tracks items by index.
#[derive(Clone, Default, Serialize, Deserialize)]
struct PersistedTodoItem {
    text: String,
    done: bool,
}

/// Handler with EventContext - adds todo from form submission
#[handler]
fn add_persisted_item(state: &mut PersistedTodoState, ctx: &EventContext) {
    let text = ctx
        .field("todo")
        .or_else(|| ctx.text())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    if let Some(text) = text {
        // No ID needed - ItemRef tracks items by their index in the Vec
        state.items.push(PersistedTodoItem {
            text: text.to_string(),
            done: false,
        });
    }
}

/// Handler with ItemRef - toggles specific item by index
#[handler]
fn toggle_persisted_item(state: &mut PersistedTodoState, ctx: &EventContext) {
    if let Some(item_ref) = ctx.item_ref::<PersistedTodoItem>() {
        if let Some(item) = item_ref.get_mut(&mut state.items) {
            item.done = !item.done;
        }
    }
}

/// Handler with ItemRef - deletes specific item by index
#[handler]
fn delete_persisted_item(state: &mut PersistedTodoState, ctx: &EventContext) {
    if let Some(idx) = ctx.item_index() {
        if idx < state.items.len() {
            state.items.remove(idx);
        }
    }
}

#[handler]
fn clear_persisted_completed(state: &mut PersistedTodoState) {
    state.items.retain(|item| !item.done);
}

// ============================================================================
// UI Components
// ============================================================================

fn build_app() -> ElementBuilder {
    el(El::Div).class("app").append([
        el(El::H1).text("rwire Multi-State TodoMVC"),
        el(El::P)
            .class("subtitle")
            .text("Three todo lists demonstrating different storage strategies with EventContext"),
        el(El::Div).class("columns").append([
            build_local_column(),
            build_memory_column(),
            build_persisted_column(),
        ]),
    ])
}

fn build_local_column() -> ElementBuilder {
    el(El::Div).class("column local").append([
        el(El::H2).text("Local State"),
        el(El::P)
            .class("description")
            .text("Instant - no server round-trip"),
        el(El::Div).class("controls").append([
            el(El::Button)
                .text("Toggle #1")
                .class("btn")
                .on(Ev::Click, toggle_local_1()),
            el(El::Button)
                .text("Toggle #2")
                .class("btn")
                .on(Ev::Click, toggle_local_2()),
            el(El::Button)
                .text("Toggle #3")
                .class("btn")
                .on(Ev::Click, toggle_local_3()),
            el(El::Button)
                .text("Toggle Show Done")
                .class("btn secondary")
                .on(Ev::Click, toggle_show_completed()),
        ]),
        render_local_items(),
    ])
}

fn build_memory_column() -> ElementBuilder {
    el(El::Div).class("column memory").append([
        el(El::H2).text("Memory State"),
        el(El::P)
            .class("description")
            .text("Server-side with text input, lost on disconnect"),
        // Input form for adding items
        el(El::Div).class("input-row").append([
            el(El::Input)
                .attr("type", "text")
                .attr("placeholder", "What needs to be done?")
                .attr("name", "todo")
                .class("todo-input")
                .on(Ev::Input, update_memory_input()),
            el(El::Button)
                .text("Add")
                .class("btn primary")
                .on(Ev::Click, add_memory_item()),
        ]),
        el(El::Div).class("controls").append([el(El::Button)
            .text("Clear Done")
            .class("btn secondary")
            .on(Ev::Click, clear_memory_completed())]),
        render_memory_items(),
    ])
}

fn build_persisted_column() -> ElementBuilder {
    el(El::Div).class("column persisted").append([
        el(El::H2).text("Persisted State"),
        el(El::P)
            .class("description")
            .text("Memory-first with SQLite background sync"),
        // Input form for adding items
        el(El::Form)
            .class("input-row")
            .on(Ev::Submit, add_persisted_item())
            .append([
                el(El::Input)
                    .attr("type", "text")
                    .attr("placeholder", "What needs to be done?")
                    .attr("name", "todo")
                    .class("todo-input"),
                el(El::Button)
                    .attr("type", "submit")
                    .text("Add")
                    .class("btn primary"),
            ]),
        el(El::Div).class("controls").append([el(El::Button)
            .text("Clear Done")
            .class("btn secondary")
            .on(Ev::Click, clear_persisted_completed())]),
        render_persisted_items(),
    ])
}

// ============================================================================
// Renderers
// ============================================================================

// NOTE: The framework does NOT properly support nested renderers during updates!
// See docs/architecture-state.md for details.
//
// When a parent renderer re-renders, it clears its wrapper's children.
// If the parent contains a nested renderer, that nested wrapper is destroyed.
// The nested renderer's update then tries to find a non-existent element.
//
// WORKAROUND: Keep renderers flat (no nested renderers).
// Split logic into separate renderers that are siblings, not nested.

#[renderer]
fn render_local_items(state: &LocalUiState) -> ElementBuilder {
    let mut items = Vec::new();

    // Fixed items with toggleable state
    let item1 = ("Buy groceries", state.item1_done);
    let item2 = ("Clean house", state.item2_done);
    let item3 = ("Read book", state.item3_done);

    for (text, done) in [item1, item2, item3] {
        if !done || state.show_completed {
            let mark = if done { "[x]" } else { "[ ]" };
            items.push(format!("{} {}", mark, text));
        }
    }

    let status = if state.show_completed {
        "showing all"
    } else {
        "hiding completed"
    };

    let display = if items.is_empty() {
        format!("(all done) - {}", status)
    } else {
        format!("{} | {}", items.join(" | "), status)
    };

    el(El::Span).class("item-list").text(&display)
}

// Memory todo - split into 3 renderers (items list, count, empty state)
// These are SIBLINGS, not nested, to avoid the nested renderer bug.

#[renderer]
fn render_memory_items(_state: &MemoryTodoState) -> ElementBuilder {
    // Container with items list and footer as siblings
    el(El::Div)
        .class("todo-list")
        .append([render_memory_items_list(), render_memory_count()])
}

#[renderer]
fn render_memory_items_list(state: &MemoryTodoState) -> ElementBuilder {
    if state.items.is_empty() {
        return el(El::Div)
            .class("empty")
            .text("No items - type above and click Add");
    }

    // Use iter_with_ref() for type-safe item references
    let items: Vec<ElementBuilder> = state
        .items
        .iter_with_ref()
        .map(|(item_ref, item)| {
            let class = if item.done { "item done" } else { "item" };
            el(El::Div).class(class).append([
                el(El::Span)
                    .class("checkbox")
                    .text(if item.done { "[x]" } else { "[ ]" })
                    .on_ref(Ev::Click, toggle_memory_item(), item_ref),
                el(El::Span).class("text").text(&item.text),
                el(El::Button).class("delete").text("x").on_ref(
                    Ev::Click,
                    delete_memory_item(),
                    item_ref,
                ),
            ])
        })
        .collect();

    el(El::Div).class("items").append(items)
}

#[renderer]
fn render_memory_count(state: &MemoryTodoState) -> ElementBuilder {
    let count = state.items.iter().filter(|i| !i.done).count();
    let count_text = format!("{} item{} left", count, if count == 1 { "" } else { "s" });
    el(El::Div).class("footer").text(&count_text)
}

// Persisted todo - split into 3 renderers (same pattern)

#[renderer]
fn render_persisted_items(_state: &PersistedTodoState) -> ElementBuilder {
    el(El::Div)
        .class("todo-list")
        .append([render_persisted_items_list(), render_persisted_count()])
}

#[renderer]
fn render_persisted_items_list(state: &PersistedTodoState) -> ElementBuilder {
    if state.items.is_empty() {
        return el(El::Div)
            .class("empty")
            .text("No items - type above and click Add");
    }

    // Use iter_with_ref() for type-safe item references
    let items: Vec<ElementBuilder> = state
        .items
        .iter_with_ref()
        .map(|(item_ref, item)| {
            let class = if item.done { "item done" } else { "item" };
            el(El::Div).class(class).append([
                el(El::Span)
                    .class("checkbox")
                    .text(if item.done { "[x]" } else { "[ ]" })
                    .on_ref(Ev::Click, toggle_persisted_item(), item_ref),
                el(El::Span).class("text").text(&item.text),
                el(El::Button).class("delete").text("x").on_ref(
                    Ev::Click,
                    delete_persisted_item(),
                    item_ref,
                ),
            ])
        })
        .collect();

    el(El::Div).class("items").append(items)
}

#[renderer]
fn render_persisted_count(state: &PersistedTodoState) -> ElementBuilder {
    let count = state.items.iter().filter(|i| !i.done).count();
    let count_text = format!("{} item{} left", count, if count == 1 { "" } else { "s" });
    el(El::Div).class("footer").text(&count_text)
}

// ============================================================================
// Main
// ============================================================================

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("rwire Server - Todo Combined Demo");
    println!("==================================");
    println!();
    println!("Demonstrating three storage types with EventContext and ItemRef:");
    println!("  - Local:     Client-side, instant UI response (fixed items)");
    println!("  - Memory:    Server-side with text input (session-scoped)");
    println!("  - Persisted: Memory-first with background SQLite persistence");
    println!();
    println!("Persistence architecture (for persisted state):");
    println!("  - Instant handler execution (memory-only mutation)");
    println!("  - Background persistence task (non-blocking DB writes)");
    println!("  - Cross-tab synchronization (broadcast to other connections)");
    println!("  - Graceful shutdown (flush dirty state before exit)");
    println!();
    println!("New features demonstrated:");
    println!("  - Text input capture via ctx.text()");
    println!("  - Form submission via ctx.field()");
    println!("  - ItemRef for type-safe item binding (no more data-id!)");
    println!("  - iter_with_ref() for clean iteration with references");
    println!("  - on_ref() for binding handlers with item context");
    println!();
    println!("Open http://127.0.0.1:9000 in your browser");
    println!();

    // To enable SQLite persistence:
    // 1. Create SqliteStore: let store = SqliteStore::new("./todo.db")?;
    // 2. Register PersistableType with store.register(...)
    // 3. Configure server: Server::bind("...")?.persist_interval(Duration::from_millis(100))
    // 4. Spawn persist_task in background
    // 5. Call flush_all_dirty() before shutdown

    Server::bind("127.0.0.1:9000")?.root(build_app).run().await
}
